use tauri::State;
use xaga_core::item::{self, EmbeddedItemSource};
use xaga_core::types::EmbeddedEntry;

use crate::AppState;
use super::parse_item_filter;

#[tauri::command]
pub async fn query_by_item(
    id: String,
    item_filter: Option<String>,
    min_similarity: Option<f32>,
    max_similarity: Option<f32>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<EmbeddedEntry>, String> {
    let library = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    let result = item::list_embedded_items(
        &library,
        EmbeddedItemSource::RelatedTo { id, limit, min_similarity, max_similarity },
        item_filter,
    )
    .await
    .map_err(|e| e.to_string());
    match &result {
        Ok(entries) => log::debug!("query_by_item: ok ({} entries)", entries.len()),
        Err(e) => log::debug!("query_by_item: err — {e}"),
    }
    result
}

#[tauri::command]
pub async fn query_by_bookmarks(
    item_filter: Option<String>,
    min_similarity: Option<f32>,
    max_similarity: Option<f32>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<EmbeddedEntry>, String> {
    let library = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    let result = item::list_embedded_items(
        &library,
        EmbeddedItemSource::RelatedToBookmarks { limit, min_similarity, max_similarity },
        item_filter,
    )
    .await
    .map_err(|e| e.to_string());
    match &result {
        Ok(entries) => log::debug!("query_by_bookmarks: ok ({} entries)", entries.len()),
        Err(e) => log::debug!("query_by_bookmarks: err — {e}"),
    }
    result
}

#[tauri::command]
pub async fn query_by_topics(
    item_filter: Option<String>,
    min_similarity: Option<f32>,
    max_similarity: Option<f32>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<EmbeddedEntry>, String> {
    let library = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    let result = item::list_embedded_items(
        &library,
        EmbeddedItemSource::RelatedToTopics { limit, min_similarity, max_similarity },
        item_filter,
    )
    .await
    .map_err(|e| e.to_string());
    match &result {
        Ok(entries) => log::debug!("query_by_topics: ok ({} entries)", entries.len()),
        Err(e) => log::debug!("query_by_topics: err — {e}"),
    }
    result
}

#[tauri::command]
pub async fn reset_embeddings(state: State<'_, AppState>) -> Result<(), String> {
    let library = state.library.read().unwrap().clone();
    if let Some(svc) = state.embedding_service.lock().unwrap().as_ref() {
        svc.reset(library);
    }
    Ok(())
}
