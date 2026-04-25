use tauri::{AppHandle, State};
use xaga_core::articles;
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::types::AnnotationSummary;

use crate::{article_loader, AppState};
use super::{require_loader_window, require_main_window};

#[tauri::command]
pub async fn receive_html(
    window: tauri::Window,
    pending: State<'_, article_loader::PendingRequests>,
    id: String,
    html: String,
) -> Result<(), String> {
    require_loader_window(&window)?;
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = map.remove(&id) {
        let _ = tx.send(html);
    }
    // HTML has been handed off — terminate the webview immediately so
    // untrusted scripts cannot continue running or spamming IPC.
    let _ = window.close();
    Ok(())
}

#[tauri::command]
pub async fn load_article(
    window: tauri::Window,
    url: String,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    require_main_window(&window)?;
    let timeout_secs = *state.loader_timeout_secs.read().unwrap();
    article_loader::load_via_webview(&app, &pending, &url, timeout_secs).await
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
    articles::read_article(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
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
    articles::remove_annotation(&data_dir, &article_id, &annotation_id)
        .await
        .map_err(|e| e.to_string())
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
