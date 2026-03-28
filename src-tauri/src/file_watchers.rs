use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_full::new_debouncer;
use notify_debouncer_full::notify::{EventKind, RecursiveMode, Watcher};
use xaga_core::embeddings::{BackgroundMsg, PriorityMsg};

/// Spawns one watcher thread per directory.
///
/// Routing:
/// - import/   create → parse frontmatter → move to unread/{hash}.md or spill/
/// - unread/   create → background: Embed
/// - unread/   modify → priority:   Embed
/// - topics/   create → priority:   Embed
/// - topics/   modify → priority:   Embed
/// - topics/   delete → background: Delete
/// - archived/ delete → background: Delete
pub fn watch(
    data_dir: PathBuf,
    priority_tx: flume::Sender<PriorityMsg>,
    background_tx: flume::Sender<BackgroundMsg>,
) {
    watch_import(data_dir.clone());
    watch_unread(
        data_dir.join("unread"),
        priority_tx.clone(),
        background_tx.clone(),
    );
    watch_topics(data_dir.join("topics"), priority_tx, background_tx.clone());
    watch_archived(data_dir.join("archived"), background_tx);
}

fn watch_unread(
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
                                log::debug!("file_watchers: unread create → background embed {id}");
                                let _ = background_tx.send(BackgroundMsg::Embed(id.to_string()));
                            }
                            EventKind::Modify(_) => {
                                log::debug!("file_watchers: unread modify → priority embed {id}");
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
                log::error!("file_watchers: unread debouncer init failed: {e}");
                return;
            }
        };

        if let Err(e) = debouncer.watcher().watch(&dir, RecursiveMode::NonRecursive) {
            log::error!("file_watchers: unread watch failed: {e}");
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

fn watch_import(data_dir: PathBuf) {
    let import_dir = data_dir.join("import");
    let unread_dir = data_dir.join("unread");
    let spill_dir = data_dir.join("spill");

    std::thread::spawn(move || {
        for dir in [&import_dir, &spill_dir] {
            if let Err(e) = std::fs::create_dir_all(dir) {
                log::error!("file_watchers: cannot create {}: {e}", dir.display());
                return;
            }
        }

        let mut debouncer = match new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: notify_debouncer_full::DebounceEventResult| {
                let events = match result {
                    Ok(e) => e,
                    Err(errs) => {
                        for e in errs {
                            log::error!("file_watchers: import watcher error: {e}");
                        }
                        return;
                    }
                };

                for event in events {
                    log::debug!("file_watchers: import event {:?} {:?}", event.kind, event.paths);

                    if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        continue;
                    }

                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }

                        process_import(path, &import_dir, &unread_dir, &spill_dir);
                    }
                }
            },
        ) {
            Ok(d) => d,
            Err(e) => {
                log::error!("file_watchers: import debouncer init failed: {e}");
                return;
            }
        };

        let watch_path = data_dir.join("import");
        log::info!("file_watchers: watching import dir: {}", watch_path.display());

        if let Err(e) = debouncer
            .watcher()
            .watch(&watch_path, RecursiveMode::Recursive)
        {
            log::error!("file_watchers: import watch failed: {e}");
            return;
        }

        log::info!("file_watchers: import watcher ready");

        loop {
            std::thread::park();
        }
    });
}

fn process_import(
    path: &std::path::Path,
    import_dir: &std::path::Path,
    unread_dir: &std::path::Path,
    spill_dir: &std::path::Path,
) {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("file_watchers: import read {}: {e}", path.display());
            move_to_spill(path, import_dir, spill_dir);
            return;
        }
    };

    let fm = match xaga_core::util::parse_frontmatter(&raw) {
        None => {
            log::warn!(
                "file_watchers: import invalid frontmatter in {}",
                path.display()
            );
            move_to_spill(path, import_dir, spill_dir);
            return;
        }
        Some(fm) if fm.source.is_empty() => {
            log::warn!(
                "file_watchers: import missing 'source:' field in {}",
                path.display()
            );
            move_to_spill(path, import_dir, spill_dir);
            return;
        }
        Some(fm) => fm,
    };

    let normalized = match xaga_core::util::normalize_url(&fm.source) {
        Ok(n) => n,
        Err(e) => {
            log::warn!("file_watchers: import bad url in {}: {e}", path.display());
            move_to_spill(path, import_dir, spill_dir);
            return;
        }
    };

    let hash = xaga_core::util::url_hash12(&normalized);
    let dest = unread_dir.join(format!("{hash}.md"));

    if dest.exists() {
        log::warn!(
            "file_watchers: import collision {hash} ({}), routing to spill",
            path.display()
        );
        move_to_spill(path, import_dir, spill_dir);
        return;
    }

    if let Err(e) = std::fs::rename(path, &dest) {
        log::error!(
            "file_watchers: import rename {} → unread/{hash}.md: {e}",
            path.display()
        );
        move_to_spill(path, import_dir, spill_dir);
    } else {
        log::debug!(
            "file_watchers: import {} → unread/{hash}.md",
            path.display()
        );
    }
}

fn move_to_spill(
    path: &std::path::Path,
    import_dir: &std::path::Path,
    spill_dir: &std::path::Path,
) {
    let rel = match path.strip_prefix(import_dir) {
        Ok(r) => r,
        Err(_) => {
            log::error!(
                "file_watchers: import spill: {} is not under import dir",
                path.display()
            );
            return;
        }
    };

    let dest = spill_dir.join(rel);

    if let Some(parent) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::error!(
                "file_watchers: import spill mkdir {}: {e}",
                parent.display()
            );
            return;
        }
    }

    if let Err(e) = std::fs::rename(path, &dest) {
        log::error!(
            "file_watchers: import spill rename {} → {}: {e}",
            path.display(),
            dest.display()
        );
    } else {
        log::debug!(
            "file_watchers: import spill {} → {}",
            path.display(),
            dest.display()
        );
    }
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
