use serde::Serialize;
use tauri::State;
use xaga_core::bookmarks;
use xaga_core::embeddings::BackgroundMsg;
use xaga_core::entries;
use xaga_core::types::Entry;

use crate::AppState;
use super::require_main_window;

#[derive(Serialize)]
pub struct BookmarkImportResult {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub failed: Vec<String>,
}

#[tauri::command]
pub async fn add_bookmark(
    window: tauri::Window,
    id: String,
    state: State<'_, AppState>,
) -> Result<Entry, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let current = entries::read_entry(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    let mut tags = current.tags;
    if !tags.contains(&"bookmark".to_string()) {
        tags.push("bookmark".to_string());
    }
    entries::set_entry_tags(&data_dir, &id, tags)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_bookmark(
    window: tauri::Window,
    id: String,
    state: State<'_, AppState>,
) -> Result<entries::Entry, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let current = entries::read_entry(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    let tags: Vec<String> = current.tags.into_iter().filter(|t| t != "bookmark").collect();
    entries::set_entry_tags(&data_dir, &id, tags)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_bookmarks(state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let mut entries = entries::list_entries(&data_dir)
        .await
        .map_err(|e| e.to_string())?;
    entries.retain(|e| e.is_bookmarked);
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
    let result = bookmarks::import_bookmarks(&data_dir, &html)
        .await
        .map_err(|e| e.to_string())?;

    // The file watcher queues embed messages for entries/ on modify events,
    // but import writes files directly without going through the watcher path.
    // Queue explicitly here so imported bookmarks get indexed.
    {
        let svc = state.embedding_service.lock().unwrap();
        if let Some(svc) = svc.as_ref() {
            let tx = svc.background_sender();
            for id in result.added.iter().chain(result.updated.iter()) {
                let _ = tx.send(BackgroundMsg::Embed(id.clone()));
            }
        }
    }

    Ok(BookmarkImportResult {
        added: result.added,
        updated: result.updated,
        failed: result.failed,
    })
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
