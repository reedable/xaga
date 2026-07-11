use tauri::{AppHandle, Manager, State};
use xaga_core::embeddings::{EmbeddingsDb, EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::articles;
use xaga_core::persistence::entries;
use xaga_core::persistence::topics;
use xaga_core::types::{Article, Kind, PartialAnnotation};

use base64::Engine as _;

use crate::{AppState, article_loader};

#[tauri::command]
pub async fn loader_will_unload(app: AppHandle, url: String) -> Result<(), String> {
    crate::events::emit_app(&app, crate::events::AppEvent::LoaderWillUnload(url));
    Ok(())
}

#[tauri::command]
pub async fn loader_page_loaded(
    app: AppHandle,
    url: String,
    html: String,
    mime_type: String,
) -> Result<(), String> {
    crate::events::emit_app(
        &app,
        crate::events::AppEvent::LoaderPageLoaded(crate::events::LoaderPageLoaded {
            url,
            html,
            mime_type,
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
) -> Result<(), String> {
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
        }),
    );
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

    // The SSRF proxy's "plaintext HTTP is disabled" refusal. Mapped before the
    // content-type check so it doesn't read as a server error; the caller falls
    // back to the loader webview, which surfaces the proxy's explanation body.
    if resp.status().as_u16() == xaga_core::persistence::feeds::HTTP_BLOCKED_STATUS {
        return Err("plaintext HTTP fetch blocked by the \"HTTP allowed\" setting".to_string());
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
        let client = crate::fetch_client::build(
            ssrf_proxy_port,
            *state.max_redirects.read().unwrap(),
            *state.referer_allowed.read().unwrap(),
            *state.gzip_allowed.read().unwrap(),
        );
        match try_fetch_pdf(&client, &url, timeout_secs).await {
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
) -> Result<Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let article = articles::save_article(
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
    .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::UpsertEntry {
            article: article.clone(),
        },
    );
    Ok(article)
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
) -> Result<Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let article = articles::create_article(
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
    .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::UpsertEntry {
            article: article.clone(),
        },
    );
    Ok(article)
}

#[tauri::command]
pub async fn find_article(
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<Article>, String> {
    let data_dir = state.library.read().unwrap().clone();
    articles::find_article(&data_dir, &url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_article(id: String, state: State<'_, AppState>) -> Result<Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let article = articles::read_article(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;

    if let Ok(tags) = entries::read_entry_tags(&data_dir, &id).await {
        let tags: Vec<String> = tags
            .into_iter()
            .filter(|t| t != xaga_core::tags::UNREAD)
            .collect();
        let _ = entries::set_entry_tags(&data_dir, &id, tags).await;
    }

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

#[tauri::command]
pub async fn add_annotation(
    article_id: String,
    id: String,
    text: Vec<String>,
    occurrence: Vec<u32>,
    color: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<Article, String> {
    let data_dir = state.library.read().unwrap().clone();
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
    let article = articles::add_annotation(&data_dir, &article_id, annotation)
        .await
        .map_err(|e| e.to_string())?;
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
) -> Result<Article, String> {
    let data_dir = state.library.read().unwrap().clone();
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
    let article = articles::replace_annotation(&data_dir, &article_id, &old_id, annotation)
        .await
        .map_err(|e| e.to_string())?;

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
        let _ = topics::replace_topic_item(
            &data_dir,
            &topic_id,
            &old_id,
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
    crate::events::emit_domain(&window, xaga_core::events::DomainEvent::AnnotationRemoved { id: old_id.clone() });

    Ok(article)
}

#[tauri::command]
pub async fn remove_annotation(
    window: tauri::Window,
    article_id: String,
    annotation_id: String,
    state: State<'_, AppState>,
) -> Result<Article, String> {
    let data_dir = state.library.read().unwrap().clone();
    let article = articles::remove_annotation(&data_dir, &article_id, &annotation_id)
        .await
        .map_err(|e| e.to_string())?;

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
        let _ = topics::remove_topic_item(&data_dir, &topic_id, &annotation_id).await;
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

    crate::events::emit_domain(&window, xaga_core::events::DomainEvent::AnnotationRemoved { id: annotation_id.clone() });

    Ok(article)
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
) -> Result<Vec<PartialAnnotation>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let db = EmbeddingsDb::open(&data_dir)
        .await
        .map_err(|e| e.to_string())?;
    db.list_annotations().await.map_err(|e| e.to_string())
}
