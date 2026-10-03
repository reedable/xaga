use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use crate::article_diagnostics::{ArticleDiagnosticOutcome, ArticleLoadAttempt};

/// What one loader window captured. A struct rather than a tuple because the
/// loader is the only context permitted to fetch remote content: anything the
/// app must not fetch later has to travel back on this payload. The favicon
/// bytes ride here alongside the page HTML for exactly that reason.
#[derive(Debug, Clone, Default)]
pub struct LoaderCapture {
    pub mime_type: String,
    pub html: String,
    /// The URL the loader resolved the favicon from, if it found one.
    pub favicon: Option<String>,
    /// That favicon's bytes as a `data:` URI, if the fetch succeeded.
    pub favicon_data: Option<String>,
}

/// One in-flight loader request.
pub struct PendingRequest {
    /// Delivers the finished capture back to `load_via_webview`.
    pub tx: oneshot::Sender<LoaderCapture>,
    pub keep_open: bool,
    pub attempt: ArticleLoadAttempt,
    pub diagnostic_event_count: usize,
    /// Whether the inline-PDF reqwest probe errored — defers the close decision
    /// to `load_via_webview` so the window can be kept open and shown when the
    /// content turns out to be a PDF.
    pub reqwest_error: bool,
    /// Set while the window is being navigated to the favicon to read its bytes:
    /// the article HTML is already captured and waiting here for the favicon to
    /// arrive (or fail) before the capture is completed. `None` at every other
    /// moment.
    pub partial: Option<LoaderCapture>,
}

