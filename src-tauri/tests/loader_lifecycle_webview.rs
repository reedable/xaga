//! Exercise the real loader, IPC handlers, downloads and teardown without
//! starting the application or opening a library. Requires a macOS desktop.
//! Include the crate privately so tests need no additional production API.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
#[path = "support/resource_server.rs"]
mod resource_server;

#[cfg(target_os = "macos")]
mod lifecycle {
    use super::resource_server::{Gate, Server, list_staging_directories};
    use super::*;
    use std::{sync::atomic::Ordering, time::Duration};

    async fn until(message: &str, mut condition: impl FnMut() -> bool) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out: {message}"));
    }

    struct Load {
        label: String,
        attempt: article_diagnostics::ArticleLoadAttempt,
        proxy: std::sync::Weak<network_proxy::ScopedProxy>,
        port: u16,
        result:
            Option<tauri::async_runtime::JoinHandle<Result<article_loader::LoaderCapture, String>>>,
    }
    async fn start(app: &tauri::AppHandle, server: &Server, name: &str, timeout: u64) -> Load {
        let proxy = Arc::new(
            network_proxy::start_scoped(
                network_proxy::ProxyConfig::deny_all()
                    .with_local_endpoint("localhost".into(), server.address)
                    .unwrap(),
            )
            .await
            .unwrap(),
        );
        let weak = Arc::downgrade(&proxy);
        let port = proxy.port;
        let load_app = app.clone();
        let pending = app
            .state::<article_loader::PendingRequests>()
            .inner()
            .clone();
        let attempt = app
            .state::<article_diagnostics::ArticleDiagnostics>()
            .create_attempt("load");
        let load_attempt = attempt.clone();
        let url = format!("http://localhost:{}/{name}", server.address.port());
        let result = tauri::async_runtime::spawn(async move {
            article_loader::load_via_webview(
                &load_app,
                &pending,
                &url,
                timeout,
                false,
                article_loader::LoaderPurpose::Article,
                false,
                &[],
                true,
                proxy,
                load_attempt,
            )
            .await
        });
        let mut label = None;
        until("pending loader registered", || {
            label = app
                .state::<article_loader::PendingRequests>()
                .lock()
                .unwrap()
                .iter()
                .find(|(_, r)| r.attempt.get_id() == attempt.get_id())
                .map(|(id, _)| format!("loader-{id}"));
            label.is_some()
        })
        .await;
        Load {
            label: label.unwrap(),
            attempt,
            proxy: weak,
            port,
            result: Some(result),
        }
    }
    async fn cancel(app: &tauri::AppHandle, load: &Load) {
        commands::articles::cancel_article_load(
            load.attempt.get_id(),
            app.clone(),
            app.state(),
            app.state(),
        )
        .await
        .unwrap();
    }
    async fn assert_cleanup(app: &tauri::AppHandle, load: &Load) {
        until("proxy owner released", || load.proxy.upgrade().is_none()).await;
        assert!(app.get_webview_window(&load.label).is_none());
        assert!(
            app.state::<loader_identity::LoaderSessions>()
                .source_url(&load.label)
                .is_none()
        );
        // ScopedProxy drops by aborting its accept task. Its last Arc can
        // disappear before Tokio has polled that task's cancellation.
        tokio::time::timeout(Duration::from_secs(3), async {
            while tokio::net::TcpStream::connect(("127.0.0.1", load.port))
                .await
                .is_ok()
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("scoped proxy listener still open");
        assert!(
            !app.state::<article_loader::PendingRequests>()
                .lock()
                .unwrap()
                .values()
                .any(|r| r.attempt.get_id() == load.attempt.get_id())
        );
    }

    pub(super) async fn verify(app: tauri::AppHandle) {
        let server = Server::new().await;
        // Production's remote URL patterns cover default ports. Grant only
        // this fixture's random local origin the existing loader callbacks.
        app.add_capability(
            tauri::ipc::CapabilityBuilder::new("lifecycle-fixture")
                .window("loader-*")
                .local(false)
                .remote(format!("http://localhost:{}/*", server.address.port()))
                .permission("allow-loader-callbacks"),
        )
        .unwrap();
        let baseline = list_staging_directories();
        for stage in ["before-capture", "after-capture", "during-download"] {
            let gate = Arc::new(Gate::default());
            let icon = server.add_article(
                stage,
                (stage == "before-capture").then(|| gate.clone()),
                (stage == "after-capture").then(|| gate.clone()),
                (stage == "during-download").then(|| gate.clone()),
                true,
            );
            let mut load = start(&app, &server, stage, 20).await;
            until(stage, || {
                let pending = app.state::<article_loader::PendingRequests>();
                let map = pending.lock().unwrap();
                map.values().any(|r| {
                    r.attempt.get_id() == load.attempt.get_id()
                        && match stage {
                            "before-capture" => server.responses.lock().unwrap()
                                [&format!("/{stage}")]
                                .seen
                                .load(Ordering::SeqCst),
                            "after-capture" => {
                                r.partial.is_some() && icon.seen.load(Ordering::SeqCst)
                            }
                            _ => {
                                r.download_active
                                    && list_staging_directories().len() > baseline.len()
                            }
                        }
                })
            })
            .await;
            let survivor_name = format!("survivor-{stage}");
            server.add_article(&survivor_name, None, None, None, true);
            let mut survivor = start(&app, &server, &survivor_name, 15).await;
            cancel(&app, &load).await;
            gate.release();
            // A cancelled capture must not be delivered as a successful load.
            assert!(load.result.take().unwrap().await.unwrap().is_err());
            assert_cleanup(&app, &load).await;
            let capture = survivor
                .result
                .take()
                .unwrap()
                .await
                .unwrap()
                .unwrap_or_else(|error| {
                    panic!(
                        "{error}; diagnostics: {}",
                        serde_json::to_string(&survivor.attempt.get_diagnostic()).unwrap()
                    )
                });
            assert!(capture.html.contains(&format!("article-{survivor_name}")));
            assert!(capture.favicon_data.is_some());
            assert!(!survivor.attempt.is_cancelled());
            assert_cleanup(&app, &survivor).await;
            until("cancelled staging removed", || {
                list_staging_directories() == baseline
            })
            .await;
            println!("PASS: cancellation {stage}");
        }

        let gate = Arc::new(Gate::default());
        server.add_article("good", None, None, None, true);
        server.add_article("bad", None, None, None, false);
        let stalled = server.add_article("stalled", None, Some(gate.clone()), None, true);
        server.add_article("stalled-download", None, None, Some(gate.clone()), true);
        let slow = start(&app, &server, "stalled", 5).await;
        until("stalled favicon requested", || {
            stalled.seen.load(Ordering::SeqCst)
        })
        .await;
        let slow_download = start(&app, &server, "stalled-download", 5).await;
        until("native download stalled after first byte", || {
            app.state::<article_loader::PendingRequests>()
                .lock()
                .unwrap()
                .values()
                .any(|r| r.attempt.get_id() == slow_download.attempt.get_id() && r.download_active)
        })
        .await;
        let good = start(&app, &server, "good", 15).await;
        let bad = start(&app, &server, "bad", 15).await;
        for (name, mut load) in [
            ("good", good),
            ("bad", bad),
            ("stalled", slow),
            ("stalled-download", slow_download),
        ] {
            let capture = load.result.take().unwrap().await.unwrap().unwrap();
            assert!(
                capture.html.contains(&format!("article-{name}")),
                "wrong article delivered for {name}"
            );
            assert_eq!(capture.favicon_data.is_some(), name == "good");
            assert!(capture.download_error.is_none());
            if name != "good" {
                let diagnostic = serde_json::to_value(load.attempt.get_diagnostic()).unwrap();
                assert!(
                    diagnostic["events"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|event| event["step"] == "favicon" && event["outcome"] == "warning")
                );
            }
            assert_cleanup(&app, &load).await;
            cancel(&app, &load).await; // completion followed by cancellation is harmless
            println!("PASS: concurrent {name} retains its own article");
        }
        gate.release();
        until("all loader windows closed", || {
            app.webview_windows().is_empty()
        })
        .await;
        assert!(
            app.state::<article_loader::PendingRequests>()
                .lock()
                .unwrap()
                .is_empty()
        );
        until("all staging removed", || {
            list_staging_directories() == baseline
        })
        .await;
    }
}

#[cfg(target_os = "macos")]
fn main() {
    let passed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let completed = passed.clone();
    let mut context = app_context();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .manage(article_loader::PendingRequests::default())
        .manage(article_diagnostics::ArticleDiagnostics::default())
        .manage(loader_identity::LoaderSessions::default())
        .invoke_handler(tauri::generate_handler![
            commands::articles::receive_html,
            commands::articles::receive_favicon,
            commands::articles::receive_image,
            commands::articles::add_loader_diagnostic_event,
            commands::articles::loader_will_unload,
            commands::articles::loader_page_loaded,
        ])
        .setup(move |app| {
            let app = app.handle().clone();
            let watchdog = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(90)).await;
                eprintln!("FAIL: loader lifecycle suite timed out");
                watchdog.exit(1);
            });
            tauri::async_runtime::spawn(async move {
                let result = tauri::async_runtime::spawn(lifecycle::verify(app.clone())).await;
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
    assert!(
        passed.load(std::sync::atomic::Ordering::SeqCst),
        "lifecycle verification did not complete"
    );
}
