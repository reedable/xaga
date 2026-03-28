use tauri::State;
use xaga_core::topics::{self, Topic};

use crate::AppState;

#[tauri::command]
pub async fn save_topic(
    id: String,
    title: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::save_topic(&data_dir, &id, &title, &content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_topic(
    title: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::create_topic(&data_dir, &title, &content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_topics(state: State<'_, AppState>) -> Result<Vec<Topic>, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::list_topics(&data_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_topic(id: String, state: State<'_, AppState>) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::read_topic(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_topic(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::delete_topic(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}
