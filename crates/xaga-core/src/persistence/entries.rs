use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use crate::{
    fm::{FrontmatterAnnotation, parse_frontmatter},
    md::{Markdown, md_to_entry, parse_document},
    types::{Article, Entry},
    util::{normalize_url, url_hash12},
};

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
pub(crate) fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && !id.contains('\0')
}

// ── Frontmatter rendering ─────────────────────────────────────────────────────

/// Encodes an [`Article`] to its `.md` representation (frontmatter + body) — the single
/// entry/article encoder. `tags` and `aliases` are reconstructed from the domain object (state
/// booleans / title); both are storage-only keys the domain doesn't carry directly.
pub(crate) fn render_entry(article: &Article) -> String {
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
        #[serde(skip_serializing_if = "Vec::is_empty")]
        annotations: Vec<FrontmatterAnnotation>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        tags: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        favicon: Option<&'a str>,
    }

    let meta = &article.meta;
    let mut tags: Vec<String> = Vec::new();
    if meta.is_unread {
        tags.push(crate::tags::UNREAD.to_string());
    }
    if meta.is_bookmarked {
        tags.push(crate::tags::BOOKMARK.to_string());
    }

    let clean_summary = meta
        .description
        .as_deref()
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "));

    let fm = Fm {
        source: &meta.url,
        title: &meta.title,
        aliases: vec![meta.title.as_str()],
        created: &meta.created_at,
        published: meta.published_at.as_deref(),
        updated: meta.updated_at.as_deref(),
        summary: clean_summary,
        annotations: article.annotations.iter().map(Into::into).collect(),
        tags: &tags,
        favicon: article.favicon.as_deref(),
    };

    let yaml = serde_yaml::to_string(&fm).expect("frontmatter serialization is infallible");
    format!("---\n{yaml}---\n\n{}\n", article.content)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Finds the current path of an entry by id, or `None` if the file does not exist.
pub(crate) async fn find_entry_path(library_dir: &Path, id: &str) -> Option<PathBuf> {
    if !is_safe_id(id) {
        return None;
    }
    let path = crate::library::entries_dir(library_dir).join(format!("{id}.md"));
    tokio::fs::try_exists(&path)
        .await
        .unwrap_or(false)
        .then_some(path)
}

/// Builds the [`Article`] for a freshly-written entry (empty body, no annotations) from its parts,
/// so the producer hands the domain object to the queue and the consumer never re-reads the file.
fn entry_article(
    id: String,
    normalized_url: String,
    title: &str,
    created_at: String,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
) -> Article {
    Article {
        meta: Entry {
            id,
            url: normalized_url,
            title: title.to_string(),
            created_at,
            published_at: published_at.map(str::to_string),
            updated_at: updated_at.map(str::to_string),
            description: summary.map(str::to_string),
            is_unread: crate::tags::is_unread(tags),
            is_bookmarked: crate::tags::is_bookmarked(tags),
            color: None,
        },
        content: String::new(),
        annotations: vec![],
        favicon: None,
    }
}

/// Writes a new Entry to `entries/{hash12}.md` with the `unread` tag, returning its [`Article`].
/// Called by `reconcile_atom_file` when a feed entry has no local file yet.
pub(crate) async fn write_entry(
    library_dir: &Path,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = crate::library::entry_path(library_dir, &hash12);

    let created_at = Utc::now().to_rfc3339();
    let tags = vec![crate::tags::UNREAD.to_string()];
    let article = entry_article(
        hash12,
        normalized,
        title,
        created_at,
        published_at,
        updated_at,
        summary,
        &tags,
    );
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&path, render_entry(&article).as_bytes()).await?;
    Ok(article)
}

/// Writes a new Entry to `entries/{hash12}.md` without the `unread` tag (already read).
/// If an Entry already exists at that path it is left untouched. Returns the [`Article`] for the
/// entry so the caller can queue an embed without re-reading the file.
pub(crate) async fn write_read_entry(
    library_dir: &Path,
    url: &str,
    title: &str,
    created_at: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = crate::library::entry_path(library_dir, &hash12);

    let article = entry_article(
        hash12,
        normalized,
        title,
        created_at.to_string(),
        published_at,
        updated_at,
        summary,
        tags,
    );

    if !tokio::fs::try_exists(&path).await? {
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        tokio::fs::write(&path, render_entry(&article).as_bytes()).await?;
    }

    Ok(article)
}

/// Reads an entry by id and returns its metadata. Does not modify the file.
pub async fn read_entry(library_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let fm = parse_frontmatter(&raw).ok_or(EntryError::NotFound)?;
    Ok(md_to_entry(Markdown {
        id: id.to_string(),
        fm,
        body: String::new(),
    }))
}

/// Returns an entry's frontmatter tags — a persistence detail, not on the domain `Entry`.
pub async fn read_entry_tags(library_dir: &Path, id: &str) -> Result<Vec<String>, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let fm = parse_frontmatter(&raw).ok_or(EntryError::NotFound)?;
    Ok(fm.tags)
}

/// Replaces the tags on an entry's frontmatter and writes the file back.
/// The file watcher picks up the write and queues a priority LanceDB update.
pub async fn set_entry_tags(
    library_dir: &Path,
    id: &str,
    tags: Vec<String>,
) -> Result<Entry, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;
    // Build the updated Article (the message) once, encode it, and return its meta. State comes from
    // the new `tags`; annotations/body/favicon are preserved from the parsed file.
    let article = Article {
        meta: Entry {
            id: id.to_string(),
            url: fm.source,
            title: fm.title,
            created_at: fm.created_at,
            published_at: fm.published_at,
            updated_at: fm.updated_at,
            description: fm.summary,
            is_unread: crate::tags::is_unread(&tags),
            is_bookmarked: crate::tags::is_bookmarked(&tags),
            color: None,
        },
        content: body,
        annotations: fm.annotations,
        favicon: fm.favicon,
    };
    tokio::fs::write(&path, render_entry(&article).as_bytes()).await?;
    Ok(article.meta)
}
