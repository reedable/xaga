use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::{TagPatch, Tier as WriteTier};
use xaga_core::persistence::entries;
use xaga_core::query::items;
use xaga_core::types::{Entry, Kind, ListItem};

use super::parse_item_filter;
use crate::error::AppError;
use crate::{AppState, EmbeddingStartupState};

#[tauri::command]
pub async fn read_entry(id: String, state: State<'_, AppState>) -> Result<Entry, AppError> {
    let data_dir = state.library_path();
    entries::read_entry(&data_dir, &id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn mark_entry_read(id: String, state: State<'_, AppState>) -> Result<Entry, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .set_entry_unread(WriteTier::Interactive, id.clone(), false)
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                is_unread: Some(false),
                ..Default::default()
            },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub async fn mark_entry_unread(id: String, state: State<'_, AppState>) -> Result<Entry, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .set_entry_unread(WriteTier::Interactive, id.clone(), true)
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                is_unread: Some(true),
                ..Default::default()
            },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub fn embedding_startup_state(state: State<'_, AppState>) -> EmbeddingStartupState {
    *state.embedding_startup_state.read().unwrap()
}

#[tauri::command]
pub async fn get_all_items_counts(
    state: State<'_, AppState>,
) -> Result<Option<xaga_core::embeddings::AllItemsCounts>, AppError> {
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Ok(None);
    };
    db.reload().await.map_err(AppError::from)?;
    db.all_items_counts()
        .await
        .map(Some)
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn list_all_items(
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ListItem>, AppError> {
    let data_dir = state.library_path();
    let item_filter = parse_item_filter(item_filter)?;

    items::list_all_items(&data_dir, item_filter)
        .await
        .map_err(AppError::from)
}

/// Pin intent is committed in the entry lane before updating the derived cache.
#[tauri::command]
pub async fn set_entry_pinned(
    id: String,
    pinned: bool,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Entry, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .patch_entry_tags(
            WriteTier::Interactive,
            id.clone(),
            TagPatch::SetPinned(pinned),
        )
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                is_pinned: Some(pinned),
                ..Default::default()
            },
        },
    );
    crate::events::emit_domain(
        &app,
        xaga_core::events::DomainEvent::EntryPinChanged(entry.clone()),
    );
    Ok(entry)
}

#[tauri::command]
pub async fn list_pinned_entries(state: State<'_, AppState>) -> Result<Vec<Entry>, AppError> {
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Ok(vec![]);
    };
    db.reload().await.map_err(AppError::from)?;
    db.filter_pinned_entries().await.map_err(AppError::from)
}

/// A source-file read for article scopes; unlike read_article, this does not mark read.
#[tauri::command]
pub async fn read_article_scope(
    id: String,
    state: State<'_, AppState>,
) -> Result<xaga_core::types::Article, AppError> {
    xaga_core::persistence::articles::read_article(&state.library_path(), &id)
        .await
        .map_err(AppError::from)
}
