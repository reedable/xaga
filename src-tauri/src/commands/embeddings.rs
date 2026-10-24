use tauri::State;
use xaga_core::query::items::{self, EmbeddedItemSource};
use xaga_core::types::{EmbeddedEntry, EmbeddedItem};

use super::parse_item_filter;
use crate::error::AppError;
use crate::{AppState, get_bookmarked_ids, get_unread_ids};

#[tauri::command]
pub async fn search_by_item(
    id: String,
    item_filter: Option<String>,
    min_similarity: Option<f32>,
    max_similarity: Option<f32>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<EmbeddedEntry>, AppError> {
    let library = state.library_path();
    let item_filter = parse_item_filter(item_filter)?;
    let result = items::list_embedded_items(
        &library,
        EmbeddedItemSource::RelatedTo {
            id,
            limit,
            min_similarity,
            max_similarity,
        },
        item_filter,
    )
    .await
    .map_err(AppError::from);
    match &result {
        Ok(entries) => log::debug!("search_by_item: ok ({} entries)", entries.len()),
        Err(e) => log::debug!("search_by_item: err — {e}"),
    }
    result
}

#[tauri::command]
pub async fn search_by_text(
    text: String,
    item_filter: Option<String>,
    min_similarity: Option<f32>,
    max_similarity: Option<f32>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<EmbeddedItem>, AppError> {
    let library = state.library_path();
    let item_filter = parse_item_filter(item_filter)?;

    let embedder = {
        let guard = state.embedding_query_model.lock().unwrap();
        guard
            .as_ref()
            .ok_or_else(|| "embedding model unavailable".to_string())?
            .clone()
    };

    let text_clone = text.clone();
    let vector = tokio::task::spawn_blocking(move || embedder.embed("", &text_clone))
        .await
        .map_err(|e| e.to_string())?
        .map_err(AppError::from)?;

    let entries = items::list_embedded_items(
        &library,
        EmbeddedItemSource::RelatedToVector {
            vector,
            limit,
            min_similarity,
            max_similarity,
        },
        item_filter,
    )
    .await
    .map_err(AppError::from)?;

    let (unread, bookmarked) = tokio::join!(get_unread_ids(&state), get_bookmarked_ids(&state));
    let result = items::enrich_embedded(&library, entries, &unread, &bookmarked).await;
    log::debug!("search_by_text: ok ({} entries)", result.len());
    Ok(result)
}

#[tauri::command]
pub async fn reset_embeddings(state: State<'_, AppState>) -> Result<(), AppError> {
    let library = state.library_path();
    if let Some(svc) = state.embedding_service.lock().unwrap().as_ref() {
        svc.reset(library);
    }
    Ok(())
}
