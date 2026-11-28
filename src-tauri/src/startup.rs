use super::*;

fn select_library_path(default: PathBuf, configured: Option<&str>) -> std::io::Result<PathBuf> {
    if let Some(path) = configured {
        return Ok(PathBuf::from(path));
    }
    std::fs::create_dir_all(&default)?;
    Ok(default)
}

#[cfg(test)]
mod library_path_tests {
    use super::*;
    use xaga_core::library::{self, LibraryManifest, LibraryVersion};

    #[test]
    fn first_launch_initializes_a_missing_default_library() {
        let directory = tempfile::tempdir().unwrap();
        let default = directory.path().join("xaga");
        assert!(!default.exists());

        let path = select_library_path(default.clone(), None).unwrap();
        assert_eq!(path, default);
        let version = LibraryVersion::new(0, 18);
        let _lock = library::open(&path, version).unwrap().unwrap();
        let manifest = LibraryManifest::read(&path).unwrap().unwrap();
        assert!(manifest.matches_version(version));
    }

    #[test]
    fn a_missing_configured_library_is_not_created_at_startup() {
        let directory = tempfile::tempdir().unwrap();
        let default = directory.path().join("xaga");
        let configured = directory.path().join("configured");

        let path =
            select_library_path(default.clone(), Some(configured.to_str().unwrap())).unwrap();
        assert_eq!(path, configured);
        assert!(!path.exists());
        assert!(!default.exists());
    }
}

/// Library mode is decided independently for each runtime path. Metadata errors
/// select read-only mode and are returned for the startup UI to report.
pub(super) fn library_writer_for_startup(
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EmbeddingStartupState {
    Loading,
    #[serde(rename = "awaiting_library_choice")]
    AwaitingLibraryChoice,
    Available,
    Unavailable,
}

async fn await_library_choice(gate: &tokio::sync::Notify, startup: &RwLock<EmbeddingStartupState>) {
    gate.notified().await;
    *startup.write().unwrap() = EmbeddingStartupState::Loading;
}

#[cfg(test)]
mod embedding_startup_state_tests {
    use super::EmbeddingStartupState;

    #[tokio::test]
    async fn model_work_is_not_started_until_explicit_choice_even_if_it_would_fail_or_stall() {
        use std::sync::{
            RwLock,
            atomic::{AtomicBool, Ordering},
        };
        let gate = tokio::sync::Notify::new();
        let startup = RwLock::new(EmbeddingStartupState::AwaitingLibraryChoice);
        let model_called = AtomicBool::new(false);
        let initialization = async {
            super::await_library_choice(&gate, &startup).await;
            model_called.store(true, Ordering::SeqCst);
            // Offline/failed initialization is independent of the choice gate.
            Err::<(), _>("fixture offline model")
        };
        tokio::pin!(initialization);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut initialization)
                .await
                .is_err()
        );
        assert!(!model_called.load(Ordering::SeqCst));
        assert_eq!(
            *startup.read().unwrap(),
            EmbeddingStartupState::AwaitingLibraryChoice
        );
        gate.notify_one();
        assert!(initialization.await.is_err());
        assert!(model_called.load(Ordering::SeqCst));
        assert_eq!(*startup.read().unwrap(), EmbeddingStartupState::Loading);
        // A click arriving before the task waits is retained, not lost.
        gate.notify_one();
        super::await_library_choice(&gate, &startup).await;
    }

    #[test]
    fn states_serialize_for_the_frontend_contract() {
        for (state, expected) in [
            (EmbeddingStartupState::Loading, "\"loading\""),
            (
                EmbeddingStartupState::AwaitingLibraryChoice,
                "\"awaiting_library_choice\"",
            ),
            (EmbeddingStartupState::Available, "\"available\""),
            (EmbeddingStartupState::Unavailable, "\"unavailable\""),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), expected);
        }
    }
}

