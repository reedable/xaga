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
    #[error("Invalid id")]
    InvalidId,
    #[error("Entry not found")]
    NotFound,
}

/// Returns true when `id` is safe to interpolate into a file path: a single
/// path component with no separators or parent-directory references. Entry and
/// article ids are URL hashes (`url_hash12`) or Atom ids — never paths — so a
/// value containing `/`, `\`, `..` or a NUL byte is an attempted traversal.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && !id.contains('\0')
}

// ── Frontmatter rendering ─────────────────────────────────────────────────────

pub fn render_entry(
    source: &str,
    title: &str,
    created_at: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    annotations: &[crate::types::Annotation],
    tags: &[String],
    favicon: Option<&str>,
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
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        tags: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        favicon: Option<&'a str>,
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
        tags,
        favicon,
    };

    let yaml = serde_yaml::to_string(&fm).expect("frontmatter serialization is infallible");
    format!("---\n{yaml}---\n\n{markdown}\n")
}

// ── Path helpers ──────────────────────────────────────────────────────────────

pub(crate) fn entry_path(data_dir: &Path, hash12: &str) -> PathBuf {
    data_dir.join("entries").join(format!("{hash12}.md"))
}

pub fn entry_from_fm(id: String, fm: crate::util::Frontmatter) -> Entry {
    let is_unread = fm.tags.contains(&"unread".to_string());
    let is_bookmarked = fm.tags.contains(&"bookmark".to_string());
    Entry {
        id,
        url: fm.source,
        title: fm.title,
        created_at: fm.created_at,
        published_at: fm.published_at,
        updated_at: fm.updated_at,
        summary: fm.summary,
        is_unread,
        is_bookmarked,
        tags: fm.tags,
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns the canonical list of directories containing entry `.md` files.
/// Use this only for directory-level operations (scanning, watching).
/// To locate a specific entry by id, use [`find_entry_path`] instead.
pub fn entry_dirs(library: &Path) -> Vec<PathBuf> {
    vec![library.join("entries")]
}

/// Finds the current path of an entry by id, checking all entry directories in order.
/// Returns `None` if the file does not exist in any of them.
pub async fn find_entry_path(data_dir: &Path, id: &str) -> Option<PathBuf> {
    if !is_safe_id(id) {
        return None;
    }
    for dir in entry_dirs(data_dir) {
        let path = dir.join(format!("{id}.md"));
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Some(path);
        }
    }
    None
}

/// Writes a new Entry to `entries/{hash12}.md` with the `unread` tag.
/// Called by `reconcile_atom_file` when a feed entry has no local file yet.
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
    let tags = vec!["unread".to_string()];
    let content = render_entry(&normalized, title, &created_at, published_at, updated_at, summary, &[], &tags, None, "");
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&path, content.as_bytes()).await?;
    Ok(path)
}

/// Writes a new Entry to `entries/{hash12}.md` without the `unread` tag (already read).
/// If an Entry already exists at that path it is left untouched and the
/// existing id is returned. The caller is responsible for queuing an embed.
pub async fn write_read_entry(
    data_dir: &Path,
    url: &str,
    title: &str,
    created_at: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
) -> Result<String, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = entry_path(data_dir, &hash12);

    if !tokio::fs::try_exists(&path).await? {
        let content = render_entry(&normalized, title, created_at, published_at, updated_at, summary, &[], tags, None, "");
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        tokio::fs::write(&path, content.as_bytes()).await?;
    }

    Ok(hash12)
}



/// Reads an entry by id and returns its metadata. Does not modify the file.
pub async fn read_entry(data_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let path = find_entry_path(data_dir, id).await.ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let fm = parse_frontmatter(&raw).ok_or(EntryError::NotFound)?;
    Ok(entry_from_fm(id.to_string(), fm))
}

/// Replaces the tags on an entry's frontmatter and writes the file back.
/// The file watcher picks up the write and queues a priority LanceDB update.
pub async fn set_entry_tags(data_dir: &Path, id: &str, tags: Vec<String>) -> Result<Entry, EntryError> {
    let path = find_entry_path(data_dir, id).await.ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = crate::util::parse_document(&raw).ok_or(EntryError::NotFound)?;
    let content = render_entry(
        &fm.source,
        &fm.title,
        &fm.created_at,
        fm.published_at.as_deref(),
        fm.updated_at.as_deref(),
        fm.summary.as_deref(),
        &fm.annotations,
        &tags,
        fm.favicon.as_deref(),
        &body,
    );
    tokio::fs::write(&path, content.as_bytes()).await?;
    let is_unread = tags.contains(&"unread".to_string());
    let is_bookmarked = tags.contains(&"bookmark".to_string());
    Ok(Entry {
        id: id.to_string(),
        url: fm.source,
        title: fm.title,
        created_at: fm.created_at,
        published_at: fm.published_at,
        updated_at: fm.updated_at,
        summary: fm.summary,
        is_unread,
        is_bookmarked,
        tags,
    })
}

/// Returns a summary for every entry in `entries/`.
/// `is_unread` is derived from each file's frontmatter tags.
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
                entries.push(entry_from_fm(crate::util::item_id(&path), fm));
            }
        }
    }

    Ok(entries)
}

// (The `render_entry` topics round-trip test was removed with the `topics:` frontmatter field.)
