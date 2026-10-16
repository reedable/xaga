//! Local, same-origin Safari/loader comparison. No public endpoints or user data.
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
    let results = Arc::new(Mutex::new(serde_json::Map::new()));
    let received = results.clone();
    std::thread::spawn(move || {
        for s in listener.incoming() {
            let mut s = s.unwrap();
            s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut head = Vec::new();
            let mut byte = [0];
            while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
                if s.read(&mut byte).unwrap_or(0) == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            let header = String::from_utf8_lossy(&head);
            let first = header.lines().next().unwrap_or("");
            if first.starts_with("POST /result?") {
                let label = first.split('?').nth(1).unwrap().split(' ').next().unwrap();
                let len = header
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                assert!(len < 100000);
                let mut body = vec![0; len];
                s.read_exact(&mut body).unwrap();
                let mut value: serde_json::Value = serde_json::from_slice(&body).unwrap();
                value["requestUserAgent"] = header
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("user-agent:"))
                    .unwrap_or("")
                    .into();
                received.lock().unwrap().insert(label.into(), value);
                s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
                    .unwrap();
            } else {
                let body=format!("<!doctype html><meta charset=utf-8><title>Xaga fingerprint comparison</title><body>Measuring<script>{}</script>",include_str!("fixtures/fingerprint_probe.js"));
                let _=write!(s,"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
            }
        }
    });
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app=tauri::Builder::default().invoke_handler(|i|{i.resolver.resolve(());true}).setup(move |app| {
        for label in ["loader1","loader2"] {
            WebviewWindowBuilder::new(app,label,WebviewUrl::External(format!("http://{address}/?{label}").parse()?))
                .visible(false).position(-10000.0,-10000.0)
                .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
                .initialization_script(format!("window.__LOADER_ID__='probe';window.__LOADER_LABEL__='{label}';window.__LOADER_URL__='http://{address}/';window.__LOADER_KEEP_OPEN__=false;window.__LOADER_IMAGE_ONLY__=false;window.__LOAD_REMOTE_IMAGE__=true;\n{}",include_str!("../src/article_loader.js")))
                .on_new_window(|_,_|tauri::webview::NewWindowResponse::Deny).build()?;
        }
        std::process::Command::new("open").args(["-a","Safari",&format!("http://{address}/?safari")]).status()?;
        let handle=app.handle().clone();
        tauri::async_runtime::spawn(async move {
            for _ in 0..45 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let values=results.lock().unwrap();
                if values.len()==3 {
                    std::fs::write("/tmp/xaga-fingerprint-comparison.json",serde_json::to_string_pretty(&*values).unwrap()).unwrap();
                    eprintln!("PASS: Safari and two fresh loader stores recorded in /tmp/xaga-fingerprint-comparison.json");handle.exit(0);return;
                }
            }
            eprintln!("FAIL: only received {:?}",results.lock().unwrap().keys());handle.exit(1);
        });Ok(())
    }).build(context).unwrap();
    app.run(|_, _| {});
}
