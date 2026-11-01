mod article_diagnostics;
mod article_loader;
mod command_policy;
mod commands;
mod error;
mod events;
mod fetch_client;
mod loader_download;
mod loader_identity;
mod native;
mod resource_download;
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

/// Library mode is decided independently for each runtime path. Metadata errors
/// select read-only mode and are returned for the startup UI to report.
fn library_writer_for_startup(
    path: &Path,
    version: xaga_core::library::LibraryVersion,
) -> (
    Option<xaga_core::library_writer::LibraryWriter>,
    Option<xaga_core::library::LibraryFormatError>,
) {
    match xaga_core::library::open(path, version) {
        Ok(lock) => (
            lock.map(|lock| {
                xaga_core::library_writer::LibraryWriter::start(path.to_path_buf(), lock)
            }),
            None,
        ),
        Err(error) => (None, Some(error)),
    }
}

/// The app's settings.json path, under xaga-core's data root — the single
/// shared derivation, deliberately not Tauri's identifier-derived
/// `app_config_dir()` (which resolves to a different directory, `app.xaga`).
pub(crate) fn settings_path() -> Result<PathBuf, String> {
    Ok(xaga_core::fs::data_root()
        .ok_or("cannot determine platform data directory")?
        .join("settings.json"))
}

// Temporary default for the future Load remote image control. All consumers
// use the resolved setting; change this one value when the product default changes.
pub(crate) const DEFAULT_LOAD_REMOTE_IMAGE: bool = false;

pub(crate) const DEFAULT_LOADER_TIMEOUT_SECS: u64 = 10;
pub(crate) const DEFAULT_FEED_TIMEOUT_SECS: u64 = 10;
pub(crate) const DEFAULT_FEED_MAX_BODY_BYTES: u64 = 10 * 1024 * 1024;
/// Maximum bytes read from a local import file (OPML or bookmarks.html), shared
/// by both formats rather than one per format.
pub(crate) const DEFAULT_IMPORT_MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EmbeddingStartupState {
    Loading,
    Available,
    Unavailable,
}

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

/// The set of unread entry ids, read from LanceDB (the `is_unread` Bitmap index) via the shared
/// read handle. Reloads first (handles are version-pinned); empty if the handle isn't ready.
pub(crate) async fn get_unread_ids(state: &AppState) -> std::collections::HashSet<String> {
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Default::default();
    };
    db.unread_ids().await.unwrap_or_default()
}

/// The set of bookmarked entry ids, read from LanceDB (the `color` Bitmap index).
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
    pub(crate) inline_pdf_reader: Option<bool>,
    pub(crate) load_remote_image: Option<bool>,
    pub(crate) html_reader_view: Option<String>,
    /// Allow plaintext `http://` fetches (feeds, article loader and downloads).
    /// Enforced by the network proxy. Defaults to false if absent.
    pub(crate) http_allowed: Option<bool>,
    /// Maximum redirects per fetch on the reqwest paths (feed XML).
    /// 0 means the initial request only. Defaults to 3 if absent; clamped to
    /// 10. The default of 3 is a UX exception to secure-by-default — see
    /// docs/architecture/security/fetching_untrusted_content.md.
    pub(crate) max_redirects: Option<u32>,
    /// Whether the `Referer` header is allowed to go out on redirect hops
    /// (reqwest fetch paths); it names the previous URL to the redirect
    /// target. Defaults to false if absent.
    pub(crate) referer_allowed: Option<bool>,
    /// Whether the reqwest fetch paths (feed XML) send
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

impl UserSettings {
    fn load_remote_image(&self) -> bool {
        self.load_remote_image.unwrap_or(DEFAULT_LOAD_REMOTE_IMAGE)
    }
}

pub(crate) fn load_settings(path: &Path) -> UserSettings {
    (|| -> Option<UserSettings> {
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    })()
    .unwrap_or_default()
}

// ── Settings write funnel ─────────────────────────────────────────────────────
//
// Every settings.json write in the app goes through `update_settings`. The
// account-wide xaga resource lock IS the flock on settings.json (holding it
// grants the right to write settings.json, download the embedding model, and
// use any future account-wide resource); writers fail fast on contention.

/// Outcome of a settings write attempt. `StaleView` is a normal concurrency
/// outcome, not an error: another process changed settings.json since this
/// process formed its view, so the write was refused and the *fresh* content
/// (read under the lock) is returned for the caller to refresh from.
pub(crate) enum SettingsOutcome {
    // The payload is the committed state — part of the funnel's API (and read
    // by its tests); the current commands use their own captured values, so
    // non-test builds see the field as unread.
    Written(#[allow(dead_code)] UserSettings),
    StaleView(UserSettings),
}

/// settings.json's current mtime; `None` when the file does not exist *or is
/// empty*. The empty case matters because acquiring the account lock creates
/// the file (`create(true)`) — a zero-length settings.json carries no
/// settings (`load_settings` yields defaults for it), so for baseline
/// purposes it is the same as absent.
fn settings_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .ok()
        .filter(|m| m.len() > 0)
        .and_then(|m| m.modified().ok())
}

