use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, State};
use xaga_core::embeddings::ShutdownReason;

use crate::{AppState, load_settings, restart_process, save_settings};

#[tauri::command]
pub async fn get_library(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.library.read().unwrap().to_string_lossy().to_string())
}

#[tauri::command]
pub async fn set_library(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
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

    // The new library path is now persisted.  Signal the embedding service to
    // restart, wait for the run loop to finish its current in-flight LanceDB
    // write and exit cleanly, then spawn a fresh process which boots against
    // the updated settings.json.  We do not update state.library here — the
    // new process will read it from disk on startup.
    if let Some(svc) = state.embedding_service.lock().unwrap().take() {
        svc.request_restart();
        app.emit("embedding:shutdown", ()).ok();
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_handle.state::<AppState>();
            let cache = state.entry_cache.lock().unwrap().clone();
            let data_dir = state.library.read().unwrap().clone();
            match svc.shutdown(cache.as_ref(), &data_dir).await {
                ShutdownReason::Restart => restart_process(),
                ShutdownReason::Exit => std::process::exit(0),
            }
        });
    }

    Ok(new_path.to_string_lossy().to_string())
}
