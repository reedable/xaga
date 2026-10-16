//! Run with `node scripts/verify-svg-webview.mjs` in a macOS GUI session.
//! Uses the exact configured CSP on custom-protocol fixture documents, not a
//! claim that the full packaged application or all SVG features were exercised.
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
        sync::{Arc, Mutex},
        time::Duration,
    };
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    use xaga_core::network_proxy::{self, ProxyConfig};

    let bootstrap = std::fs::read_to_string(
        std::env::var("XAGA_SVG_PROBE_SCRIPT").expect("run node scripts/verify-svg-webview.mjs"),
    )
    .unwrap();
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let csp = config["app"]["security"]["csp"]
        .as_str()
        .unwrap()
        .to_owned();
    let upstream = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", upstream.local_addr().unwrap());
    let reached = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_reached = reached.clone();
    std::thread::spawn(move || {
        for connection in upstream.incoming() {
            let mut stream = connection.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                if stream.read(&mut byte).unwrap_or(0) == 0 {
                    break;
                }
                request.push(byte[0]);
            }
            let line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap_or("empty connection")
                .to_owned();
            eprintln!("DESTINATION: {line}");
            server_reached.lock().unwrap().push(line);
            let body = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><g id="shape"><rect width="16" height="16" fill="red"/></g></svg>"#;
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: image/svg+xml\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(body);
        }
    });
    let results = Arc::new(Mutex::new(std::collections::HashMap::<
        String,
        serde_json::Value,
    >::new()));
    let blocked = Arc::new(Mutex::new(Vec::<String>::new()));
    let passed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = passed.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let fixture_base = base.clone();
    let app = tauri::Builder::default()
        .register_uri_scheme_protocol("svgfixture", move |_, request| {
            let label = request.uri().path().trim_start_matches('/');
            let body = format!("<!doctype html><html><head><title>SVG probe</title></head><body>{}</body></html>", svg_payload(&fixture_base, label, "inline"));
            let mut response = tauri::http::Response::builder().header("Content-Type", "text/html");
            if label != "control" { response = response.header("Content-Security-Policy", &csp); }
            response.body(body.into_bytes()).unwrap()
        })
        .setup(move |app| {
            for label in ["control", "main", "settings"] {
                let observations = blocked.clone();
                let proxy = tauri::async_runtime::block_on(network_proxy::start_scoped(
                    ProxyConfig::deny_all().with_observer(Arc::new(move |code, detail| {
                        observations.lock().unwrap().push(format!("{label}: {code}: {detail}"));
                    }))
                ))?;
                let page = serde_json::json!({
                    "label": label,
                    "base": base,
                    "payload": format!("{}<img src=\"data:image/svg+xml;base64,{}\">", svg_payload(&base, label, "reader"), {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(svg_payload(&base, label, "reader-image"))
                    }),
                    "image": svg_payload(&base, label, "image"),
                });
                let reports = results.clone();
                let handle = app.handle().clone();
                let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(format!("svgfixture://localhost/{label}").parse()?))
                    .initialization_script(format!("window.__svgProbe={page};\n{bootstrap}"))
                    .with_webview_configuration(webview_policy::ui_configuration().unwrap())
                    .proxy_url(network_proxy::proxy_url(proxy.port).parse()?)
                    .visible(false)
                    .on_document_title_changed(move |_, title| {
                        if let Some(json) = title.strip_prefix("SVG-RESULT:") {
                            let report: serde_json::Value = serde_json::from_str(json).unwrap();
                            println!("REPORT {label}: {report}");
                            reports.lock().unwrap().insert(label.into(), report);
                        } else if title.starts_with("SVG-ERROR:") {
                            eprintln!("{label}: {title}");
                            handle.exit(1);
                        }
                    }).build()?;
                let retained = Mutex::new(Some(proxy));
                window.on_window_event(move |event| if matches!(event, tauri::WindowEvent::Destroyed) {
                    retained.lock().unwrap().take();
                });
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                for _ in 0..150 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    if results.lock().unwrap().len() < 3 { continue; }
                    let reports = results.lock().unwrap();
                    let requests = reached.lock().unwrap();
                    let proxy = blocked.lock().unwrap();
                    println!("PROXY: {proxy:?}");
                    let control = &reports["control"];
                    let control_works = !control["executed"].as_array().unwrap().is_empty()
                        && requests.iter().any(|r| r.contains("/control/inline/"));
                    let protected = ["main", "settings"].iter().all(|label| {
                        let report = &reports[*label];
                        report["executed"].as_array().unwrap().is_empty()
                            && report["imageLoaded"] == true
                            && report["readerReady"] == true
                            && report["activeMarkup"] == false
                            && !requests.iter().any(|r| r.contains(&format!("/{label}/")))
                            && !proxy.iter().any(|r| r.starts_with(&format!("{label}:")))
                    });
                    let image_mode = !requests.iter().any(|r| r.contains("/image/") || r.contains("/reader-image/"));
                    let attributed = requests.iter().all(|r| r.contains("/control/"));
                    if control_works && protected && image_mode && attributed {
                        passed.store(true, std::sync::atomic::Ordering::SeqCst);
                        println!("PASS: control executes and reaches loopback; exact app CSP blocks tested UI SVG scripts/resources; data SVG image/CSS stay inert; production reader pipeline responds without image requests");
                        handle.exit(0);
                    } else {
                        eprintln!("FAIL: control={control_works}, protected={protected}, image_mode={image_mode}; requests={requests:?}");
                        handle.exit(1);
                    }
                    return;
                }
                eprintln!("FAIL: SVG probe timed out");
                handle.exit(1);
            });
            Ok(())
        }).build(context).unwrap();
    app.run(|_, _| {});
    assert!(result.load(std::sync::atomic::Ordering::SeqCst));
}

#[cfg(target_os = "macos")]
fn svg_payload(base: &str, label: &str, phase: &str) -> String {
    let target = format!("{base}/{label}/{phase}");
    let tls_target = target.replacen("http://", "https://", 1);
    let tls_image = if label == "control" {
        String::new()
    } else {
        format!(r#"<image href="{tls_target}/tls.svg" width="16" height="16"/>"#)
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100" height="100" onload="fetch('{target}/event');window.__svgExecuted?.push('event')">
      <script>fetch('{target}/script');window.__svgExecuted?.push('script'); parent.postMessage({{type:'svg-attack'}},'*');</script>
      <script xlink:href="{target}/external.js"/>
      <rect width="100" height="100" fill="green"/>
      <image href="{target}/image.svg" width="16" height="16"/>
      <image xlink:href="{target}/xlink.svg" width="16" height="16"/>
      <image href="http://svg-image.invalid/{label}/{phase}" width="16" height="16"/>
      <image href="https://svg-image.invalid/{label}/{phase}" width="16" height="16"/>
      {tls_image}
      <use href="{target}/use.svg#shape"/>
      <filter id="probe-filter"><feImage href="{target}/filter.svg"/></filter>
      <rect width="16" height="16" filter="url(#probe-filter)"/>
      <style>@import url('{target}/external.css'); @font-face{{font-family:svgProbe;src:url('{target}/font.woff')}} .probe-text{{font-family:svgProbe}}</style>
      <text class="probe-text" x="0" y="30">SVG probe</text>
      <foreignObject width="32" height="32"><div xmlns="http://www.w3.org/1999/xhtml" style="background-image:url('{target}/css.svg')"><img src="{target}/foreign.svg"/></div></foreignObject>
    </svg>"#
    )
}
