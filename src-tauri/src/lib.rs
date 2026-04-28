mod article_loader;
mod commands;
mod file_watchers;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use xaga_core::entry_cache::EntryStateCache;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use xaga_core::embeddings::{EmbeddingService, EmbeddingsError, ShutdownReason};

pub(crate) const DEFAULT_LOADER_TIMEOUT_SECS: u64 = 10;

pub(crate) struct AppState {
    pub(crate) library: Arc<RwLock<PathBuf>>,
    pub(crate) theme: Arc<RwLock<String>>,
    pub(crate) refresh_on_startup: Arc<RwLock<bool>>,
    pub(crate) loader_timeout_secs: Arc<RwLock<u64>>,
    pub(crate) reader_font_family: Arc<RwLock<String>>,
    pub(crate) reader_font_size: Arc<RwLock<f64>>,
    pub(crate) reader_line_height: Arc<RwLock<f64>>,
    pub(crate) embedding_service: Arc<Mutex<Option<EmbeddingService>>>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) entry_cache: Arc<Mutex<Option<EntryStateCache>>>,
}

pub(crate) fn feeds_dir(library: &Path) -> PathBuf {
    library.join("feeds")
}

pub(crate) fn get_unread(state: &AppState) -> std::collections::HashSet<String> {
    state.entry_cache.lock().unwrap().as_ref().map(|c| c.unread_snapshot()).unwrap_or_default()
}

pub(crate) fn get_bookmarked(state: &AppState) -> std::collections::HashSet<String> {
    state.entry_cache.lock().unwrap().as_ref().map(|c| c.bookmarked_snapshot()).unwrap_or_default()
}

// ── Settings helpers ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct UserSettings {
    pub(crate) library: Option<String>,
    pub(crate) theme: Option<String>,
    pub(crate) refresh_on_startup: Option<bool>,
    /// Maximum seconds to wait for a loader webview to deliver its HTML before
    /// the webview is force-closed. Lower values shrink the window available to
    /// malicious scripts. Defaults to 10 seconds if absent.
    pub(crate) loader_timeout_secs: Option<u64>,
    pub(crate) reader_font_family: Option<String>,
    pub(crate) reader_font_size: Option<f64>,
    pub(crate) reader_line_height: Option<f64>,
}

pub(crate) fn load_settings(path: &Path) -> UserSettings {
    (|| -> Option<UserSettings> {
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    })()
    .unwrap_or_default()
}

pub(crate) fn save_settings(path: &Path, settings: &UserSettings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings).unwrap())
}

// ── Menu helpers ───────────────────────────────────────────────────────────────

