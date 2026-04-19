use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use crate::util::{normalize_url, parse_frontmatter, url_hash12};

pub use crate::types::Entry;

#[derive(Debug, Error)]
pub enum EntryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Entry not found")]
    NotFound,
}

// ── Frontmatter rendering ─────────────────────────────────────────────────────

pub(crate) fn render_entry(
    source: &str,
    title: &str,
    created_at: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    annotations: &[crate::types::Annotation],
    markdown: &str,
) -> String {
    #[derive(serde::Serialize)]
    struct Fm<'a> {
        source: &'a str,
        title: &'a str,
        aliases: Vec<&'a str>,
        created: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        published: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        updated: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        annotations: &'a [crate::types::Annotation],
    }

    let clean_summary = summary.map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "));

    let fm = Fm {
        source,
        title,
        aliases: vec![title],
        created: created_at,
        published: published_at,
        updated: updated_at,
        summary: clean_summary.as_deref().map(str::to_string),
        annotations,
    };

    let yaml = serde_yaml::to_string(&fm).expect("frontmatter serialization is infallible");
    format!("---\n{yaml}---\n\n{markdown}\n")
}

// ── Path helpers ──────────────────────────────────────────────────────────────

pub(crate) fn entry_path(data_dir: &Path, hash12: &str) -> PathBuf {
    data_dir.join("unread").join(format!("{hash12}.md"))
}

fn entry_from_fm(id: String, fm: crate::util::Frontmatter, is_unread: bool) -> Entry {
    Entry {
        id,
        url: fm.source,
        title: fm.title,
        created_at: fm.created_at,
        published_at: fm.published_at,
        updated_at: fm.updated_at,
        summary: fm.summary,
        is_unread,
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns the canonical list of directories containing entry `.md` files.
/// Use this only for directory-level operations (scanning, watching).
/// To locate a specific entry by id, use [`find_entry_path`] instead.
pub fn entry_dirs(library: &Path) -> Vec<PathBuf> {
    vec![library.join("unread"), library.join("read")]
}

/// Finds the current path of an entry by id, checking all entry directories in order.
/// Returns `None` if the file does not exist in any of them.
pub async fn find_entry_path(data_dir: &Path, id: &str) -> Option<PathBuf> {
    for dir in entry_dirs(data_dir) {
        let path = dir.join(format!("{id}.md"));
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Some(path);
        }
    }
    None
}

/// Writes a new entry stub to `unread/{hash12}.md`. Called by
/// `reconcile_atom_file` when a feed entry has no local file yet.
pub async fn write_entry(
    data_dir: &Path,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
) -> Result<PathBuf, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = entry_path(data_dir, &hash12);

    let created_at = Utc::now().to_rfc3339();
    let content = render_entry(&normalized, title, &created_at, published_at, updated_at, summary, &[], "");
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&path, content.as_bytes()).await?;
    Ok(path)
}

/// Moves the entry to `read/{hash12}.md` regardless of which directory it currently lives in.
/// Returns the updated Entry with `is_unread = false`.
pub async fn mark_entry_read(data_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let dest = crate::articles::article_path(data_dir, id);
    let src = find_entry_path(data_dir, id).await.ok_or(EntryError::NotFound)?;
    if src != dest {
        tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
        tokio::fs::rename(&src, &dest).await?;
    }
    let raw = tokio::fs::read_to_string(&dest).await?;
    let fm = crate::util::parse_frontmatter(&raw).ok_or(EntryError::NotFound)?;
    Ok(entry_from_fm(id.to_string(), fm, false))
}

/// Moves the entry to `unread/{hash12}.md` regardless of which directory it currently lives in.
/// Returns the updated Entry with `is_unread = true`.
pub async fn mark_entry_unread(data_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let dest = entry_path(data_dir, id);
    let src = find_entry_path(data_dir, id).await.ok_or(EntryError::NotFound)?;
    if src != dest {
        tokio::fs::create_dir_all(dest.parent().unwrap()).await?;
        tokio::fs::rename(&src, &dest).await?;
    }
    let raw = tokio::fs::read_to_string(&dest).await?;
    let fm = crate::util::parse_frontmatter(&raw).ok_or(EntryError::NotFound)?;
    Ok(entry_from_fm(id.to_string(), fm, true))
}

/// Returns a summary for every entry in both `read/` and `unread/`.
/// Results are sorted by source URL.
pub async fn list_entries(data_dir: &Path) -> Result<Vec<Entry>, EntryError> {
    let mut entries: Vec<Entry> = Vec::new();

    for dir in entry_dirs(data_dir) {
        if !tokio::fs::try_exists(&dir).await? {
            continue;
        }
        let mut dir_entries = tokio::fs::read_dir(&dir).await?;
        while let Some(dir_entry) = dir_entries.next_entry().await? {
            let path = dir_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let raw = tokio::fs::read_to_string(&path).await?;
            if let Some(fm) = parse_frontmatter(&raw) {
                entries.push(entry_from_fm(crate::util::item_id(&path), fm, false));
            }
        }
    }

    crate::filter::annotate_unread_status(data_dir, &mut entries).await;
    entries.sort_by(|a, b| a.url.cmp(&b.url));
    Ok(entries)
}
