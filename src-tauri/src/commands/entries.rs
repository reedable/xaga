use tauri::State;
use xaga_core::entries;
use xaga_core::types::Entry;

use crate::AppState;
use super::require_main_window;

#[tauri::command]
pub async fn mark_entry_read(window: tauri::Window, id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let current = entries::read_entry(&data_dir, &id).await.map_err(|e| e.to_string())?;
    let tags = current.tags.into_iter().filter(|t| t != "unread").collect();
    entries::set_entry_tags(&data_dir, &id, tags).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_entry_unread(window: tauri::Window, id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let current = entries::read_entry(&data_dir, &id).await.map_err(|e| e.to_string())?;
    let mut tags = current.tags;
    if !tags.contains(&"unread".to_string()) {
        tags.push("unread".to_string());
    }
    entries::set_entry_tags(&data_dir, &id, tags).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_all_articles(state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let data_dir = state.library.read().unwrap().clone();
    entries::list_entries(&data_dir)
        .await
        .map_err(|e| e.to_string())
}
