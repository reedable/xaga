use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::types::{Entry, Kind};

use crate::AppState;
use crate::error::AppError;

#[tauri::command]
pub async fn add_bookmark(
    id: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<Entry, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .set_entry_color(WriteTier::Interactive, id.clone(), Some(color))
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                color: Some(entry.color.clone()),
                ..Default::default()
            },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub async fn remove_bookmark(id: String, state: State<'_, AppState>) -> Result<Entry, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let entry = writer
        .set_entry_color(WriteTier::Interactive, id.clone(), None)
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch {
                color: Some(None),
                ..Default::default()
            },
        },
    );
    Ok(entry)
}
