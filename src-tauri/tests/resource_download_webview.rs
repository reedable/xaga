//! Native download smoke: fake external host served only by an explicit proxy.
//! No public network requests. Run in a macOS GUI session.
#[cfg(target_os = "macos")]
#[path = "../src/resource_download.rs"]
#[allow(dead_code)]
mod resource_download;
#[cfg(target_os = "macos")]
#[path = "../src/webview_policy.rs"]
mod webview_policy;
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use resource_download::{ICON_MAX_BYTES, IconValidator, StagedDownload};
    use std::{
        io::{Cursor, Read, Write},
        net::TcpListener,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tauri::{WebviewUrl, WebviewWindowBuilder, webview::DownloadEvent};
    let mut data = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(16, 16)
        .write_to(&mut data, image::ImageFormat::Ico)
        .unwrap();
    let body = data.into_inner();
    let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = proxy.local_addr().unwrap();
    let seen = Arc::new(AtomicBool::new(false));
    let seen_server = seen.clone();
    std::thread::spawn(move || {
        for connection in proxy.incoming() {
            let mut stream = connection.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut header = Vec::new();
            let mut byte = [0];
            while !header.ends_with(b"\r\n\r\n") && header.len() < 8192 {
                if stream.read(&mut byte).unwrap_or(0) == 0 {
                    break;
                }
                header.push(byte[0]);
            }
            if String::from_utf8_lossy(&header).starts_with("CONNECT download.example.invalid:80 ")
            {
                stream
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .unwrap();
                header.clear();
                while !header.ends_with(b"\r\n\r\n") && header.len() < 8192 {
                    if stream.read(&mut byte).unwrap_or(0) == 0 {
                        break;
                    }
                    header.push(byte[0]);
                }
            }
            let text = String::from_utf8_lossy(&header);
            if text.starts_with("GET /favicon.ico ")
                || text.starts_with("GET http://download.example.invalid/favicon.ico ")
            {
                stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://download.example.invalid/icon-bytes\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                continue;
            }
            if text.starts_with("GET /icon-bytes ")
                || text.starts_with("GET http://download.example.invalid/icon-bytes ")
            {
                seen_server.store(true, Ordering::SeqCst);
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            } else {
                let _ = stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        }
    });
    let completed = Arc::new(AtomicBool::new(false));
    let passed = completed.clone();
    let staging = Arc::new(Mutex::new(None::<StagedDownload<IconValidator>>));
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default().setup(move |app| {
        let handle = app.handle().clone();
        let watchdog = handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            eprintln!("native download timed out");
                watchdog.exit(1);
        });
        WebviewWindowBuilder::new(app, "download-smoke", WebviewUrl::External("http://download.example.invalid/favicon.ico".parse().unwrap()))
            .visible(false)
            .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
            .proxy_url(format!("http://{address}").parse().unwrap())
            .on_download(move |_, event| {
                match event {
                    DownloadEvent::Requested { url, destination } => {
                        assert_eq!(url.as_str(), "http://download.example.invalid/icon-bytes", "callback reports redirected resource");
                        let job = StagedDownload::new(IconValidator, ICON_MAX_BYTES).unwrap();
                        *destination = job.path().to_owned();
                        *staging.lock().unwrap() = Some(job);
                        true
                    }
                    DownloadEvent::Finished { success, path, .. } => {
                        let job = staging.lock().unwrap().take().unwrap();
                        let assigned = job.path().to_owned();
                        let result = job.finish(success);
                        assert!(result.unwrap().starts_with("data:image/png;base64,"));
                        assert!(!assigned.parent().unwrap().exists());
                        assert!(seen.load(Ordering::SeqCst), "download did not traverse configured proxy");
                        println!("PASS: native download and redirect through proxy, callback path={path:?}, validated PNG, staging cleaned");
                        completed.store(true, Ordering::SeqCst);
                        handle.exit(0);
                        false
                    }
                    _ => false,
                }
            }).build()?;
        Ok(())
    }).build(context).unwrap();
    app.run(|_, _| {});
    assert!(
        passed.load(Ordering::SeqCst),
        "native download never completed"
    );
}
