use tauri::State;
use xaga_core::collections::{self, Collection, CollectionItem};
use xaga_core::types::SavedItem;

use crate::{get_bookmarked, get_unread, AppState};
use super::{parse_item_filter, require_main_window};

#[tauri::command]
pub async fn create_collection(
    window: tauri::Window,
    title: String,
    state: State<'_, AppState>,
) -> Result<Collection, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    collections::create_collection(&data_dir, &title)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_collection(
    window: tauri::Window,
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<Collection, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    let existing = collections::read_collection(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    collections::save_collection(&data_dir, &id, &title, existing.items)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let unread = get_unread(&state);
    let mut collections = collections::list_collections(&data_dir)
        .await
        .map_err(|e| e.to_string())?;

    if !unread.is_empty() {
        for collection in &mut collections {
            collection.unread_count = collection.items.iter().filter(|i| {
                matches!(i, CollectionItem::Entry { id } if unread.contains(id))
            }).count();
        }
    }

    Ok(collections)
}

#[tauri::command]
pub async fn list_entry_collections(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let data_dir = state.library.read().unwrap().clone();
    collections::list_entry_collections(&data_dir, &entry_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_collection_item(
    window: tauri::Window,
    collection_id: String,
    item: CollectionItem,
    state: State<'_, AppState>,
) -> Result<Collection, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    collections::add_item(&data_dir, &collection_id, item)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_collection_item(
    window: tauri::Window,
    collection_id: String,
    item_id: String,
    state: State<'_, AppState>,
) -> Result<Collection, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    collections::remove_item(&data_dir, &collection_id, &item_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_collection_items(
    id: String,
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SavedItem>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;
    let unread = get_unread(&state);
    let bookmarked = get_bookmarked(&state);
    collections::list_collection_items(&data_dir, &id, item_filter, &unread, &bookmarked)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_collection(
    window: tauri::Window,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    collections::delete_collection(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}
