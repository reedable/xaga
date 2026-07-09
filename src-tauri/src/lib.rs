mod article_loader;
mod commands;
mod fetch_client;
mod native;
mod ssrf_proxy;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::window::Color;
use tauri::{Emitter, Manager, Runtime, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_window_state::StateFlags;
use xaga_core::embeddings::{
    EmbeddingService, EmbeddingsDb, EmbeddingsError, Job, ShutdownReason, Tier,
};

pub(crate) const DEFAULT_LOADER_TIMEOUT_SECS: u64 = 10;
pub(crate) const DEFAULT_FEED_TIMEOUT_SECS: u64 = 10;
pub(crate) const DEFAULT_FEED_MAX_BODY_BYTES: u64 = 10 * 1024 * 1024;
/// Maximum bytes read from a local import file (OPML or bookmarks.html), shared
/// by both formats rather than one per format.
pub(crate) const DEFAULT_IMPORT_MAX_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) struct AppState {
    pub(crate) library: Arc<RwLock<PathBuf>>,
    pub(crate) theme: Arc<RwLock<String>>,
    pub(crate) refresh_on_startup: Arc<RwLock<bool>>,
    pub(crate) loader_timeout_secs: Arc<RwLock<u64>>,
    pub(crate) feed_timeout_secs: Arc<RwLock<u64>>,
    pub(crate) feed_max_body_bytes: Arc<RwLock<u64>>,
    /// Maximum bytes read from a local import file (OPML or bookmarks.html),
    /// checked against file metadata before the read. Shared by both formats.
    pub(crate) import_max_bytes: Arc<RwLock<u64>>,
    pub(crate) html_reader_font_family: Arc<RwLock<String>>,
    pub(crate) html_reader_font_size: Arc<RwLock<f64>>,
    pub(crate) html_reader_line_height: Arc<RwLock<f64>>,
    pub(crate) show_sidebar: Arc<RwLock<bool>>,
    pub(crate) embedding_service: Arc<Mutex<Option<EmbeddingService>>>,
    /// Read-only LanceDB handle for command reads (Phase C). Distinct connection from
    /// the embedding service's writer; reads must `reload()` to see the service's writes.
    pub(crate) embeddings_read: Arc<Mutex<Option<EmbeddingsDb>>>,
    pub(crate) allowed_domains: Arc<RwLock<Vec<String>>>,
    pub(crate) named_stores_supported: bool,
    pub(crate) expanded_sidebar_list: Arc<RwLock<Option<String>>>,
    pub(crate) inline_pdf_reader: Arc<RwLock<bool>>,
    pub(crate) html_reader_view: Arc<RwLock<String>>,
    /// Port of the local SSRF-filtering proxy that loader webviews and the
    /// per-invocation fetch clients (see [`fetch_client`]) are routed through.
    pub(crate) ssrf_proxy_port: u16,
    /// Redirect cap for the reqwest fetch paths (feed XML, PDF probe), already
    /// clamped to [`fetch_client::MAX_REDIRECTS_CEILING`]. The article loader
    /// webview is not covered (docs/architecture/security/fetching_untrusted_content.md,
    /// "Limits").
    pub(crate) max_redirects: Arc<RwLock<u32>>,
    /// Whether the `Referer` header is allowed to go out on redirect hops
    /// (reqwest fetch paths); it names the previous URL. Off by default:
    /// redirect targets don't learn the original feed URL.
    pub(crate) referer_allowed: Arc<RwLock<bool>>,
    /// Whether the reqwest fetch paths send `Accept-Encoding: gzip` (and
    /// decompress compressed responses). Off by default: the request then
    /// carries no `Accept-Encoding` header at all.
    pub(crate) gzip_allowed: Arc<RwLock<bool>>,
}

impl AppState {
    /// Enqueues a job onto the embedding service if it is running. No-op when the
    /// service isn't present (e.g. another instance holds the library lock). The
    /// `flume` send is non-blocking and is never held across an `.await`.
    pub(crate) fn enqueue(&self, tier: Tier, job: Job) {
        if let Some(svc) = self.embedding_service.lock().unwrap().as_ref() {
            svc.enqueue(tier, job);
        }
    }
}

