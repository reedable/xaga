use super::*;

// ── Menu helpers ───────────────────────────────────────────────────────────────

/// Move the checkmark to `selected` ("summary" | "reader").
pub(crate) fn update_view_checkmarks<R: Runtime>(app: &tauri::AppHandle<R>, selected: &str) {
    let Some(menu) = app.menu() else { return };
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
        ("html_reader_view:summary", "summary"),
        ("html_reader_view:reader", "reader"),
    ] {
        if let Some(MenuItemKind::Check(item)) = view.get(entity_id) {
            item.set_checked(value == selected).ok();
        }
    }
}

pub(crate) fn update_show_sidebar_checkmark<R: Runtime>(app: &tauri::AppHandle<R>, show: bool) {
    let Some(menu) = app.menu() else { return };
    let view = menu
        .items()
        .unwrap_or_default()
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(sub) if sub.id().as_ref() == "view_menu" => Some(sub),
            _ => None,
        });
    let Some(view) = view else { return };
    if let Some(MenuItemKind::Check(item)) = view.get("show_sidebar") {
        item.set_checked(show).ok();
    }
}

// ── Commands ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub(crate) fn set_find_enabled(app: tauri::AppHandle, enabled: bool) {
    if let Some(MenuItemKind::MenuItem(item)) = app.menu().and_then(|m| m.get("find")) {
        item.set_enabled(enabled).ok();
    }
}

pub(super) fn build(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    // Defaults corrected by setup() once settings are loaded.
    let initial_theme = "auto";
    let initial_html_reader_view = crate::settings::DEFAULT_HTML_READER_VIEW;

    let app_menu = Submenu::with_items(
        app,
        "Xaga",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "open_settings", "Settings...", true, Some("cmd+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit Xaga", true, Some("cmd+q"))?,
        ],
    )?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &MenuItem::with_id(app, "open_library", "Open Library", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "import_feeds",
                "Import Feeds From OPML\u{2026}",
                false,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "export_feeds",
                "Export Feeds As OPML\u{2026}",
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "import_bookmarks",
                "Import Bookmarks\u{2026}",
                false,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "export_bookmarks",
                "Export Bookmarks\u{2026}",
                true,
                None::<&str>,
            )?,
        ],
    )?;
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "find", "Find\u{2026}", false, Some("cmd+f"))?,
        ],
    )?;
    let sidebar_sort_items = sidebar_sort::build(app)?;
    let view_menu = Submenu::with_id_and_items(
        app,
        "view_menu",
        "View",
        true,
        &[
            &CheckMenuItem::with_id(
                app,
                "theme_auto",
                "Auto (System)",
                true,
                initial_theme == "auto",
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "theme_light",
                "Light",
                true,
                initial_theme == "light",
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "theme_dark",
                "Dark",
                true,
                initial_theme == "dark",
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &CheckMenuItem::with_id(
                app,
                "show_sidebar",
                "Show Sidebar",
                true,
                true,
                None::<&str>,
            )?,
            &sidebar_sort_items[0],
            &sidebar_sort_items[1],
            &PredefinedMenuItem::separator(app)?,
            &CheckMenuItem::with_id(
                app,
                "html_reader_view:summary",
                "Summary",
                true,
                initial_html_reader_view == "summary",
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "html_reader_view:reader",
                "Reader",
                true,
                initial_html_reader_view == "reader",
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, Some("Enter Full Screen"))?,
        ],
    )?;
    let history_menu = Submenu::with_items(
        app,
        "History",
        true,
        &[&MenuItem::with_id(
            app,
            "clear_history",
            "Clear History",
            true,
            None::<&str>,
        )?],
    )?;
    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
        ],
    )?;
    let shortcuts_menu = shortcuts_menu::build(app, &[])?;
    Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &history_menu,
            &shortcuts_menu,
            &window_menu,
        ],
    )
}

