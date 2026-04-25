use tauri::State;
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::topics::{self, Topic};

use crate::{get_unread, AppState};
use super::require_main_window;

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
    topics::save_topic(&data_dir, &id, &title, &content, threshold)
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
    if !topics.is_empty() && !unread.is_empty() {
        if let Ok(db) = EmbeddingsDb::open(&data_dir).await {
            for topic in &mut topics {
                if let Ok(related) = db.find_related(&topic.id, None, topic.threshold.or(Some(0.8)), None).await {
                    topic.entry_count = related.len();
                    topic.unread_count = related.iter().filter(|e| unread.contains(&e.entry.id)).count();
                }
            }
        }
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