/// The set of unread entry ids, read from LanceDB (the `is_unread` Bitmap index) via the shared
/// read handle. Reloads first (handles are version-pinned); empty if the handle isn't ready.
pub(crate) async fn get_unread_ids(state: &AppState) -> std::collections::HashSet<String> {
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Default::default();
    };
    db.unread_ids().await.unwrap_or_default()
}

/// The set of bookmarked entry ids, read from LanceDB (the `is_bookmarked` Bitmap index).
pub(crate) async fn get_bookmarked_ids(state: &AppState) -> std::collections::HashSet<String> {
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Default::default();
    };
    db.get_bookmarked_ids().await.unwrap_or_default()
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
    /// Maximum seconds for a feed fetch (connection negotiation through body
    /// read). Bounds how long a stalled feed server can hold up `add_feed` or
    /// the sequential refresh loop. Defaults to 10 seconds if absent.
    pub(crate) feed_timeout_secs: Option<u64>,
    /// Maximum bytes read from a feed response body; the fetch aborts past this
    /// limit. Bounds the memory a single response can occupy. Defaults to
    /// 10 MiB if absent.
    pub(crate) feed_max_body_bytes: Option<u64>,
    /// Maximum bytes read from a local import file (OPML or bookmarks.html),
    /// shared by both formats. The import is refused before reading the file
    /// if its size exceeds this. Defaults to 64 MiB if absent.
    pub(crate) import_max_bytes: Option<u64>,
    #[serde(alias = "reader_font_family")]
    pub(crate) html_reader_font_family: Option<String>,
    #[serde(alias = "reader_font_size")]
    pub(crate) html_reader_font_size: Option<f64>,
    #[serde(alias = "reader_line_height")]
    pub(crate) html_reader_line_height: Option<f64>,
    pub(crate) show_sidebar: Option<bool>,
    pub(crate) allowed_domains: Option<Vec<String>>,
    pub(crate) expanded_sidebar_list: Option<String>,
    pub(crate) inline_pdf_reader: Option<bool>,
    pub(crate) html_reader_view: Option<String>,
    /// Allow plaintext `http://` fetches (feeds, PDF probe, article loader).
    /// Enforced by the SSRF proxy. Defaults to false if absent.
    pub(crate) http_allowed: Option<bool>,
    /// Maximum redirects per fetch on the reqwest paths (feed XML, PDF probe).
    /// 0 means the initial request only. Defaults to 3 if absent; clamped to
    /// 10. The default of 3 is a UX exception to secure-by-default — see
    /// docs/architecture/security/fetching_untrusted_content.md.
    pub(crate) max_redirects: Option<u32>,
    /// Whether the `Referer` header is allowed to go out on redirect hops
    /// (reqwest fetch paths); it names the previous URL to the redirect
    /// target. Defaults to false if absent.
    pub(crate) referer_allowed: Option<bool>,
    /// Whether the reqwest fetch paths (feed XML, PDF probe) send
    /// `Accept-Encoding: gzip` and decompress compressed responses. When
    /// enabled, hosts learn the client negotiates gzip — one more property
    /// of the build than the default request shape reveals. Defaults to
    /// false if absent: no `Accept-Encoding` header is sent.
    pub(crate) gzip_allowed: Option<bool>,
    /// Allow `CONNECT` to target a port other than 443. Enforced by the SSRF
    /// proxy. Defaults to false if absent: `CONNECT` to any other port is
    /// refused with 403. See
    /// docs/architecture/security/fetching_remote_content.md, "CONNECT port
    /// policy," for the plaintext-tunnel hole this closes.
    pub(crate) connect_nonstandard_ports_allowed: Option<bool>,
}

pub(crate) fn load_settings(path: &Path) -> UserSettings {
    (|| -> Option<UserSettings> {
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    })()
    .unwrap_or_default()
}

pub(crate) fn save_settings(path: &Path, settings: &UserSettings) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(settings).unwrap();
    xaga_core::fs::write_atomic(path, json.as_bytes())
}

