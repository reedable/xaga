//! Native download smoke: fake external host served only by an explicit proxy.
//! No public network requests. Run in a macOS GUI session.
#[cfg(target_os = "macos")]
#[path = "../src/article_diagnostics.rs"]
#[allow(dead_code, unused_imports)]
mod article_diagnostics;
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
    use resource_download::{ByteValidator, ICON_MAX_BYTES, IconValidator, ImageValidator, PdfValidator, StagedDownload};
    use std::{
        io::{Cursor, Read, Write},
        net::TcpListener,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, webview::DownloadEvent};
    let scenario = std::env::var("XAGA_DOWNLOAD_SMOKE_CASE").unwrap_or_else(|_| "icon".into());
    assert!(["icon", "image", "image-document", "pdf", "invalid-pdf", "html", "blob-pdf"].contains(&scenario.as_str()));
    let content_image = matches!(scenario.as_str(), "image" | "image-document");
    let image_document = scenario == "image-document";
    let pdf = matches!(scenario.as_str(), "pdf" | "invalid-pdf");
    let rendering = matches!(scenario.as_str(), "html" | "blob-pdf");
    let valid = scenario != "invalid-pdf";
    if pdf {
        assert!(
            cfg!(feature = "wry-pdf-downloads"),
            "PDF smoke requires wry-pdf-downloads"
        );
    }
    enum Validator {
        Icon,
        Image,
        Pdf,
    }
    impl ByteValidator for Validator {
        type Accepted = String;
        fn validate(self, bytes: &[u8]) -> Result<String, String> {
            match self {
                Self::Icon => IconValidator.validate(bytes),
                Self::Image => ImageValidator.validate(bytes),
                Self::Pdf => PdfValidator.validate(bytes).map(|_| "pdf".into()),
            }
        }
    }
    let body = if scenario == "html" {
        b"<!doctype html><title>SMOKE-PASS</title><p>HTML remains renderable</p>".to_vec()
    } else if scenario == "blob-pdf" {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(pdf_fixture());
        format!("<!doctype html><script>const b=Uint8Array.from(atob('{encoded}'),c=>c.charCodeAt(0)); const f=document.createElement('iframe'); f.onload=()=>document.title='SMOKE-PASS'; f.src=URL.createObjectURL(new Blob([b],{{type:'application/pdf'}})); document.documentElement.append(f);</script>").into_bytes()
    } else if pdf {
        if valid {
            pdf_fixture()
        } else {
            b"not a PDF".to_vec()
        }
    } else if content_image {
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1200, 300).write_to(&mut data, image::ImageFormat::Png).unwrap();
        data.into_inner()
    } else {
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(16, 16)
            .write_to(&mut data, image::ImageFormat::Ico)
            .unwrap();
        data.into_inner()
    };
    let original_image = body.clone();
    let mime = if image_document { "image/png" } else if rendering {
        "text/html"
    } else if pdf {
        "application/pdf"
    } else {
        "application/octet-stream"
    };
    let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = proxy.local_addr().unwrap();
    let seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
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
                seen_server.fetch_add(1, Ordering::SeqCst);
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
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
    let render_seen = seen.clone();
    let render_passed = completed.clone();
    let render_scenario = scenario.clone();
    let staging = Arc::new(Mutex::new(None::<StagedDownload<Validator>>));
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let capture_passed = completed.clone();
    let app = tauri::Builder::default().invoke_handler(move |invoke| {
        if invoke.message.command() == "receive_image" {
            use base64::Engine;
            let tauri::ipc::InvokeBody::Json(body) = invoke.message.payload() else { panic!("JSON required"); };
            let data = body["dataUri"].as_str().expect("image bytes must arrive");
            let validated = resource_download::validate_image_data(data).expect("image document bytes");
            let bytes = base64::engine::general_purpose::STANDARD.decode(validated.split_once(',').unwrap().1).unwrap();
            assert_eq!(bytes, original_image);
            println!("PASS: image document capture preserved original bytes through isolated navigation and validation");
            capture_passed.store(true, Ordering::SeqCst);
            let handle = invoke.message.webview().app_handle().clone();
            invoke.resolver.resolve(());
            handle.exit(0);
        } else {
            invoke.resolver.resolve(());
        }
        true
    }).setup(move |app| {
        let handle = app.handle().clone();
        let watchdog = handle.clone();
        let render_handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            eprintln!("native download timed out");
                watchdog.exit(1);
        });
        WebviewWindowBuilder::new(app, "loader-smoke", WebviewUrl::External("http://download.example.invalid/favicon.ico".parse().unwrap()))
            .initialization_script(if image_document { format!(
                "window.__LOADER_IMAGE_ONLY__=true;window.__LOADER_ID__='smoke';\n{}",
                include_str!("../src/article_loader.js")
            ) } else { String::new() })
            .visible(false)
            .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
            .proxy_url(format!("http://{address}").parse().unwrap())
            .on_document_title_changed(move |_, title| {
                if rendering && title == "SMOKE-PASS" {
                    assert_eq!(render_seen.load(Ordering::SeqCst), 1);
                    println!("PASS: {render_scenario}, rendered without download conversion");
                    render_passed.store(true, Ordering::SeqCst);
                    render_handle.exit(0);
                }
            })
            .on_download(move |webview, event| {
                match event {
                    DownloadEvent::Requested { url, destination } => {
                        // A first navigation can become a download before any page
                        // URL exists. Exercise the production diagnostic collector.
                        let evidence = article_diagnostics::DownloadDiagnostic::from_download(
                            &webview, "http://download.example.invalid/start", &url,
                            Some(destination.to_string_lossy().into_owned()), None,
                        );
                        assert!(evidence.current_url.is_none());
                        assert_eq!(evidence.download_url, url.as_str());
                        assert!(!rendering, "renderable scenario incorrectly converted to download");
                        assert_eq!(url.as_str(), "http://download.example.invalid/icon-bytes", "callback reports redirected resource");
                        let job = StagedDownload::new(if pdf { Validator::Pdf } else if content_image { Validator::Image } else { Validator::Icon }, ICON_MAX_BYTES).unwrap();
                        *destination = job.path().to_owned();
                        *staging.lock().unwrap() = Some(job);
                        true
                    }
                    DownloadEvent::Finished { success, path, .. } => {
                        let job = staging.lock().unwrap().take().unwrap();
                        let assigned = job.path().to_owned();
                        assert!(success, "transport must succeed before testing byte validation");
                        let result = job.finish(success);
                        if !valid {
                            assert_eq!(result.unwrap_err(), "unsupported download format");
                        } else if pdf {
                            assert_eq!(result.unwrap(), "pdf");
                        } else {
                            assert!(result.unwrap().starts_with("data:image/png;base64,"));
                        }
                        assert!(!assigned.parent().unwrap().exists());
                        assert_eq!(seen.load(Ordering::SeqCst), 1, "exactly one final resource retrieval through proxy");
                        println!("PASS: {scenario}, native download and redirect through proxy, callback path={path:?}, validation checked, staging cleaned");
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

#[cfg(target_os = "macos")]
fn pdf_fixture() -> Vec<u8> {
    use std::io::Write;
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
    ]
    .iter()
    .enumerate()
    {
        offsets.push(bytes.len());
        write!(bytes, "{} 0 obj\n{object}\nendobj\n", index + 1).unwrap();
    }
    let xref = bytes.len();
    write!(bytes, "xref\n0 4\n0000000000 65535 f \n").unwrap();
    for offset in offsets.iter().skip(1) {
        writeln!(bytes, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        bytes,
        "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .unwrap();
    bytes
}
