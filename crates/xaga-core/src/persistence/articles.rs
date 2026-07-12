use std::path::Path;

use crate::{
    library_writer::WriteToken,
    md::Frontmatter,
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

// ── Public API ────────────────────────────────────────────────────────────────

/// Writes full content to the item's file wherever it lives, preserving `created_at`
/// and any existing annotations. Does not change the item's read state.
/// Returns the saved Article.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn save_article(
    library_dir: &Path,
    id: &str,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    favicon: Option<&str>,
    markdown: &str,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    if !crate::persistence::entries::is_safe_id(id) {
        return Err(EntryError::InvalidId);
    }
    let normalized = normalize_url(url)?;

    let (path, file_exists) = match find_entry_path(library_dir, id).await {
        Some(p) => (p, true),
        None => {
            // No existing file (e.g. bookmark with no synced entry) — create in entries/.
            let entries_path = crate::library::entry_path(library_dir, id);
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
    crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;
    Ok(article)
}

/// Returns the article for the given URL if it exists, or `None` if not found.
/// The ID is derived from the normalised URL hash, matching `create_article`.
pub async fn find_article(library_dir: &Path, url: &str) -> Result<Option<Article>, EntryError> {
    let normalized = normalize_url(url)?;
    let id = url_hash12(&normalized);
    match read_article(library_dir, &id).await {
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
pub async fn read_article(library_dir: &Path, id: &str) -> Result<Article, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;

    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;
    Ok(article_from_document(id, fm, body))
}

/// Adds or updates an annotation in the article's frontmatter (upsert by id). A same-id
/// update must not change the color: a color change is delete/create (color is part of the
/// annotation's identity) and must go through [`replace_annotation`] — a same-id color write
/// is exactly the pattern whose crash-stranded state no reconcile gate can see.
/// Returns the updated Article.
pub(crate) async fn add_annotation(
    library_dir: &Path,
    article_id: &str,
    annotation: Annotation,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    let path = find_entry_path(library_dir, article_id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    let mut annotations = fm.annotations.clone();
    if let Some(pos) = annotations.iter().position(|a| a.id == annotation.id) {
        if annotations[pos].color != annotation.color {
            return Err(EntryError::ColorChangeRequiresReplace);
        }
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
    crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;
    Ok(article)
}

/// Replaces an annotation under a new id — the delete and the create land in one atomic
/// frontmatter write, so no crash point exists where the annotation is deleted but not yet
/// re-created. A color change is modeled as delete/create (color is part of the annotation's
/// identity): the id-set change is what makes a crash-stranded color change visible to the
/// membership diffs in recovery and `sync_entry_row`, which a same-id update would slip past.
/// `created_at` carries over from the old annotation — the highlight is the same selection.
/// Returns the updated [`Article`].
pub(crate) async fn replace_annotation(
    library_dir: &Path,
    article_id: &str,
    old_id: &str,
    annotation: Annotation,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    let path = find_entry_path(library_dir, article_id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(EntryError::NotFound)?;

    let mut annotations = fm.annotations.clone();
    if let Some(pos) = annotations.iter().position(|a| a.id == old_id) {
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
    crate::fs::write_atomic_async(path, render_entry(&article).into_bytes()).await?;
    Ok(article)
}

/// Removes an annotation by id. Returns the updated [`Article`].
pub(crate) async fn remove_annotation(
    library_dir: &Path,
    article_id: &str,
    annotation_id: &str,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    let path = find_entry_path(library_dir, article_id)
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
    crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;
    Ok(article)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn annotation(id: &str, color: &str, note: &str) -> Annotation {
        Annotation {
            id: id.to_string(),
            title: "highlighted text".to_string(),
            reference: String::new(),
            text: vec!["highlighted text".to_string()],
            occurrence: vec![0],
            color: color.to_string(),
            content: note.to_string(),
            created_at: String::new(),
        }
    }

    #[tokio::test]
    async fn replace_annotation_swaps_id_in_one_write_and_carries_created_at() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            "body",
            WriteToken::for_test(),
        )
        .await
        .unwrap();

        let article = add_annotation(
            lib,
            "abcdef123456",
            annotation("old-id", "yellow", "n"),
            WriteToken::for_test(),
        )
            .await
            .unwrap();
        let created_at = article.annotations[0].created_at.clone();
        assert!(!created_at.is_empty());

        let article = replace_annotation(
            lib,
            "abcdef123456",
            "old-id",
            annotation("new-id", "red", "n"),
            WriteToken::for_test(),
        )
        .await
        .unwrap();

        // In-memory result and the re-read file agree: old id gone, new id present with the
        // new color and the original created_at.
        for a in [
            article.annotations.clone(),
            read_article(lib, "abcdef123456").await.unwrap().annotations,
        ] {
            assert_eq!(a.len(), 1);
            assert_eq!(a[0].id, "new-id");
            assert_eq!(a[0].color, "red");
            assert_eq!(a[0].created_at, created_at);
        }
    }

    #[tokio::test]
    async fn add_annotation_rejects_same_id_color_change() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            "body",
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        add_annotation(
            lib,
            "abcdef123456",
            annotation("a1", "yellow", "n"),
            WriteToken::for_test(),
        )
        .await
        .unwrap();

        // Same id, different color: refused — that path is replace_annotation's.
        let err = add_annotation(
            lib,
            "abcdef123456",
            annotation("a1", "red", "n"),
            WriteToken::for_test(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, EntryError::ColorChangeRequiresReplace));

        // Same id, same color, new note: a legitimate edit, accepted.
        let article = add_annotation(
            lib,
            "abcdef123456",
            annotation("a1", "yellow", "edited"),
            WriteToken::for_test(),
        )
            .await
            .unwrap();
        assert_eq!(article.annotations[0].content, "edited");
    }

    #[tokio::test]
    async fn replace_annotation_missing_old_id_adds_the_new_annotation() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            "body",
            WriteToken::for_test(),
        )
        .await
        .unwrap();

        let article = replace_annotation(
            lib,
            "abcdef123456",
            "never-existed",
            annotation("new-id", "red", ""),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(article.annotations.len(), 1);
        assert_eq!(article.annotations[0].id, "new-id");
        assert!(!article.annotations[0].created_at.is_empty());
    }
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
