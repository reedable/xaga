//! Feeds persistence: the `.atom` boundary for feeds and their entry membership.
//! Collapsed from the former `feeds/{mod,feed,feed_entry}.rs` split into one module.

use std::collections::HashSet;
use std::path::Path;

use thiserror::Error;

use crate::{
    atom::{
        Atom, AtomEntry, apply_incoming_xml, atom_to_entry, atom_to_feed, atom_to_xml, entry_ids,
        feed_to_atom, parse_atom, self_link_url, set_pinned, set_title,
    },
    types::{Article, Feed},
    util::{normalize_url, url_hash12},
};

use super::walk::for_each_atom;

// ── Shared error + result ────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("HTTP error fetching {url}: {source}")]
    Http {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Atom parse error: {0}")]
    AtomParse(String),
    #[error("feed parse error: {0}")]
    FeedParse(String),
    #[error("feed not found: {0}")]
    NotFound(String),
    #[error("feed missing self link: {0}")]
    MissingSelfLink(String),
}

/// FIXME Untangle FeedError for least visibility
pub(crate) type Result<T> = std::result::Result<T, FeedError>;

// ── Public API ───────────────────────────────────────────────────────────────

/// Derive a deterministic ID from a feed URL.
/// Pure/sync — no I/O.
pub fn url_to_feed_id(url: &str) -> Result<String> {
    let normalized = normalize_url(url).map_err(FeedError::InvalidUrl)?;
    Ok(url_hash12(&normalized))
}

/// Save an `.md` for each entry that doesn't already exist on disk, returning the Articles
/// created. The shared ingestion step behind `add_feed`, `refresh_feed`, and recovery's
/// `reconcile_atom_file`. This function is a noop if the file already exists.
async fn save_new_atom_entries(library_dir: &Path, atom_entries: &[AtomEntry]) -> Vec<Article> {
    let mut created: Vec<Article> = Vec::new();

    for atom_entry in atom_entries {
        // Convert the storage AtomEntry to the domain Entry via the serialization layer, at the
        // point the domain fields are actually needed to write the `.md`.
        let entry = atom_to_entry(atom_entry);

        if entry.url.is_empty() {
            continue;
        }

        // Only create an Entry if one doesn't already exist on disk.
        if crate::persistence::entries::find_entry_path(library_dir, &entry.id)
            .await
            .is_some()
        {
            continue;
        }

        match crate::persistence::entries::write_entry(
            library_dir,
            &entry.url,
            &entry.title,
            entry.published_at.as_deref(),
            entry.updated_at.as_deref(),
            entry.description.as_deref(),
        )
        .await
        {
            Ok(content) => created.push(content),
            Err(err) => log::warn!("save_new_entries: write_entry {}: {err}", entry.id),
        }
    }

    created
}

