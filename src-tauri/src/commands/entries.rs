use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::entries;
use xaga_core::query::items;
use xaga_core::types::{Entry, Kind, ListItem};

use super::parse_item_filter;
use crate::AppState;

#[tauri::command]
pub async fn read_entry(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    entries::read_entry(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_entry_read(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let tags: Vec<String> = entries::read_entry_tags(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|t| t != xaga_core::tags::UNREAD)
        .collect();
    let entry = entries::set_entry_tags(&data_dir, &id, tags)
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
    let data_dir = state.library.read().unwrap().clone();
    let mut tags = entries::read_entry_tags(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    if !xaga_core::tags::is_unread(&tags) {
        tags.push(xaga_core::tags::UNREAD.to_string());
    }
    let entry = entries::set_entry_tags(&data_dir, &id, tags)
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
    let data_dir = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    items::list_all_items(&data_dir, item_filter)
        .await
        .map_err(|e| e.to_string())
}