/// Move the checkmark to `selected` ("auto" | "light" | "dark").
fn update_theme_checkmarks<R: Runtime>(app: &tauri::AppHandle<R>, selected: &str) {
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

    for (item_id, value) in [
        ("theme_auto", "auto"),
        ("theme_light", "light"),
        ("theme_dark", "dark"),
    ] {
        if let Some(MenuItemKind::Check(item)) = view.get(item_id) {
            item.set_checked(value == selected).ok();
        }
    }
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_client = reqwest::Client::new();

    // Pre-created Arcs shared between .manage() and the .setup() closure.
    let library: Arc<RwLock<PathBuf>> = Arc::new(RwLock::new(PathBuf::new()));
    let theme: Arc<RwLock<String>> = Arc::new(RwLock::new("auto".to_string()));
    let refresh_on_startup: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let loader_timeout_secs: Arc<RwLock<u64>> = Arc::new(RwLock::new(DEFAULT_LOADER_TIMEOUT_SECS));
    let reader_font_family: Arc<RwLock<String>> = Arc::new(RwLock::new("system".to_string()));
    let reader_font_size: Arc<RwLock<f64>> = Arc::new(RwLock::new(16.0));
    let reader_line_height: Arc<RwLock<f64>> = Arc::new(RwLock::new(1.75));
    let embedding_service: Arc<Mutex<Option<EmbeddingService>>> = Arc::new(Mutex::new(None));
    let entry_cache: Arc<Mutex<Option<EntryStateCache>>> = Arc::new(Mutex::new(None));
    let setup_entry_cache = Arc::clone(&entry_cache);
    let setup_library = Arc::clone(&library);
    let setup_theme = Arc::clone(&theme);
    let setup_refresh_on_startup = Arc::clone(&refresh_on_startup);
    let setup_loader_timeout = Arc::clone(&loader_timeout_secs);
    let setup_reader_font_family = Arc::clone(&reader_font_family);
    let setup_reader_font_size = Arc::clone(&reader_font_size);
    let setup_reader_line_height = Arc::clone(&reader_line_height);
    let setup_svc = Arc::clone(&embedding_service);
    let setup_http_client = http_client.clone();

    let app = tauri::Builder::default()
        .manage(AppState {
            library,
            theme: theme.clone(),
            refresh_on_startup,
            loader_timeout_secs,
            reader_font_family,
            reader_font_size,
            reader_line_height,
            embedding_service: embedding_service.clone(),
            http_client,
            entry_cache,
        })
        .manage(article_loader::PendingRequests::default())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .menu(|app| {
            // Default to "auto" checked; setup() corrects this once settings are loaded.
            let initial_theme = "auto";

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
                        true,
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
                        true,
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
                ],
            )?;
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
                        "show_article_summary",
                        "Show Article Summary",
                        true,
                        false,
                        None::<&str>,
                    )?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::fullscreen(app, Some("Enter Full Screen"))?,
                ],
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
            Menu::with_items(
                app,
                &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu],
            )
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_library" => {
                app.emit("menu:open-library", ()).ok();
            }
            "import_feeds" => {
                app.emit("menu:import-feeds", ()).ok();
            }
            "export_feeds" => {
                app.emit("menu:export-feeds", ()).ok();
            }
            "import_bookmarks" => {
                app.emit("menu:import-bookmarks", ()).ok();
            }
            "export_bookmarks" => {
                app.emit("menu:export-bookmarks", ()).ok();
            }
            "open_settings" => {
                if let Some(win) = app.get_webview_window("settings") {
                    win.show().ok();
                    win.set_focus().ok();
                } else {
                    WebviewWindowBuilder::new(
                        app,
                        "settings",
                        WebviewUrl::App("index.html".into()),
                    )
                    .title("Settings")
                    .inner_size(680.0, 520.0)
                    .resizable(false)
                    .build()
                    .ok();
                }
            }
            id @ ("theme_auto" | "theme_light" | "theme_dark") => {
                let theme = match id {
                    "theme_auto" => "auto",
                    "theme_light" => "light",
                    _ => "dark",
                };
                update_theme_checkmarks(app, theme);
                app.emit("menu:set-theme", theme).ok();
            }
            "show_article_summary" => {
                let checked = app
                    .menu()
                    .and_then(|menu| {
                        menu.items().ok()?.into_iter().find_map(|item| match item {
                            MenuItemKind::Submenu(sub) if sub.id().as_ref() == "view_menu" => {
                                Some(sub)
                            }
                            _ => None,
                        })
                    })
                    .and_then(|view| view.get("show_article_summary"))
                    .and_then(|item| match item {
                        MenuItemKind::Check(c) => c.is_checked().ok(),
                        _ => None,
                    })
                    .unwrap_or(false);
                app.emit("menu:show-article-summary", checked).ok();
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .setup(move |app| {
            // Resolve settings: persisted settings.json or defaults.
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let default_library = app.path().app_data_dir()?;
            let settings = load_settings(&settings_path);
            let lib_path = settings
                .library
                .map(PathBuf::from)
                .unwrap_or(default_library);
            let theme = settings.theme.unwrap_or_else(|| "auto".to_string());
            let refresh = settings.refresh_on_startup.unwrap_or(false);
            let timeout = settings
                .loader_timeout_secs
                .unwrap_or(DEFAULT_LOADER_TIMEOUT_SECS);
            let font_family = settings
                .reader_font_family
                .unwrap_or_else(|| "system".to_string());
            let font_size = settings.reader_font_size.unwrap_or(16.0);
            let line_height = settings.reader_line_height.unwrap_or(1.75);
            *setup_library.write().unwrap() = lib_path.clone();
            *setup_theme.write().unwrap() = theme.clone();
            *setup_refresh_on_startup.write().unwrap() = refresh;
            *setup_loader_timeout.write().unwrap() = timeout;
            *setup_reader_font_family.write().unwrap() = font_family;
            *setup_reader_font_size.write().unwrap() = font_size;
            *setup_reader_line_height.write().unwrap() = line_height;

            // Correct the theme checkmarks now that we know the persisted value.
            update_theme_checkmarks(app.handle(), &theme);

            // Start the EmbeddingService; file watcher + recovery fire inside it.
            // If another instance already holds the library lock, we skip the
            // service entirely — embeddings are available for reading but this
            // instance will not write to LanceDB.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = xaga_core::embeddings::download_model_if_missing(&setup_http_client).await {
                    log::error!("startup: model download failed: {e}");
                }

                let cache = EntryStateCache::new(&lib_path).await;
                *setup_entry_cache.lock().unwrap() = Some(cache.clone());

                let unread_app_handle = app_handle.clone();
                let mut unread_rx = cache.subscribe_unread();
                tauri::async_runtime::spawn(async move {
                    while let Ok(entry) = unread_rx.recv().await {
                        unread_app_handle.emit("unread:changed", &entry).ok();
                    }
                });

                let bookmarked_app_handle = app_handle.clone();
                let mut bookmarked_rx = cache.subscribe_bookmarked();
                tauri::async_runtime::spawn(async move {
                    while let Ok(entry) = bookmarked_rx.recv().await {
                        bookmarked_app_handle.emit("bookmark:changed", &entry).ok();
                    }
                });

                match EmbeddingService::start(lib_path.clone()).await {
                    Ok(svc) => {
                        file_watchers::watch(
                            lib_path.clone(),
                            svc.priority_sender(),
                            svc.background_sender(),
                            cache,
                        );

                        let embeddings_app_handle = app_handle.clone();
                        let priority_drained_rx = svc.subscribe_to_priority_drained();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                if priority_drained_rx.recv_async().await.is_err() {
                                    break;
                                }
                                embeddings_app_handle.emit("embedding:changed", ()).ok();
                            }
                        });

                        svc.trigger_recovery(lib_path.clone());
                        *setup_svc.lock().unwrap() = Some(svc);
                    }
                    Err(EmbeddingsError::LockNotAcquired) => {
                        log::warn!("startup: another instance holds the library lock — embeddings are read-only");
                    }
                    Err(e) => log::error!("startup: embedding service failed: {e}"),
                }
            });

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .level_for("xaga_lib", log::LevelFilter::Debug)
                        .level_for("xaga_core", log::LevelFilter::Debug)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::articles::receive_html,
            commands::articles::load_article,
            commands::articles::save_article,
            commands::articles::find_article,
            commands::articles::read_article,
            commands::articles::create_article,
            commands::articles::add_annotation,
            commands::articles::remove_annotation,
            commands::articles::list_annotations,
            commands::saved::list_saved,
            commands::entries::mark_entry_read,
            commands::entries::mark_entry_unread,
            commands::entries::list_all_articles,
            commands::library::get_library,
            commands::library::set_library,
            commands::settings::get_theme,
            commands::settings::set_theme,
            commands::settings::get_refresh_on_startup,
            commands::settings::set_refresh_on_startup,
            commands::settings::get_reader_prefs,
            commands::settings::set_reader_font_family,
            commands::settings::set_reader_font_size,
            commands::settings::set_reader_line_height,
            commands::feeds::list_feeds,
            commands::feeds::add_feed,
            commands::feeds::refresh_feed,
            commands::feeds::rename_feed,
            commands::feeds::remove_feed,
            commands::feeds::import_feeds,
            commands::feeds::export_feeds,
            commands::feeds::list_entries,
            commands::bookmarks::add_bookmark,
            commands::bookmarks::remove_bookmark,
            commands::bookmarks::list_bookmarks,
            commands::bookmarks::import_bookmarks,
            commands::bookmarks::export_bookmarks,
            commands::topics::create_topic,
            commands::topics::save_topic,
            commands::topics::list_topics,
            commands::topics::read_topic,
            commands::topics::delete_topic,
            commands::embeddings::query_by_item,
            commands::embeddings::query_by_bookmarks,
            commands::embeddings::query_by_annotations,
            commands::embeddings::query_by_topics,
            commands::embeddings::query_by_text,
            commands::embeddings::reset_embeddings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    install_signal_handler(Arc::clone(&embedding_service), app.handle().clone());

    app.run(|app, event| match event {
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "main" => {
            if let Some(svc) = take_service(app) {
                api.prevent_close();
                if let Some(w) = app.get_webview_window("main") {
                    w.hide().ok();
                }
                tauri::async_runtime::spawn(async move {
                    match svc.shutdown().await {
                        ShutdownReason::Restart => restart_process(),
                        ShutdownReason::Exit => std::process::exit(0),
                    }
                });
            }
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            if let Some(svc) = take_service(app) {
                api.prevent_exit();
                shutdown_with_overlay(app, svc);
            }
        }
        _ => {}
    });
}


