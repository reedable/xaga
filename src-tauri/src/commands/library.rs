use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};
use xaga_core::embeddings::ShutdownReason;

use crate::{AppState, SettingsOutcome, restart_process, stale_view_error, update_settings};

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

    match update_settings(&state, |s| {
        s.library = Some(new_path.to_string_lossy().to_string())
    })? {
        SettingsOutcome::StaleView(fresh) => return Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {}
    }

    // The new library path is now persisted.  Signal the embedding service to
    // restart, wait for the run loop to finish its current in-flight LanceDB
    // write and exit cleanly, then spawn a fresh process which boots against
    // the updated settings.json.  We do not update state.library here — the
    // new process will read it from disk on startup.
    if let Some(svc) = state.embedding_service.lock().unwrap().take() {
        svc.request_restart();
        app.emit("embedding:shutdown", ()).ok();
        tauri::async_runtime::spawn(async move {
            match svc.shutdown().await {
                ShutdownReason::Restart => restart_process(),
                ShutdownReason::Exit => std::process::exit(0),
            }
        });
    }

    Ok(new_path.to_string_lossy().to_string())
}
