//! Feeds persistence: the `.atom` boundary for feeds and their entry membership.
//! Collapsed from the former `feeds/{mod,feed,feed_entry}.rs` split into one module.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use atom_syndication as atom;
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::{Article, Entry, Feed};
use crate::util::{feed_entry_id, normalize_url, url_hash12};

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
    NotFound(FeedId),
    #[error("feed missing self link: {0}")]
    MissingSelfLink(FeedId),
}

pub type Result<T> = std::result::Result<T, FeedError>;

// ── Shared infrastructure (private) ──────────────────────────────────────────

fn xml_path(feeds_dir: &Path, id: &FeedId) -> PathBuf {
    feeds_dir.join(format!("{id}.atom"))
}

async fn write_atom_atomic(feeds_dir: &Path, id: &FeedId, feed: &atom::Feed) -> Result<()> {
    let mut buf = Vec::new();
    feed.write_to(&mut buf)
        .map_err(|e| FeedError::AtomParse(e.to_string()))?;
    crate::fs::write_atomic_async(xml_path(feeds_dir, id), buf).await?;
    Ok(())
}

async fn list_feed_ids(feeds_dir: &Path) -> Result<Vec<FeedId>> {
    if !feeds_dir.exists() {
        return Ok(vec![]);
    }
    let mut entries = tokio::fs::read_dir(feeds_dir).await?;
    let mut ids = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = path.to_string_lossy();
        if name.ends_with(".atom") && !name.ends_with(".atom.tmp") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(FeedId(stem.to_string()));
            }
        }
    }
    ids.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(ids)
}

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

fn parse_incoming(xml: &str) -> Result<feed_rs::model::Feed> {
    let sanitized = fix_bare_ampersands(xml);
    feed_rs::parser::parse(sanitized.as_bytes()).map_err(|e| FeedError::FeedParse(e.to_string()))
}

/// Escape bare `&` characters in XML that are not part of a valid entity or character reference.
/// Skips `<![CDATA[...]]>` sections so their content is never modified.
/// This makes lenient parsing of feeds from publishers who forget to encode `&` in URLs.
fn fix_bare_ampersands(xml: &str) -> String {
    let mut result = String::with_capacity(xml.len() + 64);
    let mut rest = xml;

    // Fast path: no CDATA sections — avoids rescanning the whole string for
    // "<![CDATA[" on every `&`, which is O(n×k) for files with many entities.
    if !xml.contains("<![CDATA[") {
        loop {
            match rest.find('&') {
                None => {
                    result.push_str(rest);
                    break;
                }
                Some(a) => {
                    result.push_str(&rest[..a]);
                    let after = &rest[a + 1..];
                    let is_valid_ref = after.starts_with("amp;")
                        || after.starts_with("lt;")
                        || after.starts_with("gt;")
                        || after.starts_with("apos;")
                        || after.starts_with("quot;")
                        || after.starts_with('#');
                    if is_valid_ref {
                        result.push('&');
                    } else {
                        result.push_str("&amp;");
                    }
                    rest = after;
                }
            }
        }
        return result;
    }

    // CDATA-aware path for feeds that embed content in CDATA sections.
    loop {
        let amp = rest.find('&');
        let cdata = rest.find("<![CDATA[");

        match (amp, cdata) {
            (None, _) => {
                result.push_str(rest);
                break;
            }
            (Some(a), Some(c)) if c < a => {
                // CDATA section starts before the next `&` — copy it through verbatim.
                result.push_str(&rest[..c]);
                rest = &rest[c..];
                if let Some(end) = rest.find("]]>") {
                    result.push_str(&rest[..end + 3]);
                    rest = &rest[end + 3..];
                } else {
                    // Unclosed CDATA; copy remainder as-is.
                    result.push_str(rest);
                    break;
                }
            }
            (Some(a), _) => {
                result.push_str(&rest[..a]);
                let after = &rest[a + 1..];
                let is_valid_ref = after.starts_with("amp;")
                    || after.starts_with("lt;")
                    || after.starts_with("gt;")
                    || after.starts_with("apos;")
                    || after.starts_with("quot;")
                    || after.starts_with('#');
                if is_valid_ref {
                    result.push('&');
                } else {
                    result.push_str("&amp;");
                }
                rest = after;
            }
        }
    }

    result
}