/// Core of the settings write funnel, factored free of Tauri state so it is
/// unit-testable: takes the caller's view baseline, returns the outcome plus
/// the new baseline (the file's mtime as of this attempt, captured under the
/// lock).
///
/// INVARIANT (owner directive): the write payload must be built only from the
/// state loaded *inside* the hold plus the caller's delta — never from data
/// read before the lock. Callers therefore pass a mutation closure, not a
/// settings struct.
/// The account-wide write right (the flock on `settings.json` under `data_root`), shared
/// in-process and reference-counted. The bug this fixes: acquiring a fresh `FileLock` per
/// call means the model download (a long hold) and a settings write open two handles on the
/// same file, and `flock(2)` contends between open file descriptions even within one process
/// — so the process locked itself out.
///
/// Operation-scoped, NOT process-lifetime: the flock is acquired when the first in-process
/// account operation begins and released when the last finishes, so a second instance (e.g.
/// on a different library) can still write settings while this process is idle — required
/// because the library path lives in `settings.json`, so `set_library` is an account write.
///
/// Settings writes are synchronous critical sections that hold the mutex across `write_locked`
/// (single-streamed → no two concurrent whole-file read-modify-writes clobber each other). The
/// model download holds a [`AccountLease`] that carries only `lease_count` across its async
/// fetch (it never writes settings), so a settings write can rotate the handle underneath it.
#[derive(Default)]
pub(crate) struct AccountLock {
    /// Number of long-lived holders (download leases). Settings writes are transient (they run
    /// fully inside one mutex hold) and are not counted; they consult this to decide whether to
    /// release the handle after writing.
    lease_count: usize,
    /// The held flock, on the CURRENT `settings.json` inode. `write_locked` rotates the inode
    /// and returns the lock on the new one; the shared handle is swapped in place, so a lease's
    /// exclusion transfers to the new inode without the lease holding a specific handle.
    handle: Option<xaga_core::fs::FileLock>,
}

impl AccountLock {
    /// Acquire the flock if not already held. `Err` means another *process* holds it.
    fn ensure_handle(&mut self, path: &Path) -> Result<(), crate::error::AppError> {
        if self.handle.is_none() {
            self.handle = Some(
                xaga_core::fs::FileLock::try_acquire(path.to_path_buf())
                    .ok_or(crate::error::AppError::SettingsBusy)?,
            );
        }
        Ok(())
    }

    /// Release the flock iff no lease still needs it (operation-scoped).
    fn release_if_idle(&mut self) {
        if self.lease_count == 0 {
            self.handle = None; // drops the FileLock → releases the flock
        }
    }

    /// Apply `delta` to `settings.json` under the held right, with the stale-view check inside
    /// the turn. Synchronous and mutex-guarded (single-streamed).
    fn write_settings(
        &mut self,
        path: &Path,
        baseline: Option<std::time::SystemTime>,
        delta: impl FnOnce(&mut UserSettings),
    ) -> Result<(SettingsOutcome, Option<std::time::SystemTime>), crate::error::AppError> {
        self.ensure_handle(path)?;

        // Stale-view check (owner's ts-at-read design), read inside the turn: a mismatch means
        // another process wrote since this process's baseline; refuse, hand back the fresh content.
        let current = settings_mtime(path);
        if current != baseline {
            let fresh = load_settings(path);
            self.release_if_idle();
            return Ok((SettingsOutcome::StaleView(fresh), current));
        }

        let mut settings = load_settings(path);
        delta(&mut settings);
        let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;

        let held = self.handle.take().expect("handle ensured above");
        match xaga_core::fs::write_locked(held, path, json.as_bytes()) {
            Ok(new_handle) => self.handle = Some(new_handle),
            Err((e, held)) => {
                // Hand-back: keep the still-valid lock (a lease may need it), then release
                // only if idle.
                self.handle = Some(held);
                self.release_if_idle();
                return Err(e.to_string().into());
            }
        }
        // Still holding the lock on the new inode, so this mtime is authoritative.
        let new_baseline = settings_mtime(path);
        self.release_if_idle();
        Ok((SettingsOutcome::Written(settings), new_baseline))
    }
}

/// A refcount token proving the account right is held for the token's lifetime — held by the
/// model download across its async fetch. Dropping it releases the flock when it was the last
/// holder. `None` from [`account_lease`] means another process holds the right.
pub(crate) struct AccountLease {
    registry: Arc<Mutex<AccountLock>>,
}

impl Drop for AccountLease {
    fn drop(&mut self) {
        let mut g = self.registry.lock().unwrap();
        g.lease_count -= 1;
        g.release_if_idle();
    }
}

/// Take an account lease (bumping the refcount, acquiring the flock if this is the first
/// holder). `None` if another process holds the right.
pub(crate) fn account_lease(
    registry: &Arc<Mutex<AccountLock>>,
    path: &Path,
) -> Option<AccountLease> {
    let mut g = registry.lock().unwrap();
    if g.ensure_handle(path).is_err() {
        return None;
    }
    g.lease_count += 1;
    drop(g);
    Some(AccountLease {
        registry: Arc::clone(registry),
    })
}

