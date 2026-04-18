use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use xaga_core::embeddings::EmbeddingService;

use crate::{AppState, file_watchers, load_settings, save_settings};
use super::require_main_window;

#[tauri::command]
pub async fn get_library(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.library.read().unwrap().to_string_lossy().to_string())
}

#[tauri::command]
pub async fn set_library(
    window: tauri::Window,
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    require_main_window(&window)?;
    let new_path = PathBuf::from(&path);

    std::fs::create_dir_all(&new_path).map_err(|e| e.to_string())?;

    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.library = Some(new_path.to_string_lossy().to_string());
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.library.write().unwrap() = new_path.clone();

    // Start a new EmbeddingService. Replacing the stored instance drops the old
    // one, closing its channels → loop exits.
    let library = new_path.clone();
    let svc_store = Arc::clone(&state.embedding_service);

    tauri::async_runtime::spawn(async move {
        match EmbeddingService::start(library.clone()).await {
            Ok(svc) => {
                file_watchers::watch(library.clone(), svc.priority_sender(), svc.background_sender());
                svc.trigger_recovery(library.clone());
                *svc_store.lock().unwrap() = Some(svc);
            }

            Err(e) => log::error!("set_library: embedding service failed: {e}"),
        }
    });

    Ok(new_path.to_string_lossy().to_string())
}
