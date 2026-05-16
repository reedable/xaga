use std::collections::HashSet;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::embeddings::EmbeddingsDb;
use crate::types::{Entry, Item, ItemFilter};

/// Scans `library/entries/` and returns the set of IDs whose frontmatter contains the `unread` tag.
pub async fn unread_ids(library: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(mut dir) = tokio::fs::read_dir(library.join("entries")).await {
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let path = dirent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
            if let Ok(raw) = tokio::fs::read_to_string(&path).await {
                if let Some(fm) = crate::util::parse_frontmatter(&raw) {
                    if fm.tags.contains(&"unread".to_string()) {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            ids.insert(stem.to_string());
                        }
                    }
                }
            }
        }
    }
    ids
}

/// Scans `library/entries/` and returns the set of IDs whose frontmatter contains the `bookmark` tag.
pub async fn bookmarked_ids(library: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(mut dir) = tokio::fs::read_dir(library.join("entries")).await {
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let path = dirent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
            if let Ok(raw) = tokio::fs::read_to_string(&path).await {
                if let Some(fm) = crate::util::parse_frontmatter(&raw) {
                    if fm.tags.contains(&"bookmark".to_string()) {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            ids.insert(stem.to_string());
                        }
                    }
                }
            }
        }
    }
    ids
}

/// Sets `is_unread` on each entry based on the provided unread ID set.
pub fn annotate_unread_status(unread: &HashSet<String>, entries: &mut Vec<Entry>) {
    for entry in entries.iter_mut() {
        entry.is_unread = unread.contains(&entry.id);
    }
}

/// Sets `is_bookmarked` on each entry based on the provided bookmarked ID set.
pub fn annotate_bookmarked_status(bookmarked: &HashSet<String>, entries: &mut Vec<Entry>) {
    for entry in entries.iter_mut() {
        entry.is_bookmarked = bookmarked.contains(&entry.id);
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
            items.retain(|i| i.url().is_some() && i.is_unread());
        }

        ItemFilter::Bookmarked => {
            items.retain(|i| i.is_bookmarked());
        }

        ItemFilter::Saved => {
            let annotated_ids = {
                let Ok(db) = EmbeddingsDb::open(library).await else {
                    return Ok(items.into_iter().filter(|i| i.is_bookmarked()).collect());
                };
                db.list_annotations()
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|a| a.reference)
                    .collect::<HashSet<String>>()
            };
            items.retain(|i| i.is_bookmarked() || !i.reference().is_empty() || annotated_ids.contains(i.id()));
        }
    }

    Ok(items)
}

/// Sort `items` by most-recent timestamp, descending.
pub fn sort_items<T: Item>(items: &mut Vec<T>) {
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