/// Add a feed by URL. For an already-present feed, just refreshes it. For a new feed, builds the
/// entry-less domain [`Feed`], serializes it via `feed_to_atom`, fetches and merges in memory, then
/// commits once. If either the fetch/merge or the commit fails, the seeded (entry-less) `.atom` is
/// still written so the feed exists and can be refreshed later, and the original error is propagated.
/// Idempotent: re-adding an existing feed just refreshes it.
pub async fn add_feed(
    client: &reqwest::Client,
    library_dir: &Path,
    url: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<(Feed, Vec<Article>)> {
    let url = if url.trim_start().starts_with("http://") {
        format!("https://{}", url.trim_start()[7..].trim_start())
    } else {
        url.to_string()
    };

    let url = normalize_url(&url).map_err(FeedError::InvalidUrl)?;
    let id = url_to_feed_id(&url)?;

    tokio::fs::create_dir_all(crate::library::feeds_dir(library_dir)).await?;

    let path = crate::library::feed_path(library_dir, &id);

    if path.exists() {
        return refresh_feed(client, library_dir, &id, unread, bookmarked).await;
    }

    // New feed: serialize the entry-less domain Feed via atom.rs. Keep this seed so that, on any
    // later failure (fetch/merge or commit), a valid `.atom` is still persisted — the feed exists
    // and is refreshable, and the merge work just retries on the next refresh. The seed is the
    // entry-less atom, which serializes even when a merged entry would not.
    let empty_atom = seed_atom(id.clone(), url.clone());

    let mut atom = empty_atom.clone();

    let new_entries = match fetch_and_merge(client, &mut atom, &id).await {
        Ok(entries) => entries,
        Err(e) => {
            write_atom_atomic(library_dir, &id, &empty_atom).await?;
            return Err(e);
        }
    };

    match write_atom(library_dir, &id, &atom, &new_entries).await {
        Ok(created) => Ok((atom_to_feed(&id, &atom, unread, bookmarked), created)),
        Err(e) => {
            write_atom_atomic(library_dir, &id, &empty_atom).await?;
            Err(e)
        }
    }
}

fn seed_atom(id: String, url: String) -> Atom {
    feed_to_atom(&Feed {
        id,
        url,
        title: String::new(),
        created_at: String::new(),
        updated_at: None,
        description: None,
        is_pinned: false,
        entry_count: 0,
        unread_count: 0,
        members: vec![],
    })
}

/// Re-fetch a feed by ID: read and parse the `.atom`, fetch + merge in memory, then commit.
/// Returns the summary and the Articles created, so the caller can enqueue them for indexing.
/// Plain ingestion — persistence only.
async fn refresh_feed(
    client: &reqwest::Client,
    library_dir: &Path,
    id: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<(Feed, Vec<Article>)> {
    let path = crate::library::feed_path(library_dir, id);

    if !path.exists() {
        return Err(FeedError::NotFound(id.to_string()));
    }

    let mut existing =
        parse_atom(&tokio::fs::read_to_string(&path).await?).map_err(FeedError::AtomParse)?;

    let new_entries = fetch_and_merge(client, &mut existing, id).await?;
    let created = write_atom(library_dir, id, &existing, &new_entries).await?;

    Ok((atom_to_feed(id, &existing, unread, bookmarked), created))
}

/// Fetch a feed's live XML from its self-link and merge new entries into `existing` in memory.
/// No writes — the caller owns persistence (`commit_feed`). Returns the newly-added entries.
async fn fetch_and_merge(
    client: &reqwest::Client,
    atom: &mut Atom,
    id: &str,
) -> Result<Vec<AtomEntry>> {
    let url = self_link_url(atom).ok_or_else(|| FeedError::MissingSelfLink(id.to_string()))?;
    let incoming_xml = fetch_xml(client, &url).await?;
    apply_incoming_xml(atom, &incoming_xml, id).map_err(FeedError::FeedParse)
}

/// Commit a refreshed feed: persist each new entry's `.md` BEFORE writing the `.atom`. The `.md`
/// is the irreplaceable content; the atom is only the membership label. A crash between the two
/// leaves materialized entries that aren't yet in the atom — benign and self-healing: the next
/// refresh re-adds them from the live feed (and `save_new_entries` skips the already-written `.md`).
/// The reverse order would instead lose the content, forcing a reconciliation pass to rebuild it
/// from the atom. This ordering is the one place that owns the invariant; callers must commit here.
async fn write_atom(
    library_dir: &Path,
    id: &str,
    atom: &Atom,
    new_atom_entries: &[AtomEntry],
) -> Result<Vec<Article>> {
    let created = save_new_atom_entries(library_dir, new_atom_entries).await;
    write_atom_atomic(library_dir, id, atom).await?;
    Ok(created)
}

/// Re-fetch all feeds. Per-feed errors are collected rather than propagated early.
pub async fn refresh_all_feeds(
    client: &reqwest::Client,
    library_dir: &Path,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Vec<(String, Result<(Feed, Vec<Article>)>)> {
    // Walk + parse each `.atom` once; the parsed feed flows straight into the refresh, so we
    // never re-read the file it was just walked from.
    let mut feeds: Vec<(String, Atom)> = Vec::new();

    for_each_atom(
        &crate::library::feeds_dir(library_dir),
        &parse_atom,
        |path, feed| {
            feeds.push((crate::util::item_id(&path), feed));
        },
    )
    .await;

    let mut results = Vec::with_capacity(feeds.len());

    for (id, mut existing) in feeds {
        let result: Result<(Feed, Vec<Article>)> = async {
            let new_entries = fetch_and_merge(client, &mut existing, &id).await?;
            let created = write_atom(library_dir, &id, &existing, &new_entries).await?;
            Ok((atom_to_feed(&id, &existing, unread, bookmarked), created))
        }
        .await;
        results.push((id, result));
    }

    results
}

/// List all feeds on disk, parsed from their Atom XML.
pub async fn list_feeds(
    library_dir: &Path,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Vec<Feed>> {
    let mut feeds: Vec<Feed> = Vec::new();

    for_each_atom(
        &crate::library::feeds_dir(library_dir),
        &parse_atom,
        |path, atom_feed| {
            let id = crate::util::item_id(&path);
            feeds.push(atom_to_feed(&id, &atom_feed, unread, bookmarked));
        },
    )
    .await;

    // for_each_atom yields in directory order; sort by id for a stable listing.
    feeds.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(feeds)
}

/// Adds (or removes) the app-local `pinned` category on a feed's `.atom`, under
/// [`TAG_CATEGORY_SCHEME`]. Idempotent. Returns whether the file changed.
pub async fn set_feed_pinned(library_dir: &Path, id: &str, pinned: bool) -> Result<bool> {
    let path = crate::library::feed_path(library_dir, id);
    let xml = tokio::fs::read_to_string(&path).await?;
    let mut atom = parse_atom(&xml).map_err(FeedError::AtomParse)?;

    if !set_pinned(&mut atom, pinned) {
        return Ok(false);
    }

    write_atom_atomic(library_dir, id, &atom).await?;
    Ok(true)
}

/// Rename a feed by updating its `<title>` in the stored Atom file.
pub async fn rename_feed(
    library_dir: &Path,
    id: &str,
    new_title: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Feed> {
    let path = crate::library::feed_path(library_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.to_string()));
    }
    let xml = tokio::fs::read_to_string(&path).await?;
    let mut feed = parse_atom(&xml).map_err(FeedError::AtomParse)?;
    set_title(&mut feed, new_title);
    write_atom_atomic(library_dir, id, &feed).await?;
    Ok(atom_to_feed(id, &feed, unread, bookmarked))
}

/// Remove a feed (deletes `{id}.atom`).
pub async fn remove_feed(library_dir: &Path, id: &str) -> Result<()> {
    let path = crate::library::feed_path(library_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.to_string()));
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

/// Returns the membership id-set for `url` (one feed) or all feeds (`None`),
/// deduplicated. The `.atom` is the source of truth for feed membership; entry data
/// and state are fetched from LanceDB by the caller (see the `list_entries` command).
pub async fn list_entry_ids(library_dir: &Path, url: Option<&str>) -> Result<Vec<String>> {
    let feeds_dir = crate::library::feeds_dir(library_dir);
    let mut ids = Vec::new();
    let mut seen = HashSet::new();

    match url {
        Some(url) => {
            let normalized = normalize_url(url)?;
            let id = url_to_feed_id(&normalized)?;
            let path = crate::library::feed_path(library_dir, &id);

            if !tokio::fs::try_exists(&path).await? {
                return Err(FeedError::NotFound(id));
            }

            let xml = tokio::fs::read_to_string(&path).await?;
            let feed = parse_atom(&xml).map_err(FeedError::AtomParse)?;
            for id in entry_ids(&feed) {
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
        None => {
            for_each_atom(&feeds_dir, &parse_atom, |_path, feed| {
                for id in entry_ids(&feed) {
                    if seen.insert(id.clone()) {
                        ids.push(id);
                    }
                }
            })
            .await;
        }
    }

    Ok(ids)
}

// ── Shared infrastructure (private) ──────────────────────────────────────────

async fn fetch_xml(client: &reqwest::Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| FeedError::Http {
            url: url.to_string(),
            source: e,
        })?
        .error_for_status()
        .map_err(|e| FeedError::Http {
            url: url.to_string(),
            source: e,
        })?
        .text()
        .await
        .map_err(|e| FeedError::Http {
            url: url.to_string(),
            source: e,
        })
}

async fn write_atom_atomic(library_dir: &Path, id: &str, atom: &Atom) -> Result<()> {
    let buf = atom_to_xml(atom).map_err(FeedError::AtomParse)?;
    crate::fs::write_atomic_async(crate::library::feed_path(library_dir, id), buf).await?;
    Ok(())
}
