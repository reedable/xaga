use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    AppState, SettingsOutcome, stale_view_error, update_settings, update_theme_checkmarks,
    update_view_checkmarks,
};

#[derive(Serialize)]
pub struct HtmlReaderPrefs {
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
    theme: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.theme = Some(theme.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.theme.write().unwrap() = theme.clone();
            update_theme_checkmarks(&app, &theme);
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:theme", &theme).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_refresh_on_startup(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.refresh_on_startup.read().unwrap())
}

#[tauri::command]
pub async fn set_refresh_on_startup(
    value: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.refresh_on_startup = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.refresh_on_startup.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:refresh-on-startup", value).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_html_reader_prefs(state: State<'_, AppState>) -> Result<HtmlReaderPrefs, String> {
    Ok(HtmlReaderPrefs {
        font_family: state.html_reader_font_family.read().unwrap().clone(),
        font_size: *state.html_reader_font_size.read().unwrap(),
        line_height: *state.html_reader_line_height.read().unwrap(),
    })
}

#[tauri::command]
pub async fn set_html_reader_font_family(
    value: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.html_reader_font_family = Some(value.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_font_family.write().unwrap() = value.clone();
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:html-reader-font-family", &value).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn set_html_reader_font_size(
    value: f64,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.html_reader_font_size = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_font_size.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:html-reader-font-size", value).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn set_html_reader_line_height(
    value: f64,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.html_reader_line_height = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_line_height.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:html-reader-line-height", value).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_show_sidebar(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.show_sidebar.read().unwrap())
}

#[tauri::command]
pub async fn get_html_reader_view(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.html_reader_view.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_html_reader_view(
    value: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.html_reader_view = Some(value.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_view.write().unwrap() = value.clone();
            update_view_checkmarks(&app, &value);
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:html-reader-view", &value).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_inline_pdf_reader(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.inline_pdf_reader.read().unwrap())
}

#[tauri::command]
pub async fn set_inline_pdf_reader(
    value: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.inline_pdf_reader = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.inline_pdf_reader.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:inline-pdf-reader", value).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_allowed_domains(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.allowed_domains.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_allowed_domains(
    domains: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    match update_settings(&state, |s| s.allowed_domains = Some(domains.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.allowed_domains.write().unwrap() = domains.clone();
            if let Some(main) = app.get_webview_window("main") {
                main.emit("settings:allowed-domains", &domains).ok();
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn set_show_sidebar(
    value: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    match update_settings(&state, |s| s.show_sidebar = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.show_sidebar.write().unwrap() = value;
            Ok(())
        }
    }
}
