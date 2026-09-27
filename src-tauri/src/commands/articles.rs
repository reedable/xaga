use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use xaga_core::embeddings::{EmbeddingsDb, EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::persistence::articles;
use xaga_core::types::{Article, Kind, PartialAnnotation};

use base64::Engine as _;

use crate::article_diagnostics::{
    ArticleDiagnostic, ArticleDiagnosticOutcome, ArticleDiagnostics, ArticleLoadAttempt,
};
use crate::error::AppError;
use crate::{AppState, article_loader};

#[tauri::command]
pub async fn loader_will_unload(app: AppHandle, url: String) -> Result<(), AppError> {
    crate::events::emit_app(&app, crate::events::AppEvent::LoaderWillUnload(url));
    Ok(())
}

#[tauri::command]
pub async fn loader_page_loaded(
    app: AppHandle,
    url: String,
    html: String,
    mime_type: String,
    favicon: Option<String>,
    favicon_data: Option<String>,
) -> Result<(), AppError> {
    crate::events::emit_app(
        &app,
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
pub async fn capture_and_close_loader(
    app: AppHandle,
    label: String,
    html: String,
    loader_url: String,
    page_url: String,
    mime_type: String,
    favicon: Option<String>,
    favicon_data: Option<String>,
) -> Result<(), AppError> {
    let same_page = match (
        xaga_core::util::normalize_url(&loader_url),
        xaga_core::util::normalize_url(&page_url),
    ) {
        (Ok(a), Ok(b)) => a == b,
        // Unparseable URL on either side: fall back to exact equality.
        _ => loader_url == page_url,
    };
    crate::events::emit_app(
        &app,
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
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.close();
    }
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
    };

    // The favicon's bytes cannot be read from the article's own document (a
    // cross-origin favicon without CORS is unreadable there), so this window is
    // navigated to the favicon URL and the bytes come back through
    // `receive_favicon`, which then completes this capture and closes the
    // window. Only on the hidden single-shot path: a `keep_open` window is
    // visible and may be in use, and the reqwest-error path may still need to
    // show this window as a PDF.
    let favicon_target =
        if !request.keep_open && !request.reqwest_error && capture.mime_type != "application/pdf" {
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
    if !request.keep_open && !request.reqwest_error {
        // Terminate the webview immediately so untrusted scripts cannot
        // continue running or spamming IPC. When the inline-PDF reqwest
        // probe errored, defer the close to load_via_webview, which keeps
        // the window alive to show it if the content is a PDF.
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
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    let Some(request) = map.remove(&id) else {
        return Ok(());
    };

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
    /// Base64-encoded PDF bytes, present only on the inline-PDF fast path.
    #[serde(rename = "pdfBase64")]
    pub pdf_base64: Option<String>,
    /// The favicon URL the loader resolved for this page, and its bytes as a
    /// `data:` URI. Fetched inside the loader so the main window never has to.
    pub favicon: Option<String>,
    #[serde(rename = "faviconData")]
    pub favicon_data: Option<String>,
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
    attempt: &ArticleLoadAttempt,
) -> Result<PdfProbe, AppError> {
    // Only ever fetch real remote web content (mirror the loader's scheme guard).
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::Message(format!(
            "refusing to fetch non-web URL scheme: {}",
            parsed.scheme()
        )));
    }

    let resp = client
        .get(parsed)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(|e| {
            attempt.add_event(
                "native",
                "pdf_probe",
                ArticleDiagnosticOutcome::Warning,
                if e.is_timeout() {
                    "article:timeout"
                } else if e.is_redirect() {
                    "article:redirect-limit"
                } else if e.is_connect() {
                    "article:connect"
                } else {
                    "article:transport"
                },
                &error_chain(&e),
            );
            AppError::Message(crate::article_diagnostics::sanitize_detail(&error_chain(
                &e,
            )))
        })?;
    attempt.add_event(
        "native",
        "pdf_response",
        if resp.status().is_success() {
            ArticleDiagnosticOutcome::Succeeded
        } else {
            ArticleDiagnosticOutcome::Warning
        },
        if resp.status().is_success() {
            ""
        } else {
            "article:http"
        },
        &format!(
            "status={}; content_type={}",
            resp.status().as_u16(),
            if resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .eq_ignore_ascii_case("application/pdf"))
                .unwrap_or(false)
            {
                "application/pdf"
            } else {
                "other or missing"
            }
        ),
    );

    // The SSRF proxy's "plaintext HTTP is disabled" refusal. Mapped before the
    // content-type check so it doesn't read as a server error; the caller falls
    // back to the loader webview, which surfaces the proxy's explanation body.
    if resp.status().as_u16() == xaga_core::persistence::feeds::HTTP_BLOCKED_STATUS {
        return Err(AppError::Message(
            "plaintext HTTP fetch blocked by the \"HTTP allowed\" setting".to_string(),
        ));
    }

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
        return Err(AppError::Message(format!(
            "PDF request returned HTTP {}",
            resp.status()
        )));
    }

    attempt.add_started_event("pdf_body");
    let bytes = resp.bytes().await.map_err(|e| {
        attempt.add_event(
            "native",
            "pdf_body",
            ArticleDiagnosticOutcome::Warning,
            "article:pdf-body",
            &error_chain(&e),
        );
        AppError::Message(crate::article_diagnostics::sanitize_detail(&error_chain(
            &e,
        )))
    })?;
    attempt.add_succeeded_event("pdf_body");
    attempt.add_started_event("pdf_signature");

    if !bytes.starts_with(b"%PDF-") {
        attempt.add_failed_event(
            "pdf_signature",
            "article:spoofed-pdf",
            "PDF signature missing",
        );
        return Ok(PdfProbe::Invalid(
            "Content-Type was application/pdf but body is not a PDF".to_string(),
        ));
    }

    attempt.add_succeeded_event("pdf_signature");
    Ok(PdfProbe::Pdf(bytes.to_vec()))
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

#[tauri::command]
pub async fn load_article(
    url: String,
    keep_open: bool,
    diagnostic_id: Option<String>,
    diagnostics: State<'_, ArticleDiagnostics>,
    proxy_config: State<'_, xaga_core::ssrf_proxy::ProxyConfig>,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
    state: State<'_, AppState>,
) -> Result<LoadArticleResult, AppError> {
    let attempt = diagnostic_id
        .as_deref()
        .and_then(|id| diagnostics.get_attempt(id))
        .unwrap_or_else(|| diagnostics.create_attempt(if keep_open { "visible" } else { "load" }));
    let result = async {
        attempt.add_started_event("proxy_start");
        let proxy_attempt = attempt.clone();
        let proxy = Arc::new(
            xaga_core::ssrf_proxy::start_scoped(proxy_config.inner().clone().with_observer(
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
        let named_stores_supported = state.named_stores_supported;
        let ssrf_proxy_port = proxy.port;
        let inline_pdf_reader = *state.inline_pdf_reader.read().unwrap();

        let mut reqwest_error = false;

        // Inline-PDF fast path: only when the setting is on and the caller isn't
        // explicitly asking for the visible webview. Fetch the URL through the SSRF
        // proxy; a PDF is returned as bytes for inline rendering and skips the
        // webview entirely. Not making this request when the setting is off avoids
        // letting servers profile us.
        if inline_pdf_reader && !keep_open {
            attempt.add_started_event("pdf_client");
            let client = crate::fetch_client::try_build(
                ssrf_proxy_port,
                *state.max_redirects.read().unwrap(),
                *state.referer_allowed.read().unwrap(),
                *state.gzip_allowed.read().unwrap(),
            )
            .map_err(|e| AppError::Message(error_chain(&e)))?;
            attempt.add_succeeded_event("pdf_client");
            attempt.add_started_event("pdf_probe");
            match try_fetch_pdf(&client, &url, timeout_secs, &attempt).await {
                Ok(PdfProbe::Pdf(bytes)) => {
                    return Ok(LoadArticleResult {
                        mime_type: "application/pdf".to_string(),
                        html: String::new(),
                        pdf_base64: Some(base64::engine::general_purpose::STANDARD.encode(&bytes)),
                        favicon: None,
                        favicon_data: None,
                    });
                }
                // Not a PDF — fall through to the webview unchanged.
                Ok(PdfProbe::NotPdf) => {
                    attempt.add_event(
                        "native",
                        "pdf_probe",
                        ArticleDiagnosticOutcome::Skipped,
                        "",
                        "response is not PDF; continue with browser",
                    );
                }
                // Spoofed: Content-Type claimed PDF but the body isn't one. Dead-end;
                // do NOT render it in a webview. The `E_SPOOFED_PDF` code lets the
                // frontend show a "blocked" message instead of a generic failure.
                Ok(PdfProbe::Invalid(msg)) => return Err(AppError::SpoofedPdf(msg)),
                // Transport error (e.g. the site refused the non-browser client). We
                // can't confirm it's a PDF; hand it to the loader with reqwest_error so
                // the close decision is deferred until the webview reports the type.
                Err(e) => {
                    attempt.add_event(
                        "native",
                        "pdf_probe",
                        ArticleDiagnosticOutcome::Warning,
                        e.code(),
                        &e.to_string(),
                    );
                    attempt.add_event(
                        "native",
                        "browser_fallback",
                        ArticleDiagnosticOutcome::Started,
                        "",
                        "PDF probe failed; browser may still succeed",
                    );
                    reqwest_error = true;
                }
            }
        } else {
            attempt.add_event(
                "native",
                "pdf_probe",
                ArticleDiagnosticOutcome::Skipped,
                "",
                "inline probe disabled or visible browser requested",
            );
        }

        let capture = article_loader::load_via_webview(
            &app,
            &pending,
            &url,
            timeout_secs,
            keep_open,
            reqwest_error,
            &allowed_domains,
            named_stores_supported,
            proxy,
            attempt.clone(),
        )
        .await?;
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
/// `../images/<id>/<file>` for this entry, or whose file is missing/oversize,
/// resolves to `null` — a bad reference is data, not a fault, so the reader
/// simply leaves that image's `src` untouched.
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
                .get_entry(&old_id)
                .await
                .ok()
                .flatten()
                .map(|e| e.topics)
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

    // The old id is gone; consumers holding it (topic item lists, related lists) evict it.
    // The new row announces itself through the SyncAnnotations live event.
    crate::events::emit_domain(
        &window,
        xaga_core::events::DomainEvent::AnnotationRemoved { id: old_id.clone() },
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
                .get_entry(&annotation_id)
                .await
                .ok()
                .flatten()
                .map(|e| e.topics)
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
    let db = EmbeddingsDb::open(&data_dir)
        .await
        .map_err(AppError::from)?;
    db.list_annotations().await.map_err(AppError::from)
}
