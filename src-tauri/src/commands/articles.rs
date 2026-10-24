use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use xaga_core::embeddings::{EmbeddingsDb, EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::persistence::articles;
use xaga_core::types::{Article, Kind, PartialAnnotation};

use base64::Engine as _;

use crate::article_diagnostics::{ArticleDiagnostic, ArticleDiagnosticOutcome, ArticleDiagnostics};
use crate::error::AppError;
use crate::{AppState, article_loader};

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
pub async fn loader_will_unload<R: tauri::Runtime>(
    window: tauri::WebviewWindow<R>,
    sessions: State<'_, crate::loader_identity::LoaderSessions>,
) -> Result<(), AppError> {
    let (_, url) = loader_identity(&window, &sessions)?;
    let app = window.app_handle();
    crate::events::emit_app(app, crate::events::AppEvent::LoaderWillUnload(url));
    Ok(())
}

#[tauri::command]
pub async fn loader_page_loaded<R: tauri::Runtime>(
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
pub async fn capture_and_close_loader<R: tauri::Runtime>(
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
pub async fn receive_html(
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
pub async fn receive_favicon(
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
pub async fn receive_image(
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

#[derive(serde::Serialize)]
pub struct LoadArticleResult {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub html: String,
    /// Base64-encoded validated PDF bytes for inline viewing.
    #[serde(rename = "pdfBase64")]
    pub pdf_base64: Option<String>,
    /// The favicon URL the loader resolved for this page, and its bytes as a
    /// `data:` URI. Fetched inside the loader so the main window never has to.
    pub favicon: Option<String>,
    #[serde(rename = "faviconData")]
    pub favicon_data: Option<String>,
}

fn storage_error_detail(error: &AppError) -> String {
    // Parser errors may quote article frontmatter. Preserve the classification
    // but do not put those fragments into diagnostics or the clipboard.
    if error.code() == "entry:malformed" {
        return "article frontmatter could not be parsed".into();
    }
    error_chain(error)
}

fn error_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(error) = source {
        if parts.len() == 8 {
            break;
        }
        parts.push(error.to_string());
        source = error.source();
    }
    parts.join("; caused by: ")
}

fn stored_pdf_result(
    library: &std::path::Path,
    url: &str,
) -> Result<Option<LoadArticleResult>, AppError> {
    let Some((_, bytes)) = xaga_core::persistence::pdfs::read(library, url)? else {
        return Ok(None);
    };
    Ok(Some(LoadArticleResult {
        mime_type: "application/pdf".into(),
        html: String::new(),
        pdf_base64: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        favicon: None,
        favicon_data: None,
    }))
}

#[tauri::command]
pub async fn read_stored_pdf(
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<LoadArticleResult>, AppError> {
    let library = state.library_path();
    tauri::async_runtime::spawn_blocking(move || stored_pdf_result(&library, &url))
        .await
        .map_err(|e| AppError::Message(e.to_string()))?
}

/// The renderer supplies a source URL, never a local path. Revalidate at opening.
#[tauri::command]
pub async fn open_stored_pdf(
    url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let library = state.library_path();
    let path = tauri::async_runtime::spawn_blocking(move || {
        xaga_core::persistence::pdfs::read(&library, &url).map(|value| value.map(|(path, _)| path))
    })
    .await
    .map_err(|e| AppError::Message(e.to_string()))??
    .ok_or_else(|| AppError::Message("stored PDF is missing".into()))?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| AppError::Message(e.to_string()))
}

#[tauri::command]
pub async fn cancel_article_load(
    diagnostic_id: String,
    app: AppHandle,
    diagnostics: State<'_, ArticleDiagnostics>,
    pending: State<'_, article_loader::PendingRequests>,
) -> Result<(), AppError> {
    if let Some(attempt) = diagnostics.get_attempt(&diagnostic_id) {
        attempt.cancel();
    }
    let ids = {
        let mut map = pending.lock().unwrap_or_else(|e| e.into_inner());
        let ids: Vec<_> = map
            .iter()
            .filter(|(_, r)| r.attempt.get_id() == diagnostic_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            map.remove(id);
        }
        ids
    };
    for id in ids {
        if let Some(window) = app.get_webview_window(&format!("loader-{id}")) {
            let _ = window.destroy();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn load_article(
    url: String,
    keep_open: bool,
    diagnostic_id: Option<String>,
    diagnostics: State<'_, ArticleDiagnostics>,
    proxy_config: State<'_, xaga_core::network_proxy::ProxyConfig>,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
    state: State<'_, AppState>,
) -> Result<LoadArticleResult, AppError> {
    let attempt = diagnostic_id
        .as_deref()
        .and_then(|id| diagnostics.get_attempt(id))
        .unwrap_or_else(|| diagnostics.create_attempt(if keep_open { "visible" } else { "load" }));
    let (library, writer) = {
        let library = state.library.read().unwrap();
        (library.path.clone(), library.writer.clone())
    };
    let result = async {
        attempt.add_started_event("proxy_start");
        let proxy_attempt = attempt.clone();
        let proxy = Arc::new(
            xaga_core::network_proxy::start_scoped(proxy_config.inner().clone().with_observer(
                Arc::new(move |code, detail| {
                    proxy_attempt.add_event(
                        "proxy",
                        "network",
                        ArticleDiagnosticOutcome::Warning,
                        code,
                        detail,
                    );
                }),
            ))
            .await
            .map_err(|e| AppError::Message(e.to_string()))?,
        );
        attempt.add_succeeded_event("proxy_start");
        let timeout_secs = *state.loader_timeout_secs.read().unwrap();
        let allowed_domains = state.allowed_domains.read().unwrap().clone();
        let load_remote_image = *state.load_remote_image.read().unwrap();
        let named_stores_supported = state.named_stores_supported;
        let capture = article_loader::load_via_webview(
            &app,
            &pending,
            &url,
            timeout_secs,
            keep_open,
            article_loader::LoaderPurpose::Article,
            load_remote_image,
            &allowed_domains,
            named_stores_supported,
            proxy,
            attempt.clone(),
        )
        .await?;
        if attempt.is_cancelled() {
            return Err(AppError::Message("load cancelled".into()));
        }
        if let Some(error) = capture.download_error {
            return Err(if error == "unsupported download format" {
                AppError::UnsupportedDownload(error)
            } else {
                AppError::Message(error)
            });
        }
        if let Some(bytes) = capture.pdf_bytes {
            attempt.add_started_event("pdf_persist");
            writer
                .as_ref()
                .ok_or(AppError::ReadOnly)?
                .save_pdf(url.clone(), Some(bytes))
                .await?;
            attempt.add_succeeded_event("pdf_persist");
            return tauri::async_runtime::spawn_blocking(move || stored_pdf_result(&library, &url))
                .await
                .map_err(|e| AppError::Message(e.to_string()))??
                .ok_or_else(|| AppError::Message("saved PDF is missing".into()));
        }
        if capture.mime_type == "application/pdf" {
            return Err(AppError::Message(
                "PDF bytes were not delivered by native download".into(),
            ));
        }
        Ok(LoadArticleResult {
            mime_type: capture.mime_type,
            html: capture.html,
            pdf_base64: None,
            favicon: capture.favicon,
            favicon_data: capture.favicon_data,
        })
    }
    .await;
    if let Err(ref error) = result {
        if !attempt.is_failure_recorded() {
            attempt.add_failed_event(&attempt.get_last_step(), error.code(), &error.to_string());
        }
    }
    result
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn save_article(
    id: String,
    url: String,
    title: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    favicon: Option<String>,
    markdown: String,
    image_data: std::collections::HashMap<String, String>,
    state: State<'_, AppState>,
    diagnostic_id: Option<String>,
    diagnostics: State<'_, ArticleDiagnostics>,
    app: tauri::AppHandle,
) -> Result<Article, AppError> {
    let result: Result<_, AppError> = async {
        let writer = crate::commands::require_writer(&state)?;
        let article = writer
            .save_article(
                WriteTier::Interactive,
                id,
                url,
                title,
                published_at,
                updated_at,
                summary,
                favicon,
                markdown,
                image_data,
            )
            .await
            .map_err(AppError::from)?;
        state.enqueue(
            Tier::Interactive,
            Job::UpsertEntry {
                article: article.clone(),
            },
        );
        crate::events::emit_domain(
            &app,
            xaga_core::events::DomainEvent::FaviconChanged(article.clone()),
        );
        Ok(article)
    }
    .await;
    if let Err(ref error) = result {
        if let Some(attempt) = diagnostic_id
            .as_deref()
            .and_then(|id| diagnostics.get_attempt(id))
        {
            attempt.add_failed_event("save", error.code(), &storage_error_detail(error));
        }
    }
    result
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_article(
    url: String,
    title: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    favicon: Option<String>,
    markdown: String,
    image_data: std::collections::HashMap<String, String>,
    state: State<'_, AppState>,
    diagnostic_id: Option<String>,
    diagnostics: State<'_, ArticleDiagnostics>,
    app: tauri::AppHandle,
) -> Result<Article, AppError> {
    let result: Result<_, AppError> = async {
        let writer = crate::commands::require_writer(&state)?;
        let article = writer
            .create_article(
                WriteTier::Interactive,
                url,
                title,
                published_at,
                updated_at,
                summary,
                favicon,
                markdown,
                image_data,
            )
            .await
            .map_err(AppError::from)?;
        state.enqueue(
            Tier::Interactive,
            Job::UpsertEntry {
                article: article.clone(),
            },
        );
        crate::events::emit_domain(
            &app,
            xaga_core::events::DomainEvent::FaviconChanged(article.clone()),
        );
        Ok(article)
    }
    .await;
    if let Err(ref error) = result {
        if let Some(attempt) = diagnostic_id
            .as_deref()
            .and_then(|id| diagnostics.get_attempt(id))
        {
            attempt.add_failed_event("create", error.code(), &storage_error_detail(error));
        }
    }
    result
}

#[tauri::command]
pub async fn find_article(
    url: String,
    state: State<'_, AppState>,
    diagnostic_id: Option<String>,
    diagnostics: State<'_, ArticleDiagnostics>,
) -> Result<Option<Article>, AppError> {
    let result: Result<_, AppError> = async {
        let data_dir = state.library_path();
        articles::find_article(&data_dir, &url)
            .await
            .map_err(AppError::from)
    }
    .await;
    if let Err(ref error) = result {
        if let Some(attempt) = diagnostic_id
            .as_deref()
            .and_then(|id| diagnostics.get_attempt(id))
        {
            attempt.add_failed_event("lookup", error.code(), &storage_error_detail(error));
        }
    }
    result
}

#[tauri::command]
pub async fn read_article(
    id: String,
    state: State<'_, AppState>,
    diagnostic_id: Option<String>,
    diagnostics: State<'_, ArticleDiagnostics>,
) -> Result<Article, AppError> {
    let result: Result<_, AppError> = async {
        let data_dir = state.library_path();
        let article = articles::read_article(&data_dir, &id)
            .await
            .map_err(AppError::from)?;

        // Side effect: read_article also writes.
        //
        // This command saves the entry "read" status. When the app is in read-only mode
        // (another app instance holds library lock), however, the write is skipped. The
        // entry remains in "unread" status.
        //
        // NOTE: This command is NOT in WRITE_COMMANDS, so the command can be called by
        //       app read-only instances.
        //
        // The tag update goes through the entry lane; a failure leaves the entry unread.
        let Some(writer) = state.writer() else {
            return Ok(article);
        };
        let _ = writer
            .patch_entry_tags(
                WriteTier::Interactive,
                id.clone(),
                xaga_core::library_writer::TagPatch::MarkRead,
            )
            .await;

        state.enqueue(
            Tier::Interactive,
            Job::SyncState {
                entity: EntityRef::new(id, Kind::Entry),
                state: StatePatch {
                    is_unread: Some(false),
                    ..Default::default()
                },
            },
        );
        Ok(article)
    }
    .await;
    if let Err(ref error) = result {
        if let Some(attempt) = diagnostic_id
            .as_deref()
            .and_then(|id| diagnostics.get_attempt(id))
        {
            attempt.add_failed_event("readback", error.code(), &storage_error_detail(error));
        }
    }
    result
}

/// Cap on how many image references one call may request, bounding the IPC
/// response. Refs past this are answered `None` without touching disk.
const MAX_IMAGE_REFS_PER_CALL: usize = 256;

/// Resolves an article's stored image references to `data:` URIs for rendering,
/// index-aligned with `refs`. A reference that is not a well-formed
/// `../images/<id>/<file>` for this entry or `../favicons/<identity>`, or whose
/// file is missing/oversize, resolves to `null` — a bad reference is data, not
/// a fault, so the reader simply leaves that image's `src` untouched.
///
/// NOT in `WRITE_COMMANDS` (it only reads, so it must work on a read-only
/// instance) and NOT in `LOADER_COMMANDS` (loader webviews render untrusted
/// remote content and must never read library files).
#[tauri::command]
pub async fn read_entry_images(
    id: String,
    refs: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Option<String>>, AppError> {
    if refs.len() > MAX_IMAGE_REFS_PER_CALL {
        return Ok(vec![None; refs.len()]);
    }
    let data_dir = state.library_path();
    Ok(xaga_core::persistence::images::read_entry_images(&data_dir, &id, &refs).await)
}

#[tauri::command]
pub async fn add_annotation(
    article_id: String,
    id: String,
    text: Vec<String>,
    occurrence: Vec<u32>,
    color: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<Article, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let annotation = xaga_core::types::Annotation {
        id,
        title: text.join(" "),
        reference: article_id.clone(),
        text,
        occurrence,
        color,
        content: note.unwrap_or_default(),
        created_at: String::new(),
    };
    let article = writer
        .add_annotation(WriteTier::Interactive, article_id.clone(), annotation)
        .await
        .map_err(AppError::from)?;
    // Insert/refresh the annotation row now (embedder-free, immediately listable), then defer the
    // ONNX embed so the new annotation gets a vector — without it, search_related/rank_topics_by_item
    // raise "id not found". embed_entry_vector is checksum-gated, so it skips the unchanged article
    // body and only fills the pending annotation vector via embed_annotations. Tier mirrors the
    // live-edit embed_follow() (Interactive -> Regular). Both jobs carry the domain object.
    state.enqueue(
        Tier::Interactive,
        Job::SyncAnnotations {
            article: article.clone(),
        },
    );
    state.enqueue(
        Tier::Regular,
        Job::EmbedEntry {
            article: article.clone(),
        },
    );
    Ok(article)
}

/// Replaces an annotation under a new id — the write path for a color change, which is
/// modeled as delete/create (color is part of the annotation's identity). The delete/create
/// lands in one atomic `.md` write; a crash after it strands nothing silent, because the
/// changed id set is exactly what the recovery and sync membership diffs detect. The cost
/// (one annotation re-embed) is attributed to the annotation that changed.
#[tauri::command]
pub async fn replace_annotation(
    window: tauri::Window,
    article_id: String,
    old_id: String,
    id: String,
    text: Vec<String>,
    occurrence: Vec<u32>,
    color: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<Article, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let annotation = xaga_core::types::Annotation {
        id: id.clone(),
        title: text.join(" "),
        reference: article_id.clone(),
        text,
        occurrence,
        color,
        content: note.unwrap_or_default(),
        created_at: String::new(),
    };
    let article = writer
        .replace_annotation(
            WriteTier::Interactive,
            article_id.clone(),
            old_id.clone(),
            annotation,
        )
        .await
        .map_err(AppError::from)?;

    // Row reconcile first: SyncAnnotations deletes the old id's row (gone from the `.md`) and
    // inserts the new one with a pending vector; the deferred EmbedEntry fills it. Enqueued
    // before the member jobs below so the new row exists when AddMember writes its `topics`.
    // Tier mirrors add_annotation.
    state.enqueue(
        Tier::Interactive,
        Job::SyncAnnotations {
            article: article.clone(),
        },
    );
    state.enqueue(
        Tier::Regular,
        Job::EmbedEntry {
            article: article.clone(),
        },
    );

    // Migrate topic membership old id -> new id: swap in place in each topic `.md` (position
    // and type preserved), then move the reverse index. Topics are read from the LanceDB
    // reverse index, mirroring remove_annotation — best-effort; recovery's unconditional
    // reverse-index rebuild corrects any miss.
    let topic_ids: Vec<String> = {
        let db = state.embeddings_read.lock().unwrap().clone();
        match db {
            Some(db) => db
                .find_topic_ids_by_item_id(&old_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => Vec::new(),
        }
    };
    for topic_id in topic_ids {
        let _ = writer
            .replace_topic_item(
                WriteTier::Interactive,
                topic_id.clone(),
                old_id.clone(),
                xaga_core::types::CollectionItem::Annotation {
                    id: id.clone(),
                    reference: article_id.clone(),
                },
            )
            .await;
        state.enqueue(
            Tier::Interactive,
            Job::RemoveMember {
                topic_id: topic_id.clone(),
                member_id: old_id.clone(),
            },
        );
        state.enqueue(
            Tier::Interactive,
            Job::AddMember {
                topic_id,
                member_id: id.clone(),
            },
        );
    }

    let replacement = article
        .annotations
        .iter()
        .find(|annotation| annotation.id == id)
        .map(|annotation| xaga_core::types::ListItem {
            id: annotation.id.clone(),
            title: annotation.title.clone(),
            created_at: annotation.created_at.clone(),
            url: article.meta.url.clone(),
            is_unread: false,
            is_bookmarked: false,
            reference: Some(annotation.reference.clone()),
            color: Some(annotation.color.clone()),
        })
        .ok_or_else(|| AppError::Message("replacement annotation missing".into()))?;
    crate::events::emit_domain(
        &window,
        xaga_core::events::DomainEvent::AnnotationReplaced(
            xaga_core::events::AnnotationReplacement {
                old_id: old_id.clone(),
                item: replacement,
            },
        ),
    );

    Ok(article)
}

#[tauri::command]
pub async fn remove_annotation(
    window: tauri::Window,
    article_id: String,
    annotation_id: String,
    state: State<'_, AppState>,
) -> Result<Article, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let article = writer
        .remove_annotation(
            WriteTier::Interactive,
            article_id.clone(),
            annotation_id.clone(),
        )
        .await
        .map_err(AppError::from)?;

    // Evict the deleted annotation from any topic items lists immediately, so topic item
    // lists don't serve stale data before LanceDB catches up. Read the annotation's topics
    // from the LanceDB reverse index (the `topics` column, maintained by the topic-membership
    // reconcile) instead of scanning every topic file — mirrors `list_entry_topics`. Best-effort:
    // a missing read handle or read error just skips the eviction (the reconcile still corrects it).
    let topic_ids: Vec<String> = {
        let db = state.embeddings_read.lock().unwrap().clone();
        match db {
            Some(db) => db
                .find_topic_ids_by_item_id(&annotation_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => Vec::new(),
        }
    };
    for topic_id in topic_ids {
        let _ = writer
            .remove_topic_item(
                WriteTier::Interactive,
                topic_id.clone(),
                annotation_id.clone(),
            )
            .await;
        state.enqueue(
            Tier::Interactive,
            Job::RemoveMember {
                topic_id,
                member_id: annotation_id.clone(),
            },
        );
    }

    state.enqueue(
        Tier::Interactive,
        Job::SyncAnnotations {
            article: article.clone(),
        },
    );

    crate::events::emit_domain(
        &window,
        xaga_core::events::DomainEvent::AnnotationRemoved {
            id: annotation_id.clone(),
        },
    );

    Ok(article)
}

#[tauri::command]
pub fn clear_webview_history(app: AppHandle) -> Result<(), AppError> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let webkit_name = app.config().product_name.as_deref().unwrap_or("xaga");
    let webkit_dir = home.join("Library/WebKit").join(webkit_name);
    if webkit_dir.exists() {
        std::fs::remove_dir_all(&webkit_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_annotations(
    state: State<'_, AppState>,
) -> Result<Vec<PartialAnnotation>, AppError> {
    let data_dir = state.library_path();
    let db = EmbeddingsDb::open_readonly(&data_dir)
        .await
        .map_err(AppError::from)?;
    db.list_annotations().await.map_err(AppError::from)
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
pub async fn recover_entry_images(
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
    let current = articles::read_article(&library, &id).await?;
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
        let capture = article_loader::load_via_webview(
            &app,
            &pending,
            &patch.url,
            remaining,
            false,
            article_loader::LoaderPurpose::Image,
            true,
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
    state.enqueue(
        Tier::Interactive,
        Job::UpsertEntry {
            article: article.clone(),
        },
    );
    Ok(article)
}
