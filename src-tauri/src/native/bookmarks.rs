//! Bookmark HTML import/export, orchestrated entirely on the native side.
//!
//! Same rationale as `super::feeds`: no path is accepted from the webview.
//! See docs/architecture/security/command_surface.md and docs/architecture/layer_ownership.md.

use std::path::Path;

use serde::Serialize;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::bookmarks;
use xaga_core::types::Kind;

use crate::AppState;
use crate::error::AppError;

#[derive(Serialize, Clone)]
pub(crate) struct BookmarkImportResult {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub failed: Vec<String>,
}

pub(crate) async fn import_bookmarks(
    state: &AppState,
    file_path: &Path,
) -> Result<BookmarkImportResult, AppError> {
    let writer = crate::commands::require_writer(state)?;

    let max_bytes = *state.import_max_bytes.read().unwrap();
    let metadata = tokio::fs::metadata(file_path)
        .await
        .map_err(|e| e.to_string())?;
    if metadata.len() > max_bytes {
        // TODO(phase 3): typed, user-actionable variant directing to Settings.
        return Err(AppError::Message(format!(
            "import file is {} bytes, exceeding the {} byte limit",
            metadata.len(),
            max_bytes
        )));
    }

    let html = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|e| e.to_string())?;
    let result = bookmarks::import_bookmarks(&writer, &html)
        .await
        .map_err(AppError::from)?;

    // Newly-created entries carry their parsed payload — enqueue content indexing so the consumer
    // never re-reads. This is user-initiated CRUD (the user ran the import and is waiting), so the
    // upsert rides Interactive (T1); its embedding follows to Regular via `embed_follow`, per the
    // priority definitions in the embedding service's `run_loop`. Already-existing entries only
    // gained the bookmark tag, which is a state change → SyncState (Interactive).
    let added_ids: Vec<String> = result.added.iter().map(|a| a.meta.id.clone()).collect();
    for article in result.added {
        state.enqueue(Tier::Interactive, Job::UpsertEntry { article });
    }
    for id in &result.updated {
        state.enqueue(
            Tier::Interactive,
            Job::SyncState {
                entity: EntityRef::new(id.clone(), Kind::Entry),
                state: StatePatch {
                    is_bookmarked: Some(true),
                    ..Default::default()
                },
            },
        );
    }

    Ok(BookmarkImportResult {
        added: added_ids,
        updated: result.updated,
        failed: result.failed,
    })
}

pub(crate) async fn export_bookmarks(state: &AppState, file_path: &Path) -> Result<(), AppError> {
    let data_dir = state.library_path();
    bookmarks::export_bookmarks(&data_dir, file_path)
        .await
        .map_err(AppError::from)
}
