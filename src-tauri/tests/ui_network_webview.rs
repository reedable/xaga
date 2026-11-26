//! Native proxy enforcement: two isolated UI stores and an independent loader.
//! XAGA_VERIFY_LOOPBACK_ISOLATION=1 also checks the known WebKit bypass and
//! currently fails. Keep that reproduction until the separate gap is closed.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");
#[cfg(target_os = "macos")]
#[path = "support/app_state.rs"]
mod app_state;
#[cfg(target_os = "macos")]
#[path = "support/resource_server.rs"]
mod resource_server;
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use std::{
        io::{Read, Write},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    use xaga_core::network_proxy::{self, ProxyConfig};
    let check_loopback = std::env::var_os("XAGA_VERIFY_LOOPBACK_ISOLATION").is_some();
    let upstream = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = upstream.local_addr().unwrap();
    let reached = Arc::new(AtomicUsize::new(0));
    let server_reached = reached.clone();
    std::thread::spawn(move || {
        for stream in upstream.incoming() {
            let mut stream = stream.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let read_head = |stream: &mut std::net::TcpStream| {
                let mut head = Vec::new();
                let mut byte = [0];
                while !head.ends_with(b"\r\n\r\n") && head.len() < 8192 {
                    if stream.read(&mut byte).unwrap_or(0) == 0 {
                        break;
                    }
                    head.push(byte[0]);
                }
                head
            };
            let mut request = read_head(&mut stream);
            if request.starts_with(b"CONNECT loader.invalid:80 ") {
                stream
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .unwrap();
                request = read_head(&mut stream);
            }
            server_reached.fetch_add(1, Ordering::SeqCst);
            eprintln!(
                "UPSTREAM: {}",
                String::from_utf8_lossy(&request)
                    .lines()
                    .next()
                    .unwrap_or("empty")
            );
            let body = b"<!doctype html><title>loader-ok</title>";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(body);
        }
    });
    let done = Arc::new(AtomicUsize::new(0));
    struct DownloadLog([AtomicUsize; 2]);
    impl log::Log for DownloadLog {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.target().ends_with("::ui_network")
        }
        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                match record.args().to_string().as_str() {
                    "main: UI download refused" => self.0[0].fetch_add(1, Ordering::SeqCst),
                    "settings: UI download refused" => self.0[1].fetch_add(1, Ordering::SeqCst),
                    _ => return,
                };
            }
        }
        fn flush(&self) {}
    }
    static DOWNLOADS: DownloadLog = DownloadLog([AtomicUsize::new(0), AtomicUsize::new(0)]);
    log::set_logger(&DOWNLOADS).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    let staging_before = resource_server::list_staging_directories();
    if let Ok(case) = std::env::var("XAGA_VERIFY_BUNDLED_UI_ROUTING") {
        let (scheme, frame, expected) = match case.as_str() {
            "1" | "blob" => ("blob", "both", 2),
            "data-top" => ("data", "top", 1),
            "data-child" => ("data", "child", 0),
            _ => panic!("unsupported bundled UI routing case: {case}"),
        };
        verify_bundled_windows(&DOWNLOADS.0, staging_before, scheme, frame, expected);
        return;
    }
    let blocked = Arc::new([
        std::sync::Mutex::new(std::collections::HashSet::new()),
        std::sync::Mutex::new(std::collections::HashSet::new()),
    ]);
    let result = done.clone();
    let mut context = app_context();
    context.config_mut().app.windows.clear();
    let html = format!(
        r#"<!doctype html><body><script>
                window.localImage = false;
                const local = new Image(); local.onload=()=>window.localImage=true;
                local.src='data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
                document.body.append(local);
                for (const url of ['http://image.invalid/a','https://secure-image.invalid/a']) {{
                    const img=new Image(); img.src=url; document.body.append(img);
                }}
                if ({check_loopback}) {{ const img=new Image(); img.src='http://{address}/forbidden'; document.body.append(img); }}
                const bg=document.createElement('div'); bg.style='width:10px;height:10px;background-image:url(http://css.invalid/a)'; document.body.append(bg);
                const frame=document.createElement('iframe'); frame.src='http://frame.invalid/a'; document.body.append(frame);
                fetch('http://fetch.invalid/a').catch(()=>{{}});
                new WebSocket('wss://socket.invalid/a');
                const attachment = document.createElement('a');
                attachment.download = 'xaga-routing-fixture.txt';
                attachment.href = URL.createObjectURL(new Blob(['fixture'], {{type:'text/plain'}}));
                document.body.append(attachment);
                attachment.click();
                setTimeout(()=>document.title=window.localImage?'ui-ok':'local-image-failed', 2000);
            </script>"#
    );
    let app = tauri::Builder::default().register_uri_scheme_protocol("fixture", move |_, _| { tauri::http::Response::builder().header("Content-Type", "text/html").body(html.clone().into_bytes()).unwrap() }).setup(move |app| {
        let mut ui_ports = Vec::new();
        for (index, label) in ["main", "settings"].into_iter().enumerate() {
            let observations = blocked.clone();
            let config = ProxyConfig::deny_all().with_observer(Arc::new(move |code, detail| {
                eprintln!("{label}: {code}: {detail}");
                if code == "proxy:blocked" { observations[index].lock().unwrap().insert(detail.to_owned()); }
            }));
            let proxy = tauri::async_runtime::block_on(network_proxy::start_scoped(config))?;
            ui_ports.push(proxy.port);
            let completed = done.clone();
            // Hidden fixture views timed out before image/timer completion on
            // the test host. This probe requires actual foreground rendering.
            let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::External("fixture://localhost/test".parse()?))
                .visible(true);
            let window = ui_network::configure_window(builder, proxy.port)?
                .on_document_title_changed(move |_, title| { if title == "ui-ok" { completed.fetch_add(1, Ordering::SeqCst); } })
                .build()?;
            // Keep each proxy alive for its window lifetime.
            ui_network::retain_for_window(&window, proxy);
        }
        // Independent loader proxy fixture. Its non-loopback URL must traverse
        // this listener, while the UI policy must still reject its own requests.
        let completed = done.clone();
        let _window = WebviewWindowBuilder::new(app, "loader-test", WebviewUrl::External("http://loader.invalid/allowed".parse()?))
            .visible(false)
            .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
            .proxy_url(format!("http://{address}").parse()?)
            .on_document_title_changed(move |_, title| { if title == "loader-ok" { completed.fetch_add(1, Ordering::SeqCst); } })
            .build()?;
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..100 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if done.load(Ordering::SeqCst) >= 3
                    && DOWNLOADS.0.iter().all(|count| count.load(Ordering::SeqCst) == 1)
                    && blocked.iter().all(|v| v.lock().unwrap().len() >= if check_loopback { 7 } else { 6 }) {
                    if reached.load(Ordering::SeqCst) != 1 { eprintln!("FAIL: UI reached upstream"); handle.exit(1); return; }
                    println!("PASS: main/settings block HTTP, HTTPS, CSS, iframe, fetch and WebSocket; local image and isolated loader work");
                    for label in ["main", "settings"] {
                        handle.get_webview_window(label).unwrap().close().unwrap();
                    }
                    let closed = tokio::time::timeout(Duration::from_secs(3), async {
                        for port in ui_ports {
                            while tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                                tokio::time::sleep(Duration::from_millis(20)).await;
                            }
                        }
                    }).await;
                    if closed.is_err() || resource_server::list_staging_directories() != staging_before {
                        eprintln!("FAIL: a UI proxy or download staging directory remained after close");
                        handle.exit(1);
                        return;
                    }
                    println!("PASS: production UI download policy refuses local blobs without staging and closes both fixture proxies");
                    handle.exit(0);
                    return;
                }
            }
            eprintln!("FAIL: completed={}, blocked=[{}, {}], downloads=[{}, {}], upstream={}", done.load(Ordering::SeqCst), blocked[0].lock().unwrap().len(), blocked[1].lock().unwrap().len(), DOWNLOADS.0[0].load(Ordering::SeqCst), DOWNLOADS.0[1].load(Ordering::SeqCst), reached.load(Ordering::SeqCst));
            handle.exit(1);
        });
        Ok(())
    }).build(context).unwrap();
    app.run(|_, _| {});
    assert!(result.load(Ordering::SeqCst) >= 3);
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct RoutingAttempts([std::sync::atomic::AtomicUsize; 2]);