fn parse_atom(xml: &str) -> Result<atom::Feed> {
    let sanitized = fix_bare_ampersands(xml);
    sanitized
        .parse::<atom::Feed>()
        .map_err(|e| FeedError::AtomParse(e.to_string()))
}

fn to_fixed(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

// ── Types ────────────────────────────────────────────────────────────────────

/// Stable, filesystem-safe feed identifier.
/// SHA-256 of the normalized URL (trailing slash stripped), first 12 hex characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeedId(pub String);

impl FeedId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FeedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for FeedId {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(FeedId(s.to_string()))
    }
}

/// What happened to a feed file during `repair_feeds`.
#[derive(Debug)]
pub enum RepairOutcome {
    /// File is already consistent; no changes made.
    Ok,
    /// File could not be read or parsed as Atom XML.
    ParseError(String),
    /// Feed has no `<id>` element and no `<link rel="self">` to derive one from.
    NoId,
    /// Feed has no `<link rel="self">` and `<id>` is not an HTTP(S) URL.
    NoRepairableUrl,
    /// The correctly-named target file already exists; rename skipped.
    RenameConflict { target: FeedId },
    /// One or more repairs were applied.
    Repaired {
        /// Feed `<id>` was synthesised from `<link rel="self">` and written to the file.
        feed_id_synthesized: bool,
        /// `<link rel="self">` was synthesised from `<id>` and written to the file.
        self_link_added: bool,
        /// The file was renamed to match the hash of the feed URL.
        renamed_to: Option<FeedId>,
        /// Entry link hrefs were normalised (trailing-slash stripping, lower-casing).
        entry_links_normalized: bool,
        /// Missing `<entry><id>` values were synthesised from the entry's link href.
        entry_ids_synthesized: bool,
    },
}

/// The result of inspecting (and possibly repairing) a single feed file.
#[derive(Debug)]
pub struct RepairResult {
    pub id: FeedId,
    pub outcome: RepairOutcome,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Derive a deterministic ID from a feed URL.
/// Pure/sync — no I/O.
pub fn feed_id_from_url(url: &str) -> Result<FeedId> {
    let normalized = normalize_url(url).map_err(FeedError::InvalidUrl)?;
    Ok(FeedId(url_hash12(&normalized)))
}

/// Save an `.md` for each entry that doesn't already exist on disk, returning the Articles
/// created. The shared ingestion step behind `add_feed`, `refresh_feed`, and recovery's
/// `reconcile_atom_file`.
pub(crate) async fn save_new_entries(data_dir: &Path, entries: &[Entry]) -> Vec<Article> {
    let mut created: Vec<Article> = Vec::new();
    for e in entries {
        if e.url.is_empty() {
            continue;
        }
        // Only create an Entry if one doesn't already exist on disk.
        if crate::persistence::entries::find_entry_path(data_dir, &e.id)
            .await
            .is_some()
        {
            continue;
        }
        match crate::persistence::entries::write_entry(
            data_dir,
            &e.url,
            &e.title,
            e.published_at.as_deref(),
            e.updated_at.as_deref(),
            e.description.as_deref(),
        )
        .await
        {
            Ok(content) => created.push(content),
            Err(err) => log::warn!("save_new_entries: write_entry {}: {err}", e.id),
        }
    }
    created
}

/// Add a feed by URL: write a new `.atom` recording the source URL (if one isn't already present),
/// then refresh it. `add_feed` does not fetch — `refresh_feed` owns the fetch/merge/save.
/// Idempotent: re-adding an existing feed just refreshes it.
pub async fn add_feed(
    client: &reqwest::Client,
    feeds_dir: &Path,
    data_dir: &Path,
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
    let id = feed_id_from_url(&url)?;

    tokio::fs::create_dir_all(feeds_dir).await?;
    let path = xml_path(feeds_dir, &id);
    if !path.exists() {
        write_atom_atomic(feeds_dir, &id, &new_atom(&id, &url)).await?;
    }

    refresh_feed(client, feeds_dir, data_dir, &id, unread, bookmarked).await
}

/// Re-fetch a feed by ID, merge new entries into the `.atom`, and save an `.md` for each new
/// entry. Returns the summary and the Articles created, so the caller can enqueue them for
/// indexing. Plain ingestion — persistence only; never touches recovery.
pub async fn refresh_feed(
    client: &reqwest::Client,
    feeds_dir: &Path,
    data_dir: &Path,
    id: &FeedId,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<(Feed, Vec<Article>)> {
    let path = xml_path(feeds_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.clone()));
    }
    let existing_xml = tokio::fs::read_to_string(&path).await?;
    let mut existing = parse_atom(&existing_xml)?;

