//! Exercise the real loader, IPC handlers, downloads and teardown without
//! starting the application or opening a library. Requires a macOS desktop.
//! Include the crate privately so tests need no additional production API.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
#[path = "support/pdf_fixture.rs"]
mod pdf_fixture;
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
    struct ViewOptions {
        purpose: article_loader::LoaderPurpose,
        easyprivacy_rules: Option<article_loader::RuleList>,
        keep_open: bool,
        remote_images_allowed: bool,
        save_images: bool,
        publisher_javascript_allowed: bool,
    }

    async fn start(app: &tauri::AppHandle, server: &Server, name: &str, timeout: u64) -> Load {
        start_with_options(app, server, name, timeout, true, true, false).await
    }
    async fn start_with_options(
        app: &tauri::AppHandle,
        server: &Server,
        name: &str,
        timeout: u64,
        remote_images_allowed: bool,
        images: bool,
        publisher_javascript_allowed: bool,
    ) -> Load {
        start_with_proxy(
            app,
            server,
            name,
            timeout,
            ViewOptions {
                purpose: article_loader::LoaderPurpose::Article,
                easyprivacy_rules: None,
                keep_open: false,
                remote_images_allowed,
                save_images: images,
                publisher_javascript_allowed,
            },
            network_proxy::ProxyConfig::deny_all(),
        )
        .await
    }

    async fn start_with_proxy(
        app: &tauri::AppHandle,
        server: &Server,
        name: &str,
        timeout: u64,
        options: ViewOptions,
        config: network_proxy::ProxyConfig,
    ) -> Load {
        let attempt = app
            .state::<article_diagnostics::ArticleDiagnostics>()
            .create_attempt("load");
        let proxy_attempt = attempt.clone();
        let proxy = Arc::new(
            network_proxy::start_scoped(
                config
                    .with_local_endpoint("localhost".into(), server.address)
                    .unwrap()
                    .with_observer(Arc::new(move |code, detail| {
                        proxy_attempt.add_event(
                            "proxy",
                            "network",
                            article_diagnostics::ArticleDiagnosticOutcome::Warning,
                            code,
                            detail,
                        );
                    })),
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
        let load_attempt = attempt.clone();
        let url = format!("http://localhost:{}/{name}", server.address.port());
        let result = tauri::async_runtime::spawn(async move {
            article_loader::load_via_webview(
                &load_app,
                &pending,
                &url,
                timeout,
                options.keep_open,
                options.purpose,
                options.remote_images_allowed,
                options.save_images,
                options.publisher_javascript_allowed,
                Ok(options.easyprivacy_rules),
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

    async fn verify_rule_attachment(app: &tauri::AppHandle, server: &Server) {
        let routes = [
            "easyprivacy-filtered",
            "easyprivacy-off",
            "easyprivacy-installed",
        ];
        for name in routes {
            server.add_article(name, None, None, None, false);
            let image = server.add_response(
                &format!("/{name}/blocked-image"),
                Vec::new(),
                "image/png",
                None,
                None,
            );
            let redirect = server.add_redirect(
                &format!("/{name}/redirect-image"),
                &format!("/{name}/blocked-image"),
            );
            server.add_response(
                &format!("/{name}"),
                format!(
                    r#"<!doctype html><title>Policy fixture</title><p>article-{name}</p><img src="/{name}/redirect-image">"#
                )
                .into_bytes(),
                "text/html",
                None,
                None,
            );

            let mut load = start_with_proxy(
                app,
                server,
                name,
                8,
                ViewOptions {
                    purpose: article_loader::LoaderPurpose::Article,
                    easyprivacy_rules: match name {
                        "easyprivacy-filtered" => Some((
                            "xaga-test-easyprivacy-image-fixture-v1".into(),
                            Box::new(|| Ok(r#"[{"trigger":{"url-filter":"/easyprivacy-filtered/blocked-image$"},"action":{"type":"block"}}]"#.into())),
                        )),
                        "easyprivacy-installed" => Some((
                            "xaga-test-easyprivacy-bundled-revision-v1".into(),
                            Box::new(|| easyprivacy_rules::compile(include_str!("../resources/easyprivacy/easyprivacy.txt"))),
                        )),
                        _ => None,
                    },
                    keep_open: false,
                    remote_images_allowed: true,
                    save_images: false,
                    publisher_javascript_allowed: false,
                },
                network_proxy::ProxyConfig::deny_all(),
            )
            .await;
            let capture = load.result.take().unwrap().await.unwrap().unwrap();
            assert!(capture.html.contains(&format!("article-{name}")));
            assert_eq!(
                image.requests.load(Ordering::SeqCst),
                usize::from(name != routes[0])
            );
            assert_eq!(redirect.requests.load(Ordering::SeqCst), 1);
            assert_cleanup(app, &load).await;
        }
    }

    async fn verify_easyprivacy_navigation_redirects(app: &tauri::AppHandle, server: &Server) {
        for (kind, purpose) in [
            ("article", article_loader::LoaderPurpose::Article),
            ("image", article_loader::LoaderPurpose::Image),
            ("favicon", article_loader::LoaderPurpose::Article),
        ] {
            for filtered in [true, false] {
                let name = format!("easyprivacy-{kind}-{filtered}");
                let redirect_source = if kind == "favicon" {
                    server.add_article(&name, None, None, None, true);
                    format!("/{name}/icon")
                } else {
                    format!("/{name}")
                };
                let destination_path = format!("/{name}/target");
                let target = if kind == "article" {
                    server.add_response(
                        &destination_path,
                        b"<!doctype html><p>Redirected article</p>".to_vec(),
                        "text/html",
                        None,
                        None,
                    )
                } else {
                    server.add_article(&format!("{name}-icon"), None, None, None, true)
                };
                let destination_path = if kind == "article" {
                    destination_path
                } else {
                    format!("/{name}-icon/icon")
                };
                let redirect = server.add_redirect(&redirect_source, &destination_path);
                let mut load = start_with_proxy(
                    app,
                    server,
                    &name,
                    if kind == "favicon" { 15 } else { 5 },
                    ViewOptions {
                        purpose,
                        easyprivacy_rules: filtered.then(|| {
                            let rules = serde_json::json!([{
                                "trigger": { "url-filter": format!("{destination_path}$") },
                                "action": { "type": "block" }
                            }])
                            .to_string();
                            (
                                format!("xaga-test-{name}-v1"),
                                Box::new(move || Ok(rules)) as _,
                            )
                        }),
                        keep_open: false,
                        remote_images_allowed: true,
                        save_images: kind == "favicon",
                        publisher_javascript_allowed: false,
                    },
                    network_proxy::ProxyConfig::deny_all(),
                )
                .await;
                let result = load.result.take().unwrap().await.unwrap();
                assert!(
                    redirect.seen.load(Ordering::SeqCst),
                    "{name} did not reach first hop: {:?}; {}",
                    result.as_ref().err(),
                    serde_json::to_string(&load.attempt.get_diagnostic()).unwrap()
                );
                assert_eq!(
                    target.seen.load(Ordering::SeqCst),
                    !filtered,
                    "{name} target"
                );
                if kind == "favicon" {
                    let capture = result.unwrap();
                    assert!(capture.html.contains(&format!("article-{name}")));
                    assert_eq!(capture.favicon_data.is_some(), !filtered);
                }
                assert_cleanup(app, &load).await;
            }
        }
    }

    async fn verify_rule_preparation_failure(app: &tauri::AppHandle, server: &Server) {
        let page = server.add_response(
            "/easyprivacy-setup-failure",
            b"<!doctype html><p>Setup should fail before navigation</p>".to_vec(),
            "text/html",
            None,
            None,
        );
        let proxy = Arc::new(
            network_proxy::start_scoped(
                network_proxy::ProxyConfig::deny_all()
                    .with_local_endpoint("localhost".into(), server.address)
                    .unwrap(),
            )
            .await
            .unwrap(),
        );
        let attempt = app
            .state::<article_diagnostics::ArticleDiagnostics>()
            .create_attempt("load");
        let result = article_loader::load_via_webview(
            app,
            &app.state::<article_loader::PendingRequests>(),
            &format!(
                "http://localhost:{}/easyprivacy-setup-failure",
                server.address.port()
            ),
            5,
            false,
            article_loader::LoaderPurpose::Article,
            true,
            false,
            false,
            Err("fixture preparation error".into()),
            &[],
            true,
            proxy,
            attempt.clone(),
        )
        .await;
        assert!(matches!(result, Err(error) if error == "fixture preparation error"));
        assert!(!page.seen.load(Ordering::SeqCst));
        let report = serde_json::to_value(attempt.get_diagnostic()).unwrap();
        assert!(
            report["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["code"] == "article:webview-create")
        );
    }

    async fn verify_reachable_redirect(
        app: &tauri::AppHandle,
        server: &Server,
        endpoint: std::net::SocketAddrV4,
        keep_open: bool,
        child_frame: bool,
    ) {
        assert!(
            endpoint.ip().is_private(),
            "use the isolated fixture's private IPv4 address"
        );
        let staging_before = list_staging_directories();
        let visibility = if keep_open { "visible" } else { "hidden" };
        let frame = if child_frame {
            "child-frame"
        } else {
            "top-frame"
        };
        let name = format!("reachable-refusal-{visibility}-{frame}");
        let redirect_path = if keep_open || child_frame {
            let path = format!("/{name}/image");
            let image = format!(r#"<img src="{path}">"#);
            let content = if child_frame {
                let frame = format!("/{name}/frame");
                server.add_response(
                    &frame,
                    format!("<!doctype html>{image}").into_bytes(),
                    "text/html",
                    None,
                    None,
                );
                format!(r#"<iframe src="{frame}"></iframe>"#)
            } else {
                image
            };
            server.add_response(
                &format!("/{name}"),
                format!(
                    "<!doctype html><title>Routing fixture</title><p>article-{name}</p>{content}"
                )
                .into_bytes(),
                "text/html",
                None,
                None,
            );
            path
        } else {
            server.add_article(&name, None, None, None, true);
            format!("/{name}/icon")
        };
        let redirect = server.add_redirect(
            &redirect_path,
            &format!("http://{endpoint}/routing-fixture.png"),
        );
        let mut load = start_with_proxy(
            app,
            server,
            &name,
            7,
            ViewOptions {
                purpose: article_loader::LoaderPurpose::Article,
                easyprivacy_rules: None,
                keep_open,
                remote_images_allowed: true,
                save_images: !keep_open && !child_frame,
                publisher_javascript_allowed: false,
            },
            // Allow HTTP and nonstandard CONNECT ports in this fixture so
            // the application must enforce its private-address policy.
            network_proxy::ProxyConfig::new(
                Arc::new(RwLock::new(true)),
                Arc::new(RwLock::new(true)),
            ),
        )
        .await;
        let capture = load.result.take().unwrap().await.unwrap().unwrap();
        assert!(redirect.seen.load(Ordering::SeqCst));
        assert!(capture.html.contains(&format!("article-{name}")));
        assert!(capture.favicon_data.is_none());
        assert!(capture.pdf_bytes.is_none());
        assert!(capture.download_error.is_none());
        until("private-address refusal in the loader diagnostics", || {
            let report = serde_json::to_value(load.attempt.get_diagnostic()).unwrap();
            report["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["source"] == "proxy" && event["code"] == "proxy:address-blocked")
        })
        .await;
        if keep_open {
            app.get_webview_window(&load.label)
                .expect("visible loader remains open after capture")
                .close()
                .unwrap();
        }
        assert_cleanup(app, &load).await;
        until("reachable redirect leaves staging unchanged", || {
            list_staging_directories() == staging_before
        })
        .await;
        println!(
            "PASS: {visibility} {frame} production WebKit loader records private-address refusal and preserves capture"
        );
    }

    async fn verify_proxy_redirect_refusal(server: &Server) {
        let destination = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let destination_port = destination.local_addr().unwrap().port();
        let refused = format!("http://localhost:{destination_port}/refused");
        let redirect = server.add_redirect("/proxy-refusal", &refused);
        let reports = Arc::new(Mutex::new(Vec::new()));
        let observed = reports.clone();
        let config = network_proxy::ProxyConfig::deny_all()
            .with_local_endpoint("localhost".into(), server.address)
            .unwrap()
            .with_observer(Arc::new(move |code, detail| {
                observed
                    .lock()
                    .unwrap()
                    .push((code.to_owned(), detail.to_owned()));
            }));
        let proxy = network_proxy::start_scoped(config).await.unwrap();
        let response = network_proxy::proxied_client(proxy.port)
            .unwrap()
            .get(format!(
                "http://localhost:{}/proxy-refusal",
                server.address.port()
            ))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
        assert_eq!(response.url().as_str(), refused);
        assert_eq!(redirect.requests.load(Ordering::SeqCst), 1);
        assert!(reports.lock().unwrap().iter().any(|(code, detail)| {
            code == "proxy:blocked"
                && detail.contains(&format!("destination=localhost:{destination_port}"))
        }));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), destination.accept())
                .await
                .is_err()
        );
        println!("PASS: proxy-routed client refuses redirect before connecting to the destination");
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
        if let Ok(endpoint) = std::env::var("XAGA_LOADER_ROUTING_DESTINATION") {
            let endpoint = endpoint.parse().unwrap();
            for keep_open in [false, true] {
                for child_frame in [false, true] {
                    verify_reachable_redirect(&app, &server, endpoint, keep_open, child_frame)
                        .await;
                }
            }
            return;
        }
        verify_proxy_redirect_refusal(&server).await;
        verify_rule_attachment(&app, &server).await;
        verify_easyprivacy_navigation_redirects(&app, &server).await;
        verify_rule_preparation_failure(&app, &server).await;
        if std::env::var_os("XAGA_LOADER_EASYPRIVACY_ONLY").is_some() {
            return;
        }
        let baseline = list_staging_directories();
        let origin = format!("http://localhost:{}", server.address.port());
        let pdf = pdf_fixture::pdf_fixture();
        for (route, destination, bytes) in [
            ("redirected-pdf", "/paper", pdf.clone()),
            (
                "redirected-invalid-pdf",
                "/invalid-paper",
                b"not a PDF".to_vec(),
            ),
        ] {
            let redirect = server.add_redirect(&format!("/{route}"), destination);
            let ready = Arc::new(Gate::default());
            let resource = server.add_response(
                destination,
                bytes,
                "application/pdf",
                Some(ready.clone()),
                None,
            );
            let mut load = start(&app, &server, route, 15).await;
            ready.release();
            let capture = load.result.take().unwrap().await.unwrap().unwrap();
            if route == "redirected-pdf" {
                assert_eq!(capture.pdf_bytes.as_deref(), Some(pdf.as_slice()));
                assert!(capture.download_error.is_none());
            } else {
                assert!(capture.pdf_bytes.is_none());
                assert!(capture.download_error.is_some());
            }
            assert_eq!(redirect.requests.load(Ordering::SeqCst), 1);
            assert_eq!(resource.requests.load(Ordering::SeqCst), 1);
            let report = serde_json::to_value(load.attempt.get_diagnostic()).unwrap();
            assert!(report["events"].as_array().unwrap().iter().any(|event| {
                event["step"] == "download_requested"
                    && event["download"]["sourceUrl"] == format!("{origin}/{route}")
                    && event["download"]["downloadUrl"] == format!("{origin}{destination}")
            }));
            assert_cleanup(&app, &load).await;
            until("PDF staging removed", || {
                list_staging_directories() == baseline
            })
            .await;
            println!("PASS: {route} reaches the production PDF consumer and releases staging");
        }
        server.add_article("redirected-favicon", None, None, None, true);
        let icon = server.add_article("favicon-destination", None, None, None, true);
        let icon_redirect =
            server.add_redirect("/redirected-favicon/icon", "/favicon-destination/icon");
        let mut load = start(&app, &server, "redirected-favicon", 15).await;
        let capture = load.result.take().unwrap().await.unwrap().unwrap();
        assert!(capture.html.contains("article-redirected-favicon"));
        assert!(capture.favicon_data.is_some());
        assert!(capture.pdf_bytes.is_none());
        assert!(capture.download_error.is_none());
        assert_eq!(icon_redirect.requests.load(Ordering::SeqCst), 1);
        assert_eq!(icon.requests.load(Ordering::SeqCst), 1);
        assert_cleanup(&app, &load).await;
        until("favicon staging removed", || {
            list_staging_directories() == baseline
        })
        .await;
        println!("PASS: redirected favicon preserves the production article capture");

        server.add_article("refused-favicon", None, None, None, true);
        let refusal = server.add_redirect(
            "/refused-favicon/icon",
            "https://routing-refusal.example.invalid/icon.png",
        );
        let mut load = start(&app, &server, "refused-favicon", 7).await;
        let capture = load.result.take().unwrap().await.unwrap().unwrap();
        assert!(capture.html.contains("article-refused-favicon"));
        assert!(capture.favicon_data.is_none());
        assert!(capture.pdf_bytes.is_none());
        assert!(capture.download_error.is_none());
        assert_eq!(refusal.requests.load(Ordering::SeqCst), 1);
        let report = serde_json::to_value(load.attempt.get_diagnostic()).unwrap();
        assert!(
            report["events"].as_array().unwrap().iter().any(|event| {
                event["source"] == "proxy"
                    && event["code"] == "proxy:blocked"
                    && event["detail"].as_str().is_some_and(|detail| {
                        detail.contains("destination=routing-refusal.example.invalid:443")
                    })
            }),
            "missing refusal diagnostics: {report}"
        );
        assert_cleanup(&app, &load).await;
        until("refused favicon releases staging", || {
            list_staging_directories() == baseline
        })
        .await;
        println!(
            "PASS: refused favicon redirect preserves captured HTML and releases the production loader"
        );

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
        // Exercise actual WebKit responsive selection through production HTML delivery.
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let selected = server.add_response(
            "/responsive-selected.png",
            png.into_inner(),
            "image/png",
            None,
            None,
        );
        server.add_response("/responsive", br#"<!doctype html><title>Responsive images</title>
          <article><p>Responsive image selection</p>
          <img srcset="/responsive-selected.png 1x" alt="srcset only" title="selected image">
          <picture><source srcset="/responsive-selected.png"><img src="/fallback.png" alt="picture fallback"></picture>
          <picture><source srcset="/responsive-selected.png"><img alt="picture only"></picture>
          </article>"#.to_vec(), "text/html", None, None);
        for enabled in [false, true] {
            let before = selected.requests.load(Ordering::SeqCst);
            let mut load =
                start_with_options(&app, &server, "responsive", 15, true, enabled, false).await;
            let capture = load.result.take().unwrap().await.unwrap().unwrap();
            let marker = format!("data-xaga-selected-src=\"{origin}/responsive-selected.png\"");
            if !enabled {
                assert_eq!(
                    capture.html.matches(&marker).count(),
                    3,
                    "responsive selections missing: {}",
                    capture.html
                );
            }
            assert!(capture.html.contains("title=\"selected image\""));
            assert_eq!(
                capture.html.matches("data-xaga-url=").count(),
                if enabled { 3 } else { 0 }
            );
            let requests = selected.requests.load(Ordering::SeqCst) - before;
            if enabled {
                assert!(
                    requests >= 4,
                    "expected browser load and three capture fetches"
                );
                let captured = format!("data-xaga-url=\"{origin}/responsive-selected.png\"");
                assert_eq!(capture.html.matches(&captured).count(), 3);
            } else {
                // Snapshotting must not trigger an additional resource load.
                assert_eq!(requests, 1);
            }
            assert_cleanup(&app, &load).await;
            println!("PASS: responsive URLs retained with image capture enabled={enabled}");
        }
        server.add_response(
            "/publisher-scripts",
            br#"<!doctype html><title>Publisher scripts</title>
                <article id="article"><p>Article text</p></article>
                <script>document.getElementById("article").dataset.executed = "yes";</script>"#
                .to_vec(),
            "text/html",
            None,
            None,
        );
        for allowed in [false, true] {
            let mut load =
                start_with_options(&app, &server, "publisher-scripts", 15, true, false, allowed)
                    .await;
            let capture = load.result.take().unwrap().await.unwrap().unwrap();
            assert!(capture.html.contains("<p>Article text</p>"));
            assert_eq!(capture.html.contains("data-executed=\"yes\""), allowed);
            assert_cleanup(&app, &load).await;
        }
        let element =
            server.add_response("/blocked-element.png", Vec::new(), "image/png", None, None);
        let css = server.add_response("/blocked-css.png", Vec::new(), "image/png", None, None);
        let icon = server.add_response("/blocked-icon.png", Vec::new(), "image/png", None, None);
        let fallback = server.add_response("/favicon.ico", Vec::new(), "image/x-icon", None, None);
        for (name, icon_link) in [
            (
                "blocked-explicit-icon",
                "<link rel=icon href=/blocked-icon.png>",
            ),
            ("blocked-fallback-icon", ""),
        ] {
            server.add_response(
                &format!("/{name}"),
                format!("<!doctype html>{icon_link}<style>p {{ background-image: url(/blocked-css.png) }}</style><p>Article text</p><img src=/blocked-element.png>").into_bytes(),
                "text/html", None, None,
            );
            let mut load = start_with_options(&app, &server, name, 15, false, false, false).await;
            let capture = load.result.take().unwrap().await.unwrap().unwrap();
            assert!(capture.html.contains("<p>Article text</p>"));
            assert!(capture.favicon_data.is_none());
            assert_cleanup(&app, &load).await;
        }
        for resource in [&element, &css, &icon, &fallback] {
            assert_eq!(resource.requests.load(Ordering::SeqCst), 0);
        }
        let mut load = start_with_options(
            &app,
            &server,
            "blocked-explicit-icon",
            15,
            true,
            false,
            false,
        )
        .await;
        let capture = load.result.take().unwrap().await.unwrap().unwrap();
        assert!(element.requests.load(Ordering::SeqCst) > 0);
        assert!(css.requests.load(Ordering::SeqCst) > 0);
        assert!(capture.favicon_data.is_none());
        let diagnostic = serde_json::to_value(load.attempt.get_diagnostic()).unwrap();
        assert!(
            diagnostic["events"]
                .as_array()
                .unwrap()
                .iter()
                .all(|event| event["step"] != "favicon")
        );
        assert_cleanup(&app, &load).await;
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
            commands::loader_capture::receive_html,
            commands::loader_capture::capture_and_close_loader,
            commands::loader_capture::receive_favicon,
            commands::loader_capture::receive_image,
            commands::loader_capture::add_loader_diagnostic_event,
            commands::loader_capture::loader_will_unload,
            commands::loader_capture::loader_page_loaded,
        ])
        .setup(move |app| {
            let app = app.handle().clone();
            let watchdog = app.clone();
            tauri::async_runtime::spawn(async move {
                // Blocked top-level navigation and favicon cases can consume
                // their individual loader deadlines before the lifecycle cases run.
                tokio::time::sleep(std::time::Duration::from_secs(150)).await;
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
