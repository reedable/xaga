use std::collections::HashSet;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::embeddings::EmbeddingsDb;
use crate::types::{EmbeddedEntry, Entry, Item, ItemFilter};
use crate::util::normalize_url;

async fn unread_ids(library: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(mut dir) = tokio::fs::read_dir(library.join("unread")).await {
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let path = dirent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.insert(stem.to_string());
            }
        }
    }
    ids
}

/// Reads the `unread/` directory and sets `is_unread` on each entry.
pub async fn annotate_unread_status(library: &Path, entries: &mut Vec<Entry>) {
    let ids = unread_ids(library).await;
    for entry in entries.iter_mut() {
        entry.is_unread = ids.contains(&entry.id);
    }
}

/// Same as `annotate_unread_status` but for `EmbeddedEntry`.
pub async fn annotate_embedded_unread_status(library: &Path, entries: &mut Vec<EmbeddedEntry>) {
    let ids = unread_ids(library).await;
    for embedded in entries.iter_mut() {
        embedded.entry.is_unread = ids.contains(&embedded.entry.id);
    }
}

/// Filter `items` according to `item_filter` — predicate only, no sorting.
///
/// Works on any `T` that implements `Item`. Items without a `source` are
/// treated as authored (already read, not bookmarkable).
pub async fn filter_items<T: Item>(
    library: &Path,
    mut items: Vec<T>,
    item_filter: &ItemFilter,
) -> std::io::Result<Vec<T>> {
    match item_filter {
        ItemFilter::All => {}

        ItemFilter::Unread => {
            let ids = unread_ids(library).await;
            items.retain(|i| i.url().is_some() && ids.contains(i.id()));
        }

        ItemFilter::Bookmarked => {
            let bookmark_sources: HashSet<String> = crate::bookmarks::list_bookmarks(library)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.url)
                .collect();

            items.retain(|i| match i.url() {
                None => false,
                Some(src) => {
                    let normalized = normalize_url(src).unwrap_or_else(|_| src.to_string());
                    bookmark_sources.contains(&normalized)
                }
            });
        }

        ItemFilter::Saved => {
            let (bookmark_sources, annotated_ids) = tokio::join!(
                async {
                    crate::bookmarks::list_bookmarks(library)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .map(|e| e.url)
                        .collect::<HashSet<String>>()
                },
                async {
                    let Ok(db) = EmbeddingsDb::open(library).await else {
                        return HashSet::new();
                    };
                    db.list_annotations()
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .map(|a| a.reference)
                        .collect::<HashSet<String>>()
                },
            );

            items.retain(|i| {
                if annotated_ids.contains(i.id()) {
                    return true;
                }
                match i.url() {
                    None => false,
                    Some(src) => {
                        let normalized = normalize_url(src).unwrap_or_else(|_| src.to_string());
                        bookmark_sources.contains(&normalized)
                    }
                }
            });
        }
    }

    Ok(items)
}

/// Sort `items` by most-recent timestamp, descending.
pub fn sort_items<T: Item>(items: &mut Vec<T>, _item_filter: &ItemFilter) {
    items.sort_by(|a, b| timestamp(b).cmp(&timestamp(a)));
}

fn timestamp<T: Item>(item: &T) -> DateTime<Utc> {
    parse_ts(item.updated_at())
        .or_else(|| parse_ts(item.published_at()))
        .or_else(|| parse_ts(Some(item.created_at())))
        .unwrap_or(DateTime::UNIX_EPOCH)
}

/// Parse an RFC-3339 string, returning `None` for unparseable or epoch values.
fn parse_ts(s: Option<&str>) -> Option<DateTime<Utc>> {
    let dt = DateTime::parse_from_rfc3339(s?).ok()?.to_utc();
    if dt.timestamp() <= 0 { None } else { Some(dt) }
}