    let url = self_link_url(&existing).ok_or_else(|| FeedError::MissingSelfLink(id.clone()))?;

    let xml = fetch_xml(client, &url).await?;
    let incoming = parse_incoming(&xml)?;
    let added = apply_incoming(&mut existing, &incoming, id.as_str());

    let created_at = file_created_at(&path, Some(*existing.updated())).await;
    write_atom_atomic(feeds_dir, id, &existing).await?;

    let entries: Vec<Entry> = added.iter().map(atom_entry_to_entry).collect();
    let created = save_new_entries(data_dir, &entries).await;
    Ok((
        feed_summary(id, &existing, created_at, unread, bookmarked),
        created,
    ))
}

/// Re-fetch all feeds. Per-feed errors are collected rather than propagated early.
pub async fn refresh_all_feeds(
    client: &reqwest::Client,
    feeds_dir: &Path,
    data_dir: &Path,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Vec<(FeedId, Result<(Feed, Vec<Article>)>)> {
    let ids = match list_feed_ids(feeds_dir).await {
        Ok(ids) => ids,
        Err(e) => return vec![(FeedId(String::new()), Err(e))],
    };

    let mut results = Vec::with_capacity(ids.len());

    for id in ids {
        let result = refresh_feed(client, feeds_dir, data_dir, &id, unread, bookmarked).await;
        results.push((id, result));
    }

    results
}

/// List all feeds on disk, parsed from their Atom XML.
pub async fn list_feeds(
    feeds_dir: &Path,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Vec<Feed>> {
    let ids = list_feed_ids(feeds_dir).await?;
    let mut feeds = Vec::with_capacity(ids.len());

    for id in ids {
        let path = xml_path(feeds_dir, &id);

        if let Ok(xml) = tokio::fs::read_to_string(&path).await {
            if let Ok(atom_feed) = parse_atom(&xml) {
                let created_at = file_created_at(&path, Some(*atom_feed.updated())).await;

                feeds.push(feed_summary(
                    &id, &atom_feed, created_at, unread, bookmarked,
                ));
            }
        }
    }

    Ok(feeds)
}

/// Atom `<category>` scheme that namespaces xaga's own app-local categories (e.g. the
/// `pinned` flag) so they never collide with publisher-supplied categories.
pub const TAG_CATEGORY_SCHEME: &str = "https://xaga.app/ns/tag";

/// Whether `feed` carries the app-local [`crate::tags::PINNED`] flag, stored as an Atom
/// `<category term="…">` under [`TAG_CATEGORY_SCHEME`].
fn is_feed_pinned(feed: &atom::Feed) -> bool {
    feed.categories()
        .iter()
        .any(|c| c.term() == crate::tags::PINNED && c.scheme() == Some(TAG_CATEGORY_SCHEME))
}

/// Adds (or removes) the app-local `pinned` category on a feed's `.atom`, under
/// [`TAG_CATEGORY_SCHEME`]. Idempotent. Returns whether the file changed.
pub async fn set_feed_pinned(feeds_dir: &Path, id: &FeedId, pinned: bool) -> Result<bool> {
    let path = xml_path(feeds_dir, id);
    let xml = tokio::fs::read_to_string(&path).await?;
    let mut feed = parse_atom(&xml)?;

    let has = is_feed_pinned(&feed);

    if has == pinned {
        return Ok(false);
    }

    let mut categories = feed.categories().to_vec();

    if pinned {
        let mut cat = atom::Category::default();
        cat.set_term(crate::tags::PINNED);
        cat.set_scheme(Some(TAG_CATEGORY_SCHEME.to_string()));
        categories.push(cat);
    } else {
        categories.retain(|c| {
            !(c.term() == crate::tags::PINNED && c.scheme() == Some(TAG_CATEGORY_SCHEME))
        });
    }

    feed.set_categories(categories);
    write_atom_atomic(feeds_dir, id, &feed).await?;

    Ok(true)
}

/// Scan `feeds_dir` and repair inconsistencies:
///
/// * Missing `<link rel="self">` — synthesised from `<id>` when `<id>` is an HTTP(S) URL.
/// * Filename whose hash does not match the feed URL — file is renamed to `{hash}.atom`.
///
/// Each `.atom` file is inspected in turn; errors on individual files are collected into
/// the returned `Vec` rather than stopping the whole scan.
/// I/O errors on the directory itself are propagated immediately.
pub async fn repair_feeds(feeds_dir: &Path) -> Result<Vec<RepairResult>> {
    let ids = list_feed_ids(feeds_dir).await?;
    let mut results = Vec::with_capacity(ids.len());

    for id in ids {
        let path = xml_path(feeds_dir, &id);

        // Step 1: read and parse.
        let xml = match tokio::fs::read_to_string(&path).await {
            Ok(s) => s,
            Err(e) => {
                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::ParseError(e.to_string()),
                });
                continue;
            }
        };

        let mut feed = match parse_atom(&xml) {
            Ok(f) => f,
            Err(e) => {
                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::ParseError(e.to_string()),
                });
                continue;
            }
        };

        // Step 2: synthesise missing feed <id> from <link rel="self"> if possible.
        let feed_id_synthesized;

        if feed.id().is_empty() {
            if let Some(self_url) = self_link_url(&feed) {
                let normalized = normalize_url(&self_url).unwrap_or(self_url);
                feed.set_id(url_hash12(&normalized));
                feed_id_synthesized = true;
            } else {
                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::NoId,
                });

                continue;
            }
        } else {
            feed_id_synthesized = false;
        }

        // Step 2b: normalise entry link hrefs and synthesise missing entry <id>s.
        let (entry_links_normalized, entry_ids_synthesized) = {
            let mut links_changed = false;
            let mut ids_changed = false;
            let mut entries = feed.entries().to_vec();

            for entry in &mut entries {
                // Normalise link hrefs.
                let before: Vec<String> =
                    entry.links().iter().map(|l| l.href().to_string()).collect();

                normalize_entry_links(entry);

                let after: Vec<String> =
                    entry.links().iter().map(|l| l.href().to_string()).collect();

                if before != after {
                    links_changed = true;
                }

                // Synthesise a stable <id> when absent.
                if entry.id().is_empty() {
                    let first_href = entry.links().first().map(|l| l.href()).unwrap_or("");
                    let synthesized = feed_entry_id("", first_href);
                    entry.set_id(synthesized);
                    ids_changed = true;
                }
            }

            if links_changed || ids_changed {
                feed.set_entries(entries);
            }

            (links_changed, ids_changed)
        };

        // Step 3: check for an existing <link rel="self">.
        let self_link_added;
        let canonical_url: String;

        if let Some(existing) = self_link_url(&feed) {
            canonical_url = match normalize_url(&existing) {
                Ok(u) => u,
                Err(e) => {
                    results.push(RepairResult {
                        id,
                        outcome: RepairOutcome::ParseError(e.to_string()),
                    });
                    continue;
                }
            };

            self_link_added = false;
        } else {
            // Steps 4-5: try to derive the URL from <id>.
            let feed_id_val = feed.id().to_string();

            if !feed_id_val.starts_with("http://") && !feed_id_val.starts_with("https://") {
                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::NoRepairableUrl,
                });
                continue;
            }

            canonical_url = match normalize_url(&feed_id_val) {
                Ok(u) => u,
                Err(e) => {
                    results.push(RepairResult {
                        id,
                        outcome: RepairOutcome::ParseError(e.to_string()),
                    });
                    continue;
                }
            };

            set_self_link(&mut feed, &canonical_url);
            self_link_added = true;
        }

        // Step 6: check whether the filename matches the expected hash.
        let expected_id = feed_id_from_url(&canonical_url)?;
        let needs_rename = expected_id != id;

        let content_changed = feed_id_synthesized
            || self_link_added
            || entry_links_normalized
            || entry_ids_synthesized;

        match (content_changed, needs_rename) {
            (false, false) => {
                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::Ok,
                });
            }
            (true, false) => {
                write_atom_atomic(feeds_dir, &id, &feed).await?;

                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::Repaired {
                        feed_id_synthesized,
                        self_link_added,
                        renamed_to: None,
                        entry_links_normalized,
                        entry_ids_synthesized,
                    },
                });
            }
            (_, true) => {
                let new_path = xml_path(feeds_dir, &expected_id);

                if new_path.exists() {
                    results.push(RepairResult {
                        id,
                        outcome: RepairOutcome::RenameConflict {
                            target: expected_id,
                        },
                    });

                    continue;
                }

                if content_changed {
                    write_atom_atomic(feeds_dir, &expected_id, &feed).await?;
                    tokio::fs::remove_file(&path).await?;
                } else {
                    tokio::fs::rename(&path, &new_path).await?;
                }

                results.push(RepairResult {
                    id,
                    outcome: RepairOutcome::Repaired {
                        feed_id_synthesized,
                        self_link_added,
                        renamed_to: Some(expected_id),
                        entry_links_normalized,
                        entry_ids_synthesized,
                    },
                });
            }
        }
    }

    Ok(results)
}

