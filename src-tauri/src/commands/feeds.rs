use std::collections::HashSet;

use serde::Serialize;
use tauri::{Emitter, State};
use xaga_core::feeds;
use xaga_core::types::Entry;

use crate::{feeds_dir, get_bookmarked, get_unread, AppState};
use super::parse_item_filter;

#[derive(Serialize)]
pub struct ImportResult {
    pub added: Vec<feeds::Feed>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

#[tauri::command]
pub async fn list_feeds(state: State<'_, AppState>) -> Result<Vec<feeds::Feed>, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    let (unread, bookmarked) = (get_unread(&state).await, get_bookmarked(&state).await);
    feeds::list_feeds(&dir, &unread, &bookmarked).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_feed(url: String, state: State<'_, AppState>) -> Result<feeds::Feed, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    let (unread, bookmarked) = (get_unread(&state).await, get_bookmarked(&state).await);
    feeds::add_feed(&state.http_client, &dir, &library, &url, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_feed(window: tauri::Window, id: String, state: State<'_, AppState>) -> Result<feeds::Feed, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    let (unread, bookmarked) = (get_unread(&state).await, get_bookmarked(&state).await);
    let feed = feeds::refresh_feed(&state.http_client, &dir, &library, &feeds::FeedId(id), &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())?;
    window.emit("feed:refreshed", &feed).ok();
    Ok(feed)
}

#[tauri::command]
pub async fn set_feed_pinned(id: String, pinned: bool, state: State<'_, AppState>) -> Result<(), String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    feeds::set_feed_pinned(&dir, &feeds::FeedId(id), pinned)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_feed(id: String, title: String, state: State<'_, AppState>) -> Result<feeds::Feed, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    let (unread, bookmarked) = (get_unread(&state).await, get_bookmarked(&state).await);
    feeds::rename_feed(&dir, &feeds::FeedId(id), &title, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_feed(url: String, state: State<'_, AppState>) -> Result<String, String> {
    let dir = feeds_dir(&state.library.read().unwrap());
    let feed_id = feeds::feed_id_from_url(&url).map_err(|e| e.to_string())?;
    feeds::remove_feed(&dir, &feed_id).await.map_err(|e| e.to_string())?;
    Ok(url)
}

#[tauri::command]
pub async fn import_feeds(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);

    let xml = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| e.to_string())?;

    let raw_urls = xaga_core::opml::parse_opml(&xml).map_err(|e| e.to_string())?;

    let (unread, bookmarked) = (get_unread(&state).await, get_bookmarked(&state).await);
    let mut seen: HashSet<String> = HashSet::new();
    let mut added: Vec<feeds::Feed> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for raw in raw_urls {
        let normalized = match xaga_core::util::normalize_url(&raw) {
            Ok(u) => u,
            Err(e) => {
                log::warn!("import_feeds: invalid URL {raw:?}: {e}");
                failed.push(raw);
                continue;
            }
        };

        if !seen.insert(normalized.clone()) {
            skipped.push(normalized);
            continue;
        }

        match feeds::add_feed(&state.http_client, &dir, &library, &normalized, &unread, &bookmarked).await {
            Ok(feed) => added.push(feed),
            Err(e) => {
                log::warn!("import_feeds: failed to add {normalized:?}: {e}");
                failed.push(normalized);
            }
        }
    }

    Ok(ImportResult { added, skipped, failed })
}

#[tauri::command]
pub async fn export_feeds(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);

    let (unread, bookmarked) = (get_unread(&state).await, get_bookmarked(&state).await);
    let feed_list = feeds::list_feeds(&dir, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())?;

    let pairs: Vec<(&str, &str)> = feed_list
        .iter()
        .map(|f| (f.url.as_str(), f.title.as_str()))
        .collect();

    let opml = xaga_core::opml::build_opml(&pairs);

    tokio::fs::write(&file_path, opml)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_entries(
    url: Option<String>,
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Entry>, String> {
    let library = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    // Phase C: the .atom is the source of truth for feed membership (the id-set); LanceDB
    // returns those entries with their state. Reload the read handle so state reflects the
    // service's latest writes, then filter/sort as before.
    let ids = feeds::list_entry_ids(&library, url.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Err("embeddings read handle unavailable".to_string());
    };
    db.reload().await.map_err(|e| e.to_string())?;

    let entries = db.entries_by_ids(&ids).await.map_err(|e| e.to_string())?;
    let mut items = xaga_core::filter::filter_items(entries, &item_filter)
        .await
        .map_err(|e| e.to_string())?;
    xaga_core::filter::sort_items(&mut items);
    Ok(items)
}
