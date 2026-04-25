use tauri::State;
use xaga_core::saved;
use xaga_core::types::SavedItem;

use crate::AppState;

#[tauri::command]
pub async fn list_saved(state: State<'_, AppState>) -> Result<Vec<SavedItem>, String> {
    let data_dir = state.library.read().unwrap().clone();
    saved::list_saved(&data_dir)
        .await
        .map_err(|e| e.to_string())
}