/// Rename a feed by updating its `<title>` in the stored Atom file.
pub async fn rename_feed(
    feeds_dir: &Path,
    id: &FeedId,
    new_title: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Feed> {
    let path = xml_path(feeds_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.clone()));
    }
    let xml = tokio::fs::read_to_string(&path).await?;
    let mut feed = parse_atom(&xml)?;
    feed.set_title(atom::Text::plain(new_title));
    let created_at = file_created_at(&path, Some(*feed.updated())).await;
    write_atom_atomic(feeds_dir, id, &feed).await?;
    Ok(feed_summary(id, &feed, created_at, unread, bookmarked))
}

/// Remove a feed (deletes `{id}.atom`).
pub async fn remove_feed(feeds_dir: &Path, id: &FeedId) -> Result<()> {
    let path = xml_path(feeds_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.clone()));
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Returns the title content from a feed-rs `Text`, or `fallback` if absent or empty.
fn incoming_title<'a>(title: &'a Option<feed_rs::model::Text>, fallback: &'a str) -> &'a str {
    title
        .as_ref()
        .filter(|t| !t.content.is_empty())
        .map(|t| t.content.as_str())
        .unwrap_or(fallback)
}

/// Build a new Atom Feed from a feed-rs model, recording `source_url` as `<link rel="self">`.
/// A fresh `.atom` for a newly added feed: records the source URL as the self-link, with no
/// entries yet. `refresh_feed` then fetches from the self-link and fills it in.
fn new_atom(id: &FeedId, url: &str) -> atom::Feed {
    let mut self_link = atom::Link::default();
    self_link.set_href(xml_attr_encode(url));
    self_link.set_rel("self");

    let mut feed = atom::Feed::default();
    feed.set_id(id.as_str().to_string());
    feed.set_updated(now_fixed());
    feed.set_links(vec![self_link]);
    feed
}

