//! Native proxy enforcement: two isolated UI stores and an independent loader.
//! XAGA_VERIFY_LOOPBACK_ISOLATION=1 also checks the known WebKit bypass and
//! currently fails. Keep that reproduction until the separate gap is closed.
#[cfg(target_os = "macos")]
#[path = "../src/webview_policy.rs"]
#[allow(dead_code)]
mod webview_policy;
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
    let blocked = Arc::new([
        std::sync::Mutex::new(std::collections::HashSet::new()),
        std::sync::Mutex::new(std::collections::HashSet::new()),
    ]);
    let result = done.clone();
    let mut context = tauri::generate_context!();
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
                setTimeout(()=>document.title=window.localImage?'ui-ok':'local-image-failed', 2000);
            </script>"#
    );
    let app = tauri::Builder::default().register_uri_scheme_protocol("fixture", move |_, _| { tauri::http::Response::builder().header("Content-Type", "text/html").body(html.clone().into_bytes()).unwrap() }).setup(move |app| {
        for (index, label) in ["main", "settings"].into_iter().enumerate() {
            let observations = blocked.clone();
            let config = ProxyConfig::deny_all().with_observer(Arc::new(move |code, detail| {
                eprintln!("{label}: {code}: {detail}");
                if code == "proxy:blocked" { observations[index].lock().unwrap().insert(detail.to_owned()); }
            }));
            let proxy = tauri::async_runtime::block_on(network_proxy::start_scoped(config))?;
            let completed = done.clone();
            let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External("fixture://localhost/test".parse()?))
                .visible(false)
                .with_webview_configuration(webview_policy::ui_configuration().unwrap())
                .proxy_url(network_proxy::proxy_url(proxy.port).parse()?)
                .on_download(|_, _| false)
                .on_document_title_changed(move |_, title| { if title == "ui-ok" { completed.fetch_add(1, Ordering::SeqCst); } })
                .build()?;
            // Keep each proxy alive for its window lifetime.
            let retained = std::sync::Mutex::new(Some(proxy));
            window.on_window_event(move |event| { if matches!(event, tauri::WindowEvent::Destroyed) { retained.lock().unwrap().take(); } });
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
                if done.load(Ordering::SeqCst) >= 3 && blocked.iter().all(|v| v.lock().unwrap().len() >= if check_loopback { 7 } else { 6 }) {
                    if reached.load(Ordering::SeqCst) != 1 { eprintln!("FAIL: UI reached upstream"); handle.exit(1); return; }
                    println!("PASS: main/settings block HTTP, HTTPS, CSS, iframe, fetch and WebSocket; local image and isolated loader work");
                    handle.exit(0);
                    return;
                }
            }
            eprintln!("FAIL: completed={}, blocked=[{}, {}], upstream={}", done.load(Ordering::SeqCst), blocked[0].lock().unwrap().len(), blocked[1].lock().unwrap().len(), reached.load(Ordering::SeqCst));
            handle.exit(1);
        });
        Ok(())
    }).build(context).unwrap();
    app.run(|_, _| {});
    assert!(result.load(Ordering::SeqCst) >= 3);
}