fn update_settings_file(
    registry: &Mutex<AccountLock>,
    path: &Path,
    baseline: Option<std::time::SystemTime>,
    delta: impl FnOnce(&mut UserSettings),
) -> Result<(SettingsOutcome, Option<std::time::SystemTime>), crate::error::AppError> {
    registry
        .lock()
        .unwrap()
        .write_settings(path, baseline, delta)
}

/// The settings write funnel: applies `delta` under the account lock with the
/// stale-view check, keeping this process's baseline current on every outcome.
pub(crate) fn update_settings(
    state: &AppState,
    delta: impl FnOnce(&mut UserSettings),
) -> Result<SettingsOutcome, crate::error::AppError> {
    let path = settings_path()?;
    let baseline = *state.settings_baseline.read().unwrap();
    let (outcome, new_baseline) =
        update_settings_file(&state.account_lock, &path, baseline, delta)?;
    *state.settings_baseline.write().unwrap() = new_baseline;
    Ok(outcome)
}

/// Refreshes this process's in-memory settings mirrors from `s` — used when a
/// write came back `StaleView` so the UI-facing state reflects what is
/// actually on disk. Defaults mirror the setup seeding. `library` is
/// deliberately not touched: switching the open library is `set_library`'s
/// job, not a side effect of a refused preference write.
pub(crate) fn refresh_state_from_settings(state: &AppState, s: &UserSettings) {
    *state.load_remote_image.write().unwrap() = s.load_remote_image();
    let s = s.clone();
    *state.theme.write().unwrap() = s.theme.unwrap_or_else(|| "auto".to_string());
    *state.refresh_on_startup.write().unwrap() = s.refresh_on_startup.unwrap_or(false);
    *state.loader_timeout_secs.write().unwrap() =
        s.loader_timeout_secs.unwrap_or(DEFAULT_LOADER_TIMEOUT_SECS);
    *state.feed_timeout_secs.write().unwrap() =
        s.feed_timeout_secs.unwrap_or(DEFAULT_FEED_TIMEOUT_SECS);
    *state.feed_max_body_bytes.write().unwrap() =
        s.feed_max_body_bytes.unwrap_or(DEFAULT_FEED_MAX_BODY_BYTES);
    *state.import_max_bytes.write().unwrap() =
        s.import_max_bytes.unwrap_or(DEFAULT_IMPORT_MAX_BYTES);
    *state.max_redirects.write().unwrap() = fetch_client::effective_max_redirects(s.max_redirects);
    *state.referer_allowed.write().unwrap() = s.referer_allowed.unwrap_or(false);
    *state.gzip_allowed.write().unwrap() = s.gzip_allowed.unwrap_or(false);
    *state.html_reader_font_family.write().unwrap() = s
        .html_reader_font_family
        .unwrap_or_else(|| "system".to_string());
    *state.html_reader_font_size.write().unwrap() = s.html_reader_font_size.unwrap_or(16.0);
    *state.html_reader_line_height.write().unwrap() = s.html_reader_line_height.unwrap_or(1.75);
    *state.show_sidebar.write().unwrap() = s.show_sidebar.unwrap_or(true);
    *state.allowed_domains.write().unwrap() = s.allowed_domains.unwrap_or_default();
    *state.inline_pdf_reader.write().unwrap() = s.inline_pdf_reader.unwrap_or(false);
    *state.html_reader_view.write().unwrap() =
        s.html_reader_view.unwrap_or_else(|| "reader".to_string());
}

