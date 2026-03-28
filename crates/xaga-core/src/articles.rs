use std::path::{Path, PathBuf};

use crate::entries::{find_entry_path, render_entry, EntryError};
use crate::util::{normalize_url, parse_document, parse_frontmatter, url_hash12};

pub use crate::entries::EntryError as ArticleError;
pub use crate::types::Article;

// ── Path helpers ──────────────────────────────────────────────────────────────

pub(crate) fn article_path(data_dir: &Path, hash12: &str) -> PathBuf {
    data_dir.join("read").join(format!("{hash12}.md"))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Writes full content to the item's file wherever it lives, preserving `created_at`.
/// Does not change the item's read state. Returns the saved Article.
pub async fn save_article(
    data_dir: &Path,
    id: &str,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    markdown: &str,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;

    let (path, file_exists) = match find_entry_path(data_dir, id).await {
        Some(p) => (p, true),
        None => {
            // No existing file (e.g. bookmark with no synced entry) — create in read/.
            let read_path = article_path(data_dir, id);
            tokio::fs::create_dir_all(read_path.parent().unwrap()).await?;
            (read_path, false)
        }
    };

    let created_at = if file_exists {
        let raw = tokio::fs::read_to_string(&path).await?;
        parse_frontmatter(&raw)
            .map(|fm| fm.created_at)
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
    } else {
        chrono::Utc::now().to_rfc3339()
    };

    let is_unread = path.parent().map_or(false, |p| p.ends_with("unread"));

    let content = render_entry(&normalized, title, &created_at, published_at, updated_at, summary, markdown);
    tokio::fs::write(&path, content.as_bytes()).await?;

    Ok(Article {
        meta: crate::entries::Entry {
            id: id.to_string(),
            url: normalized,
            title: title.to_string(),
            created_at,
            published_at: published_at.map(str::to_string),
            updated_at: updated_at.map(str::to_string),
            summary: summary.map(str::to_string),
            is_unread,
        },
        content: markdown.to_string(),
    })
}

/// Creates a new article, deriving the ID from the URL hash.
/// If an article with this URL already exists (same hash), it is overwritten.
pub async fn create_article(
    data_dir: &Path,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    markdown: &str,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let id = url_hash12(&normalized);
    save_article(data_dir, &id, url, title, published_at, updated_at, summary, markdown).await
}

/// Returns the article for the given URL if it exists, or `None` if not found.
/// The ID is derived from the normalised URL hash, matching `create_article`.
pub async fn find_article(
    data_dir: &Path,
    url: &str,
) -> Result<Option<Article>, EntryError> {
    let normalized = normalize_url(url)?;
    let id = url_hash12(&normalized);
    match read_article(data_dir, &id).await {
        Ok(article) => Ok(Some(article)),
        Err(EntryError::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Returns the article for the given id, searching all entry directories.
pub async fn read_article(
    data_dir: &Path,
    id: &str,
) -> Result<Article, EntryError> {
    let path = find_entry_path(data_dir, id).await.ok_or(EntryError::NotFound)?;

    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    Ok(Article {
        meta: crate::entries::Entry {
            id: id.to_string(),
            url: fm.source,
            title: fm.title,
            created_at: fm.created_at,
            published_at: fm.published_at,
            updated_at: fm.updated_at,
            summary: fm.summary,
            is_unread: path.parent().map_or(false, |p| p.ends_with("unread")),
        },
        content: body,
    })
}
