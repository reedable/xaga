use serde::Serialize;
use tauri::State;
use xaga_core::bookmarks;
use xaga_core::filter;
use xaga_core::types::Entry;

use crate::{get_unread, AppState};
use super::require_main_window;

#[derive(Serialize)]
pub struct BookmarkImportResult {
    pub added: Vec<Entry>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

#[tauri::command]
pub async fn add_bookmark(
    window: tauri::Window,
    id: String,
    url: String,
    title: String,
    created_at: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    summary: Option<String>,
    state: State<'_, AppState>,
) -> Result<Entry, String> {
    require_main_window(&window)?;
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
pub async fn remove_bookmark(window: tauri::Window, id: String, state: State<'_, AppState>) -> Result<String, String> {
    require_main_window(&window)?;
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
    let unread = get_unread(&state);
    filter::annotate_unread_status(&unread, &mut entries);
    Ok(entries)
}

#[tauri::command]
pub async fn import_bookmarks(
    window: tauri::Window,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<BookmarkImportResult, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let html = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| e.to_string())?;
    let (added, skipped, failed) = bookmarks::import_bookmarks(&data_dir, &html)
        .await
        .map_err(|e| e.to_string())?;
    Ok(BookmarkImportResult { added, skipped, failed })

}

#[tauri::command]
pub async fn export_bookmarks(
    window: tauri::Window,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    bookmarks::export_bookmarks(&data_dir, std::path::Path::new(&file_path))
        .await
        .map_err(|e| e.to_string())
}