pub(super) struct Startup {
    pub(super) state: AppState,
    pub(super) proxy_config: network_proxy::ProxyConfig,
    pub(super) model_download_client: reqwest::Client,
    pub(super) layout_gate: Arc<std::sync::atomic::AtomicBool>,
}

pub(super) fn prepare() -> Startup {
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
    let sidebar_sort = Arc::new(RwLock::new(sidebar_sort::SidebarSort::default()));
    let embedding_service: Arc<Mutex<Option<EmbeddingService>>> = Arc::new(Mutex::new(None));
    let embedding_query_model: Arc<Mutex<Option<Arc<Embedder>>>> = Arc::new(Mutex::new(None));
    let embedding_startup_state = Arc::new(RwLock::new(EmbeddingStartupState::Loading));
    let embeddings_read: Arc<Mutex<Option<EmbeddingsDb>>> = Arc::new(Mutex::new(None));
    let allowed_domains: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    let load_remote_image = Arc::new(RwLock::new(DEFAULT_LOAD_REMOTE_IMAGE));
    let remote_images_allowed =
        Arc::new(RwLock::new(crate::settings::DEFAULT_REMOTE_IMAGES_ALLOWED));
    let publisher_javascript_allowed = Arc::new(RwLock::new(DEFAULT_PUBLISHER_JAVASCRIPT_ALLOWED));
    let inline_pdf_reader: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    let html_reader_view: Arc<RwLock<String>> = Arc::new(RwLock::new(
        crate::settings::DEFAULT_HTML_READER_VIEW.to_owned(),
    ));
    let settings_baseline: Arc<RwLock<Option<std::time::SystemTime>>> = Arc::new(RwLock::new(None));

    let read_only_startup = Arc::new(tokio::sync::Notify::new());
    let state = AppState {
        shutdown_started: std::sync::atomic::AtomicBool::new(false),
        embedding_initialization: Mutex::new(None),
        read_only_startup,
        library_upgrade: Arc::new(Mutex::new(Default::default())),
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
        sidebar_sort,
        embedding_service: embedding_service.clone(),
        embedding_query_model,
        embedding_startup_state,
        embeddings_read,
        allowed_domains,
        named_stores_supported,
        inline_pdf_reader,
        easyprivacy_refresh_days: Arc::new(RwLock::new(easyprivacy::MIN_REFRESH_DAYS)),
        easyprivacy_enabled: Arc::new(RwLock::new(true)),
        load_remote_image,
        remote_images_allowed,
        publisher_javascript_allowed,
        html_reader_view,
        network_proxy_port,
        http_allowed,
        connect_nonstandard_ports_allowed,
        max_redirects,
        referer_allowed,
        gzip_allowed,
        settings_baseline,
    };
    Startup {
        state,
        proxy_config,
        model_download_client,
        layout_gate: Arc::new(std::sync::atomic::AtomicBool::new(true)),
    }
}

pub(super) fn open_main_window(
    app: &tauri::App,
    theme: &str,
) -> Result<tauri::WebviewWindow, Box<dyn std::error::Error>> {
    // The config's `create: false` lets native policy precede navigation.
    let proxy = ui_network::start(app.handle(), "main")?;
    let builder = ui_network::configure_window(
        WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
            .initialization_script(theme_bootstrap_script(theme)),
        proxy.port,
    )?;
    let window = builder.build()?;
    ui_network::retain_for_window(&window, proxy);
    Ok(window)
}