/// The uniform caller-side handling of a [`SettingsOutcome::StaleView`]:
/// refresh the mirrors and report the refusal to the webview.
pub(crate) fn stale_view_error(state: &AppState, fresh: &UserSettings) -> crate::error::AppError {
    refresh_state_from_settings(state, fresh);
    crate::error::AppError::SettingsStaleView
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

/// Applies the persisted setting before the frontend stylesheet or React can paint.
fn theme_bootstrap_script(theme_setting: &str) -> String {
    let setting = match theme_setting {
        "light" | "dark" => theme_setting,
        _ => "auto",
    };
    let setting = serde_json::to_string(setting).expect("theme setting serializes");
    format!(
        "(() => {{ const setting = {setting}; const resolved = setting === 'auto' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : setting; window.__XAGA_THEME_SETTING__ = setting; document.documentElement.dataset.theme = resolved; document.documentElement.style.colorScheme = resolved; document.documentElement.style.backgroundColor = resolved === 'dark' ? '#121212' : '#ffffff'; }})();"
    )
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
    let proxy_config = network_proxy::ProxyConfig::new(
        Arc::clone(&http_allowed),
        Arc::clone(&connect_nonstandard_ports_allowed),
    );
    let network_proxy_port =
        tauri::async_runtime::block_on(network_proxy::start(proxy_config.clone()))
            .expect("failed to start SSRF filtering proxy for the article loader");

    // The model download dials through the proxy like every other outbound
    // request (docs/architecture/security/fetching_remote_content.md,
    // "Who dials through it").
    let model_download_client = network_proxy::proxied_client(network_proxy_port)
        .expect("failed to build the proxied model-download client");

    // Pre-created Arcs shared between .manage() and the .setup() closure.
    let library: Arc<RwLock<LibraryHandle>> = Arc::new(RwLock::new(LibraryHandle {
        path: PathBuf::new(),
        writer: None,
    }));
    let account_lock: Arc<Mutex<AccountLock>> = Arc::new(Mutex::new(AccountLock::default()));
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
    let embedding_query_model: Arc<Mutex<Option<Arc<Embedder>>>> = Arc::new(Mutex::new(None));
    let embedding_startup_state = Arc::new(RwLock::new(EmbeddingStartupState::Loading));
    let embeddings_read: Arc<Mutex<Option<EmbeddingsDb>>> = Arc::new(Mutex::new(None));
    let allowed_domains: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    let load_remote_image = Arc::new(RwLock::new(DEFAULT_LOAD_REMOTE_IMAGE));
    let inline_pdf_reader: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let html_reader_view: Arc<RwLock<String>> = Arc::new(RwLock::new("reader".to_string()));
    let settings_baseline: Arc<RwLock<Option<std::time::SystemTime>>> = Arc::new(RwLock::new(None));

    // setup_ prefixed values are consumed by Tauri's .setup() closure.
    // There is no semantic meaning here.
    let gate_library = Arc::clone(&library);
    let setup_account_lock = Arc::clone(&account_lock);
    let setup_embeddings_read = Arc::clone(&embeddings_read);
    let setup_allowed_domains = Arc::clone(&allowed_domains);
    let setup_load_remote_image = Arc::clone(&load_remote_image);
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
    let setup_embedding_query_model = Arc::clone(&embedding_query_model);
    let setup_embedding_startup_state = Arc::clone(&embedding_startup_state);
    let setup_settings_baseline = Arc::clone(&settings_baseline);

    let app = tauri::Builder::default()
        .manage(AppState {
            library,
            account_lock,
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
            embedding_query_model,
            embedding_startup_state,
            embeddings_read,
            allowed_domains,
            named_stores_supported,
            inline_pdf_reader,
            load_remote_image,
            html_reader_view,
            network_proxy_port,
            max_redirects,
            referer_allowed,
            gzip_allowed,
            settings_baseline,
        })
        .manage(article_loader::PendingRequests::default())
        .manage(loader_identity::LoaderSessions::default())
        .manage(article_diagnostics::ArticleDiagnostics::default())
        .manage(proxy_config)
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
                    .on_download(|_, _| false)
                    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                    .proxy_url(
                        network_proxy::proxy_url(proxy.port).parse().expect("local proxy URL"),
                    )
                    .title("Settings")
                    .inner_size(680.0, 520.0)
                    .visible(false);
                    #[cfg(target_os = "macos")]
                    let builder = match webview_policy::ui_configuration() {
                        Ok(config) => builder.with_webview_configuration(config),
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
            _ => {}
        })
        .setup(move |app| {
            // Resolve settings before creating the webview so its document-start script can apply
            // the persisted theme before the first page paint.
            let settings_path = settings_path()?;
            let settings = load_settings(&settings_path);
            let theme = settings
                .theme
                .clone()
                .unwrap_or_else(|| "auto".to_string());
            *setup_settings_baseline.write().unwrap() = settings_mtime(&settings_path);
            let default_library = xaga_core::fs::data_root().ok_or("cannot determine platform data directory")?;
            let lib_path = settings.library.as_ref().map(PathBuf::from).unwrap_or(default_library);
            // Acquire the library lock (or learn we are read-only) BEFORE any command can
            // run, and — if we won it — start the LibraryWriter that takes ownership of the
            // lock. The writer's presence is the single source of truth for the runtime
            // mode, and the write-command gate derives from it. Writer/lock lifetime is
            // process-lifetime — `set_library` restarts the process, and the restart path
            // drops the writer (releasing the lock) before spawning the successor.
            let (writer, metadata_error) = library_writer_for_startup(&lib_path,
                xaga_core::library::LibraryVersion::new(app.package_info().version.major, app.package_info().version.minor));
            if let Some(error) = metadata_error {
                log::error!("startup: library metadata could not be read: {error}");
                use tauri_plugin_dialog::DialogExt;
                app.dialog().message(format!("{} This library will open in read-only mode.", error.user_message()))
                    .title("Library opened read-only").kind(tauri_plugin_dialog::MessageDialogKind::Error).blocking_show();
            }
            // The EmbeddingService shares the writer's lock for its lifetime.
            let service_lock = writer.as_ref().map(|w| w.lock());
            let service_writer = writer.clone();
            if writer.is_none() {
                log::warn!(
                    "startup: opening library in read-only mode"
                );
            }
            *setup_library.write().unwrap() = LibraryHandle {
                path: lib_path.clone(),
                writer,
            };


            // Build explicitly so the native WebRTC policy precedes even the
            // first document. The config's `create: false` prevents auto-build.
            let proxy = ui_network::start(app.handle(), "main")?;
            let builder = WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .initialization_script(theme_bootstrap_script(&theme))
                .on_download(|_, _| false)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .proxy_url(network_proxy::proxy_url(proxy.port).parse()?);
            #[cfg(target_os = "macos")]
            let builder = builder.with_webview_configuration(
                webview_policy::ui_configuration().map_err(std::io::Error::other)?,
            );
            let window = builder.build()?;
            ui_network::retain_for_window(&window, proxy);
            *setup_load_remote_image.write().unwrap() = settings.load_remote_image();
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
            // Announce the mode to any already-loaded frontend; late-loading frontends
            // read it via the `is_library_read_only` command instead. Mode is fixed for
            // the process's lifetime (a library switch restarts the process).
            events::emit_domain(
                app.handle(),
                xaga_core::events::DomainEvent::LibraryModeChanged {
                    is_library_read_only: service_lock.is_none(),
                },
            );
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

            // Start the EmbeddingService; startup recovery runs inside it.
            // If another instance already holds the library lock, we skip the
            // service entirely — embeddings are available for reading but this
            // instance will not write to LanceDB.
            let app_handle = app.handle().clone();
            let account_settings_path = settings_path.clone();
            tauri::async_runtime::spawn(async move {
                // Model download (caller-acquires the account right): outer check, then take an
                // in-process account lease held across the whole fetch (so a concurrent settings
                // write shares the same handle instead of self-contending). Another process
                // holding the right → skip; the next launch retries.
                if !xaga_core::embeddings::model_present() {
                    match account_lease(&setup_account_lock, &account_settings_path) {
                        Some(_lease) => {
                            if let Err(e) =
                                xaga_core::embeddings::download_model(&model_download_client).await
                            {
                                log::error!("startup: model download failed: {e}");
                            }
                        }
                        None => log::info!(
                            "startup: another xaga process holds the account lock — skipping model download"
                        ),
                    }
                }

                // Writer status was decided at setup (the LibraryWriter in the
                // LibraryHandle); a read-only instance runs without the service —
                // embeddings stay readable, nothing is written. The service holds its own
                // Arc of the writer's lock for its lifetime, preserving its invariant of
                // never running unlocked.
                let Some(_library_lock) = service_lock else {
                    log::warn!("startup: read-only library — embedding service not started");
                    // Non-holders READ ("the library lock was only ever supposed to
                    // protect concurrent write to the database" — owner): open a
                    // standalone read-only handle and query model so embedding
                    // reads and semantic text queries remain available. The
                    // handle is version-pinned at open (reads do not reload per-call):
                    // this instance sees the library as of ITS startup; the writer's
                    // later commits appear after relaunch — same visibility rule as the
                    // `.md` side (no live watcher; recovery-at-startup is the answer).
                    let db = EmbeddingsDb::open_readonly(&lib_path).await;
                    let model = xaga_core::embeddings::load_default_embedder().await;
                    match (db, model) {
                        (Ok(db), Ok(model)) => {
                            *setup_embeddings_read.lock().unwrap() = Some(db);
                            *setup_embedding_query_model.lock().unwrap() = Some(model);
                            *setup_embedding_startup_state.write().unwrap() =
                                EmbeddingStartupState::Available;
                        }
                        (db, model) => {
                            if let Err(e) = db {
                                log::error!("startup: read-only embeddings open failed: {e}");
                            }
                            if let Err(e) = model {
                                log::error!("startup: read-only embedding model failed: {e}");
                            }
                            *setup_embedding_startup_state.write().unwrap() =
                                EmbeddingStartupState::Unavailable;
                        }
                    }
                    return;
                };

                match EmbeddingService::start(lib_path.clone(), service_writer.unwrap()).await {
                    Ok(svc) => {
                        // The read handle shares the service's write handle (same underlying table
                        // state), so command reads see the service's writes — and the synchronous
                        // post-compaction reload — without any per-read reload.
                        *setup_embeddings_read.lock().unwrap() = Some(svc.read_handle());
                        *setup_embedding_query_model.lock().unwrap() = Some(svc.embedder());

                        let embeddings_app_handle = app_handle.clone();
                        let priority_drained_rx = svc.subscribe_to_priority_drained();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                if priority_drained_rx.recv_async().await.is_err() {
                                    break;
                                }
                                events::emit_domain(&embeddings_app_handle, xaga_core::events::DomainEvent::EmbeddingChanged);
                            }
                        });

                        let embeddings_app_handle = app_handle.clone();
                        let background_drained_rx = svc.subscribe_to_background_drained();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                if background_drained_rx.recv_async().await.is_err() {
                                    break;
                                }
                                events::emit_domain(&embeddings_app_handle, xaga_core::events::DomainEvent::EmbeddingChanged);
                            }
                        });

                        let unread_app_handle = app_handle.clone();
                        let unread_rx = svc.subscribe_to_unread_changed();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                match unread_rx.recv_async().await {
                                    Ok(entry) => { events::emit_domain(&unread_app_handle, xaga_core::events::DomainEvent::UnreadChanged(entry)); }
                                    Err(_) => break,
                                }
                            }
                        });

                        let bookmarked_app_handle = app_handle.clone();
                        let bookmarked_rx = svc.subscribe_to_bookmarked_changed();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                match bookmarked_rx.recv_async().await {
                                    Ok(entry) => { events::emit_domain(&bookmarked_app_handle, xaga_core::events::DomainEvent::BookmarkChanged(entry)); }
                                    Err(_) => break,
                                }
                            }
                        });

                        let annotation_app_handle = app_handle.clone();
                        let annotation_rx = svc.subscribe_to_annotation_changed();
                        tauri::async_runtime::spawn(async move {
                            loop {
                                match annotation_rx.recv_async().await {
                                    Ok(item) => { events::emit_domain(&annotation_app_handle, xaga_core::events::DomainEvent::AnnotationChanged(item)); }
                                    Err(_) => break,
                                }
                            }
                        });

                        svc.trigger_recovery(lib_path.clone());
                        *setup_svc.lock().unwrap() = Some(svc);
                        *setup_embedding_startup_state.write().unwrap() =
                            EmbeddingStartupState::Available;
                    }
                    Err(e) => {
                        log::error!("startup: embedding service failed: {e}");
                        *setup_embedding_startup_state.write().unwrap() =
                            EmbeddingStartupState::Unavailable;
                    }
                }
            });

            // Logging is installed in every build so recovery and startup diagnostics are captured.
            // Targets are set explicitly (`.targets` replaces the plugin defaults) so the destination
            // is deterministic: every build persists to
            // the xaga log dir (macOS: ~/Library/Logs/xaga/Xaga.log — the plain `xaga` name,
            // not Tauri's identifier-derived `app.xaga`, matching the data-root rule); dev
            // additionally mirrors to stdout and the webview console. Release logs at Info
            // (recovery's key lines are info!/warn!); dev stays at Debug.
            //
            // `lance` is pinned to Warn. At Info it logs one line per row during
            // compaction (~1800 lines per pass), which fills the plugin's 40 KB
            // default log file and rotates our startup/recovery lines away within
            // seconds. The file cap is raised for the same reason.
            {
                use tauri_plugin_log::{Target, TargetKind};

                let crate_level = if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                };

                // macOS: ~/Library/Logs/xaga. Other
                // platforms: logs under the data root — no platform-native log
                // dir convention is asserted for them (cross-platform spike).
                let log_dir = if cfg!(target_os = "macos") {
                    dirs::home_dir().map(|h| h.join("Library/Logs/xaga"))
                } else {
                    xaga_core::fs::data_root().map(|d| d.join("logs"))
                }
                .ok_or("cannot determine log directory")?;
                let mut targets = vec![Target::new(TargetKind::Folder {
                    path: log_dir,
                    file_name: None,
                })];
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
                Box::new(include!(concat!(env!("OUT_DIR"), "/command_handler.rs")));
            move |invoke: tauri::ipc::Invoke| {
                let (allowed, write_refused) = {
                    let label = invoke.message.webview_ref().label();
                    let command = invoke.message.command();
                    if !matches!(command, "add_loader_diagnostic_event" | "add_article_diagnostic_event") {
                        log::info!("ipc: {label} → {command}");
                    }
                    // Library-write gate: a read-only instance (no library lock) refuses
                    // every library-writing command at this single choke point — mode is
                    // derived from the held lock, never stored (see LibraryHandle).
                    let is_library_read_only = gate_library.read().unwrap().writer.is_none();
                    (
                        commands::is_invoke_allowed(label, command),
                        commands::is_write_refused(command, is_library_read_only),
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
    events::emit_domain(app, xaga_core::events::DomainEvent::EmbeddingShutdown);
    let library = Arc::clone(&app.state::<AppState>().library);
    tauri::async_runtime::spawn(async move {
        let reason = svc.shutdown().await;
        // The service's lock Arc is now gone; drain and drop the writer to release the
        // library flock before (only on restart) the successor tries to acquire it.
        drain_and_release_writer(&library).await;
        match reason {
            ShutdownReason::Restart => restart_process(),
            ShutdownReason::Exit => std::process::exit(0),
        }
    });
}

/// Drains the library writer (Interactive backlog to disk, Regular dropped — the tier
/// contract) and drops it, releasing this process's hold on the library flock. Must run
/// before [`restart_process`] spawns the successor: the successor acquires the lock at ITS
/// startup, and if the (still-living) predecessor's writer held it, the new instance would
/// come up read-only. Caller must have already shut the `EmbeddingService` down (its Arc of
/// the same lock must be gone) so that dropping the writer releases the flock. No-op in a
/// read-only instance (no writer).
pub(crate) async fn drain_and_release_writer(library: &Arc<RwLock<LibraryHandle>>) {
    let writer = library.write().unwrap().writer.take();
    if let Some(writer) = writer {
        writer.shutdown().await;
        // `writer` drops here: its lock Arc is the last one, so the flock releases.
    }
}

/// Installs a SIGINT/SIGTERM handler that drains the embedding service before exit.
fn install_signal_handler(
    embedding_service: Arc<Mutex<Option<EmbeddingService>>>,
    app_handle: tauri::AppHandle,
) {
    ctrlc::set_handler(move || {
        let svc = embedding_service.lock().unwrap().take();
        let library = Arc::clone(&app_handle.state::<AppState>().library);
        if let Some(svc) = svc {
            log::info!("signal: waiting for embedding service to drain");
            events::emit_domain(
                &app_handle,
                xaga_core::events::DomainEvent::EmbeddingShutdown,
            );
            tauri::async_runtime::block_on(svc.shutdown());
            // Signals always exit — restart flag is intentionally ignored.
        }
        // Drain the writer's Interactive backlog to disk before exit (Regular dropped).
        tauri::async_runtime::block_on(drain_and_release_writer(&library));
        std::process::exit(0);
    })
    .expect("failed to install signal handler");
}

#[cfg(test)]
mod library_mode_tests {
    use super::*;

    fn seed_version(dir: &std::path::Path, version: &str) {
        let lock = xaga_core::library::acquire_lock(dir).unwrap();
        xaga_core::library::LibraryManifest::begin_upgrade(dir, version.parse().unwrap(), &lock)
            .unwrap()
            .unwrap()
            .complete()
            .unwrap();
    }

    fn handle_for(dir: &std::path::Path) -> Arc<RwLock<LibraryHandle>> {
        let (writer, _) = library_writer_for_startup(dir, "0.10".parse().unwrap());
        Arc::new(RwLock::new(LibraryHandle {
            path: dir.to_path_buf(),
            writer,
        }))
    }

    /// Read-write vs read-only is derived from one fact: whether a writer
    /// exists. Winning the flock starts a writer; losing does not.
    #[test]
    fn mode_derives_from_writer_presence() {
        let dir = tempfile::tempdir().unwrap();
        seed_version(dir.path(), "0.10");

        let writer = handle_for(dir.path());
        assert!(
            writer.read().unwrap().writer.is_some(),
            "first instance wins the lock and starts a writer"
        );

        // A second "instance" (second handle in this process: flock conflicts across
        // open file descriptions, so this models a contender) loses and is read-only.
        let reader = handle_for(dir.path());
        assert!(
            reader.read().unwrap().writer.is_none(),
            "contender must lose while held"
        );
    }

    #[test]
    fn mismatched_and_unversioned_libraries_have_no_writer() {
        let unversioned = tempfile::tempdir().unwrap();
        assert!(
            handle_for(unversioned.path())
                .read()
                .unwrap()
                .writer
                .is_none()
        );
        assert_eq!(std::fs::read_dir(unversioned.path()).unwrap().count(), 0);
        let older = tempfile::tempdir().unwrap();
        seed_version(older.path(), "0.9");
        let before = std::fs::read(older.path().join("library.yaml")).unwrap();
        assert!(handle_for(older.path()).read().unwrap().writer.is_none());
        assert_eq!(
            std::fs::read(older.path().join("library.yaml")).unwrap(),
            before
        );
        assert!(!xaga_core::library::index_path(older.path()).exists());
    }

    #[test]
    fn malformed_metadata_opens_readonly_and_is_returned_for_reporting() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("library.yaml"), "format_version: [broken").unwrap();
        let (writer, error) = library_writer_for_startup(dir.path(), "0.10".parse().unwrap());
        assert!(writer.is_none());
        assert!(error.is_some());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("library.yaml")).unwrap(),
            "format_version: [broken"
        );
        assert!(!dir.path().join("xaga.lock").exists());
    }

    /// The restart-ordering invariant: draining and dropping the writer frees the flock
    /// for a successor process — what the restart path must guarantee before
    /// `restart_process` spawns one. Also covers the service's shared hold: while the
    /// EmbeddingService's Arc of the lock lives, the flock stays held.
    #[tokio::test]
    async fn drain_and_release_frees_the_lock_only_after_every_holder_drops() {
        let dir = tempfile::tempdir().unwrap();
        seed_version(dir.path(), "0.10");

        let library = handle_for(dir.path());
        // The service's clone of the lock (EmbeddingService::start receives one).
        let service_hold = library.read().unwrap().writer.as_ref().unwrap().lock();

        drain_and_release_writer(&library).await;
        assert!(
            library.read().unwrap().writer.is_none(),
            "writer removed (mode is now read-only)"
        );
        assert!(
            xaga_core::library::acquire_lock(dir.path()).is_none(),
            "flock must remain held while the service still holds its Arc"
        );

        drop(service_hold);
        assert!(
            xaga_core::library::acquire_lock(dir.path()).is_some(),
            "successor must win the lock once every holder is gone"
        );
    }
}

