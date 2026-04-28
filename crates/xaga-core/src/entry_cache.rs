use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, RwLock};

use tokio::sync::broadcast;

use crate::types::Entry;

/// In-memory cache of entry tag state, built at startup and kept current by
/// the file watcher. Cloning is cheap — all clones share the same data.
#[derive(Clone)]
pub struct EntryStateCache {
    unread: Arc<RwLock<HashSet<String>>>,
    bookmarked: Arc<RwLock<HashSet<String>>>,
    unread_tx: Arc<broadcast::Sender<Entry>>,
    bookmarked_tx: Arc<broadcast::Sender<Entry>>,
}

impl EntryStateCache {
    /// Scans `data_dir/entries/` once and returns a fully populated cache.
    pub async fn new(data_dir: &Path) -> Self {
        let mut unread = HashSet::new();
        let mut bookmarked = HashSet::new();

        let entries_dir = data_dir.join("entries");
        if let Ok(mut dir) = tokio::fs::read_dir(&entries_dir).await {
            while let Ok(Some(dirent)) = dir.next_entry().await {
                let path = dirent.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|s| s.to_str().map(str::to_string)) else {
                    continue;
                };
                if let Ok(raw) = tokio::fs::read_to_string(&path).await {
                    if let Some(fm) = crate::util::parse_frontmatter(&raw) {
                        if fm.tags.contains(&"unread".to_string()) {
                            unread.insert(stem.clone());
                        }
                        if fm.tags.contains(&"bookmark".to_string()) {
                            bookmarked.insert(stem);
                        }
                    }
                }
            }
        }

        let (unread_tx, _) = broadcast::channel(256);
        let (bookmarked_tx, _) = broadcast::channel(256);

        Self {
            unread: Arc::new(RwLock::new(unread)),
            bookmarked: Arc::new(RwLock::new(bookmarked)),
            unread_tx: Arc::new(unread_tx),
            bookmarked_tx: Arc::new(bookmarked_tx),
        }
    }

    /// Update the cache for one entry. Broadcasts the entry to subscribers
    /// only when the relevant set actually changes.
    pub fn update(&self, entry: Entry) {
        let unread_changed = {
            let mut set = self.unread.write().unwrap();
            if entry.is_unread { set.insert(entry.id.clone()) } else { set.remove(&entry.id) }
        };
        let bookmarked_changed = {
            let mut set = self.bookmarked.write().unwrap();
            if entry.is_bookmarked { set.insert(entry.id.clone()) } else { set.remove(&entry.id) }
        };

        if unread_changed { let _ = self.unread_tx.send(entry.clone()); }
        if bookmarked_changed { let _ = self.bookmarked_tx.send(entry); }
    }

    /// Remove an entry from all sets (e.g. when archived or deleted).
    /// Does not broadcast — the entry data is no longer available.
    pub fn remove(&self, id: &str) {
        self.unread.write().unwrap().remove(id);
        self.bookmarked.write().unwrap().remove(id);
    }

    pub fn unread_snapshot(&self) -> HashSet<String> {
        self.unread.read().unwrap().clone()
    }

    pub fn bookmarked_snapshot(&self) -> HashSet<String> {
        self.bookmarked.read().unwrap().clone()
    }

    pub fn subscribe_unread(&self) -> broadcast::Receiver<Entry> {
        self.unread_tx.subscribe()
    }

    pub fn subscribe_bookmarked(&self) -> broadcast::Receiver<Entry> {
        self.bookmarked_tx.subscribe()
    }
}
