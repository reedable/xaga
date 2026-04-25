use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, RwLock};

use tokio::sync::Notify;

/// Tracks which entry IDs are currently in the `unread/` directory.
///
/// Initialized once at startup via a full directory scan, then kept current
/// by the file watcher calling `add` and `remove` for individual creates and
/// removes. All clones share the same underlying data.
#[derive(Clone)]
pub struct UnreadTracker {
    ids: Arc<RwLock<HashSet<String>>>,
    notify: Arc<Notify>,
}

impl UnreadTracker {
    /// Scans `data_dir/unread/` and returns a fully populated tracker.
    pub async fn new(data_dir: &Path) -> Self {
        let ids = scan_unread_dir(data_dir).await;
        Self {
            ids: Arc::new(RwLock::new(ids)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Records that an entry has become unread and notifies subscribers.
    pub fn add(&self, id: &str) {
        self.ids.write().unwrap().insert(id.to_string());
        self.notify.notify_one();
    }

    /// Records that an entry is no longer unread and notifies subscribers.
    pub fn remove(&self, id: &str) {
        self.ids.write().unwrap().remove(id);
        self.notify.notify_one();
    }

    /// Returns a point-in-time snapshot of the current unread ID set.
    pub fn snapshot(&self) -> HashSet<String> {
        self.ids.read().unwrap().clone()
    }

    /// Returns the shared `Notify` so callers can `await` change signals.
    pub fn subscribe(&self) -> Arc<Notify> {
        Arc::clone(&self.notify)
    }
}

async fn scan_unread_dir(data_dir: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    let dir = data_dir.join("unread");
    if let Ok(mut read_dir) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.insert(stem.to_string());
            }
        }
    }
    ids
}