#[cfg(test)]
mod settings_funnel_tests {
    use super::*;

    #[test]
    fn import_limit_is_persisted_without_replacing_other_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let registry = Mutex::new(AccountLock::default());
        let (_, baseline) = update_settings_file(&registry, &path, None, |s| {
            s.theme = Some("dark".into());
        })
        .unwrap();
        assert_eq!(
            load_settings(&path)
                .import_max_bytes
                .unwrap_or(DEFAULT_IMPORT_MAX_BYTES),
            64 * 1024 * 1024
        );
        update_settings_file(&registry, &path, baseline, |s| {
            s.import_max_bytes = Some(128 * 1024 * 1024);
        })
        .unwrap();
        let reloaded = load_settings(&path);
        assert_eq!(reloaded.import_max_bytes, Some(128 * 1024 * 1024));
        assert_eq!(reloaded.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn contended_settings_write_is_typed_and_can_be_retried() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let other = xaga_core::fs::FileLock::try_acquire(path.clone()).unwrap();
        let registry = Mutex::new(AccountLock::default());
        let refused = update_settings_file(&registry, &path, None, |_| panic!("must not write"));
        assert!(matches!(refused, Err(crate::error::AppError::SettingsBusy)));
        drop(other);
        assert!(update_settings_file(&registry, &path, settings_mtime(&path), |_| {}).is_ok());
    }

