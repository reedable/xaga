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
    use resource_download::{ByteValidator, ICON_MAX_BYTES, IconValidator, StagedDownload};
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
    let scenario = std::env::var("XAGA_DOWNLOAD_SMOKE_CASE").unwrap_or_else(|_| "icon".into());
    assert!(["icon", "pdf", "invalid-pdf", "html", "blob-pdf"].contains(&scenario.as_str()));
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
        Pdf,
    }
    impl ByteValidator for Validator {
        type Accepted = String;
        fn validate(self, bytes: &[u8]) -> Result<String, String> {
            match self {
                Self::Icon => IconValidator.validate(bytes),
                // Same format-signature contract as the existing PDF probe.
                Self::Pdf if bytes.starts_with(b"%PDF-") => Ok("pdf".into()),
                Self::Pdf => Err("invalid PDF signature".into()),
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
    } else {
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(16, 16)
            .write_to(&mut data, image::ImageFormat::Ico)
            .unwrap();
        data.into_inner()
    };
    let mime = if rendering {
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
    let app = tauri::Builder::default().setup(move |app| {
        let handle = app.handle().clone();
        let watchdog = handle.clone();
        let render_handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            eprintln!("native download timed out");
                watchdog.exit(1);
        });
        WebviewWindowBuilder::new(app, "download-smoke", WebviewUrl::External("http://download.example.invalid/favicon.ico".parse().unwrap()))
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
            .on_download(move |_, event| {
                match event {
                    DownloadEvent::Requested { url, destination } => {
                        assert!(!rendering, "renderable scenario incorrectly converted to download");
                        assert_eq!(url.as_str(), "http://download.example.invalid/icon-bytes", "callback reports redirected resource");
                        let job = StagedDownload::new(if pdf { Validator::Pdf } else { Validator::Icon }, ICON_MAX_BYTES).unwrap();
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
                            assert_eq!(result.unwrap_err(), "invalid PDF signature");
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
