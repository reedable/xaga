use super::*;

pub(super) fn handle_run_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "main" => {
            api.prevent_close();
            if let Some(w) = app.get_webview_window("main") {
                w.hide().ok();
            }
            begin_shutdown(app, false, false);
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
            begin_shutdown(app, false, true);
        }
        _ => {}
    }
}

/// Spawns a fresh copy of the current executable then exits the current
/// process.  The new process boots against the already-updated settings.json.
///
/// NOTE: this does not work under `tauri dev` because the Vite dev server is
/// managed by the tauri-cli watcher and dies with the binary.  Dev-mode restart
/// will be handled separately; for now callers fall through to a clean exit in
/// that environment.
fn restart_process() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    std::process::exit(0);
}

/// Claims termination once, even after the service has been taken by its owner.
fn claim_shutdown(started: &std::sync::atomic::AtomicBool) -> bool {
    !started.swap(true, std::sync::atomic::Ordering::SeqCst)
}

pub(crate) fn begin_shutdown(app: &tauri::AppHandle, restart: bool, overlay: bool) {
    if !claim_shutdown(&app.state::<AppState>().shutdown_started) {
        return;
    }
    if overlay {
        events::emit_domain(app, xaga_core::events::DomainEvent::EmbeddingShutdown);
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let reason = shutdown_library(&app.state::<AppState>(), restart).await;
        match reason {
            ShutdownReason::Restart => restart_process(),
            ShutdownReason::Exit => std::process::exit(0),
        }
    });
}

async fn shutdown_library(state: &AppState, restart: bool) -> ShutdownReason {
    // Startup can hold a writer/lock before installing the service. Join that
    // initialization before taking the service so no late service escapes shutdown.
    let initialization = state.embedding_initialization.lock().unwrap().take();
    if let Some(initialization) = initialization {
        if state.is_library_read_only() {
            // A read-only startup may be waiting indefinitely for the user's choice.
            initialization.abort();
        }
        let _ = initialization.await;
    }
    let service = state.embedding_service.lock().unwrap().take();
    finish_library_shutdown(&state.library, async move {
        match service {
            Some(service) => {
                if restart {
                    service.request_restart();
                }
                service.shutdown().await
            }
            None if restart => ShutdownReason::Restart,
            None => ShutdownReason::Exit,
        }
    })
    .await
}

async fn finish_library_shutdown(
    library: &Arc<RwLock<LibraryHandle>>,
    service_shutdown: impl std::future::Future<Output = ShutdownReason>,
) -> ShutdownReason {
    let reason = service_shutdown.await;
    drain_and_release_writer(library).await;
    reason
}

/// Drains the library writer (Interactive backlog to disk, Regular dropped — the tier
/// contract) and releases its hold on the library flock. Must run
/// before [`restart_process`] spawns the successor: the successor acquires the lock at ITS
/// startup, and if the (still-living) predecessor's writer held it, the new instance would
/// come up read-only. Caller must have already shut the `EmbeddingService` down (its Arc of
/// the same lock must be gone) so that shutting down the writer releases the flock. No-op in a
/// read-only instance (no writer).
pub(crate) async fn drain_and_release_writer(library: &Arc<RwLock<LibraryHandle>>) {
    let writer = library.write().unwrap().writer.take();
    if let Some(writer) = writer {
        writer.shutdown().await;
    }
}

/// Installs a SIGINT/SIGTERM handler that drains the embedding service before exit.
pub(super) fn install_signal_handler(app_handle: tauri::AppHandle) {
    ctrlc::set_handler(move || {
        let state = app_handle.state::<AppState>();
        if !claim_shutdown(&state.shutdown_started) {
            return;
        }
        events::emit_domain(
            &app_handle,
            xaga_core::events::DomainEvent::EmbeddingShutdown,
        );
        // Signals always exit, irrespective of the service's restart outcome.
        tauri::async_runtime::block_on(shutdown_library(&state, false));
        std::process::exit(0);
    })
    .expect("failed to install signal handler");
}

#[cfg(test)]
mod library_mode_tests {
    use super::*;
    use crate::startup::library_writer_for_startup;