pub(super) fn on_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "open_library" => {
            events::emit_app(app, events::AppEvent::MenuOpenLibrary);
        }
        // File-path operations are orchestrated entirely on the native side:
        // the dialog opens in Rust and the work runs without a webview-facing
        // command, so no path is ever taken from the renderer
        // (docs/architecture/security/command_surface.md). Completion is reported back via
        // the `feeds:*` / `bookmarks:*` events.
        "import_feeds" => native::import_feeds_dialog(app),
        "export_feeds" => native::export_feeds_dialog(app),
        "import_bookmarks" => native::import_bookmarks_dialog(app),
        "export_bookmarks" => native::export_bookmarks_dialog(app),
        "open_settings" => {
            if let Some(win) = app.get_webview_window("settings") {
                win.show().ok();
                win.set_focus().ok();
            } else {
                let proxy = match ui_network::start(app, "settings") {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        log::error!("settings network policy: {error}");
                        return;
                    }
                };
                let builder = WebviewWindowBuilder::new(
                    app,
                    "settings",
                    WebviewUrl::App("index.html".into()),
                )
                .title("Settings")
                .inner_size(680.0, 520.0)
                .visible(false);
                let builder = match ui_network::configure_window(builder, proxy.port) {
                    Ok(builder) => builder,
                    Err(error) => {
                        log::error!("settings WebView policy: {error}");
                        return;
                    }
                };
                if let Ok(win) = builder.build() {
                    ui_network::retain_for_window(&win, proxy);
                    // Build hidden, paint the theme color, then reveal — same
                    // anti-flash treatment as the main window (A3).
                    let theme = app.state::<AppState>().theme.read().unwrap().clone();
                    let color = theme_background_color(&theme, &win);
                    let _ = win.set_background_color(Some(color));
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        }
        id @ ("theme_auto" | "theme_light" | "theme_dark") => {
            let theme = match id {
                "theme_auto" => "auto",
                "theme_light" => "light",
                _ => "dark",
            };
            update_theme_checkmarks(app, theme);
            events::emit_app(app, events::AppEvent::MenuSetTheme(theme.to_string()));
        }
        id @ ("sidebar_sort_unread" | "sidebar_sort_name") => {
            let value = sidebar_sort::selection(id).unwrap();
            if let Err(error) =
                commands::settings::save_sidebar_sort(app, &app.state::<AppState>(), value)
            {
                // Native check items toggle before their handler; restore the saved state on refusal.
                sidebar_sort::update(app, *app.state::<AppState>().sidebar_sort.read().unwrap());
                events::emit_app(app, events::AppEvent::Error(error.serialized()));
            }
        }
        "show_sidebar" => {
            let checked = app
                .menu()
                .and_then(|menu| {
                    menu.items().ok()?.into_iter().find_map(|item| match item {
                        MenuItemKind::Submenu(sub) if sub.id().as_ref() == "view_menu" => Some(sub),
                        _ => None,
                    })
                })
                .and_then(|view| view.get("show_sidebar"))
                .and_then(|item| match item {
                    MenuItemKind::Check(c) => c.is_checked().ok(),
                    _ => None,
                })
                .unwrap_or(true);
            events::emit_app(app, events::AppEvent::MenuShowSidebar(checked));
        }
        id @ ("html_reader_view:summary" | "html_reader_view:reader") => {
            let view = id.split(':').nth(1).unwrap_or("");
            update_view_checkmarks(app, view);
            events::emit_app(app, events::AppEvent::MenuHtmlReaderView(view.to_string()));
        }
        "clear_history" => {
            events::emit_app(app, events::AppEvent::MenuClearHistory);
        }
        "find" => {
            events::emit_app(app, events::AppEvent::MenuFind);
        }
        "quit" => {
            app.exit(0);
        }
        id => {
            if let Some(key) = shortcuts_menu::selection(id) {
                // Native menus belong to the app; shortcut navigation belongs
                // to its main window, including when Settings owns focus.
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    events::emit_app(&window, events::AppEvent::MenuShortcut(key.to_string()));
                }
            }
        }
    }
}