    /// Plan test 5: the stale-view outcome. A write attempted with a baseline
    /// older than the file's mtime is refused with the fresh content; a retry
    /// with the refreshed baseline succeeds against the fresh state.
    #[test]
    fn stale_view_refused_then_retry_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        // One registry: the stale-view mechanic is baseline-vs-mtime, independent of the
        // in-process handle sharing, so a single registry exercises it faithfully.
        let registry = Mutex::new(AccountLock::default());

        // First-ever write: baseline None matches the missing file.
        let (outcome, baseline) =
            update_settings_file(&registry, &path, None, |s| s.theme = Some("dark".into()))
                .unwrap();
        let SettingsOutcome::Written(written) = outcome else {
            panic!("expected Written");
        };
        assert_eq!(written.theme.as_deref(), Some("dark"));

        // A second writer (simulating another process, which read the file and
        // holds the then-current baseline) writes.
        std::thread::sleep(std::time::Duration::from_millis(10));
        let (outcome2, baseline_after_other) =
            update_settings_file(&registry, &path, baseline, |s| {
                s.theme = Some("light".into())
            })
            .unwrap();
        assert!(matches!(outcome2, SettingsOutcome::Written(_)));

        // The first process still holds its old baseline → refused, and the
        // payload is the fresh on-disk content.
        let (outcome3, refreshed_baseline) =
            update_settings_file(&registry, &path, baseline, |s| {
                s.theme = Some("auto".into())
            })
            .unwrap();
        let SettingsOutcome::StaleView(fresh) = outcome3 else {
            panic!("expected StaleView");
        };
        assert_eq!(fresh.theme.as_deref(), Some("light"));
        assert_eq!(refreshed_baseline, baseline_after_other);

