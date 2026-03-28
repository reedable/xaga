use tauri::{AppHandle, Manager, State};

use crate::{AppState, load_settings, save_settings};

#[tauri::command]
pub async fn get_theme(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.theme.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_theme(
    theme: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.theme = Some(theme.clone());
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.theme.write().unwrap() = theme;

    Ok(())
}
