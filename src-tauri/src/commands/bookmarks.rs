use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::{TagPatch, Tier as WriteTier};
use xaga_core::types::{Entry, Kind};

use crate::AppState;

#[tauri::command]
pub async fn add_bookmark(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .patch_entry_tags(WriteTier::Interactive, id.clone(), TagPatch::AddBookmark)
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
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .patch_entry_tags(WriteTier::Interactive, id.clone(), TagPatch::RemoveBookmark)
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

