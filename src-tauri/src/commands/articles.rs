use tauri::{AppHandle, Emitter, Manager, State};
use xaga_core::articles;
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::entries;
use xaga_core::topics;
use xaga_core::types::AnnotationSummary;

use crate::{article_loader, AppState};
use super::{require_loader_window, require_main_window};

#[tauri::command]
pub async fn loader_will_unload(app: AppHandle, url: String) {
    app.emit("loader:will-unload", url).ok();
}

#[derive(serde::Serialize, Clone)]
struct LoaderClosed {
    url: String,
    html: String,
}

#[tauri::command]
pub async fn capture_and_close_loader(
    window: tauri::Window,
    app: AppHandle,
    label: String,
    html: String,
    url: String,
) -> Result<(), String> {
    require_loader_window(&window)?;
    app.emit("loader:closed", LoaderClosed { url, html }).ok();
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
) -> Result<(), String> {
    require_loader_window(&window)?;
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    if let Some((tx, keep_open)) = map.remove(&id) {
        let _ = tx.send(html);
        if !keep_open {
            // Terminate the webview immediately so untrusted scripts cannot
            // continue running or spamming IPC.
            let _ = window.close();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn load_article(
    window: tauri::Window,
    url: String,
    keep_open: bool,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    require_main_window(&window)?;
    let timeout_secs = *state.loader_timeout_secs.read().unwrap();
    article_loader::load_via_webview(&app, &pending, &url, timeout_secs, keep_open).await
}

#[tauri::command]
pub async fn save_article(
    window: tauri::Window,
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
    require_main_window(&window)?;
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
    window: tauri::Window,
    url: String,
    title: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    favicon: Option<String>,
    markdown: String,
    state: State<'_, AppState>,
) -> Result<articles::Article, String> {
    require_main_window(&window)?;
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
    window: tauri::Window,
    article_id: String,
    id: String,
    text: Vec<String>,
    occurrence: Vec<u32>,
    color: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<articles::Article, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let annotation = xaga_core::types::Annotation { id, reference: article_id.clone(), text, occurrence, color, note, created_at: String::new() };
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
    require_main_window(&window)?;
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
pub async fn list_annotations(state: State<'_, AppState>) -> Result<Vec<AnnotationSummary>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let db = EmbeddingsDb::open(&data_dir)
        .await
        .map_err(|e| e.to_string())?;
    db.list_annotations()
        .await
        .map_err(|e| e.to_string())
}
