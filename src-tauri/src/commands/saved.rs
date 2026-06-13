use tauri::State;
use xaga_core::saved;
use xaga_core::types::SavedItem;

use crate::AppState;

#[tauri::command]
pub async fn list_saved(state: State<'_, AppState>) -> Result<Vec<SavedItem>, String> {
    // Phase C: read saved items (bookmarks + annotations) from LanceDB via the
    // shared read handle. Reload once so both halves see the service's latest writes.
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Err("embeddings read handle unavailable".to_string());
    };
    db.reload().await.map_err(|e| e.to_string())?;
    saved::list_saved(&db).await.map_err(|e| e.to_string())
}