/// Spawns a fresh copy of the current executable then exits the current
/// process.  The new process boots against the already-updated settings.json.
///
/// NOTE: this does not work under `tauri dev` because the Vite dev server is
/// managed by the tauri-cli watcher and dies with the binary.  Dev-mode restart
/// will be handled separately; for now callers fall through to a clean exit in
/// that environment.
pub(crate) fn restart_process() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    std::process::exit(0);
}

fn take_service(app: &tauri::AppHandle) -> Option<EmbeddingService> {
    app.state::<AppState>()
        .embedding_service
        .lock()
        .unwrap()
        .take()
}

fn shutdown_with_overlay(app: &tauri::AppHandle, svc: EmbeddingService) {
    log::info!("shutdown: waiting for embedding service to drain");
    app.emit("embedding:shutdown", ()).ok();
    tauri::async_runtime::spawn(async move {
        match svc.shutdown().await {
            ShutdownReason::Restart => restart_process(),
            ShutdownReason::Exit => std::process::exit(0),
        }
    });
}

/// Installs a SIGINT/SIGTERM handler that drains the embedding service before exit.
fn install_signal_handler(
    embedding_service: Arc<Mutex<Option<EmbeddingService>>>,
    app_handle: tauri::AppHandle,
) {
    ctrlc::set_handler(move || {
        let svc = embedding_service.lock().unwrap().take();
        if let Some(svc) = svc {
            log::info!("signal: waiting for embedding service to drain");
            app_handle.emit("embedding:shutdown", ()).ok();
            tauri::async_runtime::block_on(svc.shutdown());
            // Signals always exit — restart flag is intentionally ignored.
        }
        std::process::exit(0);
    })
    .expect("failed to install signal handler");
}
