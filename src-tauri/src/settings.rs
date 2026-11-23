use super::*;

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
pub(crate) const DEFAULT_PUBLISHER_JAVASCRIPT_ALLOWED: bool = false;

pub(crate) const DEFAULT_LOADER_TIMEOUT_SECS: u64 = 10;
pub(crate) const DEFAULT_FEED_TIMEOUT_SECS: u64 = 10;
pub(crate) const DEFAULT_FEED_MAX_BODY_BYTES: u64 = 10 * 1024 * 1024;
/// Maximum bytes read from a local import file (OPML or bookmarks.html), shared
/// by both formats rather than one per format.
pub(crate) const DEFAULT_IMPORT_MAX_BYTES: u64 = 64 * 1024 * 1024;

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
    pub(crate) sidebar_sort: Option<sidebar_sort::SidebarSort>,
    pub(crate) allowed_domains: Option<Vec<String>>,
    pub(crate) inline_pdf_reader: Option<bool>,
    pub(crate) load_remote_image: Option<bool>,
    pub(crate) publisher_javascript_allowed: Option<bool>,
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
    pub(super) fn publisher_javascript_allowed(&self) -> bool {
        self.publisher_javascript_allowed
            .unwrap_or(DEFAULT_PUBLISHER_JAVASCRIPT_ALLOWED)
    }

    pub(super) fn load_remote_image(&self) -> bool {
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
pub(super) fn settings_mtime(path: &Path) -> Option<std::time::SystemTime> {
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

pub(super) fn update_settings_file(
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
    *state.publisher_javascript_allowed.write().unwrap() = s.publisher_javascript_allowed();
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
    *state.http_allowed.write().unwrap() = s.http_allowed.unwrap_or(false);
    *state.connect_nonstandard_ports_allowed.write().unwrap() =
        s.connect_nonstandard_ports_allowed.unwrap_or(false);
    *state.html_reader_font_family.write().unwrap() = s
        .html_reader_font_family
        .unwrap_or_else(|| "system".to_string());
    *state.html_reader_font_size.write().unwrap() = s.html_reader_font_size.unwrap_or(16.0);
    *state.html_reader_line_height.write().unwrap() = s.html_reader_line_height.unwrap_or(1.75);
    *state.show_sidebar.write().unwrap() = s.show_sidebar.unwrap_or(true);
    *state.sidebar_sort.write().unwrap() = s.sidebar_sort.unwrap_or_default();
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

#[cfg(test)]
mod settings_funnel_tests {
    use super::*;

    #[test]
    fn failed_replacement_retains_download_lease_and_original_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"theme":"light"}"#).unwrap();
        let before = std::fs::read(&path).unwrap();
        let registry = Arc::new(Mutex::new(AccountLock::default()));
        let lease = account_lease(&registry, &path).unwrap();
        // A directory at the staging path forces replacement to fail before rename.
        std::fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(
            update_settings_file(&registry, &path, settings_mtime(&path), |settings| {
                settings.theme = Some("dark".into());
            })
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(xaga_core::fs::FileLock::try_acquire(path.clone()).is_none());
        drop(lease);
        assert!(xaga_core::fs::FileLock::try_acquire(path).is_some());
    }

    #[test]
    fn sidebar_sort_is_one_persisted_preference_with_unread_default() {
        use sidebar_sort::SidebarSort;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let registry = Mutex::new(AccountLock::default());
        let (_, baseline) =
            update_settings_file(&registry, &path, None, |s| s.theme = Some("dark".into()))
                .unwrap();
        assert_eq!(
            load_settings(&path).sidebar_sort.unwrap_or_default(),
            SidebarSort::Unread
        );
        update_settings_file(&registry, &path, baseline, |s| {
            s.sidebar_sort = Some(SidebarSort::Alphabetical)
        })
        .unwrap();
        let reopened = load_settings(&path);
        assert_eq!(reopened.sidebar_sort, Some(SidebarSort::Alphabetical));
        assert_eq!(reopened.theme.as_deref(), Some("dark"));
        assert!(!commands::is_write_refused("set_sidebar_sort", true));
        assert!(commands::is_invoke_allowed("main", "set_sidebar_sort"));
        assert!(!commands::is_invoke_allowed("loader-1", "set_sidebar_sort"));
    }

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

#[cfg(test)]
mod article_loader_setting_tests {
    use super::*;

    #[test]
    fn publisher_javascript_defaults_to_disabled_and_round_trips() {
        let missing: UserSettings = serde_json::from_str("{}").unwrap();
        assert!(!missing.publisher_javascript_allowed());
        for value in [false, true] {
            let settings: UserSettings =
                serde_json::from_value(serde_json::json!({"publisher_javascript_allowed": value}))
                    .unwrap();
            let roundtrip: UserSettings =
                serde_json::from_value(serde_json::to_value(settings).unwrap()).unwrap();
            assert_eq!(roundtrip.publisher_javascript_allowed(), value);
        }
    }

    #[test]
    fn remote_image_loading_uses_the_default_and_preserves_explicit_values() {
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
