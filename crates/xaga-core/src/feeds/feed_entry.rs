use std::collections::HashSet;
use std::path::Path;

use atom_syndication as atom;

use crate::types::{Entry, ItemFilter};
use crate::util::{normalize_url, url_hash12};

use super::{FeedError, Result, list_feed_ids, parse_atom, xml_path};

// ── Public API ───────────────────────────────────────────────────────────────

/// Return entries filtered and sorted according to `item_filter`.
///
/// If `url` is `Some`, restricts to that feed; if `None`, returns entries
/// across every feed in `library/feeds/`.
pub async fn list_entries(
    library: &Path,
    url: Option<&str>,
    item_filter: ItemFilter,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Vec<Entry>> {
    let feeds_dir = library.join("feeds");
    let mut entries = Vec::new();

    match url {
        Some(url) => {
            let normalized = normalize_url(url)?;
            let id = super::feed::feed_id_from_url(&normalized)?;
            let path = xml_path(&feeds_dir, &id);

            if !tokio::fs::try_exists(&path).await? {
                return Err(FeedError::NotFound(id));
            }

            let xml = tokio::fs::read_to_string(&path).await?;
            let feed = parse_atom(&xml)?;
            entries.extend(feed.entries().iter().map(atom_entry_to_entry));
        }
        None => {
            let ids = list_feed_ids(&feeds_dir).await?;
            let mut seen = std::collections::HashSet::new();
            for id in ids {
                if let Ok(xml) = tokio::fs::read_to_string(xml_path(&feeds_dir, &id)).await {
                    if let Ok(feed) = parse_atom(&xml) {
                        for entry in feed.entries().iter().map(atom_entry_to_entry) {
                            if seen.insert(entry.id.clone()) {
                                entries.push(entry);
                            }
                        }
                    }
                }
            }
        }
    }

    crate::filter::annotate_unread_status(unread, &mut entries);
    crate::filter::annotate_bookmarked_status(bookmarked, &mut entries);
    let mut items = crate::filter::filter_items(entries, &item_filter)
        .await
        .map_err(FeedError::Io)?;
    crate::filter::sort_items(&mut items);
    Ok(items)
}

/// Read a single `.atom` file and return its entries.
pub(crate) async fn entries_from_path(path: &Path) -> Result<Vec<Entry>> {
    let xml = tokio::fs::read_to_string(path).await?;
    let feed = parse_atom(&xml)?;
    let entries = feed.entries().iter().map(atom_entry_to_entry).collect();
    Ok(entries)
}

pub(super) fn atom_entry_to_entry(e: &atom::Entry) -> Entry {
    let title = e.title().value.clone();
    let updated_at = e.updated().to_rfc3339();
    let url = e
        .links()
        .iter()
        .find(|l| l.rel() == "alternate")
        .or_else(|| e.links().first())
        .map(|l| l.href().to_string())
        .unwrap_or_default();
    let summary = e.summary().map(|s| s.value.clone());
    let published_at = e.published().map(|d| d.to_rfc3339());
    let normalized_url = normalize_url(&url).unwrap_or_else(|_| url.clone());
    Entry {
        id: url_hash12(&normalized_url),
        url,
        title,
        // FIXME: created_at should come from the .md Entry on disk (set when the Entry was first
        // created by reconcile_atom_file), not from the atom <updated> date. list_entries
        // currently reads only from the atom XML and does not merge with the filesystem.
        created_at: updated_at.clone(),
        published_at,
        updated_at: Some(updated_at),
        summary,
        is_unread: false,
        is_bookmarked: false,
        tags: vec![],
    }
}
