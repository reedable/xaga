use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
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
pub async fn get_theme(state: State<'_, AppState>) -> Result<String, AppError> {
    Ok(state.theme.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_theme(
    theme: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.theme = Some(theme.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.theme.write().unwrap() = theme.clone();
            update_theme_checkmarks(&app, &theme);
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsTheme(theme.clone()),
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_refresh_on_startup(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(*state.refresh_on_startup.read().unwrap())
}

#[tauri::command]
pub async fn set_refresh_on_startup(
    value: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.refresh_on_startup = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.refresh_on_startup.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsRefreshOnStartup(value),
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_html_reader_prefs(
    state: State<'_, AppState>,
) -> Result<HtmlReaderPrefs, AppError> {
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
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.html_reader_font_family = Some(value.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_font_family.write().unwrap() = value.clone();
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsHtmlReaderFontFamily(value.clone()),
                );
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
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.html_reader_font_size = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_font_size.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsHtmlReaderFontSize(value),
                );
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
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.html_reader_line_height = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_line_height.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsHtmlReaderLineHeight(value),
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_show_sidebar(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(*state.show_sidebar.read().unwrap())
}

#[tauri::command]
pub async fn get_html_reader_view(state: State<'_, AppState>) -> Result<String, AppError> {
    Ok(state.html_reader_view.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_html_reader_view(
    value: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.html_reader_view = Some(value.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.html_reader_view.write().unwrap() = value.clone();
            update_view_checkmarks(&app, &value);
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsHtmlReaderView(value.clone()),
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_inline_pdf_reader(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(*state.inline_pdf_reader.read().unwrap())
}

#[tauri::command]
pub async fn set_inline_pdf_reader(
    value: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.inline_pdf_reader = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.inline_pdf_reader.write().unwrap() = value;
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsInlinePdfReader(value),
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_allowed_domains(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    Ok(state.allowed_domains.read().unwrap().clone())
}

#[tauri::command]
pub async fn set_allowed_domains(
    domains: Vec<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.allowed_domains = Some(domains.clone()))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.allowed_domains.write().unwrap() = domains.clone();
            if let Some(main) = app.get_webview_window("main") {
                crate::events::emit_app(
                    &main,
                    crate::events::AppEvent::SettingsAllowedDomains(domains.clone()),
                );
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn set_show_sidebar(value: bool, state: State<'_, AppState>) -> Result<(), AppError> {
    match update_settings(&state, |s| s.show_sidebar = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.show_sidebar.write().unwrap() = value;
            Ok(())
        }
    }
}

/// Resolved opt-in preference for additional article image requests.
#[tauri::command]
pub async fn get_load_remote_image(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(*state.load_remote_image.read().unwrap())
}

#[tauri::command]
pub async fn set_load_remote_image(
    value: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    match update_settings(&state, |s| s.load_remote_image = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.load_remote_image.write().unwrap() = value;
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn get_import_max_bytes(state: State<'_, AppState>) -> Result<u64, AppError> {
    Ok(*state.import_max_bytes.read().unwrap())
}

fn validate_import_max_bytes(value: u64) -> Result<(), AppError> {
    // The UI transports bytes as a JavaScript number; reject values it cannot
    // represent exactly rather than persisting a rounded limit.
    if value == 0 || value % (1024 * 1024) != 0 || value > 9_007_199_254_740_991 {
        return Err(AppError::InvalidImportLimit);
    }
    Ok(())
}

#[tauri::command]
pub async fn set_import_max_bytes(value: u64, state: State<'_, AppState>) -> Result<(), AppError> {
    validate_import_max_bytes(value)?;
    match update_settings(&state, |s| s.import_max_bytes = Some(value))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            *state.import_max_bytes.write().unwrap() = value;
            Ok(())
        }
    }
}

#[cfg(test)]
mod import_limit_tests {
    use super::*;

    #[test]
    fn import_limit_must_be_positive_whole_mib_and_exact_in_the_ui() {
        for value in [
            1024 * 1024,
            crate::DEFAULT_IMPORT_MAX_BYTES,
            128 * 1024 * 1024,
            8_589_934_591 * 1024 * 1024,
        ] {
            assert!(validate_import_max_bytes(value).is_ok());
        }
        for value in [0, 1, 512 * 1024, 9_007_199_254_740_992, u64::MAX] {
            assert!(matches!(
                validate_import_max_bytes(value),
                Err(AppError::InvalidImportLimit)
            ));
        }
    }
}

#[tauri::command]
pub async fn get_sidebar_sort(
    state: State<'_, AppState>,
) -> Result<crate::sidebar_sort::SidebarSort, AppError> {
    Ok(*state.sidebar_sort.read().unwrap())
}

pub(crate) fn save_sidebar_sort(
    app: &AppHandle,
    state: &AppState,
    value: crate::sidebar_sort::SidebarSort,
) -> Result<(), AppError> {
    match update_settings(state, |s| s.sidebar_sort = Some(value))? {
        SettingsOutcome::StaleView(fresh) => {
            let error = stale_view_error(state, &fresh);
            let current = *state.sidebar_sort.read().unwrap();
            crate::sidebar_sort::update(app, current);
            crate::events::emit_app(app, crate::events::AppEvent::SettingsSidebarSort(current));
            Err(error)
        }
        SettingsOutcome::Written(_) => {
            *state.sidebar_sort.write().unwrap() = value;
            crate::sidebar_sort::update(app, value);
            crate::events::emit_app(app, crate::events::AppEvent::SettingsSidebarSort(value));
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn set_sidebar_sort(
    value: crate::sidebar_sort::SidebarSort,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    save_sidebar_sort(&app, &state, value)
}

/// Decimal strings keep the full u64 range exact across JavaScript IPC.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchSettings {
    referer_allowed: bool,
    gzip_allowed: bool,
    http_allowed: bool,
    connect_nonstandard_ports_allowed: bool,
    max_redirects: String,
    loader_timeout_secs: String,
    feed_timeout_secs: String,
    feed_max_body_bytes: String,
}

#[derive(serde::Deserialize)]
#[serde(tag = "setting", content = "value", rename_all = "camelCase")]
pub(crate) enum FetchSetting {
    RefererAllowed(bool),
    GzipAllowed(bool),
    HttpAllowed(bool),
    ConnectNonstandardPortsAllowed(bool),
    MaxRedirects(#[serde(deserialize_with = "decimal_u64")] u64),
    LoaderTimeoutSecs(#[serde(deserialize_with = "decimal_u64")] u64),
    FeedTimeoutSecs(#[serde(deserialize_with = "decimal_u64")] u64),
    FeedMaxBodyBytes(#[serde(deserialize_with = "decimal_u64")] u64),
}

fn decimal_u64<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let value = <String as serde::Deserialize>::deserialize(d)?;
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(serde::de::Error::custom("expected decimal whole number"));
    }
    value.parse().map_err(serde::de::Error::custom)
}

impl FetchSetting {
    fn validate(&self) -> Result<(), AppError> {
        if matches!(self, Self::MaxRedirects(value) if *value > u64::from(crate::fetch_client::MAX_REDIRECTS_CEILING))
        {
            return Err(AppError::InvalidFetchSetting);
        }
        Ok(())
    }

    fn apply(&self, settings: &mut crate::UserSettings) {
        match *self {
            Self::RefererAllowed(v) => settings.referer_allowed = Some(v),
            Self::GzipAllowed(v) => settings.gzip_allowed = Some(v),
            Self::HttpAllowed(v) => settings.http_allowed = Some(v),
            Self::ConnectNonstandardPortsAllowed(v) => {
                settings.connect_nonstandard_ports_allowed = Some(v)
            }
            Self::MaxRedirects(v) => settings.max_redirects = Some(v as u32),
            Self::LoaderTimeoutSecs(v) => settings.loader_timeout_secs = Some(v),
            Self::FeedTimeoutSecs(v) => settings.feed_timeout_secs = Some(v),
            Self::FeedMaxBodyBytes(v) => settings.feed_max_body_bytes = Some(v),
        }
    }

    fn apply_to_state(&self, state: &AppState) {
        match *self {
            Self::RefererAllowed(v) => *state.referer_allowed.write().unwrap() = v,
            Self::GzipAllowed(v) => *state.gzip_allowed.write().unwrap() = v,
            Self::HttpAllowed(v) => *state.http_allowed.write().unwrap() = v,
            Self::ConnectNonstandardPortsAllowed(v) => {
                *state.connect_nonstandard_ports_allowed.write().unwrap() = v
            }
            Self::MaxRedirects(v) => *state.max_redirects.write().unwrap() = v as u32,
            Self::LoaderTimeoutSecs(v) => *state.loader_timeout_secs.write().unwrap() = v,
            Self::FeedTimeoutSecs(v) => *state.feed_timeout_secs.write().unwrap() = v,
            Self::FeedMaxBodyBytes(v) => *state.feed_max_body_bytes.write().unwrap() = v,
        }
    }
}

#[tauri::command]
pub(crate) async fn get_fetch_settings(
    state: State<'_, AppState>,
) -> Result<FetchSettings, AppError> {
    Ok(FetchSettings {
        referer_allowed: *state.referer_allowed.read().unwrap(),
        gzip_allowed: *state.gzip_allowed.read().unwrap(),
        http_allowed: *state.http_allowed.read().unwrap(),
        connect_nonstandard_ports_allowed: *state.connect_nonstandard_ports_allowed.read().unwrap(),
        max_redirects: state.max_redirects.read().unwrap().to_string(),
        loader_timeout_secs: state.loader_timeout_secs.read().unwrap().to_string(),
        feed_timeout_secs: state.feed_timeout_secs.read().unwrap().to_string(),
        feed_max_body_bytes: state.feed_max_body_bytes.read().unwrap().to_string(),
    })
}

#[tauri::command]
pub(crate) async fn set_fetch_setting(
    change: FetchSetting,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    change.validate()?;
    match update_settings(&state, |settings| change.apply(settings))? {
        SettingsOutcome::StaleView(fresh) => Err(stale_view_error(&state, &fresh)),
        SettingsOutcome::Written(_) => {
            change.apply_to_state(&state);
            Ok(())
        }
    }
}

#[cfg(test)]
mod fetch_setting_tests {
    use super::*;

    #[test]
    fn fetch_changes_persist_and_update_the_running_handles() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let state = crate::test_app_state::test_state(dir.path().into());
        let registry = std::sync::Mutex::new(crate::AccountLock::default());
        let http_proxy_handle = std::sync::Arc::clone(&state.http_allowed);
        let connect_proxy_handle = std::sync::Arc::clone(&state.connect_nonstandard_ports_allowed);
        let mut baseline = None;
        for change in [
            FetchSetting::RefererAllowed(true),
            FetchSetting::GzipAllowed(true),
            FetchSetting::HttpAllowed(true),
            FetchSetting::ConnectNonstandardPortsAllowed(true),
            FetchSetting::MaxRedirects(0),
            FetchSetting::LoaderTimeoutSecs(u64::MAX),
            FetchSetting::FeedTimeoutSecs(0),
            FetchSetting::FeedMaxBodyBytes(9_007_199_254_740_993),
        ] {
            change.validate().unwrap();
            let (outcome, next) =
                crate::update_settings_file(&registry, &path, baseline, |settings| {
                    change.apply(settings)
                })
                .unwrap();
            assert!(matches!(outcome, SettingsOutcome::Written(_)));
            change.apply_to_state(&state);
            baseline = next;
        }
        let saved = crate::load_settings(&path);
        assert_eq!(saved.http_allowed, Some(true));
        assert_eq!(saved.connect_nonstandard_ports_allowed, Some(true));
        assert_eq!(saved.referer_allowed, Some(true));
        assert_eq!(saved.gzip_allowed, Some(true));
        assert_eq!(saved.max_redirects, Some(0));
        assert_eq!(saved.loader_timeout_secs, Some(u64::MAX));
        assert_eq!(saved.feed_timeout_secs, Some(0));
        assert_eq!(saved.feed_max_body_bytes, Some(9_007_199_254_740_993));
        assert!(*http_proxy_handle.read().unwrap());
        assert!(*connect_proxy_handle.read().unwrap());
        assert!(*state.referer_allowed.read().unwrap());
        assert!(*state.gzip_allowed.read().unwrap());
        assert_eq!(*state.max_redirects.read().unwrap(), 0);
        assert_eq!(*state.loader_timeout_secs.read().unwrap(), u64::MAX);
        assert_eq!(*state.feed_timeout_secs.read().unwrap(), 0);
        assert_eq!(
            *state.feed_max_body_bytes.read().unwrap(),
            9_007_199_254_740_993
        );
        crate::refresh_state_from_settings(&state, &crate::UserSettings::default());
        assert!(!*http_proxy_handle.read().unwrap());
        assert!(!*connect_proxy_handle.read().unwrap());
        assert_eq!(*state.max_redirects.read().unwrap(), 3);
        assert_eq!(*state.loader_timeout_secs.read().unwrap(), 10);
        assert_eq!(*state.feed_timeout_secs.read().unwrap(), 10);
        assert_eq!(*state.feed_max_body_bytes.read().unwrap(), 10 * 1024 * 1024);
    }

    #[test]
    fn numeric_settings_preserve_u64_without_new_policy_limits() {
        for setting in ["loaderTimeoutSecs", "feedTimeoutSecs", "feedMaxBodyBytes"] {
            for value in ["0", "10", "9007199254740993", "18446744073709551615"] {
                let change: FetchSetting =
                    serde_json::from_value(serde_json::json!({"setting": setting, "value": value}))
                        .unwrap();
                change.validate().unwrap();
                let mut settings = crate::UserSettings::default();
                change.apply(&mut settings);
                let expected = Some(value.parse::<u64>().unwrap());
                match setting {
                    "loaderTimeoutSecs" => assert_eq!(settings.loader_timeout_secs, expected),
                    "feedTimeoutSecs" => assert_eq!(settings.feed_timeout_secs, expected),
                    _ => assert_eq!(settings.feed_max_body_bytes, expected),
                }
            }
            for value in ["", "-1", "1.5", "1e2", "18446744073709551616"] {
                assert!(
                    serde_json::from_value::<FetchSetting>(
                        serde_json::json!({"setting": setting, "value": value})
                    )
                    .is_err()
                );
            }
        }
        assert!(FetchSetting::MaxRedirects(0).validate().is_ok());
        assert!(FetchSetting::MaxRedirects(10).validate().is_ok());
        assert!(FetchSetting::MaxRedirects(11).validate().is_err());
    }
}