// ── Appearance helpers ───────────────────────────────────────────────────────────

/// Native background color matching the resolved theme, used to pre-paint the
/// window so it doesn't flash white before the frontend's CSS loads.
///
/// `theme_setting` is the persisted "auto" | "light" | "dark" value. "auto"
/// (and any unexpected value) follows the OS via the window's reported theme.
/// Colors mirror `--background-color` in `src/index.css`.
fn theme_background_color<R: Runtime>(theme_setting: &str, window: &WebviewWindow<R>) -> Color {
    const LIGHT: Color = Color(0xFF, 0xFF, 0xFF, 0xFF); // #ffffff
    const DARK: Color = Color(0x12, 0x12, 0x12, 0xFF); // #121212

    let dark = match theme_setting {
        "dark" => true,
        "light" => false,
        _ => window.theme().map(|t| t == Theme::Dark).unwrap_or(false),
    };

    if dark { DARK } else { LIGHT }
}

// ── Menu helpers ───────────────────────────────────────────────────────────────

/// Move the checkmark to `selected` ("summary" | "reader").
fn update_view_checkmarks<R: Runtime>(app: &tauri::AppHandle<R>, selected: &str) {
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

    for (item_id, value) in [
        ("html_reader_view:summary", "summary"),
        ("html_reader_view:reader", "reader"),
    ] {
        if let Some(MenuItemKind::Check(item)) = view.get(item_id) {
            item.set_checked(value == selected).ok();
        }
    }
}

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

