//! Native download smoke: fake external host served only by an explicit proxy.
//! No public network requests. Run in a macOS GUI session.
#[cfg(target_os = "macos")]
#[path = "../src/article_diagnostics.rs"]
#[allow(dead_code, unused_imports)]
mod article_diagnostics;
#[cfg(target_os = "macos")]
#[path = "support/pdf_fixture.rs"]
mod pdf_fixture;
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
    use resource_download::{
        ByteValidator, IMAGE_MAX_BYTES, ImageValidator, PdfValidator, StagedDownload,
        prepare_favicon,
    };
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
    let visible = std::env::var("XAGA_DOWNLOAD_SMOKE_VISIBLE").as_deref() == Ok("1");
    let subframe = std::env::var("XAGA_DOWNLOAD_SMOKE_SUBFRAME").as_deref() == Ok("1");
    assert!(
        [
            "icon",
            "image",
            "image-attachment",
            "malformed-image-attachment",
            "html-as-image-attachment",
            "image-document",
            "svg",
            "svg-document",
            "svg-icon-document",
            "avif",
            "avif-document",
            "avif-icon-document",
            "pdf",
            "pdf-attachment",
            "invalid-pdf",
            "favicon-pdf",
            "html",
            "blob-pdf",
            "relative-image",
            "cross-origin-image",
            "missing-image",
            "data-image",
            "blob-image",
            "file-image",
            "unsupported-image-scheme",
        ]
        .contains(&scenario.as_str())
    );
    let content_image = matches!(
        scenario.as_str(),
        "image"
            | "image-attachment"
            | "malformed-image-attachment"
            | "html-as-image-attachment"
            | "image-document"
            | "svg"
            | "svg-document"
            | "avif"
            | "avif-document"
    );
    let attachment = scenario.ends_with("-attachment");
    let malformed_image = scenario == "malformed-image-attachment";
    let html_as_image = scenario == "html-as-image-attachment";
    let missing_image = scenario == "missing-image";
    let svg = scenario.starts_with("svg");
    let avif = scenario.starts_with("avif");
    let image_document = scenario.ends_with("-document");
    let pdf = matches!(scenario.as_str(), "pdf" | "pdf-attachment" | "invalid-pdf");
    let favicon_pdf = scenario == "favicon-pdf";
    let pdf_response = pdf || favicon_pdf;
    let rendered_image_url = match scenario.as_str() {
        "relative-image" | "missing-image" => Some("/rendered.png"),
        "cross-origin-image" => Some("http://images.example.invalid/rendered.png"),
        _ => None,
    };
    let inline_image = matches!(scenario.as_str(), "data-image" | "blob-image");
    let file_image = scenario == "file-image";
    let unsupported_image_scheme = scenario == "unsupported-image-scheme";
    let rendering = matches!(scenario.as_str(), "html" | "blob-pdf")
        || rendered_image_url.is_some()
        || inline_image
        || file_image
        || unsupported_image_scheme;
    assert!(
        !subframe || rendering || attachment,
        "subframe mode requires a rendering or attachment case"
    );
    let valid = scenario != "invalid-pdf";
    if pdf_response {
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
                Self::Icon => ImageValidator.validate(bytes).and_then(prepare_favicon),
                Self::Image => ImageValidator.validate(bytes),
                Self::Pdf => PdfValidator.validate(bytes).map(|_| "pdf".into()),
            }
        }
    }
    let rendered_image = (rendered_image_url.is_some() || inline_image || file_image).then(|| {
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(16, 16)
            .write_to(&mut data, image::ImageFormat::Png)
            .unwrap();
        data.into_inner()
    });
    let file_fixture = file_image.then(|| tempfile::tempdir().unwrap());
    let file_url = file_fixture.as_ref().map(|directory| {
        let path = directory.path().join("fixture.png");
        std::fs::write(&path, rendered_image.as_ref().unwrap()).unwrap();
        tauri::Url::from_file_path(path).unwrap().to_string()
    });
    let broken_image_url = if missing_image {
        Some("/rendered.png")
    } else if unsupported_image_scheme {
        Some("xaga-smoke-unsupported:image.png")
    } else {
        file_url.as_deref()
    };
    let body = if inline_image {
        use base64::Engine;
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(rendered_image.as_ref().unwrap());
        let source = if scenario == "data-image" {
            format!("'data:image/png;base64,{encoded}'")
        } else {
            format!(
                "URL.createObjectURL(new Blob([Uint8Array.from(atob('{encoded}'), c => c.charCodeAt(0))], {{type:'image/png'}}))"
            )
        };
        format!("<!doctype html><p>Article text</p><img id=image><script>const image = document.getElementById('image'); image.onload = () => {{ if (image.naturalWidth === 16) document.title='SMOKE-PASS'; }}; image.src = {source};</script>").into_bytes()
    } else if let Some(url) = broken_image_url {
        format!(r#"<!doctype html><p>Article text</p><img src="{url}" onerror="if (document.querySelector('p').textContent === 'Article text') document.title='SMOKE-PASS';">"#).into_bytes()
    } else if let Some(url) = rendered_image_url {
        format!(
            "<!doctype html><p>Article text</p><img src=\"{url}\" onload=\"if (this.naturalWidth === 16) document.title='SMOKE-PASS';\">"
        )
        .into_bytes()
    } else if scenario == "html" {
        b"<!doctype html><title>SMOKE-PASS</title><p>HTML remains renderable</p>".to_vec()
    } else if scenario == "blob-pdf" {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(pdf_fixture::pdf_fixture());
        format!("<!doctype html><script>const b=Uint8Array.from(atob('{encoded}'),c=>c.charCodeAt(0)); const f=document.createElement('iframe'); f.onload=()=>document.title='SMOKE-PASS'; f.src=URL.createObjectURL(new Blob([b],{{type:'application/pdf'}})); document.documentElement.append(f);</script>").into_bytes()
    } else if pdf_response {
        if valid {
            pdf_fixture::pdf_fixture()
        } else {
            b"not a PDF".to_vec()
        }
    } else if svg {
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="32" height="32" fill="red"/></svg>"#.to_vec()
    } else if avif {
        include_bytes!("fixtures/color-profile.avif").to_vec()
    } else if malformed_image {
        b"\x89PNG\r\n\x1a\n".to_vec()
    } else if html_as_image {
        b"<!doctype html><title>Not image bytes</title>".to_vec()
    } else if content_image {
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1200, 300)
            .write_to(&mut data, image::ImageFormat::Png)
            .unwrap();
        data.into_inner()
    } else {
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(16, 16)
            .write_to(&mut data, image::ImageFormat::Ico)
            .unwrap();
        data.into_inner()
    };
    let original_image = body.clone();
    let image_mime = if svg {
        "image/svg+xml"
    } else if avif {
        "image/avif"
    } else {
        "image/png"
    };
    let mime = if pdf_response {
        "application/pdf"
    } else if image_document || attachment {
        image_mime
    } else if rendering {
        "text/html"
    } else {
        "application/octet-stream"
    };
    let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = proxy.local_addr().unwrap();
    let start_path = if subframe { "/frame" } else { "/favicon.ico" };
    let image_requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let image_requests_server = image_requests.clone();
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
            let request = String::from_utf8_lossy(&header);
            if request.starts_with("CONNECT download.example.invalid:80 ")
                || (rendered_image_url.is_some()
                    && request.starts_with("CONNECT images.example.invalid:80 "))
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
            if let (Some(url), Some(bytes)) = (rendered_image_url, &rendered_image) {
                let host = if url.starts_with('/') {
                    "download.example.invalid"
                } else {
                    "images.example.invalid"
                };
                if text.starts_with(&format!("GET http://{host}/rendered.png "))
                    || (text.starts_with("GET /rendered.png ")
                        && text
                            .lines()
                            .any(|line| line.eq_ignore_ascii_case(&format!("Host: {host}"))))
                {
                    image_requests_server.fetch_add(1, Ordering::SeqCst);
                    if missing_image {
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                        continue;
                    }
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
                    stream.write_all(bytes).unwrap();
                    continue;
                }
            }
            if subframe
                && (text.starts_with("GET /frame ")
                    || text.starts_with("GET http://download.example.invalid/frame "))
            {
                let page: &[u8] = if rendering {
                    br#"<!doctype html><iframe src="/frame-content" onload="if (this.contentDocument.title === 'SMOKE-PASS') document.title='SMOKE-PASS';"></iframe>"#
                } else {
                    br#"<!doctype html><p>Article text</p><iframe id="download-frame"></iframe>"#
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", page.len()).unwrap();
                stream.write_all(page).unwrap();
                continue;
            }
            let redirect_path = if subframe {
                "/frame-content"
            } else {
                "/favicon.ico"
            };
            if text.starts_with(&format!("GET {redirect_path} "))
                || text.starts_with(&format!(
                    "GET http://download.example.invalid{redirect_path} "
                ))
            {
                stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://download.example.invalid/icon-bytes\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                continue;
            }
            if text.starts_with("GET /icon-bytes ")
                || text.starts_with("GET http://download.example.invalid/icon-bytes ")
            {
                seen_server.fetch_add(1, Ordering::SeqCst);
                let disposition = if attachment {
                    "Content-Disposition: attachment\r\n"
                } else {
                    ""
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\n{disposition}Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
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
    let parent_loaded = Arc::new(AtomicBool::new(false));
    let download_parent_loaded = parent_loaded.clone();
    let staging = Arc::new(Mutex::new(None::<StagedDownload<Validator>>));
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let capture_passed = completed.clone();
    let app = tauri::Builder::default().invoke_handler(move |invoke| {
        if matches!(invoke.message.command(), "receive_image" | "receive_favicon") {
            assert_eq!(invoke.message.command(), if content_image { "receive_image" } else { "receive_favicon" });
            use base64::Engine;
            let tauri::ipc::InvokeBody::Json(body) = invoke.message.payload() else { panic!("JSON required"); };
            let data = body["dataUri"].as_str().expect("image bytes must arrive");
            let validated = resource_download::validate_image_data(data).expect("image document bytes");
            let validated = if content_image { validated } else { prepare_favicon(validated).unwrap() };
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
        let ui = webview_policy::ui_configuration().expect("UI WebView policy");
        let loader = webview_policy::loader_configuration(None).expect("loader WebView policy");
        assert_ne!(unsafe { ui.websiteDataStore() }, unsafe { loader.websiteDataStore() }, "UI and download stores must remain isolated");
        let handle = app.handle().clone();
        let watchdog = handle.clone();
        let render_handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            eprintln!("native download timed out");
                watchdog.exit(1);
        });
        WebviewWindowBuilder::new(app, "loader-smoke", WebviewUrl::External(format!("http://download.example.invalid{start_path}").parse().unwrap()))
            .initialization_script(if image_document { format!(
                "window.__LOADER_IMAGE_ONLY__={content_image};window.__LOAD_REMOTE_IMAGE__=true;window.__LOADER_ID__='smoke';\n{}",
                include_str!("../src/article_loader.js")
            ) } else { String::new() })
            .visible(visible)
            .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
            .proxy_url(format!("http://{address}").parse().unwrap())
            .on_page_load(move |webview, payload| {
                if subframe && !rendering
                    && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                    && payload.url().path() == "/frame"
                {
                    parent_loaded.store(true, Ordering::SeqCst);
                    webview.eval("document.getElementById('download-frame').src='/frame-content'").unwrap();
                }
            })
            .on_document_title_changed(move |_, title| {
                if rendering && title == "SMOKE-PASS" {
                    assert_eq!(render_seen.load(Ordering::SeqCst), 1);
                    if rendered_image_url.is_some() {
                        assert_eq!(image_requests.load(Ordering::SeqCst), 1);
                    }
                    println!("PASS: {render_scenario}, rendered without download conversion");
                    render_passed.store(true, Ordering::SeqCst);
                    render_handle.exit(0);
                }
            })
            .on_download(move |webview, event| {
                match event {
                    DownloadEvent::Requested { url, destination } => {
                        // Exercise the production collector without asking Wry for
                        // the current URL during its download callback.
                        let evidence = article_diagnostics::DownloadDiagnostic::from_download(
                            &webview, if subframe { "http://download.example.invalid/frame" } else { "http://download.example.invalid/start" }, &url,
                            Some(destination.to_string_lossy().into_owned()), None,
                        );
                        assert!(evidence.current_url.is_none());
                        if subframe {
                            assert!(download_parent_loaded.load(Ordering::SeqCst));
                            assert_eq!(evidence.source_url, "http://download.example.invalid/frame");
                        }
                        assert_eq!(evidence.download_url, url.as_str());
                        assert!(!rendering, "renderable scenario incorrectly converted to download");
                        assert_eq!(url.as_str(), "http://download.example.invalid/icon-bytes", "callback reports redirected resource");
                        let job = StagedDownload::new(if pdf { Validator::Pdf } else if content_image { Validator::Image } else { Validator::Icon }, IMAGE_MAX_BYTES).unwrap();
                        *destination = job.path().to_owned();
                        *staging.lock().unwrap() = Some(job);
                        true
                    }
                    DownloadEvent::Finished { success, path, .. } => {
                        let job = staging.lock().unwrap().take().unwrap();
                        let assigned = job.path().to_owned();
                        assert!(success, "transport must succeed before testing byte validation");
                        let result = job.finish(success);
                        if malformed_image {
                            assert!(result.is_err(), "truncated PNG must fail image validation");
                        } else if html_as_image {
                            assert!(result.is_err(), "HTML bytes must fail image validation");
                        } else if favicon_pdf {
                            assert!(result.is_err(), "favicon PDF must fail image validation");
                        } else if !valid {
                            assert_eq!(result.unwrap_err(), "unsupported download format");
                        } else if pdf {
                            assert_eq!(result.unwrap(), "pdf");
                        } else {
                            assert!(result.unwrap().starts_with(&format!("data:{image_mime};base64,")));
                        }
                        assert!(!assigned.parent().unwrap().exists());
                        assert_eq!(seen.load(Ordering::SeqCst), 1, "exactly one final resource retrieval through proxy");
                        println!("PASS: {scenario}, native download and redirect through proxy, callback path={path:?}, request-purpose validation checked, staging cleaned");
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
