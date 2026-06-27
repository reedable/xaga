use serde::Serialize;
use tauri::State;
use xaga_core::persistence::bookmarks;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::entries;
use xaga_core::types::{Entry, Kind};

use crate::AppState;

#[derive(Serialize)]
pub struct BookmarkImportResult {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub failed: Vec<String>,
}

#[tauri::command]
pub async fn add_bookmark(
    id: String,
    state: State<'_, AppState>,
) -> Result<Entry, String> {
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
            state: StatePatch { is_bookmarked: Some(true), ..Default::default() },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub async fn remove_bookmark(
    id: String,
    state: State<'_, AppState>,
) -> Result<Entry, String> {
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
            state: StatePatch { is_bookmarked: Some(false), ..Default::default() },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub async fn import_bookmarks(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<BookmarkImportResult, String> {
    let data_dir = state.library.read().unwrap().clone();
    let html = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| e.to_string())?;
    let result = bookmarks::import_bookmarks(&data_dir, &html)
        .await
        .map_err(|e| e.to_string())?;

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
                state: StatePatch { is_bookmarked: Some(true), ..Default::default() },
            },
        );
    }

    Ok(BookmarkImportResult {
        added: added_ids,
        updated: result.updated,
        failed: result.failed,
    })
}

#[tauri::command]
pub async fn export_bookmarks(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.library.read().unwrap().clone();
    bookmarks::export_bookmarks(&data_dir, std::path::Path::new(&file_path))
        .await
        .map_err(|e| e.to_string())
}
