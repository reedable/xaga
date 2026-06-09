use tauri::{AppHandle, Emitter, Manager, State};
use xaga_core::articles;
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::entries;
use xaga_core::topics;
use xaga_core::types::AnnotationSummary;

use base64::Engine as _;

use crate::{AppState, article_loader};

#[tauri::command]
pub async fn loader_will_unload(app: AppHandle, url: String) -> Result<(), String> {
    app.emit("loader:will-unload", url).ok();
    Ok(())
}

#[derive(serde::Serialize, Clone)]
struct LoaderPageLoaded {
    url: String,
    html: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
}

#[tauri::command]
pub async fn loader_page_loaded(
    app: AppHandle,
    url: String,
    html: String,
    mime_type: String,
) -> Result<(), String> {
    app.emit(
        "loader:page-loaded",
        LoaderPageLoaded {
            url,
            html,
            mime_type,
        },
    )
    .ok();
    Ok(())
}

#[derive(serde::Serialize, Clone)]
struct LoaderClosed {
    url: String,
    html: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
}

#[tauri::command]
pub async fn capture_and_close_loader(
    app: AppHandle,
    label: String,
    html: String,
    url: String,
    mime_type: String,
) -> Result<(), String> {
    app.emit(
        "loader:closed",
        LoaderClosed {
            url,
            html,
            mime_type,
        },
    )
    .ok();
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.close();
    }
    Ok(())
}

