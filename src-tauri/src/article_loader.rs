use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

pub type PendingRequests = Arc<Mutex<HashMap<String, (oneshot::Sender<String>, bool)>>>;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn load_via_webview(
    app: &AppHandle,
    pending: &PendingRequests,
    url: &str,
    timeout_secs: u64,
    keep_open: bool,
) -> Result<String, String> {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let label = format!("loader-{}", id);

    let (tx, rx) = oneshot::channel::<String>();

    pending
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id.clone(), (tx, keep_open));

    // Prepend window globals so the static init script can reference them.
    // serde_json::to_string produces valid JSON string literals, preventing
    // any injection via special characters in id, label, or url.
    let vars = format!(
        "window.__LOADER_ID__={id};window.__LOADER_LABEL__={label};window.__LOADER_URL__={url};",
        id = serde_json::to_string(&id).unwrap_or_default(),
        label = serde_json::to_string(&label).unwrap_or_default(),
        url = serde_json::to_string(url).unwrap_or_default(),
    );

    let init_script = format!("{}\n{}", vars, include_str!("article_loader.js"));

    let parsed: tauri::Url = url
        .parse()
        .map_err(|e: <tauri::Url as std::str::FromStr>::Err| e.to_string())?;

    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed))
        .initialization_script(&init_script);

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
        Ok(Ok(html)) => Ok(html),
        Ok(Err(_)) => Err("channel closed before HTML was received".to_string()),
        Err(_) => Err("timed out waiting for page to load".to_string()),
    }
}
