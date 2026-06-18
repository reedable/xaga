use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_full::new_debouncer;
use notify_debouncer_full::notify::{EventKind, RecursiveMode, Watcher};
use xaga_core::embeddings::{BackgroundMsg, PriorityMsg};

/// Spawns one watcher thread per directory.
///
/// Routing:
/// - entries/  create → background: Embed
/// - entries/  modify → priority:   Embed
/// - topics/   create → priority:   Embed
/// - topics/   modify → priority:   Embed
/// - topics/   delete → background: Delete
/// - archived/ delete → background: Delete
pub fn watch(
    data_dir: PathBuf,
    priority_tx: flume::Sender<PriorityMsg>,
    background_tx: flume::Sender<BackgroundMsg>,
) {
    watch_entries(
        data_dir.join("entries"),
        priority_tx.clone(),
        background_tx.clone(),
    );
    watch_topics(data_dir.join("topics"), priority_tx, background_tx.clone());
    watch_archived(data_dir.join("archived"), background_tx);
}

fn watch_entries(
    dir: PathBuf,
    priority_tx: flume::Sender<PriorityMsg>,
    background_tx: flume::Sender<BackgroundMsg>,
) {
    std::thread::spawn(move || {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("file_watchers: cannot create {}: {e}", dir.display());
            return;
        }

        let mut debouncer = match new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: notify_debouncer_full::DebounceEventResult| {
                let Ok(events) = result else { return };

                for event in events {
                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }
                        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                            continue;
                        };
                        match event.kind {
                            EventKind::Create(_) => {
                                log::debug!("file_watchers: entries create → background embed {id}");
                                let _ = background_tx.send(BackgroundMsg::Embed(id.to_string()));
                            }
                            EventKind::Modify(_) if path.exists() => {
                                log::debug!("file_watchers: entries modify → priority embed {id}");
                                let _ = priority_tx.send(PriorityMsg::Embed(id.to_string()));
                            }
                            _ => {}
                        }
                    }
                }
            },
        ) {
            Ok(d) => d,
            Err(e) => {
                log::error!("file_watchers: entries debouncer init failed: {e}");
                return;
            }
        };

        if let Err(e) = debouncer.watcher().watch(&dir, RecursiveMode::NonRecursive) {
            log::error!("file_watchers: entries watch failed: {e}");
            return;
        }

        std::thread::park();
    });
}

fn watch_topics(
    dir: PathBuf,
    priority_tx: flume::Sender<PriorityMsg>,
    background_tx: flume::Sender<BackgroundMsg>,
) {
    std::thread::spawn(move || {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("file_watchers: cannot create {}: {e}", dir.display());
            return;
        }

        let mut debouncer = match new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: notify_debouncer_full::DebounceEventResult| {
                let Ok(events) = result else { return };

                for event in events {
                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }
                        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                            continue;
                        };
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                log::debug!(
                                    "file_watchers: topics create/modify → priority embed {id}"
                                );
                                let _ = priority_tx.send(PriorityMsg::Embed(id.to_string()));
                            }
                            EventKind::Remove(_) => {
                                log::debug!(
                                    "file_watchers: topics delete → background delete {id}"
                                );
                                let _ = background_tx.send(BackgroundMsg::Delete(id.to_string()));
                            }
                            _ => {}
                        }
                    }
                }
            },
        ) {
            Ok(d) => d,
            Err(e) => {
                log::error!("file_watchers: topics debouncer init failed: {e}");
                return;
            }
        };

        if let Err(e) = debouncer.watcher().watch(&dir, RecursiveMode::NonRecursive) {
            log::error!("file_watchers: topics watch failed: {e}");
            return;
        }

        std::thread::park();
    });
}

fn watch_archived(dir: PathBuf, background_tx: flume::Sender<BackgroundMsg>) {
    std::thread::spawn(move || {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("file_watchers: cannot create {}: {e}", dir.display());
            return;
        }

        let mut debouncer = match new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: notify_debouncer_full::DebounceEventResult| {
                let Ok(events) = result else { return };

                for event in events {
                    if !matches!(event.kind, EventKind::Remove(_)) {
                        continue;
                    }
                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }
                        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                            continue;
                        };
                        log::debug!("file_watchers: archived delete → background delete {id}");
                        let _ = background_tx.send(BackgroundMsg::Delete(id.to_string()));
                    }
                }
            },
        ) {
            Ok(d) => d,
            Err(e) => {
                log::error!("file_watchers: archived debouncer init failed: {e}");
                return;
            }
        };

        if let Err(e) = debouncer.watcher().watch(&dir, RecursiveMode::NonRecursive) {
            log::error!("file_watchers: archived watch failed: {e}");
            return;
        }

        std::thread::park();
    });
}
