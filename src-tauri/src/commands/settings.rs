use std::collections::HashMap;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{AppState, load_settings, save_settings, update_theme_checkmarks};
use super::require_main_window;

#[derive(Serialize)]
pub struct ReaderPrefs {
    #[serde(rename = "fontFamily")]
    pub font_family: String,
    #[serde(rename = "fontSize")]
    pub font_size: f64,
    #[serde(rename = "lineHeight")]
    pub line_height: f64,
}

#[tauri::command]
pub async fn get_theme(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.theme.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_theme(
    window: tauri::Window,
    theme: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.theme = Some(theme.clone());
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.theme.write().unwrap() = theme.clone();

    update_theme_checkmarks(&app, &theme);
    if let Some(main) = app.get_webview_window("main") {
        main.emit("settings:theme", &theme).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn get_refresh_on_startup(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.refresh_on_startup.read().unwrap())
}

#[tauri::command]
pub async fn set_refresh_on_startup(
    window: tauri::Window,
    value: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.refresh_on_startup = Some(value);
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.refresh_on_startup.write().unwrap() = value;

    if let Some(main) = app.get_webview_window("main") {
        main.emit("settings:refresh-on-startup", value).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn get_reader_prefs(state: State<'_, AppState>) -> Result<ReaderPrefs, String> {
    Ok(ReaderPrefs {
        font_family: state.reader_font_family.read().unwrap().clone(),
        font_size: *state.reader_font_size.read().unwrap(),
        line_height: *state.reader_line_height.read().unwrap(),
    })
}

#[tauri::command]
pub async fn set_reader_font_family(
    window: tauri::Window,
    value: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.reader_font_family = Some(value.clone());
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.reader_font_family.write().unwrap() = value.clone();

    if let Some(main) = app.get_webview_window("main") {
        main.emit("settings:reader-font-family", &value).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn set_reader_font_size(
    window: tauri::Window,
    value: f64,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.reader_font_size = Some(value);
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.reader_font_size.write().unwrap() = value;

    if let Some(main) = app.get_webview_window("main") {
        main.emit("settings:reader-font-size", value).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn set_reader_line_height(
    window: tauri::Window,
    value: f64,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.reader_line_height = Some(value);
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.reader_line_height.write().unwrap() = value;

    if let Some(main) = app.get_webview_window("main") {
        main.emit("settings:reader-line-height", value).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn get_show_sidebar(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.show_sidebar.read().unwrap())
}

#[tauri::command]
pub async fn get_ui_pref(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    Ok(state.ui_prefs.read().unwrap().get(&key).cloned())
}

#[tauri::command]
pub async fn set_ui_pref(
    window: tauri::Window,
    key: String,
    value: serde_json::Value,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");
    let mut settings = load_settings(&settings_path);
    settings
        .ui_prefs
        .get_or_insert_with(HashMap::new)
        .insert(key.clone(), value.clone());
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;
    state.ui_prefs.write().unwrap().insert(key, value);
    Ok(())
}

#[tauri::command]
pub async fn get_allowed_domains(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.allowed_domains.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_allowed_domains(
    window: tauri::Window,
    domains: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.allowed_domains = Some(domains.clone());
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.allowed_domains.write().unwrap() = domains.clone();

    if let Some(main) = app.get_webview_window("main") {
        main.emit("settings:allowed-domains", &domains).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn set_show_sidebar(
    window: tauri::Window,
    value: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    let settings_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");

    let mut settings = load_settings(&settings_path);
    settings.show_sidebar = Some(value);
    save_settings(&settings_path, &settings).map_err(|e| e.to_string())?;

    *state.show_sidebar.write().unwrap() = value;

    Ok(())
}
