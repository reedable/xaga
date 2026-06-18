use serde::Serialize;
use tauri::State;
use xaga_core::bookmarks;
use xaga_core::embeddings::{EntityRef, Job, Tier};
use xaga_core::entries;
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
    let current = entries::read_entry(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    let mut tags = current.tags;
    if !tags.contains(&"bookmark".to_string()) {
        tags.push("bookmark".to_string());
    }
    entries::set_entry_tags(&data_dir, &id, tags)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_bookmark(
    id: String,
    state: State<'_, AppState>,
) -> Result<entries::Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let current = entries::read_entry(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    let tags: Vec<String> = current.tags.into_iter().filter(|t| t != "bookmark").collect();
    entries::set_entry_tags(&data_dir, &id, tags)
        .await
        .map_err(|e| e.to_string())
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

    // Import writes entry files directly; enqueue their indexing explicitly (bulk →
    // Maintenance tier) so imported bookmarks get indexed without the watcher.
    {
        let svc = state.embedding_service.lock().unwrap();
        if let Some(svc) = svc.as_ref() {
            for id in result.added.iter().chain(result.updated.iter()) {
                svc.enqueue(
                    Tier::Maintenance,
                    Job::UpsertContent { entity: EntityRef::new(id.clone(), Kind::Entry) },
                );
            }
        }
    }

    Ok(BookmarkImportResult {
        added: result.added,
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
