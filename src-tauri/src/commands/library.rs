use std::path::PathBuf;

use tauri::{AppHandle, State};
use xaga_core::embeddings::ShutdownReason;

use crate::error::AppError;
use crate::{AppState, SettingsOutcome, restart_process, stale_view_error, update_settings};

#[tauri::command]
pub async fn get_library(state: State<'_, AppState>) -> Result<String, AppError> {
    Ok(state.library_path().to_string_lossy().to_string())
}

/// Whether this instance is read-only for the open library (another xaga process holds
/// the library lock). Same name as the `AppState` accessor and the `library:mode-changed`
/// event field — one concept, one name at every layer.
#[tauri::command]
pub async fn is_library_read_only(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(state.is_library_read_only())
}

#[tauri::command]
pub async fn set_library(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, AppError> {
    let new_path = PathBuf::from(&path);
    {
        let mut operation = state.library_upgrade.lock().unwrap();
        if matches!(
            *operation,
            UpgradeState::Running { .. } | UpgradeState::Succeeded { .. }
        ) {
            return Err(
                "Finish the library upgrade and restart before switching libraries."
                    .to_string()
                    .into(),
            );
        }
        std::fs::create_dir_all(&new_path).map_err(|e| e.to_string())?;
        match update_settings(&state, |s| {
            s.library = Some(new_path.to_string_lossy().to_string())
        })? {
            SettingsOutcome::StaleView(fresh) => return Err(stale_view_error(&state, &fresh)),
            SettingsOutcome::Written(_) => {}
        }
        // Serialize switching with upgrade submission through the restart.
        *operation = UpgradeState::Running { progress: None };
    }

    // The new library path is now persisted.  Signal the embedding service to
    // restart, wait for the run loop to finish its current in-flight LanceDB
    // write and exit cleanly, then spawn a fresh process which boots against
    // the updated settings.json.  We do not update state.library here — the
    // new process will read it from disk on startup.
    if let Some(svc) = state.embedding_service.lock().unwrap().take() {
        svc.request_restart();
        crate::events::emit_domain(&app, xaga_core::events::DomainEvent::EmbeddingShutdown);
        let library = std::sync::Arc::clone(&state.library);
        tauri::async_runtime::spawn(async move {
            let reason = svc.shutdown().await;
            // Drain and drop the writer (Interactive backlog to disk) to release the
            // (old) library's flock before the successor tries to acquire the new one.
            crate::drain_and_release_writer(&library).await;
            match reason {
                ShutdownReason::Restart => restart_process(),
                ShutdownReason::Exit => std::process::exit(0),
            }
        });
    } else {
        // Read-only instance: no service to drain (and no writer to release). Restart
        // directly so the successor attempts the NEW library's lock at its startup — a
        // read-only instance may become the writer of a different library. (Without this,
        // a read-only instance would keep running against the old library path.)
        crate::drain_and_release_writer(&state.library).await;
        restart_process();
    }

    Ok(new_path.to_string_lossy().to_string())
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub(crate) enum UpgradeState {
    #[default]
    Idle,
    Running {
        progress: Option<xaga_core::library::UpgradeProgress>,
    },
    Failed {
        message: String,
        errors: Vec<String>,
        read_error: Option<String>,
    },
    Succeeded {
        report: xaga_core::library::UpgradeReport,
        restarting: bool,
        read_error: Option<String>,
    },
}
fn application_version(app: &AppHandle) -> xaga_core::library::LibraryVersion {
    let version = &app.package_info().version;
    xaga_core::library::LibraryVersion::new(version.major, version.minor)
}
#[tauri::command]
pub async fn get_library_status(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<xaga_core::library::LibraryStatus, AppError> {
    let library = state.library.read().unwrap();
    Ok(xaga_core::library::LibraryStatus::read(
        &library.path,
        application_version(&app),
        library.writer.is_some(),
    ))
}
#[tauri::command]
pub async fn get_library_upgrade(state: State<'_, AppState>) -> Result<UpgradeState, AppError> {
    Ok(state.library_upgrade.lock().unwrap().clone())
}

/// Only an explicit read-only choice releases model startup. Notify retains a
/// permit if the command arrives before the startup task begins waiting.
#[tauri::command]
pub async fn continue_library_read_only(state: State<'_, AppState>) -> Result<(), AppError> {
    let operation = state.library_upgrade.lock().unwrap();
    if matches!(*operation, UpgradeState::Running { .. }) {
        return Err(AppError::ReadOnly);
    }
    state.read_only_startup.notify_one();
    Ok(())
}

/// This explicit upgrade is a separate write boundary: ordinary write commands
/// remain refused because this process has no runtime writer. Reserve before
/// spawning so duplicate IPC submissions cannot queue a second conversion.
#[tauri::command]
pub async fn start_library_upgrade(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<UpgradeState, AppError> {
    let mut operation = state.library_upgrade.lock().unwrap();
    let selected = state.library.read().unwrap();
    let target = application_version(&app);
    if let Err(snapshot) = reserve_upgrade(
        &mut operation,
        &selected,
        std::path::Path::new(&path),
        target,
    ) {
        return Ok(snapshot);
    }
    let dir = selected.path.clone();
    let snapshot = operation.clone();
    let operation = std::sync::Arc::clone(&state.library_upgrade);
    let reader = std::sync::Arc::clone(&state.embeddings_read);
    tauri::async_runtime::spawn_blocking(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tauri::async_runtime::block_on(perform_upgrade(&dir, target, &reader, |progress| {
                *operation.lock().unwrap() = UpgradeState::Running {
                    progress: Some(progress),
                };
            }))
        }));
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(_) => UpgradeState::Failed {
                message: "The library upgrade was interrupted. Keep the library read-only and retry the upgrade.".into(),
                errors: vec![],
                read_error: tauri::async_runtime::block_on(restore_reader(&dir, &reader)),
            },
        };
        *operation.lock().unwrap() = outcome;
    });
    Ok(snapshot)
}

