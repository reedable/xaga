use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_full::new_debouncer;
use notify_debouncer_full::notify::{EventKind, RecursiveMode, Watcher};
use xaga_core::embeddings::{EntityRef, Job, JobSender, Tier};
use xaga_core::types::Kind;

/// Spawns one watcher thread per directory. Interim producer: the watcher has no
/// intent, so it emits generic jobs (`UpsertContent`/`Delete`) and lets the consumer
/// figure out what changed. Tiers mirror the pre-refactor routing. (Retired in a later
/// task once the command layer is the sole live producer.)
///
/// Routing:
/// - entries/  create → Maintenance: UpsertContent
/// - entries/  modify → Interactive: UpsertContent
/// - topics/   create → Interactive: UpsertContent
/// - topics/   modify → Interactive: UpsertContent
/// - topics/   delete → Maintenance: Delete
/// - archived/ delete → Maintenance: Delete
pub fn watch(data_dir: PathBuf, jobs: JobSender) {
    watch_entries(data_dir.join("entries"), jobs.clone());
    watch_topics(data_dir.join("topics"), jobs.clone());
    watch_archived(data_dir.join("archived"), jobs);
}

fn watch_entries(dir: PathBuf, jobs: JobSender) {
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
                                log::debug!("file_watchers: entries create → upsert (maintenance) {id}");
                                jobs.enqueue(
                                    Tier::Maintenance,
                                    Job::UpsertContent { entity: EntityRef::new(id, Kind::Entry) },
                                );
                            }
                            EventKind::Modify(_) if path.exists() => {
                                log::debug!("file_watchers: entries modify → upsert (interactive) {id}");
                                jobs.enqueue(
                                    Tier::Interactive,
                                    Job::UpsertContent { entity: EntityRef::new(id, Kind::Entry) },
                                );
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

fn watch_topics(dir: PathBuf, jobs: JobSender) {
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
                                    "file_watchers: topics create/modify → upsert (interactive) {id}"
                                );
                                jobs.enqueue(
                                    Tier::Interactive,
                                    Job::UpsertContent { entity: EntityRef::new(id, Kind::Topic) },
                                );
                            }
                            EventKind::Remove(_) => {
                                log::debug!(
                                    "file_watchers: topics delete → delete (maintenance) {id}"
                                );
                                jobs.enqueue(
                                    Tier::Maintenance,
                                    Job::Delete { entity: EntityRef::new(id, Kind::Topic) },
                                );
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

fn watch_archived(dir: PathBuf, jobs: JobSender) {
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
                        log::debug!("file_watchers: archived delete → delete (maintenance) {id}");
                        jobs.enqueue(
                            Tier::Maintenance,
                            Job::Delete { entity: EntityRef::new(id, Kind::Entry) },
                        );
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
