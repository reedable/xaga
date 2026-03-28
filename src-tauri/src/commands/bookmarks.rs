use tauri::State;
use xaga_core::bookmarks;
use xaga_core::filter;
use xaga_core::types::Entry;

use crate::AppState;

#[tauri::command]
pub async fn add_bookmark(
    id: String,
    url: String,
    title: String,
    created_at: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    state: State<'_, AppState>,
) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let entry = Entry {
        id,
        url,
        title,
        created_at,
        published_at,
        updated_at,
        summary,
        is_unread: false,
    };
    bookmarks::add_bookmark(&data_dir, &entry)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_bookmark(id: String, state: State<'_, AppState>) -> Result<String, String> {
    let data_dir = state.library.read().unwrap().clone();
    bookmarks::remove_bookmark(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
        .map(|removed| removed.unwrap_or(id))
}

#[tauri::command]
pub async fn list_bookmarks(state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let mut entries = bookmarks::list_bookmarks(&data_dir)
        .await
        .map_err(|e| e.to_string())?;
    filter::annotate_unread_status(&data_dir, &mut entries).await;
    Ok(entries)
}
