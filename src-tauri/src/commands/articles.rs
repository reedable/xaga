use tauri::{AppHandle, State};
use xaga_core::articles;

use crate::{article_loader, AppState};

#[tauri::command]
pub async fn receive_html(
    pending: State<'_, article_loader::PendingRequests>,
    id: String,
    html: String,
) -> Result<(), String> {
    let mut map = pending.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = map.remove(&id) {
        let _ = tx.send(html);
    }
    Ok(())
}

#[tauri::command]
pub async fn load_article(
    url: String,
    app: AppHandle,
    pending: State<'_, article_loader::PendingRequests>,
) -> Result<String, String> {
    article_loader::load_via_webview(&app, &pending, &url).await
}

#[tauri::command]
pub async fn save_article(
    id: String,
    url: String,
    title: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
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
