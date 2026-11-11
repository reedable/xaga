//! Real refresh command → IPC → TauriPlatform → useFeeds → ErrorSinkProvider.
//! Uses only temporary library data and a controlled fixture proxy.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");
#[path = "support/app_state.rs"]
mod app_state;

#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };
    let library = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(xaga_core::library::feeds_dir(library.path())).unwrap();
    for (id, title) in [("good", "Working feed"), ("bad", "Unavailable feed")] {
        std::fs::write(xaga_core::library::feeds_dir(library.path()).join(format!("{id}.atom")), format!(r#"<feed xmlns="http://www.w3.org/2005/Atom"><id>{id}</id><title>{title}</title><updated>2026-01-01T00:00:00Z</updated><link rel="self" href="http://publisher.invalid/{id}"/></feed>"#)).unwrap();
    }
    let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = proxy.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for connection in proxy.incoming() {
            let mut stream = connection.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut head = Vec::new();
            let mut byte = [0];
            while !head.ends_with(b"\r\n\r\n") && head.len() < 8192 {
                if stream.read(&mut byte).unwrap_or(0) == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            let request = String::from_utf8_lossy(&head);
            assert!(
                request.contains("publisher.invalid/"),
                "unexpected request {request}"
            );
            let (status, body) = if request.contains("/bad") {
                ("403 Forbidden", "denied")
            } else {
                (
                    "200 OK",
                    r#"<feed xmlns="http://www.w3.org/2005/Atom"><id>good</id><title>Working feed</title><updated>2026-01-01T00:00:00Z</updated></feed>"#,
                )
            };
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/atom+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let mut state = app_state::test_state(library.path().into());
    state.network_proxy_port = port;
    state.library.write().unwrap().writer = Some(xaga_core::library_writer::LibraryWriter::start(
        library.path().into(),
        xaga_core::library::acquire_lock(library.path()).unwrap(),
    ));
    let mut context = app_context();
    context.config_mut().app.windows.clear();
    context.config_mut().build.dev_url = Some("http://127.0.0.1:1422".parse().unwrap());
    tauri::Builder::default()
        .manage(state)
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                )])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::feeds::list_feeds,
            commands::feeds::refresh_all_feeds
        ])
        .setup(|app| {
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("e2e/native-feed-refresh.html".into()),
            )
            .visible(true)
            .build()?;
            window.set_focus()?;
            let watchdog = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(45)).await;
                eprintln!("FAIL: native refresh verification watchdog");
                watchdog.exit(1);
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                for _ in 0..450 {
                    let title = handle.get_webview_window("main").unwrap().title().unwrap();
                    if title.starts_with("PASS:") {
                        assert!(title.contains("Unavailable feed"), "{title}");
                        let state = handle.state::<AppState>();
                        let good = std::fs::read_to_string(xaga_core::library::feed_path(&state.library_path(), "good")).unwrap();
                        let bad = std::fs::read_to_string(xaga_core::library::feed_path(&state.library_path(), "bad")).unwrap();
                        assert!(!good.contains("2026-01-01T00:00:00Z"), "successful feed did not commit");
                        assert!(bad.contains("2026-01-01T00:00:00Z"), "failed feed was changed");
                        println!("PASS: production native failure displayed in shared UI; successful feed committed and retry enabled: {title}");
                        handle.exit(0);
                        return;
                    }
                    if title.starts_with("FAIL:") {
                        eprintln!("{title}");
                        handle.exit(1);
                        return;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                eprintln!("FAIL: native refresh verification timed out");
                handle.exit(1);
            });
            Ok(())
        })
        .build(context)
        .unwrap()
        .run(|_, _| {});
}
