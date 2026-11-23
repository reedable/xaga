use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::persistence::articles;
use xaga_core::types::{Article, Kind};

use base64::Engine as _;

use crate::article_diagnostics::{ArticleDiagnosticOutcome, ArticleDiagnostics};
use crate::error::AppError;
use crate::{AppState, article_loader};

#[derive(serde::Serialize)]
pub struct LoadArticleResult {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub html: String,
    /// Base64-encoded validated PDF bytes for inline viewing.
    #[serde(rename = "pdfBase64")]
    pub pdf_base64: Option<String>,
    #[serde(rename = "pdfEntryId", skip_serializing_if = "Option::is_none")]
    pub pdf_entry_id: Option<String>,
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

async fn stored_pdf_result(
    library: &std::path::Path,
    url: &str,
    writer: Option<&xaga_core::library_writer::LibraryWriter>,
    index_saved_entry: Option<&AppState>,
) -> Result<Option<LoadArticleResult>, AppError> {
    let Some((_, bytes)) =
        xaga_core::persistence::pdfs::read_with_writer(library, url, writer).await?
    else {
        return Ok(None);
    };
    let article = articles::find_article_with_writer(library, url, writer).await?;
    let pdf_entry_id = article.as_ref().map(|article| article.meta.id.clone());
    if let (Some(state), Some(article)) = (index_saved_entry, article) {
        state.enqueue_entry(Tier::Interactive, article).await;
    }
    Ok(Some(LoadArticleResult {
        mime_type: "application/pdf".into(),
        html: String::new(),
        pdf_base64: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        pdf_entry_id,
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
    stored_pdf_result(&library, &url, state.writer().as_ref(), None).await
}

/// The renderer supplies a source URL, never a local path. Revalidate at opening.
#[tauri::command]
pub async fn open_stored_pdf(
    url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let library = state.library_path();
    let path =
        xaga_core::persistence::pdfs::read_with_writer(&library, &url, state.writer().as_ref())
            .await?
            .map(|(path, _)| path)
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
            return stored_pdf_result(&library, &url, writer.as_ref(), Some(&state))
                .await?
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
            pdf_entry_id: None,
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
        state
            .enqueue_entry(Tier::Interactive, article.clone())
            .await;
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
        state
            .enqueue_entry(Tier::Interactive, article.clone())
            .await;
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
        articles::find_article_with_writer(&data_dir, &url, state.writer().as_ref())
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
        let article = articles::read_article_with_writer(&data_dir, &id, state.writer().as_ref())
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
        // The reading-state update goes through the entry lane; a failure leaves the entry unread.
        let Some(writer) = state.writer() else {
            return Ok(article);
        };
        let _ = writer
            .set_entry_unread(WriteTier::Interactive, id.clone(), false)
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
/// `../images/<id>/<file>` for this entry or `../metadata/favicons/<identity>`, or whose
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
pub fn clear_webview_history(app: AppHandle) -> Result<(), AppError> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let webkit_name = app.config().product_name.as_deref().unwrap_or("xaga");
    let webkit_dir = home.join("Library/WebKit").join(webkit_name);
    if webkit_dir.exists() {
        std::fs::remove_dir_all(&webkit_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