/// Normalise every HTTP(S) link `href` on an entry in-place.
/// Falls back to the raw href when `normalize_url` rejects the URL.
/// Used as a shared helper by both `to_atom_entry` and `repair_feeds`.
fn normalize_entry_links(entry: &mut atom::Entry) {
    let links: Vec<atom::Link> = entry
        .links()
        .iter()
        .map(|l| {
            let mut link = l.clone();
            let raw = if let Ok(norm) = normalize_url(l.href()) {
                norm
            } else {
                l.href().to_string()
            };
            link.set_href(xml_attr_encode(&raw));
            link
        })
        .collect();
    entry.set_links(links);
}

/// Encode a URL string for use as an XML attribute value.
/// `atom_syndication`/`quick_xml` writes href values verbatim, so bare `&` must be
/// pre-encoded as `&amp;` here; the XML parser decodes them back when reading.
fn xml_attr_encode(s: &str) -> String {
    if s.contains('&') {
        s.replace('&', "&amp;")
    } else {
        s.to_string()
    }
}

fn to_atom_entry(src: &feed_rs::model::Entry) -> atom::Entry {
    let title = src
        .title
        .as_ref()
        .map(|t| t.content.as_str())
        .unwrap_or(src.id.as_str());

    // Derive a stable id: use the upstream id when present, otherwise hash the
    // first link href so the id is persisted and stays consistent across refreshes.
    let first_link_href = src.links.first().map(|l| l.href.as_str()).unwrap_or("");
    let entry_id = feed_entry_id(&src.id, first_link_href);

    let mut entry = atom::Entry::default();
    entry.set_id(entry_id);
    entry.set_title(atom::Text::plain(title));
    entry.set_updated(src.updated.map(to_fixed).unwrap_or_else(now_fixed));
    if let Some(pub_date) = src.published {
        entry.set_published(to_fixed(pub_date));
    }
    entry.set_authors(src.authors.iter().map(to_atom_person).collect::<Vec<_>>());
    entry.set_links(
        src.links
            .iter()
            .map(|l| {
                let mut link = atom::Link::default();
                link.set_href(l.href.clone());
                link.set_rel(l.rel.clone().unwrap_or_else(|| "alternate".to_string()));
                link
            })
            .collect::<Vec<_>>(),
    );
    if let Some(summary) = &src.summary {
        entry.set_summary(atom::Text::plain(summary.content.clone()));
    }
    if let Some(content) = &src.content {
        if let Some(body) = &content.body {
            let mut c = atom::Content::default();
            c.set_value(body.clone());
            c.set_content_type("html".to_string());
            entry.set_content(c);
        }
    }
    normalize_entry_links(&mut entry);
    entry
}

