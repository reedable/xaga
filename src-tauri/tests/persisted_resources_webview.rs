//! Production recovery command and writer against a temporary library and local
//! server. Requires a macOS desktop; never opens user settings or a user library.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");

#[cfg(target_os = "macos")]
#[path = "support/resource_server.rs"]
mod resource_server;

fn test_state(path: PathBuf) -> AppState {
    fn shared<T>(value: T) -> Arc<RwLock<T>> {
        Arc::new(RwLock::new(value))
    }
    AppState {
        library: shared(LibraryHandle { path, writer: None }),
        account_lock: Arc::new(Mutex::new(AccountLock::default())),
        theme: shared("light".into()),
        refresh_on_startup: shared(false),
        loader_timeout_secs: shared(10),
        feed_timeout_secs: shared(10),
        feed_max_body_bytes: shared(DEFAULT_FEED_MAX_BODY_BYTES),
        import_max_bytes: shared(DEFAULT_IMPORT_MAX_BYTES),
        html_reader_font_family: shared("system".into()),
        html_reader_font_size: shared(16.0),
        html_reader_line_height: shared(1.5),
        show_sidebar: shared(true),
        embedding_service: Arc::new(Mutex::new(None)),
        embedding_query_model: Arc::new(Mutex::new(None)),
        embedding_startup_state: shared(EmbeddingStartupState::Unavailable),
        embeddings_read: Arc::new(Mutex::new(None)),
        allowed_domains: shared(Vec::new()),
        named_stores_supported: true,
        inline_pdf_reader: shared(false),
        load_remote_image: shared(true),
        html_reader_view: shared("summary".into()),
        network_proxy_port: 0,
        max_redirects: shared(0),
        referer_allowed: shared(false),
        gzip_allowed: shared(false),
        settings_baseline: shared(None),
    }
}

