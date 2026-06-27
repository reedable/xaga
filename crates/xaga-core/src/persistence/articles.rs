use std::path::{Path, PathBuf};

use crate::{
    fm::Frontmatter,
    md::{Markdown, md_to_entry, parse_document},
    persistence::entries::{EntryError, find_entry_path, render_entry},
    types::{Annotation, Article, Entry},
    util::{normalize_url, url_hash12},
};

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
    crate::library::entries_dir(data_dir).join(format!("{hash12}.md"))
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
    if !crate::persistence::entries::is_safe_id(id) {
        return Err(EntryError::InvalidId);
    }
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

    let article = Article {
        meta: Entry {
            id: id.to_string(),
            url: normalized,
            title: title.to_string(),
            created_at,
            published_at: published_at.map(str::to_string),
            updated_at: updated_at.map(str::to_string),
            description: summary.map(str::to_string),
            is_unread: crate::tags::is_unread(&existing_tags),
            is_bookmarked: crate::tags::is_bookmarked(&existing_tags),
            color: None,
        },
        content: markdown.to_string(),
        annotations: with_reference(existing_annotations, id),
        favicon: favicon.map(str::to_string),
    };
    tokio::fs::write(&path, render_entry(&article).as_bytes()).await?;
    Ok(article)
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
    save_article(
        data_dir,
        &id,
        url,
        title,
        published_at,
        updated_at,
        summary,
        favicon,
        markdown,
    )
    .await
}

/// Returns the article for the given URL if it exists, or `None` if not found.
/// The ID is derived from the normalised URL hash, matching `create_article`.
pub async fn find_article(data_dir: &Path, url: &str) -> Result<Option<Article>, EntryError> {
    let normalized = normalize_url(url)?;
    let id = url_hash12(&normalized);
    match read_article(data_dir, &id).await {
        Ok(article) => Ok(Some(article)),
        Err(EntryError::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Builds an [`Article`] from an already-parsed document (used by recovery, which parses once
/// during its scan and hands the domain object straight to indexing).
pub(crate) fn article_from_document(id: &str, fm: Frontmatter, body: String) -> Article {
    let annotations = fm.annotations.clone();
    let favicon = fm.favicon.clone();
    article_from_fm(id, fm, annotations, favicon, body)
}

/// Returns the article for the given id, searching all entry directories.
pub async fn read_article(data_dir: &Path, id: &str) -> Result<Article, EntryError> {
    let path = find_entry_path(data_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;

    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;
    Ok(article_from_document(id, fm, body))
}

/// Adds or replaces an annotation in the article's frontmatter (upsert by id).
/// Returns the updated Article.
pub async fn add_annotation(
    data_dir: &Path,
    article_id: &str,
    annotation: Annotation,
) -> Result<Article, EntryError> {
    let path = find_entry_path(data_dir, article_id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    let mut annotations = fm.annotations.clone();
    if let Some(pos) = annotations.iter().position(|a| a.id == annotation.id) {
        let created_at = annotations[pos].created_at.clone();
        annotations[pos] = Annotation {
            created_at,
            ..annotation
        };
    } else {
        annotations.push(Annotation {
            created_at: chrono::Utc::now().to_rfc3339(),
            ..annotation
        });
    }

    let favicon = fm.favicon.clone();
    let article = article_from_fm(article_id, fm, annotations, favicon, body);
    tokio::fs::write(&path, render_entry(&article).as_bytes()).await?;
    Ok(article)
}

/// Removes an annotation by id. Returns the updated [`Article`].
pub async fn remove_annotation(
    data_dir: &Path,
    article_id: &str,
    annotation_id: &str,
) -> Result<Article, EntryError> {
    let path = find_entry_path(data_dir, article_id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    let annotations: Vec<Annotation> = fm
        .annotations
        .iter()
        .filter(|a| a.id != annotation_id)
        .cloned()
        .collect();

    let favicon = fm.favicon.clone();
    let article = article_from_fm(article_id, fm, annotations, favicon, body);
    tokio::fs::write(&path, render_entry(&article).as_bytes()).await?;
    Ok(article)
}

/// Builds the updated [`Article`] from its parsed frontmatter and the new annotation set, so the
/// annotation edit parses once and the producer hands the domain object straight to the queue.
fn article_from_fm(
    article_id: &str,
    fm: Frontmatter,
    annotations: Vec<Annotation>,
    favicon: Option<String>,
    body: String,
) -> Article {
    Article {
        meta: md_to_entry(Markdown {
            id: article_id.to_string(),
            fm,
            body: String::new(),
        }),
        content: body,
        annotations: with_reference(annotations, article_id),
        favicon,
    }
}
