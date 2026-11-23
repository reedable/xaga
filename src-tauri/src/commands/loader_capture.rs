use crate::article_diagnostics::{ArticleDiagnostic, ArticleDiagnosticOutcome, ArticleDiagnostics};
use crate::error::AppError;
use crate::{AppState, article_loader};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use xaga_core::embeddings::Tier;
use xaga_core::persistence::articles;
use xaga_core::types::Article;

// Identity and current location are native values, never IPC payload fields.
fn loader_identity<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    sessions: &crate::loader_identity::LoaderSessions,
) -> Result<(String, String), AppError> {
    let source = sessions
        .source_url(window.label())
        .ok_or(AppError::Unauthorized)?;
    let current = window.url().map_err(|_| AppError::Unauthorized)?;
    if !matches!(current.scheme(), "http" | "https") {
        return Err(AppError::Unauthorized);
    }
    Ok((source, current.to_string()))
}

#[tauri::command]
pub(crate) async fn loader_will_unload<R: tauri::Runtime>(
    window: tauri::WebviewWindow<R>,
    sessions: State<'_, crate::loader_identity::LoaderSessions>,
) -> Result<(), AppError> {
    let (_, url) = loader_identity(&window, &sessions)?;
    let app = window.app_handle();
    crate::events::emit_app(app, crate::events::AppEvent::LoaderWillUnload(url));
    Ok(())
}

#[tauri::command]
pub(crate) async fn loader_page_loaded<R: tauri::Runtime>(
    window: tauri::WebviewWindow<R>,
    sessions: State<'_, crate::loader_identity::LoaderSessions>,
    html: String,
    mime_type: String,
    favicon: Option<String>,
    favicon_data: Option<String>,
) -> Result<(), AppError> {
    let (_, url) = loader_identity(&window, &sessions)?;
    crate::events::emit_app(
        window.app_handle(),
        crate::events::AppEvent::LoaderPageLoaded(crate::events::LoaderPageLoaded {
            url,
            html,
            mime_type,
            favicon,
            favicon_data,
        }),
    );
    Ok(())
}

