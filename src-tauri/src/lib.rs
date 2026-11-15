mod menus;
mod settings;
mod shutdown;
mod startup;
mod theme;

use menus::{set_find_enabled, update_show_sidebar_checkmark, update_view_checkmarks};
use settings::{
    AccountLock, DEFAULT_FEED_MAX_BODY_BYTES, DEFAULT_FEED_TIMEOUT_SECS, DEFAULT_IMPORT_MAX_BYTES,
    DEFAULT_LOAD_REMOTE_IMAGE, DEFAULT_LOADER_TIMEOUT_SECS, SettingsOutcome, UserSettings,
    account_lease, load_settings, settings_mtime, settings_path, stale_view_error, update_settings,
};
use shutdown::{
    begin_shutdown, drain_and_release_writer, handle_run_event, install_signal_handler,
};
use startup::EmbeddingStartupState;
use theme::{theme_background_color, theme_bootstrap_script, update_theme_checkmarks};

mod article_diagnostics;
mod article_loader;
mod command_policy;
mod commands;
mod error;
mod events;
mod fetch_client;
mod library_menu;
mod loader_download;
mod loader_identity;
mod native;
mod resource_download;
mod shortcuts_menu;
mod sidebar_sort;
mod ui_network;
#[cfg(target_os = "macos")]
mod webview_policy;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::window::Color;
use tauri::{Manager, Runtime, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_window_state::StateFlags;
use xaga_core::embeddings::{Embedder, EmbeddingService, EmbeddingsDb, Job, ShutdownReason, Tier};
use xaga_core::network_proxy;

/// The open library and the write right to it, guarded as ONE unit: the path and the writer
/// change together (both settled at startup; `set_library` restarts the process), so a
/// reader always sees "which library" and "may I write it" as one consistent snapshot.
///
/// The `LibraryWriter` (present only in read-write mode) owns the library flock and is the
/// single component through which every `.md` write flows. Its presence IS the runtime mode
/// — no mode value is stored anywhere. `writer` is `None` when another xaga process holds
/// the library's flock: this instance is read-only (`.md` files and LanceDB alike). The
/// `EmbeddingService` shares the writer's lock (`writer.lock()`) for its own lifetime.
pub(crate) struct LibraryHandle {
    pub(crate) path: PathBuf,
    pub(crate) writer: Option<xaga_core::library_writer::LibraryWriter>,
}

pub(crate) struct AppState {
    pub(crate) shutdown_started: std::sync::atomic::AtomicBool,
    pub(crate) embedding_initialization: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    pub(crate) read_only_startup: Arc<tokio::sync::Notify>,
    pub(crate) library_upgrade: Arc<Mutex<commands::library::UpgradeState>>,
    pub(crate) library: Arc<RwLock<LibraryHandle>>,
    /// The account-wide write right (flock on `settings.json`), shared in-process and
    /// reference-counted so settings writes and the model download never self-contend.
    pub(crate) account_lock: Arc<Mutex<AccountLock>>,
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
    pub(crate) sidebar_sort: Arc<RwLock<sidebar_sort::SidebarSort>>,
    pub(crate) embedding_service: Arc<Mutex<Option<EmbeddingService>>>,
    /// Model used by interactive text queries. Kept separately from the
    /// writer service so read-only instances can still search.
    pub(crate) embedding_query_model: Arc<Mutex<Option<Arc<Embedder>>>>,
    pub(crate) embedding_startup_state: Arc<RwLock<EmbeddingStartupState>>,
    /// Read-only LanceDB handle for command reads. Distinct connection from
    /// the embedding service's writer; reads must `reload()` to see the service's writes.
    pub(crate) embeddings_read: Arc<Mutex<Option<EmbeddingsDb>>>,
    pub(crate) allowed_domains: Arc<RwLock<Vec<String>>>,
    pub(crate) named_stores_supported: bool,
    pub(crate) inline_pdf_reader: Arc<RwLock<bool>>,
    pub(crate) load_remote_image: Arc<RwLock<bool>>,
    pub(crate) html_reader_view: Arc<RwLock<String>>,
    /// Port of the local SSRF-filtering proxy that loader webviews and the
    /// per-invocation fetch clients (see [`fetch_client`]) are routed through.
    pub(crate) network_proxy_port: u16,
    pub(crate) http_allowed: Arc<RwLock<bool>>,
    pub(crate) connect_nonstandard_ports_allowed: Arc<RwLock<bool>>,
    /// Redirect cap for the reqwest fetch paths (feed XML), already
    /// clamped to [`fetch_client::MAX_REDIRECTS_CEILING`]. The article loader
    /// webview is not covered (docs/architecture/security/fetching_untrusted_content.md,
    /// "Limits").
    pub(crate) max_redirects: Arc<RwLock<u32>>,
    /// Whether the `Referer` header is allowed to go out on redirect hops
    /// (reqwest fetch paths); it names the previous URL. Off by default:
    /// redirect targets don't learn the original feed URL.
    pub(crate) referer_allowed: Arc<RwLock<bool>>,
    /// settings.json's mtime at the read that formed this process's view of
    /// the settings (startup load; refreshed on every funnel outcome). The
    /// baseline for the stale-view check in [`update_settings`].
    pub(crate) settings_baseline: Arc<RwLock<Option<std::time::SystemTime>>>,
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

    /// Carry source-resolved assignments with content before a rename can retire its aliases.
    pub(crate) async fn enqueue_entry(&self, tier: Tier, article: xaga_core::types::Article) {
        if self.embedding_service.lock().unwrap().is_none() {
            return;
        }
        let Some(writer) = self.writer() else {
            return;
        };
        match writer.resolve_entry_tags(article.meta.id.clone()).await {
            Ok((_, tag_ids)) => match writer
                .resolve_annotation_tags(article.meta.id.clone())
                .await
            {
                Ok((_, annotation_tag_ids)) => self.enqueue(
                    tier,
                    Job::UpsertEntry {
                        article,
                        tag_ids,
                        annotation_tag_ids,
                    },
                ),
                Err(error) => log::error!("resolve annotation tags before indexing: {error}"),
            },
            Err(error) => log::error!(
                "resolve entry tags before indexing {}: {error}",
                article.meta.id
            ),
        }
    }

    pub(crate) async fn enqueue_annotations(&self, article: xaga_core::types::Article) {
        let Some(writer) = self.writer() else {
            return;
        };
        match writer
            .resolve_annotation_tags(article.meta.id.clone())
            .await
        {
            Ok((_, annotation_tag_ids)) => self.enqueue(
                Tier::Interactive,
                Job::SyncAnnotations {
                    article,
                    annotation_tag_ids,
                },
            ),
            Err(error) => log::error!("resolve annotation tags before indexing: {error}"),
        }
    }

    /// The open library's path.
    pub(crate) fn library_path(&self) -> PathBuf {
        self.library.read().unwrap().path.clone()
    }

    /// Whether this instance is read-only for the open library — true when another xaga
    /// process holds the library lock (so there is no writer). Derived from the writer's
    /// presence (the single source of truth); no mode value is stored anywhere. The same
    /// name carries this concept at every layer: this accessor, the Tauri command, and the
    /// domain-event field.
    pub(crate) fn is_library_read_only(&self) -> bool {
        self.library.read().unwrap().writer.is_none()
    }

    /// The library writer, if this instance is the writer (read-write mode). `None` in
    /// read-only mode — but write commands are already refused at the IPC gate, so a
    /// write command body that reaches for this and finds `None` is a defense-in-depth
    /// path, mapped to `E_READ_ONLY`. Cloneable; all clones feed the one dispatcher.
    pub(crate) fn writer(&self) -> Option<xaga_core::library_writer::LibraryWriter> {
        self.library.read().unwrap().writer.clone()
    }
}

/// Refuse derived reads while conversion has detached the handle, or if recovery
/// could not reopen it. Never translate that condition into empty memberships.
fn ensure_library_reads_available(state: &AppState) -> Result<(), error::AppError> {
    use commands::library::UpgradeState;
    if matches!(
        &*state.library_upgrade.lock().unwrap(),
        UpgradeState::Running { .. }
            | UpgradeState::Failed {
                read_error: Some(_),
                ..
            }
            | UpgradeState::Succeeded {
                read_error: Some(_),
                ..
            }
    ) {
        return Err(error::AppError::LibraryReadUnavailable);
    }
    Ok(())
}

pub(crate) async fn get_unread_ids(
    state: &AppState,
) -> Result<std::collections::HashSet<String>, error::AppError> {
    ensure_library_reads_available(state)?;
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Ok(Default::default());
    };
    db.unread_ids().await.map_err(error::AppError::from)
}

