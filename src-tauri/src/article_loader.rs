use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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

    let init_script = format!(
        r#"
        window.open = function() {{ return null; }};
        window.focus = function() {{}};
        window.addEventListener('load', function() {{
            var debounceTimer = null;
            var sent = false;
            function send() {{
                if (sent) return;
                sent = true;
                observer.disconnect();
                try {{
                    window.__TAURI_INTERNALS__.invoke('receive_html', {{
                        id: '{}',
                        html: document.documentElement.outerHTML
                    }});
                }} catch(e) {{}}
            }}
            var observer = new MutationObserver(function() {{
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(send, 300);
            }});
            observer.observe(document.documentElement, {{
                childList: true,
                subtree: true,
                characterData: true
            }});
            debounceTimer = setTimeout(send, 300);
        }});
    "#,
        id
    );

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