pub type PendingRequests = Arc<Mutex<HashMap<String, PendingRequest>>>;

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
fn find_matching_domain<'a>(host: &str, allowed: &'a [String]) -> Option<&'a str> {
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
    reqwest_error: bool,
    allowed_domains: &[String],
    named_stores_supported: bool,
    proxy: Arc<xaga_core::ssrf_proxy::ScopedProxy>,
    attempt: ArticleLoadAttempt,
) -> Result<LoaderCapture, String> {
    attempt.add_started_event("url_validation");
    let parsed: tauri::Url = url.parse().map_err(|e| format!("invalid URL: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        attempt.add_failed_event(
            "url_validation",
            "article:scheme",
            "only HTTP and HTTPS are accepted",
        );
        return Err("refusing to load non-web URL scheme".into());
    }
    attempt.add_succeeded_event("url_validation");
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let label = format!("loader-{}", id);

    let (tx, rx) = oneshot::channel::<LoaderCapture>();

    pending.lock().map_err(|e| e.to_string())?.insert(
        id.clone(),
        PendingRequest {
            tx,
            keep_open,
            attempt: attempt.clone(),
            diagnostic_event_count: 0,
            reqwest_error,
            partial: None,
        },
    );

    // Prepend window globals so the static init script can reference them.
    // serde_json::to_string produces valid JSON string literals, preventing
    // any injection via special characters in id, label, or url.
    let vars = format!(
        "window.__LOADER_ID__={id};window.__LOADER_LABEL__={label};window.__LOADER_URL__={url};window.__LOADER_KEEP_OPEN__={keep_open};window.__LOADER_TIMEOUT_MS__={timeout_ms};",
        id = serde_json::to_string(&id).unwrap_or_default(),
        label = serde_json::to_string(&label).unwrap_or_default(),
        url = serde_json::to_string(url).unwrap_or_default(),
        keep_open = keep_open,
        timeout_ms = timeout_secs.saturating_mul(1000),
    );

    let init_script = format!("{}\n{}", vars, include_str!("article_loader.js"));

    let host = parsed.host_str().unwrap_or("");
    let matched = find_matching_domain(host, allowed_domains);
    let identifier = matched
        .filter(|_| named_stores_supported)
        .map(domain_store_id);

    // WKWebViewConfiguration must be created on the main thread and hardened
    // BEFORE WebView construction starts the first navigation.
    let build_app = app.clone();
    let build_label = label.clone();
    let (built_tx, built_rx) = oneshot::channel();
    attempt.add_started_event("webview_create");
    let build_attempt = attempt.clone();
    let navigation_attempt = attempt.clone();
    let page_attempt = attempt.clone();
    let build_pending = pending.clone();
    let build_id = id.clone();
    let build_proxy = proxy.clone();
    let sessions = app
        .state::<crate::loader_identity::LoaderSessions>()
        .inner()
        .clone();
    let source_url = url.to_owned();
    let scheduled = app.run_on_main_thread(move || {
        // A timed-out scheduling request must not create an orphan webview.
        if !build_pending
            .lock()
            .map(|p| p.contains_key(&build_id))
            .unwrap_or(false)
        {
            return;
        }
        let result = (|| {
            let mut builder =
                WebviewWindowBuilder::new(&build_app, &build_label, WebviewUrl::External(parsed))
                    .initialization_script(&init_script)
                    .on_navigation(move |_| {
                        navigation_attempt.add_event(
                            "native",
                            "navigation",
                            ArticleDiagnosticOutcome::Started,
                            "",
                            "navigation requested; not a verified redirect hop",
                        );
                        true
                    })
                    .on_page_load(move |_, payload| {
                        page_attempt.add_event(
                            "native",
                            "navigation",
                            match payload.event() {
                                tauri::webview::PageLoadEvent::Started => {
                                    ArticleDiagnosticOutcome::Started
                                }
                                tauri::webview::PageLoadEvent::Finished => {
                                    ArticleDiagnosticOutcome::Succeeded
                                }
                            },
                            "",
                            &format!("{:?}", payload.event()),
                        );
                    })
                    // Do not let remote content create unmanaged browser windows.
                    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);

            let proxy = xaga_core::ssrf_proxy::proxy_url(build_proxy.port)
                .parse::<tauri::Url>()
                .map_err(|e| e.to_string())?;
            builder = builder.proxy_url(proxy);

            #[cfg(target_os = "macos")]
            {
                build_attempt.add_started_event("webview_policy");
                builder = builder.with_webview_configuration(
                    crate::webview_policy::loader_configuration(identifier)?,
                );
                build_attempt.add_succeeded_event("webview_policy");
            }
            #[cfg(not(target_os = "macos"))]
            {
                builder = match identifier {
                    Some(id) => builder.data_store_identifier(id),
                    None => builder.incognito(true),
                };
            }

            builder = if keep_open {
                builder
                    .visible(true)
                    .inner_size(1200.0, 900.0)
                    .position(100.0, 100.0)
            } else {
                builder.visible(false).position(-10000.0, -10000.0)
            };
            build_attempt.add_started_event("webview_create");
            let window = builder.build().map_err(|e| e.to_string())?;
            sessions.register(build_label.clone(), source_url);
            // Keep the dedicated proxy alive for a visible browser, including
            // the PDF fallback. Release it when the native window is destroyed.
            let lifetime = Arc::new(Mutex::new(Some(build_proxy)));
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    lifetime.lock().unwrap_or_else(|e| e.into_inner()).take();
                    sessions.remove(&build_label);
                }
            });
            Ok(window)
        })();
        if let Err(Ok(window)) = built_tx.send(result) {
            // The deadline may expire while native construction is running.
            let _ = window.destroy();
        }
    });
    let built = match scheduled {
        Ok(()) => match tokio::time::timeout_at(deadline, built_rx).await {
            Ok(result) => result.map_err(|e| e.to_string()).and_then(|r| r),
            Err(_) => Err("timed out creating loader webview".into()),
        },
        Err(error) => Err(error.to_string()),
    };
    let loader = match built {
        Ok(loader) => loader,
        Err(error) => {
            pending.lock().map_err(|e| e.to_string())?.remove(&id);
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.destroy();
            }
            attempt.add_failed_event(&attempt.get_last_step(), "article:webview-create", &error);
            return Err(error);
        }
    };

    attempt.add_succeeded_event("webview_create");
    attempt.add_started_event("capture_wait");

    // Set when an inline-PDF reqwest error is followed by the loader detecting a
    // PDF: the hidden window is then shown to the user (only the webview can fetch
    // it), so it must not be hidden-on-focus or closed.
    let keep_open_on_error = Arc::new(AtomicBool::new(false));

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
        // the remote page does (e.g. WKWebView-level focus/show callbacks) — unless
        // we deliberately show it for the reqwest-error PDF case.
        let loader_handle = loader.clone();
        let suppress_hide = Arc::clone(&keep_open_on_error);

        loader.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Focused(true))
                && !suppress_hide.load(Ordering::Acquire)
            {
                let _ = loader_handle.hide();
            }
        });
    }

    // Keep the receiver alive after the deadline so a best-effort snapshot
    // can still complete the normal extraction pipeline.
    let mut rx = rx;
    let mut result = tokio::time::timeout_at(deadline, &mut rx).await;
    if result.is_err() {
        let has_partial = pending
            .lock()
            .map_err(|e| e.to_string())?
            .get(&id)
            .is_some_and(|request| request.partial.is_some());
        if !has_partial {
            attempt.add_failed_event(
                "capture_wait",
                "article:timeout",
                "capture deadline expired; attempting immediate snapshot of available content",
            );
            if loader.eval("window.__xagaCaptureOnTimeout()").is_ok() {
                result = tokio::time::timeout(std::time::Duration::from_secs(1), &mut rx).await;
            }
        }
    }

    let request = pending.lock().map_err(|e| e.to_string())?.remove(&id);
    if result.is_err() {
        if let Some(capture) = recover_favicon_timeout(request, &attempt) {
            result = Ok(Ok(capture));
        }
    }

    // If the reqwest probe errored and the loader resolved the content as a PDF,
    // show the existing (hidden) window to the user instead of closing it: the
    // real browser got past whatever refused reqwest, and a PDF can only be
    // presented this way.
    let show_pdf = reqwest_error
        && matches!(&result, Ok(Ok(capture)) if capture.mime_type == "application/pdf");

    if show_pdf {
        keep_open_on_error.store(true, Ordering::Release);
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.set_size(tauri::LogicalSize::new(1200.0, 900.0));
            let _ = w.set_position(tauri::LogicalPosition::new(100.0, 100.0));
            let _ = w.show();
            let _ = w.set_focus();
        }
    } else if !keep_open {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.close();
        }
    }

    match result {
        Ok(Ok(capture)) => {
            attempt.add_succeeded_event("capture");
            Ok(capture)
        }
        Ok(Err(_)) => {
            attempt.add_failed_event(
                "capture_delivery",
                "article:channel-closed",
                "channel closed before HTML was received",
            );
            Err("channel closed before HTML was received".into())
        }
        Err(_) => {
            let step = attempt.get_last_step();
            attempt.add_failed_event("capture_wait", "article:timeout", &format!("deadline expired waiting for HTML delivery; last observed milestone={step}; underlying browser cause unknown"));
            Err(format!(
                "timed out waiting for HTML delivery; last observed milestone={step}; underlying browser cause unknown"
            ))
        }
    }
}

