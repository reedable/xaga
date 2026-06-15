use std::collections::HashSet;
use std::path::Path;

use atom_syndication as atom;

use crate::types::Entry;
use crate::util::{normalize_url, url_hash12};

use super::{FeedError, Result, list_feed_ids, parse_atom, xml_path};

// ── Public API ───────────────────────────────────────────────────────────────

/// Returns the membership id-set for `url` (one feed) or all feeds (`None`),
/// deduplicated. The `.atom` is the source of truth for feed membership; entry data
/// and state are fetched from LanceDB by the caller (see the `list_entries` command).
pub async fn list_entry_ids(library: &Path, url: Option<&str>) -> Result<Vec<String>> {
    let feeds_dir = library.join("feeds");
    let mut ids = Vec::new();
    let mut seen = HashSet::new();

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
            for e in feed.entries() {
                let id = atom_entry_to_entry(e).id;
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
        None => {
            let feed_ids = list_feed_ids(&feeds_dir).await?;
            for fid in feed_ids {
                if let Ok(xml) = tokio::fs::read_to_string(xml_path(&feeds_dir, &fid)).await {
                    if let Ok(feed) = parse_atom(&xml) {
                        for e in feed.entries() {
                            let id = atom_entry_to_entry(e).id;
                            if seen.insert(id.clone()) {
                                ids.push(id);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ids)
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
        created_at: updated_at.clone(),
        published_at,
        updated_at: Some(updated_at),
        summary,
        is_unread: false,
        is_bookmarked: false,
        tags: vec![],
    }
}