#[cfg(target_os = "macos")]
async fn verify(app: tauri::AppHandle) {
    use base64::Engine;
    use std::{
        collections::{BTreeMap, HashMap},
        sync::atomic::Ordering,
    };
    use xaga_core::{
        library_writer::{LibraryWriter, Tier},
        persistence::{
            articles,
            image_recovery::{ImagePatch, ImageRecovery},
            images,
        },
    };

    let server = resource_server::Server::new().await;
    let staging_before = resource_server::list_staging_directories();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(8, 8)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let bytes = png.into_inner();
    let expected_image = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    );
    let response = server.add_response(
        "/image",
        bytes.clone(),
        "application/octet-stream",
        None,
        None,
    );
    let url = format!("http://localhost:{}/image", server.address.port());
    app.add_capability(
        tauri::ipc::CapabilityBuilder::new("persisted-fixture")
            .window("loader-*")
            .local(false)
            .remote(format!("http://localhost:{}/*", server.address.port()))
            .permission("allow-loader-callbacks"),
    )
    .unwrap();
    app.manage(
        network_proxy::ProxyConfig::deny_all()
            .with_local_endpoint("localhost".into(), server.address)
            .unwrap(),
    );
    let library = app.state::<AppState>().library.read().unwrap().path.clone();
    let writer = LibraryWriter::start(
        library.clone(),
        xaga_core::library::acquire_lock(&library).unwrap(),
    );
    app.state::<AppState>().library.write().unwrap().writer = Some(writer.clone());
    let source = format!("![image]({url})");
    let recovery = || ImageRecovery {
        summary: true,
        expected: source.clone(),
        patches: vec![ImagePatch {
            start: source.find(&url).unwrap(),
            end: source.find(&url).unwrap() + url.len(),
            original: url.clone(),
            url: url.clone(),
            css: false,
        }],
    };
    async fn recover(app: &tauri::AppHandle, recovery: ImageRecovery) -> xaga_core::types::Article {
        commands::articles::recover_entry_images(
            "article".into(),
            recovery,
            None,
            app.state(),
            app.clone(),
            app.state(),
            app.state(),
            app.state(),
        )
        .await
        .unwrap()
    }
    async fn seed(writer: &LibraryWriter, source: &str, body: &str) {
        writer
            .save_article(
                Tier::Interactive,
                "article".into(),
                "https://example.org/article".into(),
                "Article".into(),
                None,
                None,
                Some(source.into()),
                None,
                body.into(),
                HashMap::new(),
            )
            .await
            .unwrap();
    }
    fn snapshot(root: &std::path::Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(
            root: &std::path::Path,
            path: &std::path::Path,
            files: &mut BTreeMap<PathBuf, Vec<u8>>,
        ) {
            for item in std::fs::read_dir(path).unwrap() {
                let path = item.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root).unwrap().into(),
                        std::fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(root, root, &mut files);
        files
    }

    seed(&writer, &source, "unchanged body").await;
    *app.state::<AppState>().load_remote_image.write().unwrap() = false;
    let before = snapshot(&library);
    let disabled = recover(&app, recovery()).await;
    assert_eq!(disabled.meta.description.as_deref(), Some(source.as_str()));
    assert_eq!(snapshot(&library), before);
    assert_eq!(response.requests.load(Ordering::SeqCst), 0);
    println!("PASS: disabled recovery performs no acquisition or writes");

    *app.state::<AppState>().load_remote_image.write().unwrap() = true;
    let saved = recover(&app, recovery()).await;
    assert_eq!(saved.content, "unchanged body");
    let local_summary = saved.meta.description.unwrap();
    assert!(local_summary.contains("../images/article/"));
    assert_eq!(response.requests.load(Ordering::SeqCst), 1);
    let local = local_summary
        .strip_prefix("![image](")
        .unwrap()
        .strip_suffix(')')
        .unwrap();
    assert_eq!(
        images::read_entry_images(&library, "article", &[local.into()]).await[0].as_deref(),
        Some(expected_image.as_str())
    );
    println!(
        "PASS: enabled recovery acquires, validates, persists and rewrites through production command"
    );

    // Keep the asset referenced from the body while presenting the same remote
    // URL in the summary. Recovery must reuse its bytes without another request.
    seed(&writer, &source, &local_summary).await;
    let reused = recover(&app, recovery()).await;
    assert_eq!(
        reused.meta.description.as_deref(),
        Some(local_summary.as_str())
    );
    assert_eq!(response.requests.load(Ordering::SeqCst), 1);
    println!("PASS: known remote URL reuses local bytes without acquisition");

    let gate = Arc::new(resource_server::Gate::default());
    let stale_response = server.add_response(
        "/stale",
        bytes,
        "application/octet-stream",
        None,
        Some(gate.clone()),
    );
    let stale_url = format!("http://localhost:{}/stale", server.address.port());
    let stale_source = format!("![stale]({stale_url})");
    seed(&writer, &stale_source, &local_summary).await;
    let start = stale_source.find(&stale_url).unwrap();
    let patch = ImageRecovery {
        summary: true,
        expected: stale_source,
        patches: vec![ImagePatch {
            start,
            end: start + stale_url.len(),
            original: stale_url.clone(),
            url: stale_url,
            css: false,
        }],
    };
    let recovery_app = app.clone();
    let pending_recovery =
        tauri::async_runtime::spawn(async move { recover(&recovery_app, patch).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while stale_response.requests.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    seed(&writer, "newer summary", &local_summary).await;
    let after_edit = snapshot(&library);
    gate.release();
    let edited = pending_recovery.await.unwrap();
    assert_eq!(edited.meta.description.as_deref(), Some("newer summary"));
    assert_eq!(
        snapshot(&library),
        after_edit,
        "stale recovery changed files or left orphan resources"
    );
    println!("PASS: a concurrent summary edit survives acquisition without orphan image writes");

    seed(&writer, &source, &local_summary).await;
    app.state::<AppState>().library.write().unwrap().writer = None;
    let before = snapshot(&library);
    for enabled in [false, true] {
        *app.state::<AppState>().load_remote_image.write().unwrap() = enabled;
        let article = recover(&app, recovery()).await;
        assert_eq!(article.meta.description.as_deref(), Some(source.as_str()));
        assert_eq!(response.requests.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot(&library), before);
    }
    let refused = commands::articles::save_article(
        "article".into(),
        "https://example.org/article".into(),
        "replacement".into(),
        None,
        None,
        None,
        None,
        "replacement".into(),
        HashMap::new(),
        app.state(),
        None,
        app.state(),
        app.clone(),
    )
    .await;
    assert!(matches!(refused, Err(error::AppError::ReadOnly)));
    assert_eq!(snapshot(&library), before);
    assert!(images::read_entry_images(&library, "article", &[local.into()]).await[0].is_some());
    println!(
        "PASS: read-only recovery and save commands do not acquire or mutate; cached reads remain usable"
    );

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !app.webview_windows().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        app.state::<article_loader::PendingRequests>()
            .lock()
            .unwrap()
            .is_empty()
    );
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while resource_server::list_staging_directories() != staging_before {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("private download staging leaked");
    writer.shutdown().await;
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    let library = tempfile::tempdir().unwrap();
    let passed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let completed = passed.clone();
    let mut context = app_context();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .manage(test_state(library.path().into()))
        .manage(article_loader::PendingRequests::default())
        .manage(article_diagnostics::ArticleDiagnostics::default())
        .manage(loader_identity::LoaderSessions::default())
        .invoke_handler(tauri::generate_handler![
            commands::articles::receive_html,
            commands::articles::receive_image,
            commands::articles::receive_favicon,
            commands::articles::add_loader_diagnostic_event
        ])
        .setup(move |app| {
            let app = app.handle().clone();
            let watchdog = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(45)).await;
                watchdog.exit(1);
            });
            tauri::async_runtime::spawn(async move {
                let result = tauri::async_runtime::spawn(verify(app.clone())).await;
                if let Err(error) = &result {
                    eprintln!("FAIL: {error}");
                }
                completed.store(result.is_ok(), std::sync::atomic::Ordering::SeqCst);
                app.exit(if result.is_ok() { 0 } else { 1 });
            });
            Ok(())
        })
        .build(context)
        .unwrap()
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
    assert!(passed.load(std::sync::atomic::Ordering::SeqCst));
}
