use std::path::PathBuf;

use tauri::{AppHandle, State};
use xaga_core::embeddings::ShutdownReason;

use crate::{AppState, SettingsOutcome, restart_process, stale_view_error, update_settings};

#[tauri::command]
pub async fn get_library(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.library_path().to_string_lossy().to_string())
}

/// Whether this instance is read-only for the open library (another xaga process holds
/// the library lock). Same name as the `AppState` accessor and the `library:mode-changed`
/// event field — one concept, one name at every layer.
#[tauri::command]
pub async fn is_library_read_only(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_library_read_only())
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
        crate::events::emit_domain(&app, xaga_core::events::DomainEvent::EmbeddingShutdown);
        let library = std::sync::Arc::clone(&state.library);
        tauri::async_runtime::spawn(async move {
            match svc.shutdown().await {
                ShutdownReason::Restart => {
                    // Release our hold on the (old) library's lock before the successor
                    // process starts and tries to acquire the new library's.
                    crate::release_library_lock(&library);
                    restart_process()
                }
                ShutdownReason::Exit => std::process::exit(0),
            }
        });
    } else {
        // Read-only instance: no service to drain. Restart directly so the successor
        // process attempts the NEW library's lock at its startup — a read-only instance
        // may become the writer of a different library. (Without this, a read-only
        // instance would keep running against the old library path.)
        crate::release_library_lock(&state.library);
        restart_process();
    }

    Ok(new_path.to_string_lossy().to_string())
}
