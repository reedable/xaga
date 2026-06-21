use std::path::{Path, PathBuf};

use atom_syndication as atom;
use chrono::{DateTime, FixedOffset, Utc};
use thiserror::Error;

pub mod feed;
pub mod feed_entry;

pub use crate::types::Feed;

pub use feed_entry::list_entry_ids;
pub use feed::{
    add_feed, feed_id_from_url, list_feeds, refresh_all_feeds, refresh_feed,
    remove_feed, rename_feed, repair_feeds, set_feed_pinned, FeedId, RepairOutcome,
    RepairResult, TAG_CATEGORY_SCHEME,
};

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

// ── Shared infrastructure (pub(super)) ──────────────────────────────────────

pub(super) fn xml_path(feeds_dir: &Path, id: &FeedId) -> PathBuf {
    feeds_dir.join(format!("{id}.atom"))
}

pub(super) async fn write_atom_atomic(
    feeds_dir: &Path,
    id: &FeedId,
    feed: &atom::Feed,
) -> Result<()> {
    let mut buf = Vec::new();
    feed.write_to(&mut buf)
        .map_err(|e| FeedError::AtomParse(e.to_string()))?;
    crate::fs::write_atomic_async(xml_path(feeds_dir, id), buf).await?;
    Ok(())
}

pub(super) async fn list_feed_ids(feeds_dir: &Path) -> Result<Vec<FeedId>> {
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

pub(super) async fn fetch_xml(client: &reqwest::Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| FeedError::Http { url: url.to_string(), source: e })?
        .error_for_status()
        .map_err(|e| FeedError::Http { url: url.to_string(), source: e })?
        .text()
        .await
        .map_err(|e| FeedError::Http { url: url.to_string(), source: e })
}

pub(super) fn parse_incoming(xml: &str) -> Result<feed_rs::model::Feed> {
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
                None => { result.push_str(rest); break; }
                Some(a) => {
                    result.push_str(&rest[..a]);
                    let after = &rest[a + 1..];
                    let is_valid_ref = after.starts_with("amp;")
                        || after.starts_with("lt;")
                        || after.starts_with("gt;")
                        || after.starts_with("apos;")
                        || after.starts_with("quot;")
                        || after.starts_with('#');
                    if is_valid_ref { result.push('&'); } else { result.push_str("&amp;"); }
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
                if is_valid_ref { result.push('&'); } else { result.push_str("&amp;"); }
                rest = after;
            }
        }
    }

    result
}

pub(super) fn parse_atom(xml: &str) -> Result<atom::Feed> {
    let sanitized = fix_bare_ampersands(xml);
    sanitized.parse::<atom::Feed>().map_err(|e| FeedError::AtomParse(e.to_string()))
}

pub(super) fn to_fixed(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

pub(super) fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}
