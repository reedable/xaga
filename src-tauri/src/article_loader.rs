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
    pub pdf_bytes: Option<Vec<u8>>,
    pub image_data: Option<String>,
    pub download_error: Option<String>,
    pub mime_type: String,
    pub html: String,
    /// The URL the loader resolved the favicon from, if it found one.
    pub favicon: Option<String>,
    /// That favicon's bytes as a `data:` URI, if the fetch succeeded.
    pub favicon_data: Option<String>,
}

/// Native purpose chooses the consumer, independently of page-reported MIME.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderPurpose {
    Article,
    Image,
}

/// One in-flight loader request.
pub struct PendingRequest {
    /// Delivers the finished capture back to `load_via_webview`.
    pub tx: oneshot::Sender<LoaderCapture>,
    pub keep_open: bool,
    pub purpose: LoaderPurpose,
    pub attempt: ArticleLoadAttempt,
    pub diagnostic_event_count: usize,
    pub download_active: bool,
    /// Set while the window is being navigated to the favicon to read its bytes:
    /// the article HTML is already captured and waiting here for the favicon to
    /// arrive (or fail) before the capture is completed. `None` at every other
    /// moment.
    pub partial: Option<LoaderCapture>,
}

pub type PendingRequests = Arc<Mutex<HashMap<String, PendingRequest>>>;

// A completed HTML capture removes its pending request while a visible loader
// stays open. That absence must still allow a fresh capture when the user closes it.
fn capture_on_close(request: Option<&PendingRequest>, closing: &AtomicBool) -> bool {
    !request.is_some_and(|request| request.download_active) && !closing.swap(true, Ordering::SeqCst)
}

#[cfg(test)]
#[path = "article_loader_close_tests.rs"]
mod close_tests;

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
    purpose: LoaderPurpose,
    load_remote_image: bool,
    allowed_domains: &[String],
    named_stores_supported: bool,
    proxy: Arc<xaga_core::network_proxy::ScopedProxy>,
    attempt: ArticleLoadAttempt,
) -> Result<LoaderCapture, String> {
    if attempt.is_cancelled() {
        return Err("load cancelled".into());
    }
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
    if purpose == LoaderPurpose::Article {
        attempt.add_event(
            "native", "image_capture",
            if load_remote_image { ArticleDiagnosticOutcome::Started } else { ArticleDiagnosticOutcome::Skipped },
            if load_remote_image { "image:enabled" } else { "image:disabled" },
            if load_remote_image { "additional article image capture enabled; awaiting page readiness" }
            else { "additional article image capture disabled by preference" },
        );
    }

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let label = format!("loader-{}", id);

    let (tx, rx) = oneshot::channel::<LoaderCapture>();

    pending.lock().map_err(|e| e.to_string())?.insert(
        id.clone(),
        PendingRequest {
            tx,
            keep_open,
            purpose,
            attempt: attempt.clone(),
            diagnostic_event_count: 0,
            download_active: false,
            partial: None,
        },
    );

    // Prepend window globals so the static init script can reference them.
    // serde_json::to_string produces valid JSON string literals, preventing
    // any injection via special characters in id, label, or url.
    let vars = format!(
        "window.__LOADER_ID__={id};window.__LOADER_LABEL__={label};window.__LOADER_URL__={url};window.__LOADER_KEEP_OPEN__={keep_open};window.__LOADER_TIMEOUT_MS__={timeout_ms};window.__LOADER_IMAGE_ONLY__={image_only};window.__LOAD_REMOTE_IMAGE__={load_remote_image};",
        id = serde_json::to_string(&id).unwrap_or_default(),
        label = serde_json::to_string(&label).unwrap_or_default(),
        url = serde_json::to_string(url).unwrap_or_default(),
        keep_open = keep_open,
        image_only = purpose == LoaderPurpose::Image,
        load_remote_image = load_remote_image,
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
    let download_source_url = source_url.clone();
    let download_attempt = attempt.clone();
    let downloads =
        crate::loader_download::LoaderDownloads::new(pending.clone(), id.clone(), attempt.clone());
    let download_handler = downloads.clone();
    let download_cleanup = downloads.clone();
    let build_attempt = attempt.clone();
    let scheduled = app.run_on_main_thread(move || {
        if build_attempt.is_cancelled() {
            build_pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&build_id);
            return;
        }
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
                    .on_download(move |webview, event| {
                        use tauri::webview::DownloadEvent;
                        let (requested, url, destination, success, accepted) = match event {
                            DownloadEvent::Requested { url, destination } => {
                                let result = download_handler.request(url.as_str());
                                let accepted = match result {
                                    Ok(path) => {
                                        *destination = path;
                                        true
                                    }
                                    Err(reason) => {
                                        download_handler.rejected(reason);
                                        false
                                    }
                                };
                                (
                                    true,
                                    url,
                                    Some(destination.to_string_lossy().into_owned()),
                                    None,
                                    accepted,
                                )
                            }
                            DownloadEvent::Finished { url, path, success } => {
                                download_handler.finish(url.as_str(), success);
                                (
                                    false,
                                    url,
                                    path.map(|p| p.to_string_lossy().into_owned()),
                                    Some(success),
                                    false,
                                )
                            }
                            _ => return false,
                        };
                        download_attempt.record_download(
                            requested,
                            accepted,
                            crate::article_diagnostics::DownloadDiagnostic::from_download(
                                &webview,
                                &download_source_url,
                                &url,
                                destination,
                                success,
                            ),
                        );
                        accepted
                    })
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

            let proxy = xaga_core::network_proxy::proxy_url(build_proxy.port)
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
            // Keep the dedicated proxy alive until the native window is destroyed.
            let lifetime = Arc::new(Mutex::new(Some(build_proxy)));
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    lifetime.lock().unwrap_or_else(|e| e.into_inner()).take();
                    sessions.remove(&build_label);
                    download_cleanup.cancel();
                    build_pending
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&build_id);
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

    let closing = Arc::new(AtomicBool::new(false));
    if keep_open {
        let _ = loader.show();
        let _ = loader.set_focus();
        // `closing` flips to true after the first close attempt so the second
        // CloseRequested (from w.close() in capture_and_close_loader) passes through.
        let closing = closing.clone();
        let close_handle = loader.clone();
        let close_pending = pending.clone();
        let close_id = id.clone();

        loader.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let capture = capture_on_close(
                    close_pending
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&close_id),
                    &closing,
                );
                if !capture {
                    return;
                }
                api.prevent_close();
                close_handle.eval("window.__captureAndClose()").ok();
            }
        });
    } else {
        // Hidden resource loaders stay hidden even if remote content requests focus.
        let loader_handle = loader.clone();

        loader.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Focused(true)) {
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
            .is_some_and(|request| request.partial.is_some() || request.download_active);
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
    let had_download = request
        .as_ref()
        .is_some_and(|request| request.download_active);
    if result.is_err() {
        if let Some(capture) = recover_favicon_timeout(request, &attempt) {
            result = Ok(Ok(capture));
        }
    }

    if !keep_open
        || had_download
        || matches!(&result, Ok(Ok(c)) if c.pdf_bytes.is_some() || c.download_error.is_some())
    {
        if let Some(w) = app.get_webview_window(&label) {
            // Download cleanup must close directly, even after removing the request.
            closing.store(true, Ordering::SeqCst);
            let _ = w.close();
        }
    }

    downloads.cancel();
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
            purpose: LoaderPurpose::Article,
            attempt: attempt.clone(),
            diagnostic_event_count: 0,
            download_active: false,
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
