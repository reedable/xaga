use std::path::{Path, PathBuf};

use crate::entries::{entry_from_fm, find_entry_path, render_entry, EntryError};
use crate::util::{normalize_url, parse_document, url_hash12};

pub use crate::entries::EntryError as ArticleError;
pub use crate::types::{Annotation, Article};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Populates the `reference` field on each annotation with the given article ID.
/// Annotations loaded from YAML frontmatter have `reference = ""` by default.
fn with_reference(mut annotations: Vec<Annotation>, article_id: &str) -> Vec<Annotation> {
    for a in &mut annotations {
        a.reference = article_id.to_string();
    }
    annotations
}

// ── Path helpers ──────────────────────────────────────────────────────────────

pub(crate) fn article_path(data_dir: &Path, hash12: &str) -> PathBuf {
    data_dir.join("entries").join(format!("{hash12}.md"))
}


// ── Public API ────────────────────────────────────────────────────────────────

/// Writes full content to the item's file wherever it lives, preserving `created_at`
/// and any existing annotations. Does not change the item's read state.
/// Returns the saved Article.
pub async fn save_article(
    data_dir: &Path,
    id: &str,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    favicon: Option<&str>,
    markdown: &str,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;

    let (path, file_exists) = match find_entry_path(data_dir, id).await {
        Some(p) => (p, true),
        None => {
            // No existing file (e.g. bookmark with no synced entry) — create in entries/.
            let entries_path = article_path(data_dir, id);
            tokio::fs::create_dir_all(entries_path.parent().unwrap()).await?;
            (entries_path, false)
        }
    };

    let (created_at, existing_annotations, existing_tags) = if file_exists {
        let raw = tokio::fs::read_to_string(&path).await?;
        match parse_document(&raw) {
            Some((fm, _)) => (fm.created_at, fm.annotations, fm.tags),
            None => (chrono::Utc::now().to_rfc3339(), vec![], vec![]),
        }
    } else {
        (chrono::Utc::now().to_rfc3339(), vec![], vec![])
    };

    let content = render_entry(
        &normalized,
        title,
        &created_at,
        published_at,
        updated_at,
        summary,
        &existing_annotations,
        &existing_tags,
        favicon,
        markdown,
    );
    tokio::fs::write(&path, content.as_bytes()).await?;

    let is_unread = existing_tags.contains(&"unread".to_string());
    let is_bookmarked = existing_tags.contains(&"bookmark".to_string());
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
            is_bookmarked,
            tags: existing_tags,
        },
        content: markdown.to_string(),
        annotations: with_reference(existing_annotations, id),
        favicon: favicon.map(str::to_string),
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
    favicon: Option<&str>,
    markdown: &str,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let id = url_hash12(&normalized);
    save_article(data_dir, &id, url, title, published_at, updated_at, summary, favicon, markdown).await
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
    let annotations = fm.annotations.clone();
    let favicon = fm.favicon.clone();

    Ok(Article {
        meta: entry_from_fm(id.to_string(), fm),
        content: body,
        annotations: with_reference(annotations, id),
        favicon,
    })
}

/// Adds or replaces an annotation in the article's frontmatter (upsert by id).
/// Returns the updated Article.
pub async fn add_annotation(
    data_dir: &Path,
    article_id: &str,
    annotation: Annotation,
) -> Result<Article, EntryError> {
    let path = find_entry_path(data_dir, article_id).await.ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    let mut annotations = fm.annotations.clone();
    if let Some(pos) = annotations.iter().position(|a| a.id == annotation.id) {
        let created_at = annotations[pos].created_at.clone();
        annotations[pos] = Annotation { created_at, ..annotation };
    } else {
        annotations.push(Annotation {
            created_at: chrono::Utc::now().to_rfc3339(),
            ..annotation
        });
    }

    let favicon = fm.favicon.clone();
    let content = render_entry(
        &fm.source,
        &fm.title,
        &fm.created_at,
        fm.published_at.as_deref(),
        fm.updated_at.as_deref(),
        fm.summary.as_deref(),
        &annotations,
        &fm.tags,
        favicon.as_deref(),
        &body,
    );
    tokio::fs::write(&path, content.as_bytes()).await?;

    // TODO: trigger annotation embedding (annotation.text + annotation.note) for Search feature
    // once LanceDB semantic search is implemented. Each annotation should be indexed separately,
    // keyed by annotation id, combining the highlighted text and the user note into one embedding.

    Ok(Article {
        meta: entry_from_fm(article_id.to_string(), fm),
        content: body,
        annotations: with_reference(annotations, article_id),
        favicon,
    })
}

/// Removes an annotation by id. Returns the updated Article.
pub async fn remove_annotation(
    data_dir: &Path,
    article_id: &str,
    annotation_id: &str,
) -> Result<Article, EntryError> {
    let path = find_entry_path(data_dir, article_id).await.ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    let annotations: Vec<Annotation> = fm.annotations
        .iter()
        .filter(|a| a.id != annotation_id)
        .cloned()
        .collect();

    let favicon = fm.favicon.clone();
    let content = render_entry(
        &fm.source,
        &fm.title,
        &fm.created_at,
        fm.published_at.as_deref(),
        fm.updated_at.as_deref(),
        fm.summary.as_deref(),
        &annotations,
        &fm.tags,
        favicon.as_deref(),
        &body,
    );
    tokio::fs::write(&path, content.as_bytes()).await?;

    Ok(Article {
        meta: entry_from_fm(article_id.to_string(), fm),
        content: body,
        annotations: with_reference(annotations, article_id),
        favicon,
    })
}
