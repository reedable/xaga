use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

pub type PendingRequests = Arc<Mutex<HashMap<String, (oneshot::Sender<(String, String)>, bool)>>>;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns true when WKWebsiteDataStore named identifiers are available.
///
/// Named data stores require macOS 14+, which is the app's minimum supported
/// version (`bundle.macOS.minimumSystemVersion`), so on macOS this is always
/// true; other platforms don't support them. This is now a compile-time
/// constant — no runtime version probe needed.
pub fn supports_named_data_stores() -> bool {
    cfg!(target_os = "macos")
}

/// Deterministically maps a registered domain to a 16-byte WKWebsiteDataStore identifier.
/// The versioned prefix ensures the namespace is unique to Xaga.
fn domain_store_id(domain: &str) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"xaga-domain-store-v1:");
    h.update(domain.as_bytes());
    h.finalize()[..16].try_into().unwrap()
}

/// Returns the allow-listed domain entry that matches `host`, or None.
/// An entry of "nytimes.com" matches both "nytimes.com" and "www.nytimes.com".
fn matching_domain<'a>(host: &str, allowed: &'a [String]) -> Option<&'a str> {
    allowed.iter().find_map(|domain| {
        if host == domain.as_str() || host.ends_with(&format!(".{domain}")) {
            Some(domain.as_str())
        } else {
            None
        }
    })
}

pub async fn load_via_webview(
    app: &AppHandle,
    pending: &PendingRequests,
    url: &str,
    timeout_secs: u64,
    keep_open: bool,
    allowed_domains: &[String],
    named_stores_supported: bool,
    ssrf_proxy_port: u16,
) -> Result<(String, String), String> {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let label = format!("loader-{}", id);

    let (tx, rx) = oneshot::channel::<(String, String)>();

    pending
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id.clone(), (tx, keep_open));

    // Prepend window globals so the static init script can reference them.
    // serde_json::to_string produces valid JSON string literals, preventing
    // any injection via special characters in id, label, or url.
    let vars = format!(
        "window.__LOADER_ID__={id};window.__LOADER_LABEL__={label};window.__LOADER_URL__={url};window.__LOADER_KEEP_OPEN__={keep_open};",
        id = serde_json::to_string(&id).unwrap_or_default(),
        label = serde_json::to_string(&label).unwrap_or_default(),
        url = serde_json::to_string(url).unwrap_or_default(),
        keep_open = keep_open,
    );

    let init_script = format!("{}\n{}", vars, include_str!("article_loader.js"));

    let parsed: tauri::Url = url
        .parse()
        .map_err(|e: <tauri::Url as std::str::FromStr>::Err| e.to_string())?;

    // Only ever drive the loader webview to real remote web content. Without this
    // guard a crafted feed/article link (file://, data:, custom schemes, …) would
    // be loaded and its DOM shipped back by the init script, enabling local-file
    // and intranet disclosure. Clean up the pending entry on rejection.
    if !matches!(parsed.scheme(), "http" | "https") {
        pending.lock().map_err(|e| e.to_string())?.remove(&id);
        return Err(format!(
            "refusing to load non-web URL scheme: {}",
            parsed.scheme()
        ));
    }

    let host = parsed.host_str().unwrap_or("");
    let matched = matching_domain(host, allowed_domains);

    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed))
        .initialization_script(&init_script);

    // Route all loader traffic through the local SSRF-filtering proxy: the
    // webview never resolves DNS or connects directly, so it cannot reach
    // private/loopback/intranet hosts and DNS rebinding / redirect-based SSRF
    // are defeated. Requires the `macos-proxy` feature to take effect on macOS.
    if let Ok(proxy) = format!("http://127.0.0.1:{ssrf_proxy_port}").parse::<tauri::Url>() {
        builder = builder.proxy_url(proxy);
    }

    // Default: incognito (ephemeral — no cookies written to disk).
    // Allow-listed domains on macOS get a persistent, per-domain named store.
    builder = match matched {
        Some(domain) if named_stores_supported => {
            builder.data_store_identifier(domain_store_id(domain))
        }
        _ => builder.incognito(true),
    };

    builder = if keep_open {
        builder
            .visible(true)
            .inner_size(1200.0, 900.0)
            .position(100.0, 100.0)
    } else {
        builder.visible(false).position(-10000.0, -10000.0)
    };

    let loader = builder.build().map_err(|e| e.to_string())?;

    if keep_open {
        let _ = loader.show();
        let _ = loader.set_focus();
        // `captured` flips to true after the first close attempt so the second
        // CloseRequested (from w.close() in capture_and_close_loader) passes through.
        let captured = Arc::new(AtomicBool::new(false));
        let close_handle = loader.clone();

        loader.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if captured.swap(true, Ordering::SeqCst) {
                    return;
                }
                api.prevent_close();
                close_handle.eval("window.__captureAndClose()").ok();
            }
        });
    } else {
        // Prevent the native window from ever becoming visible regardless of what
        // the remote page does (e.g. WKWebView-level focus/show callbacks).
        let loader_handle = loader.clone();

        loader.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                let _ = loader_handle.hide();
            }
        });
    }

    let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await;

    pending.lock().map_err(|e| e.to_string())?.remove(&id);

    if !keep_open {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.close();
        }
    }

    match result {
        Ok(Ok((mime_type, html))) => Ok((mime_type, html)),
        Ok(Err(_)) => Err("channel closed before HTML was received".to_string()),
        Err(_) => Err("timed out waiting for page to load".to_string()),
    }
}