#[tauri::command]
pub(crate) async fn capture_and_close_loader<R: tauri::Runtime>(
    window: tauri::WebviewWindow<R>,
    sessions: State<'_, crate::loader_identity::LoaderSessions>,
    html: String,
    mime_type: String,
    favicon: Option<String>,
    favicon_data: Option<String>,
) -> Result<(), AppError> {
    let (loader_url, page_url) = loader_identity(&window, &sessions)?;
    let same_page = crate::loader_identity::same_page(&loader_url, &page_url);
    crate::events::emit_app(
        window.app_handle(),
        crate::events::AppEvent::LoaderClosed(crate::events::LoaderClosed {
            loader_url,
            page_url,
            same_page,
            html,
            mime_type,
            favicon,
            favicon_data,
        }),
    );
    window
        .close()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn receive_html(
    window: tauri::WebviewWindow,
    pending: State<'_, article_loader::PendingRequests>,
    id: String,
    html: String,
    mime_type: String,
    favicon: Option<String>,
    favicon_data: Option<String>,
) -> Result<bool, AppError> {
    if window.label() != format!("loader-{id}") {
        return Err(AppError::Unauthorized);
    }
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    let Some(mut request) = map.remove(&id) else {
        return Ok(false);
    };

    if request.download_active {
        map.insert(id, request);
        return Ok(false);
    }

    if request.purpose == article_loader::LoaderPurpose::Image {
        let _ = request.tx.send(article_loader::LoaderCapture {
            download_error: Some("image URL returned a non-image document".into()),
            ..Default::default()
        });
        let _ = window.close();
        return Ok(true);
    }

    // A failed favicon navigation can load an HTML error document. Never
    // replace the already captured article with that document.
    if let Some(capture) = request.partial.take() {
        request.attempt.add_event(
            "native",
            "favicon",
            ArticleDiagnosticOutcome::Warning,
            "article:favicon-not-image",
            "favicon returned a non-image document",
        );
        let _ = request.tx.send(capture);
        let _ = window.close();
        return Ok(true);
    }
    request.attempt.add_succeeded_event("capture_delivery");
    let capture = article_loader::LoaderCapture {
        mime_type,
        html,
        favicon,
        favicon_data,
        ..Default::default()
    };

    // The favicon's bytes cannot be read from the article's own document (a
    // cross-origin favicon without CORS is unreadable there), so this window is
    // navigated to the favicon URL and the bytes come back through
    // `receive_favicon`, which then completes this capture and closes the
    // window. Only on the hidden single-shot path: a `keep_open` window is
    // visible and may be in use.
    let favicon_target = if !request.keep_open && capture.mime_type != "application/pdf" {
        capture
            .favicon
            .as_deref()
            .and_then(|url| url.parse::<tauri::Url>().ok())
            .filter(|url| matches!(url.scheme(), "http" | "https"))
    } else {
        None
    };

    if let Some(favicon_url) = favicon_target {
        request.attempt.add_started_event("favicon");
        if window.navigate(favicon_url).is_ok() {
            // Deliberately neither resolves nor closes yet. This defers the
            // "kill after capture" close (see fetching_remote_content.md,
            // "Fetch-adjacent hardening in the loader") by one IPC round trip —
            // the article document is destroyed by the navigation instead, so
            // its scripts stop either way. `load_via_webview`'s overall timeout
            // still bounds the wait and closes the window if nothing arrives.
            request.partial = Some(capture);
            map.insert(id, request);
            return Ok(true);
        }
        request.attempt.add_event(
            "native",
            "favicon",
            ArticleDiagnosticOutcome::Warning,
            "article:favicon-navigation",
            "navigation refused",
        );
        // Navigation refused: fall through and complete without favicon bytes.
    }

    let _ = request.tx.send(capture);
    if !request.keep_open {
        // Terminate the captured document immediately to stop untrusted scripts.
        let _ = window.close();
    }
    Ok(true)
}

/// Receives the favicon bytes read off a canvas in the favicon's own document,
/// after `receive_html` navigated this window there. `data_uri` is `None` when
/// the read failed — the capture still completes, with the URL recorded and no
/// bytes, so the entry stores a recoverable broken ref rather than stalling.
#[tauri::command]
pub(crate) async fn receive_favicon(
    window: tauri::WebviewWindow,
    pending: State<'_, article_loader::PendingRequests>,
    id: String,
    data_uri: Option<String>,
    reason: Option<String>,
) -> Result<(), AppError> {
    if window.label() != format!("loader-{id}") {
        return Err(AppError::Unauthorized);
    }
    {
        let map = pending.lock().map_err(|e| e.to_string())?;
        if !map
            .get(&id)
            .is_some_and(|r| r.partial.is_some() && !r.download_active)
        {
            return Ok(());
        }
    }
    let data_uri = if let Some(data) = data_uri {
        tauri::async_runtime::spawn_blocking(move || {
            crate::resource_download::validate_image_data(&data)
                .and_then(crate::resource_download::prepare_favicon)
        })
        .await
        .map_err(|e| AppError::Message(e.to_string()))?
        .ok()
    } else {
        None
    };
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    let Some(request) = map.remove(&id) else {
        return Ok(());
    };

    if request.download_active {
        map.insert(id, request);
        return Ok(());
    }

    match request.partial {
        Some(mut capture) => {
            request.attempt.add_event(
                "native",
                "favicon",
                if data_uri.is_some() {
                    ArticleDiagnosticOutcome::Succeeded
                } else {
                    ArticleDiagnosticOutcome::Warning
                },
                if data_uri.is_some() {
                    ""
                } else {
                    "article:favicon-unavailable"
                },
                "",
            );
            if let Some(reason) = reason.as_deref().filter(|r| {
                [
                    "no-image",
                    "incomplete",
                    "decode-failed",
                    "encode-failed",
                    "canvas-unreadable",
                    "image-unreadable",
                ]
                .contains(r)
            }) {
                request.attempt.add_event(
                    "loader",
                    "favicon",
                    ArticleDiagnosticOutcome::Warning,
                    reason,
                    "page-reported reason",
                );
            }
            capture.favicon_data = data_uri;
            let _ = request.tx.send(capture);
            let _ = window.close();
        }
        // No article capture is waiting: this window is not in the favicon
        // phase, so put the request back untouched rather than dropping it.
        None => {
            map.insert(id, request);
        }
    }
    Ok(())
}

/// Only an image-purpose pending request may accept image-document bytes.
#[tauri::command]
pub(crate) async fn receive_image(
    window: tauri::WebviewWindow,
    pending: State<'_, article_loader::PendingRequests>,
    id: String,
    data_uri: Option<String>,
) -> Result<(), AppError> {
    if window.label() != format!("loader-{id}") {
        return Err(AppError::Unauthorized);
    }
    {
        let map = pending.lock().map_err(|e| e.to_string())?;
        if !map.get(&id).is_some_and(|r| {
            r.purpose == article_loader::LoaderPurpose::Image && !r.download_active
        }) {
            return Ok(());
        }
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        data_uri
            .ok_or_else(|| "image bytes unavailable".to_string())
            .and_then(|data| crate::resource_download::validate_image_data(&data))
    })
    .await
    .map_err(|e| e.to_string())?;
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    // A timeout, cancellation or native download may have won while decoding.
    if !map
        .get(&id)
        .is_some_and(|r| r.purpose == article_loader::LoaderPurpose::Image && !r.download_active)
    {
        return Ok(());
    }
    let request = map.remove(&id).unwrap();
    let capture = match result {
        Ok(data) => article_loader::LoaderCapture {
            image_data: Some(data),
            ..Default::default()
        },
        Err(error) => article_loader::LoaderCapture {
            download_error: Some(error),
            ..Default::default()
        },
    };
    let _ = request.tx.send(capture);
    let _ = window.close();
    Ok(())
}