#[cfg(target_os = "macos")]
fn verify_bundled_windows(
    downloads: &'static [std::sync::atomic::AtomicUsize; 2],
    staging_before: Vec<PathBuf>,
    scheme: &'static str,
    frame: &'static str,
    expected: usize,
) {
    use std::{sync::atomic::Ordering, time::Duration};

    assert!(
        !tauri::is_dev(),
        "build with --features tauri/custom-protocol"
    );
    let directory = tempfile::tempdir().unwrap();
    let passed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let completed = passed.clone();
    // Evaluate only after the compiled asset page is loaded and the real
    // theme command responds. The child frame inherits the bundled origin.
    let script = r#"((scheme, scope) => {
        if (!window.__xagaRoutingProbe
            && document.readyState === 'complete'
            && document.querySelector('script[type="module"][src*="/assets/"]')) {
          window.__xagaRoutingProbe = true;
          window.__TAURI_INTERNALS__.invoke('get_theme').then(theme => {
            if (theme !== 'light') return;
            const download = (target) => {
              const owner = target.defaultView;
              const link = target.createElement('a');
              link.download = 'xaga-bundled-routing-fixture.txt';
              link.href = scheme === 'data'
                ? 'data:text/plain,fixture'
                : owner.URL.createObjectURL(new owner.Blob(['fixture'], {type:'text/plain'}));
              target.body.append(link);
              link.click();
              window.__TAURI_INTERNALS__.invoke('get_theme', {routingProbeAttempt: true});
            };
            if (scope !== 'top') {
              const frame = document.createElement('iframe');
              frame.addEventListener('load', () => download(frame.contentDocument), {once: true});
              frame.src = URL.createObjectURL(new Blob(['<!doctype html><body></body>'], {type:'text/html'}));
              document.body.append(frame);
            }
            if (scope !== 'child') download(document);
          });
        }
    })"#;
    let probe = format!("{script}({scheme:?}, {frame:?})");
    let theme_handler: fn(tauri::ipc::Invoke) -> bool =
        tauri::generate_handler![commands::settings::get_theme];
    let app = tauri::Builder::default()
        .manage(app_state::test_state(directory.path().to_path_buf()))
        .manage(ui_network::UiErrors::default())
        .manage(article_diagnostics::ArticleDiagnostics::default())
        .manage(RoutingAttempts::default())
        .invoke_handler(move |invoke: tauri::ipc::Invoke| {
            if invoke.message.command() == "get_theme"
                && let tauri::ipc::InvokeBody::Json(payload) = invoke.message.payload()
                && payload["routingProbeAttempt"] == true
            {
                let webview = invoke.message.webview_ref();
                let index = match webview.label() {
                    "main" => 0,
                    "settings" => 1,
                    other => panic!("unexpected fixture window: {other}"),
                };
                webview.state::<RoutingAttempts>().0[index].fetch_add(1, Ordering::SeqCst);
            }
            theme_handler(invoke)
        })
        .setup(move |app| {
            let main = startup::open_main_window(app, "light")?;
            main.show()?;
            menus::on_event(app.handle(), tauri::menu::MenuEvent { id: "open_settings".into() });
            let settings = app.get_webview_window("settings").expect("production Settings window");
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let attempts = handle.state::<RoutingAttempts>();
                let expected_attempts = if frame == "both" { 2 } else { 1 };
                let result = tokio::time::timeout(Duration::from_secs(12), async {
                    loop {
                        main.eval(&probe).unwrap();
                        settings.eval(&probe).unwrap();
                        if attempts.0.iter().all(|count| count.load(Ordering::SeqCst) == expected_attempts)
                            && downloads.iter().all(|count| count.load(Ordering::SeqCst) >= expected) {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }).await;
                if result.is_err() {
                    eprintln!("FAIL: bundled attempts main={}, settings={}; native refusals main={}, settings={}", attempts.0[0].load(Ordering::SeqCst), attempts.0[1].load(Ordering::SeqCst), downloads[0].load(Ordering::SeqCst), downloads[1].load(Ordering::SeqCst));
                    handle.exit(1);
                    return;
                }
                if expected == 0 {
                    // Allow time for an unexpected native callback to arrive.
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                let observed = downloads.each_ref().map(|count| count.load(Ordering::SeqCst));
                assert_eq!(observed, [expected; 2], "native refusals for {frame} {scheme}");
                main.close().unwrap();
                settings.close().unwrap();
                assert_eq!(resource_server::list_staging_directories(), staging_before);
                println!("PASS: {frame} {scheme} requests in production main and Settings; {expected} native refusal(s) per window, no staging");
                completed.store(true, Ordering::SeqCst);
                handle.exit(0);
            });
            Ok(())
        })
        .build(app_context())
        .unwrap();
    app.run(|_, _| {});
    assert!(passed.load(Ordering::SeqCst));
}