pub(crate) async fn get_bookmarked_ids(
    state: &AppState,
) -> Result<std::collections::HashSet<String>, error::AppError> {
    ensure_library_reads_available(state)?;
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Ok(Default::default());
    };
    db.get_bookmarked_ids().await.map_err(error::AppError::from)
}

// ── Settings helpers ───────────────────────────────────────────────────────────

// ── Entry point ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup = startup::prepare();
    let gate_library = Arc::clone(&startup.state.library);
    let gate_layout = Arc::clone(&startup.layout_gate);
    let setup_gate_layout = Arc::clone(&startup.layout_gate);
    let model_download_client = startup.model_download_client;
    let app = tauri::Builder::default()
        .manage(startup.state)
        .manage(article_loader::PendingRequests::default())
        .manage(loader_identity::LoaderSessions::default())
        .manage(article_diagnostics::ArticleDiagnostics::default())
        .manage(startup.proxy_config)
        .manage(ui_network::UiErrors::default())
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
        .menu(menus::build)
        .on_menu_event(menus::on_event)
        .setup(move |app| {
            let state = app.state::<AppState>();
            startup::initialize(app, &state, model_download_client, setup_gate_layout)
        })
        .invoke_handler({
            // Central IPC gate: a single allowlist check runs before dispatch,
            // so loader (untrusted) webviews can reach only the loader callbacks
            // and every other command is protected without a per-command guard.
            let app_commands: Box<dyn Fn(tauri::ipc::Invoke) -> bool + Send + Sync> =
                Box::new(include!(concat!(env!("OUT_DIR"), "/command_handler.rs")));
            move |invoke: tauri::ipc::Invoke| {
                let (allowed, write_refused) = {
                    let label = invoke.message.webview_ref().label();
                    let command = invoke.message.command();
                    if !matches!(
                        command,
                        "add_loader_diagnostic_event" | "add_article_diagnostic_event"
                    ) {
                        log::info!("ipc: {label} → {command}");
                    }
                    // Library-write gate: a read-only instance (no library lock) refuses
                    // every library-writing command at this single choke point — mode is
                    // derived from the held lock, never stored (see LibraryHandle).
                    let is_library_read_only = gate_library.read().unwrap().writer.is_none();
                    (
                        commands::is_invoke_allowed(label, command),
                        commands::is_write_refused(command, is_library_read_only)
                            || (gate_layout.load(std::sync::atomic::Ordering::SeqCst)
                                && !commands::is_layout_gate_allowed(command)),
                    )
                };
                if !allowed {
                    invoke.resolver.reject(crate::error::AppError::Unauthorized);
                    return true;
                }
                if write_refused {
                    invoke.resolver.reject(crate::error::AppError::ReadOnly);
                    return true;
                }
                app_commands(invoke)
            }
        })
        .build(app_context())
        .expect("error while building tauri application");

    install_signal_handler(app.handle().clone());

    app.run(handle_run_event);
}

#[cfg(test)]
#[path = "../tests/support/app_state.rs"]
mod test_app_state;

fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}
