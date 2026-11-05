use tauri::State;
use xaga_core::types::{Entry, SiteSummary};

use super::parse_entity_filter;
use crate::{AppState, error::AppError};

#[tauri::command]
pub async fn list_sites(state: State<'_, AppState>) -> Result<Vec<SiteSummary>, AppError> {
    let db = state
        .embeddings_read
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| AppError::Message("embeddings read handle unavailable".to_string()))?;
    xaga_core::query::sites::list_sites(&db)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn list_site_entries(
    hostname: String,
    entity_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Entry>, AppError> {
    let entity_filter = parse_entity_filter(entity_filter)?;
    let db = state
        .embeddings_read
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| AppError::Message("embeddings read handle unavailable".to_string()))?;
    xaga_core::query::sites::list_site_entries(&db, &hostname, entity_filter)
        .await
        .map_err(AppError::from)
}
