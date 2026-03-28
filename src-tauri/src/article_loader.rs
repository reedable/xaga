use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

pub type PendingRequests = Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn load_via_webview(
    app: &AppHandle,
    pending: &PendingRequests,
    url: &str,
) -> Result<String, String> {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let label = format!("loader-{}", id);

    let (tx, rx) = oneshot::channel::<String>();
    pending.lock().map_err(|e| e.to_string())?.insert(id.clone(), tx);

    let init_script = format!(r#"
        window.open = function() {{ return null; }};
        window.focus = function() {{}};
        window.addEventListener('load', function() {{
            setTimeout(function() {{
                try {{
                    window.__TAURI_INTERNALS__.invoke('receive_html', {{
                        id: '{}',
                        html: document.documentElement.outerHTML
                    }});
                }} catch(e) {{}}
            }}, 1500);
        }});
    "#, id);

    let parsed: tauri::Url = url.parse().map_err(|e: <tauri::Url as std::str::FromStr>::Err| e.to_string())?;

    let loader = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed))
        .visible(false)
        .position(-10000.0, -10000.0)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;

    // Prevent the native window from ever becoming visible regardless of what
    // the remote page does (e.g. WKWebView-level focus/show callbacks).
    let loader_handle = loader.clone();
    loader.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Focused(true)) {
            let _ = loader_handle.hide();
        }
    });

    let result = tokio::time::timeout(std::time::Duration::from_secs(20), rx).await;

    pending.lock().map_err(|e| e.to_string())?.remove(&id);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.close();
    }

    match result {
        Ok(Ok(html)) => Ok(html),
        Ok(Err(_)) => Err("channel closed before HTML was received".to_string()),
        Err(_) => Err("timed out waiting for page to load".to_string()),
    }
}