fn to_atom_person(src: &feed_rs::model::Person) -> atom::Person {
    let mut person = atom::Person::default();
    person.set_name(src.name.clone());
    if let Some(email) = &src.email {
        person.set_email(email.clone());
    }
    if let Some(uri) = &src.uri {
        person.set_uri(uri.clone());
    }
    person
}

/// Apply a freshly-fetched `incoming` feed onto an `existing` stored feed:
/// - `<id>` is taken verbatim from `incoming`; if absent, `fallback_id` is used.
/// - New entries are merged in (duplicates skipped, all sorted newest-first).
/// - The feed's `updated` timestamp is refreshed.
/// Returns the atom entries that were newly added.
fn apply_incoming(
    existing: &mut atom::Feed,
    incoming: &feed_rs::model::Feed,
    fallback_id: &str,
) -> Vec<atom::Entry> {
    let feed_id = if incoming.id.is_empty() {
        fallback_id.to_string()
    } else {
        incoming.id.clone()
    };
    existing.set_id(feed_id);

    if existing.title().value.is_empty() {
        let fallback = self_link_url(existing)
            .map(|u| normalize_url(&u).unwrap_or(u))
            .unwrap_or_default();
        existing.set_title(atom::Text::plain(incoming_title(
            &incoming.title,
            &fallback,
        )));
    }

    let new_entries: Vec<_> = incoming.entries.iter().map(to_atom_entry).collect();
    let added = merge_entries(existing, new_entries);
    existing.set_updated(now_fixed());
    added
}