fn recover_favicon_timeout(
    request: Option<PendingRequest>,
    attempt: &ArticleLoadAttempt,
) -> Option<LoaderCapture> {
    let capture = request?.partial?;
    attempt.add_event(
        "native",
        "favicon",
        ArticleDiagnosticOutcome::Warning,
        "article:timeout",
        "article HTML received; favicon did not finish before deadline",
    );
    Some(capture)
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use crate::article_diagnostics::ArticleDiagnostics;
    #[test]
    fn favicon_timeout_preserves_captured_article() {
        let attempt = ArticleDiagnostics::default().create_attempt("load");
        let (tx, _rx) = oneshot::channel();
        let request = PendingRequest {
            tx,
            keep_open: false,
            attempt: attempt.clone(),
            diagnostic_event_count: 0,
            reqwest_error: false,
            partial: Some(LoaderCapture {
                html: "original article".into(),
                mime_type: "text/html".into(),
                ..Default::default()
            }),
        };
        let recovered = recover_favicon_timeout(Some(request), &attempt).unwrap();
        assert_eq!(recovered.html, "original article");
        assert!(!attempt.is_failure_recorded());
        let report = serde_json::to_value(attempt.get_diagnostic()).unwrap();
        assert_eq!(report["events"][0]["outcome"], "warning");
        assert_eq!(report["events"][0]["step"], "favicon");
        assert!(recover_favicon_timeout(None, &attempt).is_none());
    }
}