#[tauri::command]
pub async fn restart_after_library_upgrade(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<String>, AppError> {
    let result = restart_upgraded_library(
        &state,
        application_version(&app),
        || {
            let path = state.library_path().to_string_lossy().into_owned();
            match update_settings(&state, |settings| settings.library = Some(path)) {
                Ok(SettingsOutcome::Written(_)) => Ok(()),
                Ok(SettingsOutcome::StaleView(_)) => Err("The library upgrade completed, but settings changed in another Xaga process. Retry Restart Xaga to reopen this library.".into()),
                Err(error) => {
                    log::error!("upgrade restart settings: {error}");
                    Err("The library upgrade completed, but its reopening path could not be saved. Close other Xaga processes and retry Restart Xaga.".into())
                }
            }
        },
        || std::env::current_exe().and_then(|exe| std::process::Command::new(exe).spawn()).map(|_| ()),
    ).await;
    match result {
        Ok(true) => std::process::exit(0),
        Ok(false) => Ok(None), // A simultaneous restart already owns this attempt.
        Err(message) => Ok(Some(message)),
    }
}

async fn restart_upgraded_library(
    state: &AppState,
    target: xaga_core::library::LibraryVersion,
    save: impl FnOnce() -> Result<(), String>,
    spawn: impl FnOnce() -> std::io::Result<()>,
) -> Result<bool, String> {
    {
        let mut operation = state.library_upgrade.lock().unwrap();
        match &mut *operation {
            UpgradeState::Succeeded { restarting, .. } if !*restarting => *restarting = true,
            UpgradeState::Succeeded { .. } => return Ok(false),
            _ => return Err("Complete the library upgrade before restarting.".into()),
        }
    }
    let _restart_guard = RestartGuard(std::sync::Arc::clone(&state.library_upgrade));
    if !xaga_core::library::LibraryManifest::read(&state.library_path())
        .ok()
        .flatten()
        .is_some_and(|m| m.matches_version(target))
    {
        return Err("The library metadata changed. Close Xaga and reopen this library to check compatibility.".into());
    }
    // Keep the restored reader alive throughout settings and successor failures.
    save()?;
    crate::drain_and_release_writer(&state.library).await;
    spawn().map_err(|error| {
        log::error!("upgrade restart failed: {error}");
        "The library upgrade completed, but Xaga could not restart. Try again, or close Xaga and reopen this library.".to_owned()
    })?;
    Ok(true)
}

fn reserve_upgrade(
    operation: &mut UpgradeState,
    selected: &crate::LibraryHandle,
    expected_path: &std::path::Path,
    target: xaga_core::library::LibraryVersion,
) -> Result<(), UpgradeState> {
    if matches!(
        operation,
        UpgradeState::Running { .. } | UpgradeState::Succeeded { .. }
    ) {
        return Err(operation.clone());
    }
    if selected.path != expected_path || selected.writer.is_some() {
        return Err(UpgradeState::Failed { message: "The selected library changed or already has write access. Reopen its library options.".into(), errors: vec![], read_error: None });
    }
    if !xaga_core::library::LibraryStatus::read(&selected.path, target, false).can_upgrade() {
        return Err(UpgradeState::Failed { message: "This library cannot be upgraded with this Xaga version. Reopen its library options to check its status.".into(), errors: vec![], read_error: None });
    }
    *operation = UpgradeState::Running { progress: None };
    Ok(())
}
async fn perform_upgrade(
    dir: &std::path::Path,
    target: xaga_core::library::LibraryVersion,
    reader: &std::sync::Mutex<Option<xaga_core::embeddings::EmbeddingsDb>>,
    progress: impl Fn(xaga_core::library::UpgradeProgress) + Send + Sync,
) -> UpgradeState {
    use xaga_core::library;
    let Some(lock) = library::acquire_lock(dir) else {
        return UpgradeState::Failed { message: "Could not acquire the library write lock. Close other processes using this library and retry.".into(), errors: vec![], read_error: None };
    };
    if !library::LibraryStatus::read(dir, target, false).can_upgrade() {
        return UpgradeState::Failed { message: "The library version changed before the upgrade started. Reopen its library options.".into(), errors: vec![], read_error: None };
    }
    reader.lock().unwrap().take();
    let report = library::upgrade_library(dir, &lock, target, progress).await;
    let read_error = restore_reader(dir, reader).await;
    if report.errors.is_empty() {
        UpgradeState::Succeeded {
            report,
            restarting: false,
            read_error,
        }
    } else {
        UpgradeState::Failed { message: "Library upgrade incomplete. Changes may already be applied. Resolve the reported problems and retry, or restore your complete backup.".into(), errors: report.errors, read_error }
    }
}

/// Reopen independently of the model: counts and memberships do not require it.
async fn restore_reader(
    dir: &std::path::Path,
    reader: &std::sync::Mutex<Option<xaga_core::embeddings::EmbeddingsDb>>,
) -> Option<String> {
    match xaga_core::embeddings::EmbeddingsDb::open_readonly(dir).await {
        Ok(db) => {
            *reader.lock().unwrap() = Some(db);
            None
        }
        Err(error) => {
            log::error!("upgrade read-only recovery: {error}");
            Some("Counts, saved-state indicators, and Topic memberships are unavailable in this window. Restart Xaga to retry loading them.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use xaga_core::library::{self, LibraryManifest, LibraryVersion};
    fn fixture() -> (tempfile::TempDir, crate::LibraryHandle) {
        let dir = tempfile::tempdir().unwrap();
        let _lock = library::open(dir.path(), LibraryVersion::new(0, 15))
            .unwrap()
            .unwrap();
        let selected = crate::LibraryHandle {
            path: dir.path().into(),
            writer: None,
        };
        (dir, selected)
    }
    #[tokio::test]
    async fn recovery_preserves_reads_after_conversion_and_restart_failures() {
        let (dir, _) = fixture();
        let state = crate::test_app_state::test_state(dir.path().into());
        let target = LibraryVersion::new(0, 16);
        let lock = library::acquire_lock(dir.path()).unwrap();
        let db = xaga_core::embeddings::EmbeddingsDb::open_writer(dir.path(), &lock)
            .await
            .unwrap();
        *state.embeddings_read.lock().unwrap() = Some(db);
        drop(lock);
        std::fs::write(library::pins_path(dir.path()), "bad: [").unwrap();
        let failed = perform_upgrade(dir.path(), target, &state.embeddings_read, |_| {}).await;
        assert!(matches!(
            failed,
            UpgradeState::Failed {
                read_error: None,
                ..
            }
        ));
        *state.library_upgrade.lock().unwrap() = failed;
        assert!(state.is_library_read_only());
        assert!(crate::get_unread_ids(&state).await.is_ok());
        std::fs::remove_file(library::pins_path(dir.path())).unwrap();
        let success = perform_upgrade(dir.path(), target, &state.embeddings_read, |_| {}).await;
        assert!(matches!(
            success,
            UpgradeState::Succeeded {
                read_error: None,
                ..
            }
        ));
        *state.library_upgrade.lock().unwrap() = success;
        let spawned = std::cell::Cell::new(false);
        let saved_failure = restart_upgraded_library(
            &state,
            target,
            || Err("fixture settings failure".into()),
            || {
                spawned.set(true);
                Ok(())
            },
        )
        .await;
        assert!(saved_failure.is_err());
        assert!(!spawned.get());
        assert!(matches!(
            *state.library_upgrade.lock().unwrap(),
            UpgradeState::Succeeded {
                restarting: false,
                ..
            }
        ));
        let spawn_failure = restart_upgraded_library(
            &state,
            target,
            || Ok(()),
            || Err(std::io::Error::other("fixture spawn failure")),
        )
        .await;
        assert!(spawn_failure.is_err());
        assert!(state.is_library_read_only());
        let db = state.embeddings_read.lock().unwrap().clone().unwrap();
        assert!(db.all_entities_counts().await.is_ok());
        assert!(db.find_topic_ids_by_entity_id("fixture").await.is_ok());
        assert!(
            restart_upgraded_library(&state, target, || Ok(()), || Ok(()))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn failed_reader_recovery_is_explicit_and_retry_restores_it() {
        let (dir, _) = fixture();
        let state = crate::test_app_state::test_state(dir.path().into());
        let target = LibraryVersion::new(0, 16);
        std::fs::write(library::index_path(dir.path()), "fixture corrupt cache").unwrap();
        std::fs::write(library::pins_path(dir.path()), "bad: [").unwrap();
        let failed = perform_upgrade(dir.path(), target, &state.embeddings_read, |_| {}).await;
        assert!(matches!(
            failed,
            UpgradeState::Failed {
                read_error: Some(_),
                ..
            }
        ));
        *state.library_upgrade.lock().unwrap() = failed;
        assert!(matches!(
            crate::get_unread_ids(&state).await,
            Err(AppError::LibraryReadUnavailable)
        ));
        assert!(matches!(
            crate::get_bookmarked_ids(&state).await,
            Err(AppError::LibraryReadUnavailable)
        ));
        assert!(crate::ensure_library_reads_available(&state).is_err());
        std::fs::remove_file(library::index_path(dir.path())).unwrap();
        std::fs::remove_file(library::pins_path(dir.path())).unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        drop(
            xaga_core::embeddings::EmbeddingsDb::open_writer(dir.path(), &lock)
                .await
                .unwrap(),
        );
        drop(lock);
        let retry = perform_upgrade(dir.path(), target, &state.embeddings_read, |_| {}).await;
        assert!(matches!(
            retry,
            UpgradeState::Succeeded {
                read_error: None,
                ..
            }
        ));
        *state.library_upgrade.lock().unwrap() = retry;
        assert!(crate::get_unread_ids(&state).await.is_ok());
    }

    // Invoked by the process test below, with only a disposable settings path.
    #[tokio::test]
    async fn successor_startup_child() {
        let Some(settings_path) = std::env::var_os("XAGA_UPGRADE_TEST_SETTINGS") else {
            return;
        };
        let settings: crate::UserSettings =
            serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();
        let path = PathBuf::from(settings.library.unwrap());
        let (writer, error) = crate::library_writer_for_startup(&path, LibraryVersion::new(0, 16));
        assert!(error.is_none());
        let result = serde_json::json!({"path": path, "writable": writer.is_some()});
        std::fs::write(
            std::env::var_os("XAGA_UPGRADE_TEST_RESULT").unwrap(),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        if let Some(writer) = writer {
            writer.shutdown().await;
        }
    }

    #[tokio::test]
    async fn successor_process_uses_saved_library_and_normal_lock_ownership() {
        let (dir, _) = fixture();
        let state = crate::test_app_state::test_state(dir.path().into());
        let target = LibraryVersion::new(0, 16);
        *state.library_upgrade.lock().unwrap() =
            perform_upgrade(dir.path(), target, &state.embeddings_read, |_| {}).await;
        let account = tempfile::tempdir().unwrap();
        let settings_path = account.path().join("settings.json");
        let result_path = account.path().join("successor.json");
        let mut baseline = None;
        for contended in [false, true] {
            let contender = contended.then(|| library::acquire_lock(dir.path()).unwrap());
            assert!(
                restart_upgraded_library(
                    &state,
                    target,
                    || {
                        let (outcome, fresh) = crate::update_settings_file(
                            &state.account_lock,
                            &settings_path,
                            baseline,
                            |s| s.library = Some(dir.path().to_string_lossy().into_owned()),
                        )
                        .map_err(|e| e.to_string())?;
                        baseline = fresh;
                        assert!(matches!(outcome, SettingsOutcome::Written(_)));
                        Ok(())
                    },
                    || {
                        let status = std::process::Command::new(std::env::current_exe()?)
                            .args([
                                "--exact",
                                "commands::library::tests::successor_startup_child",
                                "--nocapture",
                            ])
                            .env("XAGA_UPGRADE_TEST_SETTINGS", &settings_path)
                            .env("XAGA_UPGRADE_TEST_RESULT", &result_path)
                            .status()?;
                        assert!(status.success());
                        Ok(())
                    }
                )
                .await
                .unwrap()
            );
            let result: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap();
            assert_eq!(result["path"], dir.path().to_string_lossy().as_ref());
            assert_eq!(result["writable"], !contended);
            assert!(state.is_library_read_only());
            drop(contender);
        }
    }

    #[tokio::test]
    async fn app_submission_rechecks_path_lock_and_manifest_and_prevents_duplicates() {
        let (dir, selected) = fixture();
        let target = LibraryVersion::new(0, 16);
        let mut operation = UpgradeState::Idle;
        assert!(
            reserve_upgrade(&mut operation, &selected, &dir.path().join("other"), target).is_err()
        );
        assert!(matches!(operation, UpgradeState::Idle));
        assert!(reserve_upgrade(&mut operation, &selected, dir.path(), target).is_ok());
        assert!(matches!(
            reserve_upgrade(&mut operation, &selected, dir.path(), target),
            Err(UpgradeState::Running { .. })
        ));
        let lock = library::acquire_lock(dir.path()).unwrap();
        let reader = Mutex::new(None);
        assert!(matches!(
            perform_upgrade(dir.path(), target, &reader, |_| {}).await,
            UpgradeState::Failed { .. }
        ));
        assert_eq!(
            LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .migration_target(),
            None
        );
        drop(lock);
        // An external converter changed the target after submission.
        let lock = library::acquire_lock(dir.path()).unwrap();
        drop(
            LibraryManifest::begin_upgrade(dir.path(), LibraryVersion::new(0, 17), &lock).unwrap(),
        );
        drop(lock);
        assert!(matches!(
            perform_upgrade(dir.path(), target, &reader, |_| {}).await,
            UpgradeState::Failed { .. }
        ));
        assert_eq!(
            LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .migration_target(),
            Some(LibraryVersion::new(0, 17))
        );
    }
    #[tokio::test]
    async fn app_failure_retry_and_cli_share_results_and_reopen_through_normal_lock_checks() {
        let (app_dir, _) = fixture();
        let (cli_dir, _) = fixture();
        let target = LibraryVersion::new(0, 16);
        let reader = Mutex::new(None);
        std::fs::write(library::pins_path(app_dir.path()), "bad: [").unwrap();
        assert!(matches!(
            perform_upgrade(app_dir.path(), target, &reader, |_| {}).await,
            UpgradeState::Failed { .. }
        ));
        assert!(library::open(app_dir.path(), target).unwrap().is_none());
        std::fs::remove_file(library::pins_path(app_dir.path())).unwrap();
        let phases = Mutex::new(Vec::new());
        let result = perform_upgrade(app_dir.path(), target, &reader, |p| {
            phases.lock().unwrap().push(p.phase)
        })
        .await;
        let UpgradeState::Succeeded { report, .. } = result else {
            panic!("app retry failed")
        };
        assert!(!phases.lock().unwrap().is_empty());
        let lock = library::acquire_lock(cli_dir.path()).unwrap();
        let cli_report = library::upgrade_library(cli_dir.path(), &lock, target, |_| {}).await;
        assert_eq!(
            serde_json::to_value(report).unwrap(),
            serde_json::to_value(cli_report).unwrap()
        );
        assert_eq!(
            std::fs::read(app_dir.path().join("library.yaml")).unwrap(),
            std::fs::read(cli_dir.path().join("library.yaml")).unwrap()
        );
        // Migration completion cannot bypass another writer on normal reopen.
        let other_writer = library::acquire_lock(app_dir.path()).unwrap();
        assert!(library::open(app_dir.path(), target).unwrap().is_none());
        drop(other_writer);
        assert!(library::open(app_dir.path(), target).unwrap().is_some());
    }
}

// A failed restart can be retried; simultaneous requests spawn only one successor.
struct RestartGuard(std::sync::Arc<std::sync::Mutex<UpgradeState>>);
impl Drop for RestartGuard {
    fn drop(&mut self) {
        if let UpgradeState::Succeeded { restarting, .. } = &mut *self.0.lock().unwrap() {
            *restarting = false;
        }
    }
}
