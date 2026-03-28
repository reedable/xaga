use tauri::State;
use xaga_core::feeds;
use xaga_core::item::{self, ItemSource};
use xaga_core::types::Entry;

use crate::{feeds_dir, AppState};
use super::parse_item_filter;

#[tauri::command]
pub async fn list_feeds(state: State<'_, AppState>) -> Result<Vec<feeds::Feed>, String> {
    let dir = feeds_dir(&state.library.read().unwrap());
    feeds::list_feeds(&dir).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_feed(url: String, state: State<'_, AppState>) -> Result<feeds::Feed, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    feeds::add_feed(&state.http_client, &dir, &library, &url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_feed(id: String, state: State<'_, AppState>) -> Result<feeds::Feed, String> {
    let library = state.library.read().unwrap().clone();
    let dir = feeds_dir(&library);
    feeds::refresh_feed(&state.http_client, &dir, &library, &feeds::FeedId(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_feed(id: String, title: String, state: State<'_, AppState>) -> Result<feeds::Feed, String> {
    let dir = feeds_dir(&state.library.read().unwrap());
    feeds::rename_feed(&dir, &feeds::FeedId(id), &title)
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
pub async fn list_entries(
    url: Option<String>,
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Entry>, String> {
    let library = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;
    item::list_items(&library, ItemSource::FeedEntries { url }, item_filter)
        .await
        .map_err(|e| e.to_string())
}