    #[tokio::test]
    async fn close_waits_for_startup_then_drains_without_a_service() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_app_state::test_state(dir.path().to_path_buf());
        let lock = xaga_core::library::acquire_lock(dir.path()).unwrap();
        let writer =
            xaga_core::library_writer::LibraryWriter::start(dir.path().to_path_buf(), lock);
        state.library.write().unwrap().writer = Some(writer.clone());
        let (release, pending) = tokio::sync::oneshot::channel();
        *state.embedding_initialization.lock().unwrap() =
            Some(tauri::async_runtime::spawn(async move {
                pending.await.unwrap();
                writer
                    .save_article(
                        xaga_core::library_writer::Tier::Interactive,
                        "abcdef123456".into(),
                        "https://example.com/test".into(),
                        "Title".into(),
                        None,
                        None,
                        None,
                        None,
                        "saved before close".into(),
                        Default::default(),
                    )
                    .await
                    .unwrap();
            }));
        assert!(claim_shutdown(&state.shutdown_started));
        let closing = shutdown_library(&state, false);
        tokio::pin!(closing);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut closing)
                .await
                .is_err()
        );
        // Close and Quit cannot take over a termination already in progress.
        assert!(!claim_shutdown(&state.shutdown_started));
        assert!(!claim_shutdown(&state.shutdown_started));
        assert!(xaga_core::library::acquire_lock(dir.path()).is_none());
        release.send(()).unwrap();
        assert!(matches!(closing.await, ShutdownReason::Exit));
        assert!(
            std::fs::read_to_string(xaga_core::library::entry_path(dir.path(), "abcdef123456"))
                .unwrap()
                .contains("saved before close")
        );
        assert!(xaga_core::library::acquire_lock(dir.path()).is_some());
    }

    #[tokio::test]
    async fn close_preserves_service_outcome_and_releases_all_holders() {
        for reason in [ShutdownReason::Exit, ShutdownReason::Restart] {
            let dir = tempfile::tempdir().unwrap();
            let library = handle_for(dir.path());
            let hold = library.read().unwrap().writer.as_ref().unwrap().lock();
            let restart = matches!(reason, ShutdownReason::Restart);
            let path = dir.path().to_path_buf();
            let outcome = finish_library_shutdown(&library, async move {
                assert!(xaga_core::library::acquire_lock(&path).is_none());
                drop(hold);
                reason
            })
            .await;
            assert_eq!(matches!(outcome, ShutdownReason::Restart), restart);
            assert!(xaga_core::library::acquire_lock(dir.path()).is_some());
        }
    }

    #[tokio::test]
    async fn readonly_close_cancels_startup_choice_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_app_state::test_state(dir.path().to_path_buf());
        *state.embedding_initialization.lock().unwrap() =
            Some(tauri::async_runtime::spawn(std::future::pending()));
        assert!(matches!(
            shutdown_library(&state, false).await,
            ShutdownReason::Exit
        ));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        assert!(state.library.read().unwrap().writer.is_none());
    }

    fn seed_version(dir: &std::path::Path, version: &str) {
        let lock = xaga_core::library::acquire_lock(dir).unwrap();
        xaga_core::library::LibraryManifest::begin_upgrade(dir, version.parse().unwrap(), &lock)
            .unwrap()
            .unwrap()
            .complete()
            .unwrap();
    }

    fn handle_for(dir: &std::path::Path) -> Arc<RwLock<LibraryHandle>> {
        let (writer, _) = library_writer_for_startup(dir, "0.10".parse().unwrap());
        Arc::new(RwLock::new(LibraryHandle {
            path: dir.to_path_buf(),
            writer,
        }))
    }

    /// Read-write vs read-only is derived from one fact: whether a writer
    /// exists. Winning the flock starts a writer; losing does not.
    #[test]
    fn mode_derives_from_writer_presence() {
        let dir = tempfile::tempdir().unwrap();
        seed_version(dir.path(), "0.10");

        let writer = handle_for(dir.path());
        assert!(
            writer.read().unwrap().writer.is_some(),
            "first instance wins the lock and starts a writer"
        );

        // A second "instance" (second handle in this process: flock conflicts across
        // open file descriptions, so this models a contender) loses and is read-only.
        let reader = handle_for(dir.path());
        assert!(
            reader.read().unwrap().writer.is_none(),
            "contender must lose while held"
        );
    }

    #[test]
    fn empty_library_starts_a_writer() {
        let dir = tempfile::tempdir().unwrap();
        let (writer, error) = library_writer_for_startup(dir.path(), "0.10".parse().unwrap());
        assert!(error.is_none());
        assert!(writer.is_some());
        assert!(
            xaga_core::library::LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .matches_version("0.10".parse().unwrap())
        );
    }

    #[test]
    fn mismatched_and_unversioned_libraries_have_no_writer() {
        let unversioned = tempfile::tempdir().unwrap();
        std::fs::write(unversioned.path().join("existing.md"), "existing content").unwrap();
        assert!(
            handle_for(unversioned.path())
                .read()
                .unwrap()
                .writer
                .is_none()
        );
        assert_eq!(std::fs::read_dir(unversioned.path()).unwrap().count(), 1);
        let older = tempfile::tempdir().unwrap();
        seed_version(older.path(), "0.9");
        let before = std::fs::read(older.path().join("metadata/library.yaml")).unwrap();
        assert!(handle_for(older.path()).read().unwrap().writer.is_none());
        assert_eq!(
            std::fs::read(older.path().join("metadata/library.yaml")).unwrap(),
            before
        );
        assert!(!xaga_core::library::index_path(older.path()).exists());
    }

    #[test]
    fn malformed_metadata_opens_readonly_and_is_returned_for_reporting() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("library.yaml"), "format_version: [broken").unwrap();
        let (writer, error) = library_writer_for_startup(dir.path(), "0.10".parse().unwrap());
        assert!(writer.is_none());
        assert!(error.is_some());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("library.yaml")).unwrap(),
            "format_version: [broken"
        );
        assert!(!dir.path().join("xaga.lock").exists());
    }

    /// The restart-ordering invariant: draining and dropping the writer frees the flock
    /// for a successor process — what the restart path must guarantee before
    /// `restart_process` spawns one. Also covers the service's shared hold: while the
    /// EmbeddingService's Arc of the lock lives, the flock stays held.
    #[tokio::test]
    async fn drain_and_release_frees_the_lock_only_after_every_holder_drops() {
        let dir = tempfile::tempdir().unwrap();
        seed_version(dir.path(), "0.10");

        let library = handle_for(dir.path());
        // The service's clone of the lock (EmbeddingService::start receives one).
        let service_hold = library.read().unwrap().writer.as_ref().unwrap().lock();

        drain_and_release_writer(&library).await;
        assert!(
            library.read().unwrap().writer.is_none(),
            "writer removed (mode is now read-only)"
        );
        assert!(
            xaga_core::library::acquire_lock(dir.path()).is_none(),
            "flock must remain held while the service still holds its Arc"
        );

        drop(service_hold);
        assert!(
            xaga_core::library::acquire_lock(dir.path()).is_some(),
            "successor must win the lock once every holder is gone"
        );
    }
}

#[cfg(test)]
mod retained_command_lock_probe {
    use super::*;

    #[tokio::test]
    async fn draining_releases_lock_with_a_retained_command_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_app_state::test_state(dir.path().to_path_buf());
        state.library.write().unwrap().writer =
            Some(xaga_core::library_writer::LibraryWriter::start(
                dir.path().to_path_buf(),
                xaga_core::library::acquire_lock(dir.path()).unwrap(),
            ));
        let command_handle = crate::commands::require_writer(&state).unwrap();
        shutdown_library(&state, false).await;
        assert!(matches!(
            command_handle
                .save_pdf("https://example.com/late.pdf".into(), None)
                .await,
            Err(xaga_core::library_writer::WriterError::ShuttingDown)
        ));
        assert!(
            xaga_core::library::acquire_lock(dir.path()).is_some(),
            "a successor must acquire the lock after shutdown, even while a command retains its refused writer handle"
        );
        drop(command_handle);
    }
}