fn update_show_sidebar_checkmark<R: Runtime>(app: &tauri::AppHandle<R>, show: bool) {
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
fn set_find_enabled(app: tauri::AppHandle, enabled: bool) {
    if let Some(MenuItemKind::MenuItem(item)) = app.menu().and_then(|m| m.get("find")) {
        item.set_enabled(enabled).ok();
    }
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_client = reqwest::Client::new();
    let named_stores_supported = article_loader::supports_named_data_stores();

    // The "HTTP allowed" handle, created before the proxy so the proxy can
    // enforce it from its first request. Seeded from settings.json in setup;
    // false (blocked) until then.
    let http_allowed = fetch_client::http_allowed_handle();
    // Same startup-ordering reason as `http_allowed`: false (443 only) until
    // settings load.
    let connect_nonstandard_ports_allowed: Arc<std::sync::RwLock<bool>> =
        Arc::new(std::sync::RwLock::new(false));

    // Start the local SSRF-filtering proxy that every loader webview and every
    // fetch client is routed through. Fail fast: binding 127.0.0.1:0
    // essentially never fails, and running the loader without this proxy would
    // silently reopen the SSRF hole.
    let ssrf_proxy_port =
        tauri::async_runtime::block_on(ssrf_proxy::start(ssrf_proxy::ProxyConfig::new(
            Arc::clone(&http_allowed),
            Arc::clone(&connect_nonstandard_ports_allowed),
        )))
        .expect("failed to start SSRF filtering proxy for the article loader");

    // Pre-created Arcs shared between .manage() and the .setup() closure.
    let library: Arc<RwLock<PathBuf>> = Arc::new(RwLock::new(PathBuf::new()));
    let theme: Arc<RwLock<String>> = Arc::new(RwLock::new("auto".to_string()));
    let refresh_on_startup: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let loader_timeout_secs: Arc<RwLock<u64>> = Arc::new(RwLock::new(DEFAULT_LOADER_TIMEOUT_SECS));
    let feed_timeout_secs: Arc<RwLock<u64>> = Arc::new(RwLock::new(DEFAULT_FEED_TIMEOUT_SECS));
    let feed_max_body_bytes: Arc<RwLock<u64>> = Arc::new(RwLock::new(DEFAULT_FEED_MAX_BODY_BYTES));
    let import_max_bytes: Arc<RwLock<u64>> = Arc::new(RwLock::new(DEFAULT_IMPORT_MAX_BYTES));
    let max_redirects: Arc<RwLock<u32>> =
        Arc::new(RwLock::new(fetch_client::DEFAULT_MAX_REDIRECTS));
    let referer_allowed: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let gzip_allowed: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let html_reader_font_family: Arc<RwLock<String>> = Arc::new(RwLock::new("system".to_string()));
    let html_reader_font_size: Arc<RwLock<f64>> = Arc::new(RwLock::new(16.0));
    let html_reader_line_height: Arc<RwLock<f64>> = Arc::new(RwLock::new(1.75));
    let show_sidebar: Arc<RwLock<bool>> = Arc::new(RwLock::new(true));
    let embedding_service: Arc<Mutex<Option<EmbeddingService>>> = Arc::new(Mutex::new(None));
    let embeddings_read: Arc<Mutex<Option<EmbeddingsDb>>> = Arc::new(Mutex::new(None));
    let allowed_domains: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    let expanded_sidebar_list: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    let inline_pdf_reader: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let html_reader_view: Arc<RwLock<String>> = Arc::new(RwLock::new("reader".to_string()));

    // setup_ prefixed values are consumed by Tauri's .setup() closure.
    // There is no semantic meaning here.
    let setup_embeddings_read = Arc::clone(&embeddings_read);
    let setup_allowed_domains = Arc::clone(&allowed_domains);
    let setup_expanded_sidebar_list = Arc::clone(&expanded_sidebar_list);
    let setup_inline_pdf_reader = Arc::clone(&inline_pdf_reader);
    let setup_html_reader_view = Arc::clone(&html_reader_view);
    let setup_show_sidebar = Arc::clone(&show_sidebar);
    let setup_library = Arc::clone(&library);
    let setup_theme = Arc::clone(&theme);
    let setup_refresh_on_startup = Arc::clone(&refresh_on_startup);
    let setup_loader_timeout = Arc::clone(&loader_timeout_secs);
    let setup_feed_timeout = Arc::clone(&feed_timeout_secs);
    let setup_feed_max_body = Arc::clone(&feed_max_body_bytes);
    let setup_import_max_bytes = Arc::clone(&import_max_bytes);
    let setup_http_allowed = Arc::clone(&http_allowed);
    let setup_connect_nonstandard_ports_allowed = Arc::clone(&connect_nonstandard_ports_allowed);
    let setup_max_redirects = Arc::clone(&max_redirects);
    let setup_referer_allowed = Arc::clone(&referer_allowed);
    let setup_gzip_allowed = Arc::clone(&gzip_allowed);
    let setup_reader_font_family = Arc::clone(&html_reader_font_family);
    let setup_reader_font_size = Arc::clone(&html_reader_font_size);
    let setup_reader_line_height = Arc::clone(&html_reader_line_height);
    let setup_svc = Arc::clone(&embedding_service);

    let app = tauri::Builder::default()
        .manage(AppState {
            library,
            theme: theme.clone(),
            refresh_on_startup,
            loader_timeout_secs,
            feed_timeout_secs,
            feed_max_body_bytes,
            import_max_bytes,
            html_reader_font_family,
            html_reader_font_size,
            html_reader_line_height,
            show_sidebar,
            embedding_service: embedding_service.clone(),
            embeddings_read,
            allowed_domains,
            named_stores_supported,
            expanded_sidebar_list,
            inline_pdf_reader,
            html_reader_view,
            ssrf_proxy_port,
            max_redirects,
            referer_allowed,
            gzip_allowed,
        })
        .manage(article_loader::PendingRequests::default())
        .plugin(
            // Transient article-loader webviews (label "loader-{id}", see
            // article_loader.rs) are created and closed constantly; persisting
            // their geometry bloats .window-state.json without ever being
            // reused. Skip them; "main"/"settings" are still tracked.
            // (Returns true to save, false to skip.)
            // VISIBLE is dropped so the plugin does not show() the window on
            // restore — `setup()` reveals the main window itself, only after
            // painting it the theme color, to avoid the white startup flash
            // (A3). Geometry (size/position/maximized) is still restored.
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all().difference(StateFlags::VISIBLE))
                .with_filter(|label| !label.starts_with("loader-"))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .menu(|app| {
            // Defaults corrected by setup() once settings are loaded.
            let initial_theme = "auto";
            let initial_html_reader_view = "reader";

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
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, "find", "Find\u{2026}", false, Some("cmd+f"))?,
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
                        "show_sidebar",
                        "Show Sidebar",
                        true,
                        true,
                        None::<&str>,
                    )?,
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
            Menu::with_items(
                app,
                &[
                    &app_menu,
                    &file_menu,
                    &edit_menu,
                    &view_menu,
                    &history_menu,
                    &window_menu,
                ],
            )
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_library" => {
                app.emit("menu:open-library", ()).ok();
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
                } else if let Ok(win) = WebviewWindowBuilder::new(
                    app,
                    "settings",
                    WebviewUrl::App("index.html".into()),
                )
                .title("Settings")
                .inner_size(680.0, 520.0)
                .visible(false)
                .build()
                {
                    // Build hidden, paint the theme color, then reveal — same
                    // anti-flash treatment as the main window (A3).
                    let theme = app.state::<AppState>().theme.read().unwrap().clone();
                    let color = theme_background_color(&theme, &win);
                    let _ = win.set_background_color(Some(color));
                    let _ = win.show();
                    let _ = win.set_focus();
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
            "show_sidebar" => {
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
                    .and_then(|view| view.get("show_sidebar"))
                    .and_then(|item| match item {
                        MenuItemKind::Check(c) => c.is_checked().ok(),
                        _ => None,
                    })
                    .unwrap_or(true);
                app.emit("menu:show-sidebar", checked).ok();
            }
            id @ ("html_reader_view:summary" | "html_reader_view:reader") => {
                let view = id.split(':').nth(1).unwrap_or("");
                update_view_checkmarks(app, view);
                app.emit("menu:html-reader-view", view).ok();
            }
            "clear_history" => {
                app.emit("menu:clear-history", ()).ok();
            }
            "find" => {
                app.emit("menu:find", ()).ok();
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
            let feed_timeout = settings
                .feed_timeout_secs
                .unwrap_or(DEFAULT_FEED_TIMEOUT_SECS);
            let feed_max_body = settings
                .feed_max_body_bytes
                .unwrap_or(DEFAULT_FEED_MAX_BODY_BYTES);
            let import_max = settings
                .import_max_bytes
                .unwrap_or(DEFAULT_IMPORT_MAX_BYTES);
            let http_allowed_setting = settings.http_allowed.unwrap_or(false);
            let connect_nonstandard_ports_allowed_setting = settings
                .connect_nonstandard_ports_allowed
                .unwrap_or(false);
            let max_redirects_setting =
                fetch_client::effective_max_redirects(settings.max_redirects);
            let referer_allowed_setting = settings.referer_allowed.unwrap_or(false);
            let gzip_allowed_setting = settings.gzip_allowed.unwrap_or(false);
            let font_family = settings
                .html_reader_font_family
                .unwrap_or_else(|| "system".to_string());
            let font_size = settings.html_reader_font_size.unwrap_or(16.0);
            let line_height = settings.html_reader_line_height.unwrap_or(1.75);
            let show_sidebar = settings.show_sidebar.unwrap_or(true);
            *setup_library.write().unwrap() = lib_path.clone();
            *setup_theme.write().unwrap() = theme.clone();
            *setup_refresh_on_startup.write().unwrap() = refresh;
            *setup_loader_timeout.write().unwrap() = timeout;
            *setup_feed_timeout.write().unwrap() = feed_timeout;
            *setup_feed_max_body.write().unwrap() = feed_max_body;
            *setup_import_max_bytes.write().unwrap() = import_max;
            *setup_http_allowed.write().unwrap() = http_allowed_setting;
            *setup_connect_nonstandard_ports_allowed.write().unwrap() =
                connect_nonstandard_ports_allowed_setting;
            *setup_max_redirects.write().unwrap() = max_redirects_setting;
            *setup_referer_allowed.write().unwrap() = referer_allowed_setting;
            *setup_gzip_allowed.write().unwrap() = gzip_allowed_setting;
            *setup_reader_font_family.write().unwrap() = font_family;
            *setup_reader_font_size.write().unwrap() = font_size;
            *setup_reader_line_height.write().unwrap() = line_height;
            *setup_show_sidebar.write().unwrap() = show_sidebar;
            *setup_allowed_domains.write().unwrap() = settings.allowed_domains.unwrap_or_default();
            *setup_expanded_sidebar_list.write().unwrap() = settings.expanded_sidebar_list;
            *setup_inline_pdf_reader.write().unwrap() = settings.inline_pdf_reader.unwrap_or(false);
            let html_reader_view_str = settings.html_reader_view.unwrap_or_else(|| "reader".to_string());
            *setup_html_reader_view.write().unwrap() = html_reader_view_str.clone();

            // Correct the menu checkmarks now that we know the persisted values.
            update_theme_checkmarks(app.handle(), &theme);
            update_show_sidebar_checkmark(app.handle(), show_sidebar);
            update_view_checkmarks(app.handle(), &html_reader_view_str);

            // The main window is created hidden ("visible": false in
            // tauri.conf.json). Paint it the theme color, then reveal it, so it
            // never shows the white default frame before the frontend loads (A3).
            if let Some(main) = app.get_webview_window("main") {
                let color = theme_background_color(&theme, &main);
                let _ = main.set_background_color(Some(color));
                let _ = main.show();
            }

            // Start the EmbeddingService; file watcher + recovery fire inside it.
            // If another instance already holds the library lock, we skip the
            // service entirely — embeddings are available for reading but this
            // instance will not write to LanceDB.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = xaga_core::embeddings::download_model_if_missing(&http_client).await {
                    log::error!("startup: model download failed: {e}");
                }

                match EmbeddingService::start(lib_path.clone()).await {
                    Ok(svc) => {
                        // The read handle shares the service's write handle (same underlying table
                        // state), so command reads see the service's writes — and the synchronous
                        // post-compaction reload — without any per-read reload.
                        *setup_embeddings_read.lock().unwrap() = Some(svc.read_handle());

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

                        let embeddings_app_handle = app_handle.clone();
                        let background_drained_rx = svc.subscribe_to_background_drained();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                if background_drained_rx.recv_async().await.is_err() {
                                    break;
                                }
                                embeddings_app_handle.emit("embedding:changed", ()).ok();
                            }
                        });

                        let unread_app_handle = app_handle.clone();
                        let unread_rx = svc.subscribe_to_unread_changed();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                match unread_rx.recv_async().await {
                                    Ok(entry) => { unread_app_handle.emit("unread:changed", &entry).ok(); }
                                    Err(_) => break,
                                }
                            }
                        });

                        let bookmarked_app_handle = app_handle.clone();
                        let bookmarked_rx = svc.subscribe_to_bookmarked_changed();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                match bookmarked_rx.recv_async().await {
                                    Ok(entry) => { bookmarked_app_handle.emit("bookmark:changed", &entry).ok(); }
                                    Err(_) => break,
                                }
                            }
                        });

                        let annotation_app_handle = app_handle.clone();
                        let annotation_rx = svc.subscribe_to_annotation_changed();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                match annotation_rx.recv_async().await {
                                    Ok(item) => { annotation_app_handle.emit("annotation:changed", &item).ok(); }
                                    Err(_) => break,
                                }
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

            // Logging is installed in every build (not only dev) so release recovery/startup
            // diagnostics are captured — previously the plugin was gated behind
            // `cfg!(debug_assertions)`, so release builds wrote nothing and issues like a wiped
            // embeddings mirror left no trace. Targets are set explicitly (`.targets` replaces the
            // plugin defaults) so the destination is deterministic: every build persists to the OS
            // app-log dir (macOS: ~/Library/Logs/app.xaga/Xaga.log); dev additionally mirrors to
            // stdout and the webview console. Release logs at Info (recovery's key lines are
            // info!/warn!); dev stays at Debug.
            //
            // `lance` is pinned to Warn: at Info it logs one line *per row* during compaction
            // (~1800 lines per pass), which — against the plugin's 40 KB default rotation — erases
            // our own startup/recovery diagnostics within seconds. The file cap is also raised so a
            // burst of noise can't rotate the useful lines away before they're read.
            {
                use tauri_plugin_log::{Target, TargetKind};

                let crate_level = if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                };

                let mut targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
                if cfg!(debug_assertions) {
                    targets.push(Target::new(TargetKind::Stdout));
                    targets.push(Target::new(TargetKind::Webview));
                }

                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .level_for("xaga_lib", crate_level)
                        .level_for("xaga_core", crate_level)
                        .level_for("lance", log::LevelFilter::Warn)
                        .max_file_size(5_000_000)
                        .targets(targets)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler({
            // Central IPC gate: a single allowlist check runs before dispatch,
            // so loader (untrusted) webviews can reach only the loader callbacks
            // and every other command is protected without a per-command guard.
            let app_commands: Box<dyn Fn(tauri::ipc::Invoke) -> bool + Send + Sync> =
                Box::new(tauri::generate_handler![
            set_find_enabled,
            commands::articles::receive_html,
            commands::articles::loader_will_unload,
            commands::articles::loader_page_loaded,
            commands::articles::capture_and_close_loader,
            commands::articles::load_article,
            commands::articles::clear_webview_history,
            commands::articles::save_article,
            commands::articles::find_article,
            commands::articles::read_article,
            commands::articles::create_article,
            commands::articles::add_annotation,
            commands::articles::replace_annotation,
            commands::articles::remove_annotation,
            commands::articles::list_annotations,
            commands::entries::is_cache_ready,
            commands::entries::read_entry,
            commands::entries::mark_entry_read,
            commands::entries::mark_entry_unread,
            commands::entries::list_all_items,
            commands::library::get_library,
            commands::library::set_library,
            commands::settings::get_theme,
            commands::settings::set_theme,
            commands::settings::get_refresh_on_startup,
            commands::settings::set_refresh_on_startup,
            commands::settings::get_html_reader_prefs,
            commands::settings::set_html_reader_font_family,
            commands::settings::set_html_reader_font_size,
            commands::settings::set_html_reader_line_height,
            commands::settings::get_show_sidebar,
            commands::settings::set_show_sidebar,
            commands::settings::get_allowed_domains,
            commands::settings::set_allowed_domains,
            commands::settings::get_html_reader_view,
            commands::settings::set_html_reader_view,
            commands::feeds::set_feed_pinned,
            commands::topics::set_topic_pinned,
            commands::settings::get_expanded_sidebar_list,
            commands::settings::set_expanded_sidebar_list,
            commands::settings::get_inline_pdf_reader,
            commands::settings::set_inline_pdf_reader,
            commands::feeds::list_feeds,
            commands::feeds::add_feed,
            commands::feeds::refresh_all_feeds,
            commands::feeds::rename_feed,
            commands::feeds::remove_feed,
            commands::feeds::list_entries,
            commands::bookmarks::add_bookmark,
            commands::bookmarks::remove_bookmark,
            commands::topics::create_topic,
            commands::topics::save_topic,
            commands::topics::list_topics,
            commands::topics::read_topic,
            commands::topics::delete_topic,
            commands::topics::add_topic_item,
            commands::topics::remove_topic_item,
            commands::topics::list_topic_items,
            commands::topics::list_entry_topics,
            commands::topics::rank_topics_by_item,
            commands::embeddings::search_by_item,
            commands::embeddings::search_by_text,
            commands::embeddings::reset_embeddings,
            ]);
            move |invoke: tauri::ipc::Invoke| {
                let allowed = {
                    let label = invoke.message.webview_ref().label();
                    commands::is_invoke_allowed(label, invoke.message.command())
                };
                if !allowed {
                    invoke
                        .resolver
                        .reject("unauthorized: command not available from this window");
                    return true;
                }
                app_commands(invoke)
            }
        })
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