        // Retry with the refreshed baseline: succeeds, delta applied to the
        // fresh state.
        let (outcome4, _) = update_settings_file(&registry, &path, refreshed_baseline, |s| {
            s.theme = Some("auto".into())
        })
        .unwrap();
        let SettingsOutcome::Written(w) = outcome4 else {
            panic!("expected Written");
        };
        assert_eq!(w.theme.as_deref(), Some("auto"));
        // The stale writer's intended value never landed silently: the file
        // holds the retry's value, written knowingly against the fresh view.
    }

    /// The self-contention bug this whole change fixes: a held account lease (simulating an
    /// in-progress model download) must NOT block a settings write in the *same* process —
    /// they share the one refcounted handle instead of opening two contending `flock`s. And
    /// the flock is released only once the last holder (the lease) drops.
    #[test]
    fn settings_write_succeeds_while_a_lease_is_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let registry = Arc::new(Mutex::new(AccountLock::default()));

        let lease = account_lease(&registry, &path).expect("first holder acquires the flock");
        // A settings write while the lease is held goes through (shared handle, no self-contention).
        let (outcome, _) =
            update_settings_file(&registry, &path, None, |s| s.theme = Some("dark".into()))
                .unwrap();
        assert!(matches!(outcome, SettingsOutcome::Written(_)));
        // Still held while the lease is alive (not released after the transient write).
        assert!(registry.lock().unwrap().handle.is_some());

        drop(lease);
        // Last holder gone → flock released (operation-scoped), so another instance could take it.
        assert!(registry.lock().unwrap().handle.is_none());
    }
}

fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg(test)]
mod remote_image_setting_tests {
    use super::*;

    #[test]
    fn missing_value_uses_central_default_and_explicit_values_win() {
        let missing: UserSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing.load_remote_image(), DEFAULT_LOAD_REMOTE_IMAGE);
        for value in [false, true] {
            let settings: UserSettings =
                serde_json::from_value(serde_json::json!({"load_remote_image": value})).unwrap();
            assert_eq!(settings.load_remote_image(), value);
            let roundtrip: UserSettings =
                serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
            assert_eq!(roundtrip.load_remote_image(), value);
        }
    }
}

#[cfg(test)]
mod embedding_startup_state_tests {
    use super::EmbeddingStartupState;

    #[test]
    fn states_serialize_for_the_frontend_contract() {
        for (state, expected) in [
            (EmbeddingStartupState::Loading, "\"loading\""),
            (EmbeddingStartupState::Available, "\"available\""),
            (EmbeddingStartupState::Unavailable, "\"unavailable\""),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), expected);
        }
    }
}
