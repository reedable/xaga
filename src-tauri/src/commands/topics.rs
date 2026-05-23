use tauri::State;
use xaga_core::item::{self, EmbeddedItemSource};
use xaga_core::topics::{self, Topic};
use xaga_core::types::{CollectionItem, ItemFilter, SavedItem};

use crate::{get_unread, AppState};
use super::{parse_item_filter, require_main_window};

#[tauri::command]
pub async fn save_topic(
    window: tauri::Window,
    id: String,
    title: String,
    content: String,
    threshold: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    // Preserve existing manual items when editing from the UI
    let existing_items = topics::read_topic(&data_dir, &id).await
        .map(|t| t.items)
        .unwrap_or_default();
    topics::save_topic(&data_dir, &id, &title, &content, threshold, existing_items)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_topic(
    window: tauri::Window,
    title: String,
    content: String,
    threshold: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    topics::create_topic(&data_dir, &title, &content, threshold)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_topics(state: State<'_, AppState>) -> Result<Vec<Topic>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let mut topics = topics::list_topics(&data_dir)
        .await
        .map_err(|e| e.to_string())?;

    let unread = get_unread(&state);

    for topic in &mut topics {
        let mut all_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Semantic matches via proven path — no content guard, title-only embedding is valid.
        if let Ok(embedded) = item::list_embedded_items(
            &data_dir,
            EmbeddedItemSource::RelatedTo {
                id: topic.id.clone(),
                limit: None,
                min_similarity: topic.threshold.or(Some(0.8)),
                max_similarity: None,
            },
            ItemFilter::All,
        )
        .await
        {
            for e in &embedded {
                all_ids.insert(e.entry.id.clone());
            }
        }

        // Manual items — both entries and annotations contribute to the count.
        for item in &topic.items {
            match item {
                CollectionItem::Entry { id } => { all_ids.insert(id.clone()); }
                CollectionItem::Annotation { id, .. } => { all_ids.insert(id.clone()); }
            }
        }

        topic.entry_count = all_ids.len();
        topic.unread_count = all_ids.iter().filter(|id| unread.contains(*id)).count();
    }

    Ok(topics)
}

#[tauri::command]
pub async fn read_topic(id: String, state: State<'_, AppState>) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::read_topic(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_topic(window: tauri::Window, id: String, state: State<'_, AppState>) -> Result<(), String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    topics::delete_topic(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_topic_item(
    window: tauri::Window,
    topic_id: String,
    item: CollectionItem,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    topics::add_topic_item(&data_dir, &topic_id, item)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_topic_item(
    window: tauri::Window,
    topic_id: String,
    item_id: String,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    require_main_window(&window)?;
    let data_dir = state.library.read().unwrap().clone();
    topics::remove_topic_item(&data_dir, &topic_id, &item_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_topic_items(
    id: String,
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SavedItem>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;
    topics::list_topic_items(&data_dir, &id, item_filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_entry_topics(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::list_entry_topics(&data_dir, &entry_id)
        .await
        .map_err(|e| e.to_string())
}
