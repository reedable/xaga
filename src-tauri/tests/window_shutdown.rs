//! Runs production close/Quit coordination in child processes with isolated libraries.
#![allow(dead_code, unused_imports)]
include!("../src/lib.rs");
#[path = "support/app_state.rs"]
mod app_state;

fn main() {
    if let Ok(directory) = std::env::var("XAGA_SHUTDOWN_FIXTURE") {
        let directory = PathBuf::from(directory);
        if directory.join("predecessor").exists() {
            let lock = xaga_core::library::acquire_lock(&directory).expect("successor write mode");
            assert!(
                std::fs::read_to_string(xaga_core::library::entry_path(&directory, "abcdef123456"))
                    .unwrap()
                    .contains("pending edit")
            );
            std::fs::write(directory.join("successor"), "write mode").unwrap();
            drop(lock);
            return;
        }
        fixture(directory);
        return;
    }
    for mode in ["close", "quit", "restart", "readonly"] {
        let directory = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .env("XAGA_SHUTDOWN_FIXTURE", directory.path())
            .env("XAGA_SHUTDOWN_MODE", mode)
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{mode}: {status}");
                break;
            }
            if std::time::Instant::now() > deadline {
                child.kill().ok();
                panic!("{mode}: timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if mode == "readonly" {
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        } else {
            assert!(
                std::fs::read_to_string(xaga_core::library::entry_path(
                    directory.path(),
                    "abcdef123456"
                ))
                .unwrap()
                .contains("pending edit")
            );
            while mode == "restart" && !directory.path().join("successor").exists() {
                assert!(std::time::Instant::now() < deadline, "successor failed");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert!(xaga_core::library::acquire_lock(directory.path()).is_some());
        }
        println!("PASS: {mode}");
    }
}

fn fixture(directory: PathBuf) {
    let mode = std::env::var("XAGA_SHUTDOWN_MODE").unwrap();
    let state = app_state::test_state(directory.clone());
    let retained_command_handle = if mode == "readonly" {
        None
    } else {
        state.library.write().unwrap().writer =
            Some(xaga_core::library_writer::LibraryWriter::start(
                directory.clone(),
                xaga_core::library::acquire_lock(&directory).unwrap(),
            ));
        Some(commands::require_writer(&state).unwrap())
    };
    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External("about:blank".parse()?),
            )
            .build()?;
            let state = app.state::<AppState>();
            let (release, pending) = tokio::sync::oneshot::channel();
            if mode != "readonly" {
                let writer = commands::require_writer(&state).unwrap();
                *state.embedding_initialization.lock().unwrap() =
                    Some(tauri::async_runtime::spawn(async move {
                        pending.await.unwrap();
                        writer
                            .save_article(
                                xaga_core::library_writer::Tier::Interactive,
                                "abcdef123456".into(),
                                "https://example.org/test".into(),
                                "Fixture".into(),
                                None,
                                None,
                                None,
                                None,
                                "pending edit".into(),
                                Default::default(),
                            )
                            .await
                            .unwrap();
                    }));
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if mode == "restart" {
                    std::fs::write(directory.join("predecessor"), "restarting").unwrap();
                    begin_shutdown(&handle, true, false);
                } else if mode == "quit" {
                    handle.exit(0);
                } else {
                    window.close().unwrap();
                }
                if mode != "readonly" {
                    // Repeat native close and Quit while initialization holds the lock.
                    while !handle
                        .state::<AppState>()
                        .shutdown_started
                        .load(std::sync::atomic::Ordering::SeqCst)
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    window.close().unwrap();
                    handle.exit(0);
                    assert!(xaga_core::library::acquire_lock(&directory).is_none());
                    release.send(()).unwrap();
                }
            });
            Ok(())
        })
        .build(app_context())
        .unwrap()
        .run(handle_run_event);
    // Keep a command handle alive even after startup finishes and shutdown drains the writer.
    drop(retained_command_handle);
}
