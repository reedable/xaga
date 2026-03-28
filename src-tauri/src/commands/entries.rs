use tauri::State;
use xaga_core::entries;
use xaga_core::types::Entry;

use crate::AppState;

#[tauri::command]
pub async fn mark_entry_read(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    entries::mark_entry_read(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_entry_unread(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    entries::mark_entry_unread(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_all_articles(state: State<'_, AppState>) -> Result<Vec<Entry>, String> {
    let data_dir = state.library.read().unwrap().clone();
    entries::list_entries(&data_dir)
        .await
        .map_err(|e| e.to_string())
}
