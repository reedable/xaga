use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::{TagPatch, Tier as WriteTier};
use xaga_core::persistence::entries;
use xaga_core::query::items;
use xaga_core::types::{Entry, Kind, ListItem};

use super::parse_item_filter;
use crate::AppState;

#[tauri::command]
pub async fn read_entry(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library_path();
    entries::read_entry(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_entry_read(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .patch_entry_tags(WriteTier::Interactive, id.clone(), TagPatch::MarkRead)
        .await
        .map_err(|e| e.to_string())?;
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
pub async fn mark_entry_unread(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .patch_entry_tags(WriteTier::Interactive, id.clone(), TagPatch::MarkUnread)
        .await
        .map_err(|e| e.to_string())?;
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
pub fn is_cache_ready(state: State<'_, AppState>) -> bool {
    state.embeddings_read.lock().unwrap().is_some()
}

#[tauri::command]
pub async fn list_all_items(
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ListItem>, String> {
    let data_dir = state.library_path();
    let item_filter = parse_item_filter(item_filter)?;

    items::list_all_items(&data_dir, item_filter)
        .await
        .map_err(|e| e.to_string())
}