#[tauri::command]
pub async fn receive_html(
    window: tauri::Window,
    pending: State<'_, article_loader::PendingRequests>,
    id: String,
    html: String,
    mime_type: String,
) -> Result<bool, String> {
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    if let Some((tx, keep_open, reqwest_error)) = map.remove(&id) {
        let _ = tx.send((mime_type, html));
        if !keep_open && !reqwest_error {
            // Terminate the webview immediately so untrusted scripts cannot
            // continue running or spamming IPC. When the inline-PDF reqwest
            // probe errored, defer the close to load_via_webview, which keeps
            // the window alive to show it if the content is a PDF.
            let _ = window.close();
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(serde::Serialize)]
pub struct LoadArticleResult {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub html: String,
    /// Base64-encoded PDF bytes, present only on the inline-PDF fast path.
    #[serde(rename = "pdfBase64")]
    pub pdf_base64: Option<String>,
}

enum PdfProbe {
    /// A valid PDF (`application/pdf` Content-Type + `%PDF-` signature); bytes read.
    Pdf(Vec<u8>),
    /// Not a PDF; the body was NOT read, so the caller lets the loader webview
    /// perform the real load without a double download.
    NotPdf,
    /// `application/pdf` Content-Type but the body is not a PDF. This is a
    /// spoofed response — dead-end it; do NOT render it in a webview.
    Invalid(String),
}

/// Fetches `url` through the SSRF proxy to decide whether it is a PDF.
/// `Err(_)` is a transport/timeout error only (the caller may fall back to the
/// visible loader webview); content mismatches are reported via [`PdfProbe`].
async fn try_fetch_pdf(
    client: &reqwest::Client,
    url: &str,
    timeout_secs: u64,
) -> Result<PdfProbe, String> {
    // Only ever fetch real remote web content (mirror the loader's scheme guard).
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "refusing to fetch non-web URL scheme: {}",
            parsed.scheme()
        ));
    }

    let resp = client
        .get(parsed)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // MIME type only — ignore parameters like charset.
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    if content_type != "application/pdf" {
        // Not a PDF — drop the response without reading the body; the webview
        // (any status) performs the real load as today.
        return Ok(PdfProbe::NotPdf);
    }

    if !resp.status().is_success() {
        return Err(format!("PDF request returned HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;

    if !bytes.starts_with(b"%PDF-") {
        return Ok(PdfProbe::Invalid(
            "Content-Type was application/pdf but body is not a PDF".to_string(),
        ));
    }

    Ok(PdfProbe::Pdf(bytes.to_vec()))
}

#[tauri::command]
pub async fn load_article(
    url: String,
    keep_open: bool,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
    state: State<'_, AppState>,
) -> Result<LoadArticleResult, String> {
    let timeout_secs = *state.loader_timeout_secs.read().unwrap();
    let allowed_domains = state.allowed_domains.read().unwrap().clone();
    let named_stores_supported = state.named_stores_supported;
    let ssrf_proxy_port = state.ssrf_proxy_port;
    let inline_pdf_reader = *state.inline_pdf_reader.read().unwrap();

    let mut reqwest_error = false;

    // Inline-PDF fast path: only when the setting is on and the caller isn't
    // explicitly asking for the visible webview. Fetch the URL through the SSRF
    // proxy; a PDF is returned as bytes for inline rendering and skips the
    // webview entirely. Not making this request when the setting is off avoids
    // letting servers profile us.
    if inline_pdf_reader && !keep_open {
        match try_fetch_pdf(&state.ssrf_http_client, &url, timeout_secs).await {
            Ok(PdfProbe::Pdf(bytes)) => {
                return Ok(LoadArticleResult {
                    mime_type: "application/pdf".to_string(),
                    html: String::new(),
                    pdf_base64: Some(base64::engine::general_purpose::STANDARD.encode(&bytes)),
                });
            }
            // Not a PDF — fall through to the webview unchanged.
            Ok(PdfProbe::NotPdf) => {}
            // Spoofed: Content-Type claimed PDF but the body isn't one. Dead-end;
            // do NOT render it in a webview. The `E_SPOOFED_PDF` code lets the
            // frontend show a "blocked" message instead of a generic failure.
            Ok(PdfProbe::Invalid(msg)) => return Err(format!("E_SPOOFED_PDF: {msg}")),
            // Transport error (e.g. the site refused the non-browser client). We
            // can't confirm it's a PDF; hand it to the loader with reqwest_error so
            // the close decision is deferred until the webview reports the type.
            Err(e) => {
                log::warn!("inline PDF probe failed for {url}: {e}");
                reqwest_error = true;
            }
        }
    }

    let (mime_type, html) = article_loader::load_via_webview(
        &app,
        &pending,
        &url,
        timeout_secs,
        keep_open,
        reqwest_error,
        &allowed_domains,
        named_stores_supported,
        ssrf_proxy_port,
    )
    .await?;
    Ok(LoadArticleResult {
        mime_type,
        html,
        pdf_base64: None,
    })
}

#[tauri::command]
pub async fn save_article(
    id: String,
    url: String,
    title: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    favicon: Option<String>,
    markdown: String,
    state: State<'_, AppState>,
) -> Result<articles::Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    articles::save_article(
        &data_dir,
        &id,
        &url,
        &title,
        published_at.as_deref(),
        updated_at.as_deref(),
        summary.as_deref(),
        favicon.as_deref(),
        &markdown,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_article(
    url: String,
    title: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    favicon: Option<String>,
    markdown: String,
    state: State<'_, AppState>,
) -> Result<articles::Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    articles::create_article(
        &data_dir,
        &url,
        &title,
        published_at.as_deref(),
        updated_at.as_deref(),
        summary.as_deref(),
        favicon.as_deref(),
        &markdown,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_article(
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<articles::Article>, String> {
    let data_dir = state.library.read().unwrap().clone();
    articles::find_article(&data_dir, &url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_article(
    id: String,
    state: State<'_, AppState>,
) -> Result<articles::Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let article = articles::read_article(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;

    if let Ok(current) = entries::read_entry(&data_dir, &id).await {
        let tags = current.tags.into_iter().filter(|t| t != "unread").collect();
        let _ = entries::set_entry_tags(&data_dir, &id, tags).await;
    }

    Ok(article)
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
) -> Result<articles::Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let annotation = xaga_core::types::Annotation {
        id,
        reference: article_id.clone(),
        text,
        occurrence,
        color,
        note,
        created_at: String::new(),
    };
    articles::add_annotation(&data_dir, &article_id, annotation)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_annotation(
    window: tauri::Window,
    article_id: String,
    annotation_id: String,
    state: State<'_, AppState>,
) -> Result<articles::Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let result = articles::remove_annotation(&data_dir, &article_id, &annotation_id)
        .await
        .map_err(|e| e.to_string())?;

    // Evict the deleted annotation from any topic items lists immediately,
    // so topic item lists don't serve stale data before LanceDB catches up.
    if let Ok(topic_ids) = topics::list_entry_topics(&data_dir, &annotation_id).await {
        for topic_id in topic_ids {
            let _ = topics::remove_topic_item(&data_dir, &topic_id, &annotation_id).await;
        }
    }

    window.emit("annotation:removed", &annotation_id).ok();

    Ok(result)
}

#[tauri::command]
pub fn clear_webview_history(app: AppHandle) -> Result<(), String> {
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
) -> Result<Vec<AnnotationSummary>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let db = EmbeddingsDb::open(&data_dir)
        .await
        .map_err(|e| e.to_string())?;
    db.list_annotations().await.map_err(|e| e.to_string())
}
