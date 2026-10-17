//! Feed MIME feasibility probe using the production article loader script and native policy.
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
        net::TcpListener,
        sync::{Arc, Mutex},
        time::Duration,
    };
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            std::thread::spawn(move || {
                let mut stream = stream.unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let read_head = |stream: &mut std::net::TcpStream| {
                    let mut head = Vec::new();
                    let mut b = [0];
                    while head.len() < 16384 && !head.ends_with(b"\r\n\r\n") {
                        if stream.read(&mut b).unwrap_or(0) == 0 {
                            break;
                        }
                        head.push(b[0]);
                    }
                    String::from_utf8_lossy(&head).into_owned()
                };
                let mut head = read_head(&mut stream);
                if head.starts_with("CONNECT publisher.invalid:80 ") {
                    stream
                        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                        .unwrap();
                    head = read_head(&mut stream);
                }
                let path = head.split_whitespace().nth(1).unwrap_or("");
                let mime = if path.contains("/rss") {
                    "application/rss+xml"
                } else if path.contains("/atom") {
                    "application/atom+xml"
                } else if path.contains("/html") {
                    "text/html"
                } else {
                    "application/xml"
                };
                let body = if path.contains("/html") {
                    "<!doctype html><html><body>CONTROL</body></html>".to_owned()
                } else if path.contains("/atom") {
                    "<?xml version=\"1.0\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>ATOM_MARKER</title><id>urn:probe</id><updated>2026-10-16T00:00:00Z</updated></feed>".to_owned()
                } else {
                    format!(
                        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>RSS_MARKER</title><link>https://example.invalid/?a=1{}b=2</link><description>probe</description></channel></rss>",
                        if path.contains("bad") { "&" } else { "&amp;" }
                    )
                };
                eprintln!("RESPONSE {path} {mime}");
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            });
        }
    });
    let reports = Arc::new(Mutex::new(std::collections::HashSet::new()));
    let captured = reports.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default().invoke_handler(move |invoke| {
        if invoke.message.command() == "receive_html" {
            if let tauri::ipc::InvokeBody::Json(body) = invoke.message.payload() {
                eprintln!("CAPTURE {}", body);
                captured.lock().unwrap().insert(body["id"].as_str().unwrap().to_owned());
            }
        }
        invoke.resolver.resolve(true);
        true
    }).setup(move |app| {
        for label in ["html", "xml", "rss", "atom", "xml-bad", "rss-bad"] {
            WebviewWindowBuilder::new(app,format!("loader-{label}"),WebviewUrl::External(format!("http://publisher.invalid/{label}").parse()?))
                .visible(false)
                .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
                .proxy_url(format!("http://{address}").parse()?)
                .initialization_script(format!("window.__LOADER_ID__='{label}';window.__LOADER_KEEP_OPEN__=false;window.__LOAD_REMOTE_IMAGE__=false;\n{}",include_str!("../src/article_loader.js")))
                .on_new_window(|_,_|tauri::webview::NewWindowResponse::Deny)
                .on_download(move |_,_| {eprintln!("DOWNLOAD {label}"); false})
                .on_page_load(move |_,payload| {eprintln!("PAGE {label} {:?}",payload.event());})
                .build()?;
        }
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let reports = reports.lock().unwrap();
            eprintln!("OBSERVED captures={reports:?}");
            handle.exit(if reports.len() == 6 {0} else {1});
        });
        Ok(())
    }).build(context).unwrap();
    app.run(|_, _| {});
}