pub(super) fn initialize(
    app: &tauri::App,
    state: &AppState,
    model_download_client: reqwest::Client,
    setup_gate_layout: Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let setup_account_lock = Arc::clone(&state.account_lock);
    let setup_embeddings_read = Arc::clone(&state.embeddings_read);
    let setup_allowed_domains = Arc::clone(&state.allowed_domains);
    let setup_load_remote_image = Arc::clone(&state.load_remote_image);
    let setup_remote_images_allowed = Arc::clone(&state.remote_images_allowed);
    let setup_publisher_javascript_allowed = Arc::clone(&state.publisher_javascript_allowed);
    let setup_inline_pdf_reader = Arc::clone(&state.inline_pdf_reader);
    let setup_html_reader_view = Arc::clone(&state.html_reader_view);
    let setup_show_sidebar = Arc::clone(&state.show_sidebar);
    let setup_sidebar_sort = Arc::clone(&state.sidebar_sort);
    let setup_library = Arc::clone(&state.library);
    let setup_theme = Arc::clone(&state.theme);
    let setup_refresh_on_startup = Arc::clone(&state.refresh_on_startup);
    let setup_loader_timeout = Arc::clone(&state.loader_timeout_secs);
    let setup_feed_timeout = Arc::clone(&state.feed_timeout_secs);
    let setup_feed_max_body = Arc::clone(&state.feed_max_body_bytes);
    let setup_import_max_bytes = Arc::clone(&state.import_max_bytes);
    let setup_http_allowed = Arc::clone(&state.http_allowed);
    let setup_connect_nonstandard_ports_allowed =
        Arc::clone(&state.connect_nonstandard_ports_allowed);
    let setup_max_redirects = Arc::clone(&state.max_redirects);
    let setup_referer_allowed = Arc::clone(&state.referer_allowed);
    let setup_gzip_allowed = Arc::clone(&state.gzip_allowed);
    let setup_reader_font_family = Arc::clone(&state.html_reader_font_family);
    let setup_reader_font_size = Arc::clone(&state.html_reader_font_size);
    let setup_reader_line_height = Arc::clone(&state.html_reader_line_height);
    let setup_svc = Arc::clone(&state.embedding_service);
    let setup_embedding_query_model = Arc::clone(&state.embedding_query_model);
    let setup_read_only_startup = Arc::clone(&state.read_only_startup);
    let setup_embedding_startup_state = Arc::clone(&state.embedding_startup_state);
    let setup_settings_baseline = Arc::clone(&state.settings_baseline);

    // Resolve settings before creating the webview so its document-start script can apply
    // the persisted theme before the first page paint.
    let settings_path = settings_path()?;
    let settings = load_settings(&settings_path);
    *setup_settings_baseline.write().unwrap() = settings_mtime(&settings_path);
    if settings.easyprivacy_enabled() {
        if let Err(error) = settings::install_easyprivacy_at_startup(
            app.handle(),
            &state.account_lock,
            &settings_path,
        ) {
            log::error!("startup: EasyPrivacy installation failed: {error}");
        }
    }
    *state.easyprivacy_enabled.write().unwrap() = settings.easyprivacy_enabled();
    *state.easyprivacy_refresh_days.write().unwrap() = settings.easyprivacy_refresh_days();
    let theme = settings.theme.clone().unwrap_or_else(|| "auto".to_string());
    let default_library =
        xaga_core::fs::data_root().ok_or("cannot determine platform data directory")?;
    let lib_path = select_library_path(default_library, settings.library.as_deref())?;
    // Acquire the library lock (or learn we are read-only) BEFORE any command can
    // run, and — if we won it — start the LibraryWriter that takes ownership of the
    // lock. The writer's presence is the single source of truth for the runtime
    // mode, and the write-command gate derives from it. Writer/lock lifetime is
    // process-lifetime — `set_library` restarts the process, and the restart path
    // drops the writer (releasing the lock) before spawning the successor.
    let (writer, metadata_error) = library_writer_for_startup(
        &lib_path,
        xaga_core::library::LibraryVersion::new(
            app.package_info().version.major,
            app.package_info().version.minor,
        ),
    );
    if let Some(error) = metadata_error {
        log::error!("startup: library metadata could not be read: {error}");
        use tauri_plugin_dialog::DialogExt;
        app.dialog()
            .message(format!(
                "{} Check the library options before opening this library.",
                error.user_message()
            ))
            .title("Library metadata unavailable")
            .kind(tauri_plugin_dialog::MessageDialogKind::Error)
            .blocking_show();
    }
    setup_gate_layout.store(
        xaga_core::library::LibraryStatus::read(
            &lib_path,
            xaga_core::library::LibraryVersion::new(
                app.package_info().version.major,
                app.package_info().version.minor,
            ),
            writer.is_some(),
        )
        .layout_upgrade_required,
        std::sync::atomic::Ordering::SeqCst,
    );
    // The EmbeddingService shares the writer's lock for its lifetime.
    let service_lock = writer.as_ref().map(|w| w.lock());
    let service_writer = writer.clone();
    if let Some(rename_writer) = writer.clone() {
        let updates = rename_writer.subscribe_to_tag_updates();
        let rename_app = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = rename_writer.resume_tag_renames().await {
                events::emit_error(
                    &rename_app,
                    "main",
                    crate::error::AppError::from(error).serialized(),
                );
            }
            drop(rename_writer);
            while let Ok(update) = updates.recv_async().await {
                match update {
                    Ok(snapshot) => events::emit_domain(
                        &rename_app,
                        xaga_core::events::DomainEvent::TagsChanged(snapshot),
                    ),
                    Err(error) => events::emit_error(
                        &rename_app,
                        "main",
                        crate::error::AppError::from(error).serialized(),
                    ),
                }
            }
        });
    }
    if writer.is_none() {
        log::warn!("startup: opening library in read-only mode");
    }
    *setup_library.write().unwrap() = LibraryHandle {
        path: lib_path.clone(),
        writer,
    };

    let html_reader_view_str = settings
        .html_reader_view
        .as_deref()
        .unwrap_or(crate::settings::DEFAULT_HTML_READER_VIEW);
    *setup_html_reader_view.write().unwrap() = html_reader_view_str.to_owned();
    *setup_load_remote_image.write().unwrap() = settings.load_remote_image();
    *setup_remote_images_allowed.write().unwrap() = settings.remote_images_allowed();
    *setup_publisher_javascript_allowed.write().unwrap() = settings.publisher_javascript_allowed();
    open_main_window(app, &theme)?;
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
    let connect_nonstandard_ports_allowed_setting =
        settings.connect_nonstandard_ports_allowed.unwrap_or(false);
    let max_redirects_setting = fetch_client::effective_max_redirects(settings.max_redirects);
    let referer_allowed_setting = settings.referer_allowed.unwrap_or(false);
    let gzip_allowed_setting = settings.gzip_allowed.unwrap_or(false);
    let font_family = settings
        .html_reader_font_family
        .unwrap_or_else(|| "system".to_string());
    let font_size = settings.html_reader_font_size.unwrap_or(16.0);
    let line_height = settings.html_reader_line_height.unwrap_or(1.75);
    let show_sidebar = settings.show_sidebar.unwrap_or(true);
    let sidebar_sort = settings.sidebar_sort.unwrap_or_default();
    library_menu::update_imports(app.handle(), service_lock.is_some())?;
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
    *setup_sidebar_sort.write().unwrap() = sidebar_sort;
    *setup_allowed_domains.write().unwrap() = settings.allowed_domains.unwrap_or_default();
    *setup_inline_pdf_reader.write().unwrap() = settings.inline_pdf_reader.unwrap_or(false);

    if *state.easyprivacy_enabled.read().unwrap() {
        let app_handle = app.handle().clone();
        let account_lock = Arc::clone(&state.account_lock);
        let settings_path = settings_path.clone();
        let proxy_port = state.network_proxy_port;
        tauri::async_runtime::spawn(async move {
            if let Err(error) =
                easyprivacy::refresh_at_launch(app_handle, account_lock, settings_path, proxy_port)
                    .await
            {
                log::warn!("startup: EasyPrivacy update check failed: {error}");
            }
        });
    }

    // Correct the menu checkmarks now that we know the persisted values.
    update_theme_checkmarks(app.handle(), &theme);
    update_show_sidebar_checkmark(app.handle(), show_sidebar);
    sidebar_sort::update(app.handle(), sidebar_sort);
    update_view_checkmarks(app.handle(), html_reader_view_str);

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
    if service_lock.is_none() {
        *setup_embedding_startup_state.write().unwrap() =
            EmbeddingStartupState::AwaitingLibraryChoice;
    }
    let initialization = tauri::async_runtime::spawn(async move {
        if service_lock.is_none() {
            await_library_choice(&setup_read_only_startup, &setup_embedding_startup_state).await;
        }
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
            let db_available = match db {
                Ok(db) => {
                    *setup_embeddings_read.lock().unwrap() = Some(db);
                    // A subsequent normal startup can recover a handle
                    // that the immediate post-conversion reopen lacked.
                    use commands::library::UpgradeState;
                    if let UpgradeState::Failed { read_error, .. }
                    | UpgradeState::Succeeded { read_error, .. } = &mut *app_handle
                        .state::<AppState>()
                        .library_upgrade
                        .lock()
                        .unwrap()
                    {
                        *read_error = None;
                    }
                    true
                }
                Err(e) => {
                    log::error!("startup: read-only embeddings open failed: {e}");
                    false
                }
            };
            let model_available = match model {
                Ok(model) => {
                    *setup_embedding_query_model.lock().unwrap() = Some(model);
                    true
                }
                Err(e) => {
                    log::error!("startup: read-only embedding model failed: {e}");
                    false
                }
            };
            *setup_embedding_startup_state.write().unwrap() = if db_available && model_available {
                EmbeddingStartupState::Available
            } else {
                EmbeddingStartupState::Unavailable
            };
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
                        events::emit_domain(
                            &embeddings_app_handle,
                            xaga_core::events::DomainEvent::EmbeddingChanged,
                        );
                    }
                });

                let embeddings_app_handle = app_handle.clone();
                let background_drained_rx = svc.subscribe_to_background_drained();
                tauri::async_runtime::spawn(async move {
                    loop {
                        if background_drained_rx.recv_async().await.is_err() {
                            break;
                        }
                        events::emit_domain(
                            &embeddings_app_handle,
                            xaga_core::events::DomainEvent::EmbeddingChanged,
                        );
                    }
                });

                let unread_app_handle = app_handle.clone();
                let unread_rx = svc.subscribe_to_unread_changed();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match unread_rx.recv_async().await {
                            Ok(entry) => {
                                events::emit_domain(
                                    &unread_app_handle,
                                    xaga_core::events::DomainEvent::UnreadChanged(entry),
                                );
                            }
                            Err(_) => break,
                        }
                    }
                });

                let bookmarked_app_handle = app_handle.clone();
                let bookmarked_rx = svc.subscribe_to_bookmarked_changed();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match bookmarked_rx.recv_async().await {
                            Ok(entry) => {
                                events::emit_domain(
                                    &bookmarked_app_handle,
                                    xaga_core::events::DomainEvent::BookmarkChanged(entry),
                                );
                            }
                            Err(_) => break,
                        }
                    }
                });

                let annotation_app_handle = app_handle.clone();
                let annotation_rx = svc.subscribe_to_annotation_changed();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match annotation_rx.recv_async().await {
                            Ok(item) => {
                                events::emit_domain(
                                    &annotation_app_handle,
                                    xaga_core::events::DomainEvent::AnnotationChanged(item),
                                );
                            }
                            Err(_) => break,
                        }
                    }
                });

                svc.trigger_recovery(lib_path.clone());
                *setup_svc.lock().unwrap() = Some(svc);
                *setup_embedding_startup_state.write().unwrap() = EmbeddingStartupState::Available;
            }
            Err(e) => {
                log::error!("startup: embedding service failed: {e}");
                *setup_embedding_startup_state.write().unwrap() =
                    EmbeddingStartupState::Unavailable;
            }
        }
    });

    *app.state::<AppState>()
        .embedding_initialization
        .lock()
        .unwrap() = Some(initialization);

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
}
