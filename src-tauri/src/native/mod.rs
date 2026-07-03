//! Native-only orchestration: operations reachable from the app shell (menu),
//! never exposed on the webview invoke surface.
//!
//! These take ambient authority (a filesystem path) that must not come from the
//! renderer. Each `*_dialog` helper opens the file dialog on the Rust side and
//! calls the corresponding orchestration function directly, so there is no
//! `#[tauri::command]` for a compromised webview to invoke. Completion is
//! reported to the frontend via an event carrying the pre-refactor payload
//! (`{ result?, error? }`): the UI refreshes on success and shows the error on
//! failure, exactly as when the JS layer drove the call.
//!
//! See docs/architecture/security/command_surface.md (why) and
//! docs/architecture/layer_ownership.md (why the orchestration stays here).

pub(crate) mod bookmarks;
pub(crate) mod feeds;

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::AppState;

/// Event payload for a native-triggered import/export.
#[derive(Serialize, Clone)]
struct Outcome<T: Serialize + Clone> {
    result: Option<T>,
    error: Option<String>,
}

/// Convert the dialog's picked file to a `PathBuf`, or `None` (cancelled / bad
/// path) — in which case no work runs and no event fires.
fn picked_path(picked: Option<tauri_plugin_dialog::FilePath>) -> Option<PathBuf> {
    match picked?.into_path() {
        Ok(p) => Some(p),
        Err(e) => {
            log::warn!("dialog returned an unusable path: {e}");
            None
        }
    }
}

pub(crate) fn import_feeds_dialog(app: &AppHandle) {
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter("OPML", &["opml", "xml"])
        .pick_file(move |picked| {
            let Some(path) = picked_path(picked) else {
                return;
            };
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let outcome = match feeds::import_feeds(state.inner(), &path).await {
                    Ok(result) => Outcome {
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => {
                        log::warn!("import_feeds: {e}");
                        Outcome {
                            result: None,
                            error: Some(e),
                        }
                    }
                };
                app.emit("feeds:imported", outcome).ok();
            });
        });
}

pub(crate) fn export_feeds_dialog(app: &AppHandle) {
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter("OPML", &["opml"])
        .set_file_name("feeds.opml")
        .save_file(move |picked| {
            let Some(path) = picked_path(picked) else {
                return;
            };
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let error = feeds::export_feeds(state.inner(), &path).await.err();
                if let Some(ref e) = error {
                    log::warn!("export_feeds: {e}");
                }
                app.emit("feeds:exported", Outcome::<()> { result: None, error })
                    .ok();
            });
        });
}

pub(crate) fn import_bookmarks_dialog(app: &AppHandle) {
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter("Bookmarks", &["html", "htm"])
        .pick_file(move |picked| {
            let Some(path) = picked_path(picked) else {
                return;
            };
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let outcome = match bookmarks::import_bookmarks(state.inner(), &path).await {
                    Ok(result) => Outcome {
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => {
                        log::warn!("import_bookmarks: {e}");
                        Outcome {
                            result: None,
                            error: Some(e),
                        }
                    }
                };
                app.emit("bookmarks:imported", outcome).ok();
            });
        });
}

pub(crate) fn export_bookmarks_dialog(app: &AppHandle) {
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter("Bookmarks", &["html"])
        .set_file_name("bookmarks.html")
        .save_file(move |picked| {
            let Some(path) = picked_path(picked) else {
                return;
            };
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let error = bookmarks::export_bookmarks(state.inner(), &path).await.err();
                if let Some(ref e) = error {
                    log::warn!("export_bookmarks: {e}");
                }
                app.emit("bookmarks:exported", Outcome::<()> { result: None, error })
                    .ok();
            });
        });
}
