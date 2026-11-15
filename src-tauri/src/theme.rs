use super::*;

// ── Appearance helpers ───────────────────────────────────────────────────────────

/// Native background color matching the resolved theme, used to pre-paint the
/// window so it doesn't flash white before the frontend's CSS loads.
///
/// `theme_setting` is the persisted "auto" | "light" | "dark" value. "auto"
/// (and any unexpected value) follows the OS via the window's reported theme.
/// Colors mirror `--background-color` in `src/index.css`.
pub(super) fn theme_background_color<R: Runtime>(
    theme_setting: &str,
    window: &WebviewWindow<R>,
) -> Color {
    const LIGHT: Color = Color(0xFF, 0xFF, 0xFF, 0xFF); // #ffffff
    const DARK: Color = Color(0x12, 0x12, 0x12, 0xFF); // #121212

    let dark = match theme_setting {
        "dark" => true,
        "light" => false,
        _ => window.theme().map(|t| t == Theme::Dark).unwrap_or(false),
    };

    if dark { DARK } else { LIGHT }
}

/// Applies the persisted setting before the frontend stylesheet or React can paint.
pub(super) fn theme_bootstrap_script(theme_setting: &str) -> String {
    let setting = match theme_setting {
        "light" | "dark" => theme_setting,
        _ => "auto",
    };
    let setting = serde_json::to_string(setting).expect("theme setting serializes");
    format!(
        "(() => {{ const setting = {setting}; const resolved = setting === 'auto' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : setting; window.__XAGA_THEME_SETTING__ = setting; document.documentElement.dataset.theme = resolved; document.documentElement.style.colorScheme = resolved; document.documentElement.style.backgroundColor = resolved === 'dark' ? '#121212' : '#ffffff'; }})();"
    )
}

/// Move the checkmark to `selected` ("auto" | "light" | "dark").
pub(super) fn update_theme_checkmarks<R: Runtime>(app: &tauri::AppHandle<R>, selected: &str) {
    let Some(menu) = app.menu() else { return };
    // The theme items live inside the "view_menu" submenu, one level down.
    let view = menu
        .items()
        .unwrap_or_default()
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(sub) if sub.id().as_ref() == "view_menu" => Some(sub),
            _ => None,
        });

    let Some(view) = view else { return };

    for (entity_id, value) in [
        ("theme_auto", "auto"),
        ("theme_light", "light"),
        ("theme_dark", "dark"),
    ] {
        if let Some(MenuItemKind::Check(item)) = view.get(entity_id) {
            item.set_checked(value == selected).ok();
        }
    }
}
