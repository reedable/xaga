use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::entries;
use xaga_core::types::{Entry, Kind};

use crate::AppState;

#[tauri::command]
pub async fn add_bookmark(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let mut tags = entries::read_entry_tags(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    if !xaga_core::tags::is_bookmarked(&tags) {
        tags.push(xaga_core::tags::BOOKMARK.to_string());
    }
    let entry = entries::set_entry_tags(&data_dir, &id, tags)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                is_bookmarked: Some(true),
                ..Default::default()
            },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub async fn remove_bookmark(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let tags: Vec<String> = entries::read_entry_tags(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|t| t != xaga_core::tags::BOOKMARK)
        .collect();
    let entry = entries::set_entry_tags(&data_dir, &id, tags)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                is_bookmarked: Some(false),
                ..Default::default()
            },
        },
    );
    Ok(entry)
}