#[tauri::command]
pub(crate) fn create_article_diagnostic(
    diagnostics: State<'_, ArticleDiagnostics>,
    mode: String,
) -> String {
    let mode = match mode.as_str() {
        "refresh" | "visible" | "extract" | "read" | "render" => mode.as_str(),
        _ => "load",
    };
    diagnostics.create_attempt(mode).get_id()
}

#[tauri::command]
pub(crate) fn get_article_diagnostic(
    diagnostics: State<'_, ArticleDiagnostics>,
    id: String,
) -> Option<ArticleDiagnostic> {
    diagnostics.get_attempt(&id).map(|t| t.get_diagnostic())
}

#[tauri::command]
pub(crate) fn add_article_diagnostic_event(
    diagnostics: State<'_, ArticleDiagnostics>,
    id: String,
    step: String,
    outcome: ArticleDiagnosticOutcome,
    code: String,
) -> Result<(), AppError> {
    crate::article_diagnostics::validate_event(&step, &code).map_err(|_| AppError::Unauthorized)?;
    if let Some(attempt) = diagnostics.get_attempt(&id) {
        attempt.add_event("frontend", &step, outcome, &code, "");
    }
    Ok(())
}

/// Remote loader reports vocabulary only; identity comes from the invoking
/// window, never an arbitrary attempt ID supplied by the page.
#[tauri::command]
pub(crate) fn add_loader_diagnostic_event(
    window: tauri::WebviewWindow,
    pending: State<'_, article_loader::PendingRequests>,
    event: String,
    count: Option<u32>,
) -> Result<(), AppError> {
    let Some(id) = window.label().strip_prefix("loader-") else {
        return Err(AppError::Unauthorized);
    };
    let (step, outcome, code) = match event.as_str() {
        "script_started" => ("script", ArticleDiagnosticOutcome::Started, ""),
        "page_loaded" => ("page_load", ArticleDiagnosticOutcome::Succeeded, ""),
        "dom_settled" => ("dom_settle", ArticleDiagnosticOutcome::Succeeded, ""),
        "images_skipped_pdf" => (
            "image_capture",
            ArticleDiagnosticOutcome::Skipped,
            "image:pdf",
        ),
        "images_attempted" => (
            "image_capture",
            ArticleDiagnosticOutcome::Started,
            "image:attempted",
        ),
        "images_skipped_disabled" => (
            "image_capture",
            ArticleDiagnosticOutcome::Skipped,
            "image:disabled",
        ),
        "images_skipped_no_candidates" => (
            "image_capture",
            ArticleDiagnosticOutcome::Skipped,
            "image:no-candidates",
        ),
        "images_skipped_deadline" => (
            "image_capture",
            ArticleDiagnosticOutcome::Skipped,
            "image:deadline-before-fetch",
        ),
        "images_interrupted_deadline" => (
            "image_capture",
            ArticleDiagnosticOutcome::Warning,
            "image:deadline-after-fetch",
        ),
        "images_started" => ("images", ArticleDiagnosticOutcome::Started, ""),
        "images_finished" => ("images", ArticleDiagnosticOutcome::Succeeded, ""),
        "image_captured" => (
            "images",
            ArticleDiagnosticOutcome::Succeeded,
            "image:captured",
        ),
        "image_timeout" => ("images", ArticleDiagnosticOutcome::Warning, "image:timeout"),
        "image_http" => ("images", ArticleDiagnosticOutcome::Warning, "image:http"),
        "image_size" => (
            "images",
            ArticleDiagnosticOutcome::Warning,
            "image:size-limit",
        ),
        "image_type" => (
            "images",
            ArticleDiagnosticOutcome::Warning,
            "image:not-image",
        ),
        "image_unreadable" => (
            "images",
            ArticleDiagnosticOutcome::Warning,
            "image:unreadable",
        ),
        "capture_started" => (
            "capture_serialization",
            ArticleDiagnosticOutcome::Started,
            "",
        ),
        "capture_sent" => ("capture_delivery", ArticleDiagnosticOutcome::Started, ""),
        "capture_failed" => (
            "capture_delivery",
            ArticleDiagnosticOutcome::Failed,
            "article:capture",
        ),
        "ipc_failed" => (
            "capture_delivery",
            ArticleDiagnosticOutcome::Failed,
            "article:ipc",
        ),
        "favicon_started" => ("favicon", ArticleDiagnosticOutcome::Started, ""),
        _ => return Err(AppError::Unauthorized),
    };
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    if let Some(request) = map.get_mut(id) {
        if request.diagnostic_event_count < 64 {
            request.diagnostic_event_count += 1;
            request.attempt.add_event(
                "loader",
                step,
                outcome,
                code,
                &format!("page-reported; count={}", count.unwrap_or(1).min(100000)),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod loader_security_tests {
    use super::*;
    use std::sync::Mutex;
    use tauri::{
        Listener, WebviewUrl, WebviewWindowBuilder, ipc::CallbackFn, webview::InvokeRequest,
    };

    #[test]
    fn loader_callbacks_ignore_forged_identity_and_require_live_session() {
        let sessions = crate::loader_identity::LoaderSessions::default();
        sessions.register("loader-7".into(), "https://source.example/article".into());
        sessions.register("loader-8".into(), "https://other.example/article".into());
        let app = tauri::test::mock_builder()
            .manage(sessions.clone())
            .invoke_handler(tauri::generate_handler![
                loader_will_unload,
                loader_page_loaded,
                capture_and_close_loader
            ])
            .build(crate::app_context())
            .unwrap();
        let window = WebviewWindowBuilder::new(
            &app,
            "loader-7",
            WebviewUrl::External("https://redirect.example/page".parse().unwrap()),
        )
        .build()
        .unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        for name in ["loader:will-unload", "loader:page-loaded", "loader:closed"] {
            let events = events.clone();
            app.listen(name, move |event| {
                events.lock().unwrap().push((
                    name,
                    serde_json::from_str::<serde_json::Value>(event.payload()).unwrap(),
                ));
            });
        }
        let invoke = |command: &str| {
            tauri::test::get_ipc_response(&window, InvokeRequest {
            cmd: command.into(), callback: CallbackFn(0), error: CallbackFn(1),
            url: "https://redirect.example/page".parse().unwrap(),
            body: serde_json::json!({
                "label": "main", "url": "https://other.example/article",
                "loaderUrl": "https://other.example/article", "pageUrl": "https://other.example/article",
                "html": "<p>untrusted capture</p>", "mimeType": "text/html"
            }).into(), headers: Default::default(), invoke_key: tauri::test::INVOKE_KEY.into(),
        })
        };
        assert!(invoke("loader_will_unload").is_ok());
        assert!(invoke("loader_page_loaded").is_ok());
        assert!(invoke("capture_and_close_loader").is_ok());
        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 3);
        assert_eq!(captured[0].1, "https://redirect.example/page");
        assert_eq!(captured[1].1["url"], "https://redirect.example/page");
        assert_eq!(captured[2].1["loaderUrl"], "https://source.example/article");
        assert_eq!(captured[2].1["pageUrl"], "https://redirect.example/page");
        assert_eq!(captured[2].1["samePage"], false);
        drop(captured);
        sessions.remove("loader-7");
        assert!(invoke("loader_will_unload").is_err());
        assert!(invoke("loader_page_loaded").is_err());
        assert!(invoke("capture_and_close_loader").is_err());
        assert_eq!(events.lock().unwrap().len(), 3);
        assert_eq!(
            sessions.source_url("loader-8").as_deref(),
            Some("https://other.example/article")
        );
    }
}

/// Trusted orchestration: resource loading is isolated, and only the entry
/// writer may commit recovered references. No renderer-provided bytes are saved.
#[tauri::command]
pub(crate) async fn recover_entry_images(
    id: String,
    recovery: xaga_core::persistence::image_recovery::ImageRecovery,
    diagnostic_id: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
    diagnostics: State<'_, ArticleDiagnostics>,
    proxy_config: State<'_, xaga_core::network_proxy::ProxyConfig>,
) -> Result<Article, AppError> {
    let (library, writer) = {
        let library = state.library.read().unwrap();
        (library.path.clone(), library.writer.clone())
    };
    let current =
        articles::read_article_with_writer(&library, &id, state.writer().as_ref()).await?;
    let expected = if recovery.summary {
        current.meta.description.as_deref().unwrap_or("")
    } else {
        &current.content
    };
    if expected != recovery.expected
        || !*state.load_remote_image.read().unwrap()
        || writer.is_none()
    {
        return Ok(current);
    }
    let writer = writer.unwrap();
    let attempt = diagnostic_id
        .as_deref()
        .and_then(|id| diagnostics.get_attempt(id))
        .unwrap_or_else(|| diagnostics.create_attempt("render"));
    let timeout_secs = *state.loader_timeout_secs.read().unwrap();
    let allowed_domains = state.allowed_domains.read().unwrap().clone();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut data = std::collections::HashMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut bytes = 0;
    for patch in &recovery.patches {
        if !seen.insert(patch.url.clone()) {
            continue;
        }
        if attempt.is_cancelled() {
            return Err(AppError::Message("load cancelled".into()));
        }
        if let Some(image) =
            xaga_core::persistence::images::read_entry_image_url(&library, &id, &patch.url).await
        {
            bytes += image.len();
            if bytes > 32 * 1024 * 1024 {
                break;
            }
            data.insert(patch.url.clone(), image);
            continue;
        }
        let remaining = deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .as_secs();
        if remaining == 0 {
            break;
        }
        let proxy = Arc::new(
            xaga_core::network_proxy::start_scoped(proxy_config.inner().clone())
                .await
                .map_err(|e| AppError::Message(e.to_string()))?,
        );
        let publisher_javascript_allowed = *state.publisher_javascript_allowed.read().unwrap();
        let capture = article_loader::load_via_webview(
            &app,
            &pending,
            &patch.url,
            remaining,
            false,
            article_loader::LoaderPurpose::Image,
            true,
            publisher_javascript_allowed,
            &allowed_domains,
            state.named_stores_supported,
            proxy,
            attempt.clone(),
        )
        .await;
        match capture {
            Ok(capture) if capture.image_data.is_some() => {
                let image = capture.image_data.unwrap();
                bytes += image.len();
                // Base64 transport budget corresponds to the existing 24 MiB resource budget.
                if bytes > 32 * 1024 * 1024 {
                    break;
                }
                data.insert(patch.url.clone(), image);
            }
            _ => attempt.add_event(
                "native",
                "images",
                ArticleDiagnosticOutcome::Warning,
                "image:recovery-unavailable",
                "image recovery did not return usable bytes",
            ),
        }
    }
    if attempt.is_cancelled() {
        return Err(AppError::Message("load cancelled".into()));
    }
    let article = writer
        .recover_images(id, recovery, data)
        .await
        .map_err(AppError::from)?;
    state
        .enqueue_entry(Tier::Interactive, article.clone())
        .await;
    Ok(article)
}