/// Append entries from `new_entries` that are not already in `feed` (matched by id).
/// Sorts all entries newest-first after merging.
/// Returns the entries that were actually added.
fn merge_entries(feed: &mut atom::Feed, new_entries: Vec<atom::Entry>) -> Vec<atom::Entry> {
    let existing_ids: HashSet<String> = feed.entries().iter().map(|e| e.id().to_string()).collect();

    let to_add: Vec<_> = new_entries
        .into_iter()
        .filter(|e| !existing_ids.contains(e.id()))
        .collect();

    if !to_add.is_empty() {
        let mut all = feed.entries().to_vec();
        all.extend(to_add.clone());
        all.sort_by(|a: &atom::Entry, b: &atom::Entry| b.updated().cmp(a.updated()));
        feed.set_entries(all);
    }

    to_add
}

/// Set (or replace) the `<link rel="self">` on `feed`.
fn set_self_link(feed: &mut atom::Feed, url: &str) {
    let mut links: Vec<atom::Link> = feed
        .links()
        .iter()
        .filter(|l| l.rel() != "self")
        .cloned()
        .collect();
    let mut link = atom::Link::default();
    link.set_href(xml_attr_encode(url));
    link.set_rel("self");
    links.push(link);
    feed.set_links(links);
}

fn self_link_url(feed: &atom::Feed) -> Option<String> {
    feed.links()
        .iter()
        .find(|l| l.rel() == "self")
        .map(|l| l.href().to_string())
}

fn feed_summary(
    id: &FeedId,
    feed: &atom::Feed,
    created_at: DateTime<FixedOffset>,
    unread_ids: &HashSet<String>,
    bookmarked_ids: &HashSet<String>,
) -> Feed {
    let entry_ids: Vec<String> = feed
        .entries()
        .iter()
        .map(|e| {
            let url = e
                .links()
                .iter()
                .find(|l| l.rel() == "alternate")
                .or_else(|| e.links().first())
                .map(|l| l.href())
                .unwrap_or("");
            normalize_url(url)
                .map(|n| url_hash12(&n))
                .unwrap_or_else(|_| url_hash12(url))
        })
        .collect();

    let unread_count = entry_ids
        .iter()
        .filter(|id| unread_ids.contains(*id))
        .count();
    let _bookmarked_count = entry_ids
        .iter()
        .filter(|id| bookmarked_ids.contains(*id))
        .count();

    Feed {
        id: id.to_string(),
        url: self_link_url(feed).unwrap_or_default(),
        title: feed.title().value.clone(),
        description: feed.subtitle().map(|s| s.value.clone()),
        entry_count: feed.entries().len(),
        unread_count,
        created_at: created_at.to_rfc3339(),
        updated_at: Some(feed.updated().to_rfc3339()),
        is_pinned: is_feed_pinned(feed),
        members: vec![],
    }
}

/// Returns the file creation time as a `DateTime<FixedOffset>`.
/// Falls back to `updated_at` if the OS does not track creation time.
/// Falls back to the UNIX epoch if that is also unavailable.
async fn file_created_at(
    path: &Path,
    updated_at: Option<DateTime<FixedOffset>>,
) -> DateTime<FixedOffset> {
    let epoch = || {
        chrono::DateTime::from_timestamp(0, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap())
    };
    tokio::fs::metadata(path)
        .await
        .ok()
        .and_then(|m| m.created().ok())
        .map(|st| {
            let utc = chrono::DateTime::<chrono::Utc>::from(st);
            utc.with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .or(updated_at)
        .unwrap_or_else(epoch)
}

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
            let id = feed_id_from_url(&normalized)?;
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

fn atom_entry_to_entry(e: &atom::Entry) -> Entry {
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
        description: summary,
        is_unread: false,
        is_bookmarked: false,
        color: None,
    }
}
