use std::path::Path;

use crate::{
    library_writer::WriteToken,
    md::Frontmatter,
    md::{FrontmatterAnnotation, Markdown, md_to_entry, parse_document},
    persistence::entries::{EntryError, find_entry_path},
    types::{Annotation, Article},
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
    url_data: &std::collections::HashMap<String, String>,
    token: WriteToken,
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

    // Fail closed before writing images or replacing any existing document.
    let existing = if file_exists {
        let raw = crate::fs::read_md_capped(&path).await?;
        let (fm, _) = parse_document(&raw)?;
        Some((raw, fm))
    } else {
        None
    };
    let favicon = favicon.or_else(|| existing.as_ref().and_then(|(_, fm)| fm.favicon.as_deref()));

    // Prepare candidate local URLs, write image files before Markdown, and
    // commit only URLs whose files were successfully persisted.
    let mut extraction = crate::persistence::images::extract_images(id, markdown, url_data);

    // The favicon is localised exactly like a body image: its bytes join the same
    // per-entry directory and the stored value becomes a local ref, so displaying
    // the entry issues no request from the main window. `extract_favicon` returns
    // `None` for a value that is already local, which is what makes a re-save a
    // no-op rather than a re-hash.
    let favicon = match favicon {
        Some(url) => match crate::persistence::images::extract_favicon(url, url_data) {
            Some((hash, image)) => {
                if let Some(image) = image {
                    if !extraction
                        .images
                        .iter()
                        .any(|existing| existing.file_name == image.file_name)
                    {
                        extraction.images.push(image);
                    }
                }
                Some(crate::library::entry_image_ref(id, &hash))
            }
            None => Some(url.to_string()),
        },
        None => None,
    };

    let landed =
        crate::persistence::images::write_entry_images(library_dir, id, &extraction.images, token)
            .await;

    let body = extraction.persisted_body(&landed);
    let mut updates = vec![
        ("source", serde_json::json!(normalized)),
        ("title", serde_json::json!(title)),
    ];
    // Initial metadata is supplied only for a newly discovered article.
    if existing.is_none() {
        updates.push(("created", serde_json::json!(chrono::Utc::now().to_rfc3339())));
        updates.push(("aliases", serde_json::json!([title])));
    }
    for (key, value) in [
        ("published", published_at),
        ("updated", updated_at),
        ("summary", summary),
        ("favicon", favicon.as_deref()),
    ] {
        if let Some(value) = value {
            updates.push((key, serde_json::json!(value)));
        }
    }
    let rendered = super::documents::upsert(&path, &updates, Some(&body), token).await?;
    let (fm, body) = parse_document(&rendered)?;
    let article = article_from_document(id, fm, body);

    // Sweep image files no longer referenced by the committed document, including
    // frontmatter (summaries, annotations, custom fields). Keeps files whose
    // hash the document still points at (matched by file stem, since the
    // ref is ext-less and the file is `<hash>.<ext>`), so a previously-fetched
    // image that is still referenced but not re-fetched this save is preserved.
    // Best-effort; never fails the save.
    let keep = crate::persistence::images::referenced_image_hashes(id, &rendered);
    crate::persistence::images::gc_entry_images(library_dir, id, &keep, token).await;

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

    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;

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
    super::documents::upsert(
            &path,
            &[(
                "annotations",
                serde_json::json!(
                    article
                        .annotations
                        .iter()
                        .map(FrontmatterAnnotation::from)
                        .collect::<Vec<_>>()
                ),
            )],
            None, _token,
    )
    .await?;
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;

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
    super::documents::upsert(
            &path,
            &[(
                "annotations",
                serde_json::json!(
                    article
                        .annotations
                        .iter()
                        .map(FrontmatterAnnotation::from)
                        .collect::<Vec<_>>()
                ),
            )],
            None, _token,
    )
    .await?;
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;

    let annotations: Vec<Annotation> = fm
        .annotations
        .iter()
        .filter(|a| a.id != annotation_id)
        .cloned()
        .collect();

    let favicon = fm.favicon.clone();
    let article = article_from_fm(article_id, fm, annotations, favicon, body);
    super::documents::upsert(
            &path,
            &[(
                "annotations",
                serde_json::json!(
                    article
                        .annotations
                        .iter()
                        .map(FrontmatterAnnotation::from)
                        .collect::<Vec<_>>()
                ),
            )],
            None, _token,
    )
    .await?;
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

    const USER_DOCUMENT: &str = r#"---
source: https://example.com/a
title: Original
created: '2026-01-01T00:00:00Z'
published: '2026-01-02T00:00:00Z'
updated: '2026-01-03T00:00:00Z'
summary: |
  First line
  Second line
# User-maintained metadata
aliases: [Original, My alias]
tags: [unread, bookmark, research, 'project/custom']
custom:
  nested: [one, two]
annotations:
  - id: existing
    text: [highlighted text]
    occurrence: [0]
    color: yellow
    note: My irreplaceable note
    created_at: '2026-01-04T00:00:00Z'
favicon: ../images/abcdef123456/112233445566
---

Original body


"#;

    fn user_document(lib: &Path) -> std::path::PathBuf {
        let path = crate::library::entry_path(lib, "abcdef123456");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, USER_DOCUMENT).unwrap();
        path
    }

    #[tokio::test]
    async fn refresh_preserves_user_frontmatter_and_absent_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = user_document(dir.path());
        let article = save_article(
            dir.path(),
            "abcdef123456",
            "https://example.com/a",
            "Refreshed",
            None,
            None,
            None,
            None,
            "New body",
            &no_data(),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let stored = std::fs::read_to_string(path).unwrap();
        let stored_fields = crate::md::test_frontmatter_values(&stored);
        let original_fields = crate::md::test_frontmatter_values(USER_DOCUMENT);
        for key in ["aliases", "tags", "custom", "annotations"] {
            assert_eq!(stored_fields[key], original_fields[key], "changed {key}");
        }
        assert_eq!(
            article.meta.description.as_deref(),
            Some("First line\nSecond line\n")
        );
        assert_eq!(
            article.meta.published_at.as_deref(),
            Some("2026-01-02T00:00:00Z")
        );
        assert_eq!(
            article.meta.updated_at.as_deref(),
            Some("2026-01-03T00:00:00Z")
        );
        assert!(article.meta.is_unread);
        assert!(article.meta.is_bookmarked);
        assert_eq!(article.annotations[0].content, "My irreplaceable note");
        assert_eq!(
            article.favicon.as_deref(),
            Some("../images/abcdef123456/112233445566")
        );
        assert_eq!(article.content, "New body");
    }

    #[tokio::test]
    async fn malformed_frontmatter_refuses_refresh_without_changing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = user_document(dir.path());
        let malformed = USER_DOCUMENT.replace("occurrence: [0]", "occurrence: not-a-list");
        std::fs::write(&path, &malformed).unwrap();
        let result = save_article(
            dir.path(),
            "abcdef123456",
            "https://example.com/a",
            "Refreshed",
            None,
            None,
            None,
            None,
            "Replacement",
            &no_data(),
            WriteToken::for_test(),
        )
        .await;
        assert!(matches!(result, Err(EntryError::Malformed(_))));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
        assert!(!crate::library::entry_images_dir(dir.path(), "abcdef123456").exists());
    }

    #[tokio::test]
    async fn annotation_and_tag_edits_preserve_unrelated_frontmatter_and_body() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let path = user_document(lib);
        add_annotation(
            lib,
            id,
            annotation("new", "yellow", "New note"),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        replace_annotation(
            lib,
            id,
            "new",
            annotation("replacement", "blue", "Recolored"),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        remove_annotation(lib, id, "replacement", WriteToken::for_test())
            .await
            .unwrap();
        let tags = vec![
            "bookmark".into(),
            "research".into(),
            "project/custom".into(),
        ];
        crate::persistence::entries::set_entry_tags(lib, id, tags.clone(), WriteToken::for_test())
            .await
            .unwrap();
        let stored = std::fs::read_to_string(path).unwrap();
        for key in ["aliases", "custom"] {
            assert_eq!(crate::md::test_frontmatter_values(&stored)[key], crate::md::test_frontmatter_values(USER_DOCUMENT)[key]);
        }
        assert!(stored.ends_with("---\n\nOriginal body\n\n\n"));
        let (fm, _) = parse_document(&stored).unwrap();
        assert_eq!(fm.tags, tags);
        assert_eq!(fm.annotations.len(), 1);
        assert_eq!(fm.annotations[0].content, "My irreplaceable note");
        assert_eq!(fm.summary.as_deref(), Some("First line\nSecond line\n"));
    }

    #[tokio::test]
    async fn refresh_keeps_images_referenced_only_from_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let image_url = "https://example.com/comic.png";
        let first = save_article(
            lib,
            id,
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            &format!("![comic]({image_url})"),
            &data(image_url, PNG_DATA),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let hash = crate::persistence::images::image_url_hash(image_url);
        save_article(
            lib,
            id,
            "https://example.com/a",
            "Title",
            None,
            None,
            Some(&first.content),
            None,
            "Text-only replacement body",
            &no_data(),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert!(crate::library::entry_image_path(lib, id, &format!("{hash}.png")).exists());
    }

    const PNG_DATA: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC";

    fn no_data() -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }

    fn data(url: &str, data_uri: &str) -> std::collections::HashMap<String, String> {
        std::iter::once((url.to_string(), data_uri.to_string())).collect()
    }

    #[tokio::test]
    async fn save_article_stores_images_and_body_is_local_refs() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let url = "https://example.com/p.png";
        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            &format!("intro ![a cat]({url}) outro"),
            &data(url, PNG_DATA),
            WriteToken::for_test(),
        )
        .await
        .unwrap();

        let article = read_article(lib, "abcdef123456").await.unwrap();
        let hash = crate::persistence::images::image_url_hash(url);
        // Body is a local ref; no remote URL and no base64 survive.
        assert!(article
            .content
            .contains(&format!("../images/abcdef123456/{hash}")));
        assert!(!article.content.contains("http"));
        assert!(!article.content.contains("base64"));
        // The file was written.
        let images_dir = crate::library::entry_images_dir(lib, "abcdef123456");
        assert_eq!(std::fs::read_dir(&images_dir).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn legacy_image_map_is_ignored_on_read_and_removed_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let id = "abcdef123456";
        let path = crate::library::entry_path(dir.path(), id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Even an obsolete map with an unexpected shape is unused metadata.
        let raw = "---\nsource: https://example.com/article\ntitle: Legacy\ncreated_at: 2026-01-01T00:00:00Z\nimages: obsolete-metadata\n---\n![alt](../images/abcdef123456/123456abcdef)\n";
        std::fs::write(&path, raw).unwrap();
        let article = read_article(dir.path(), id).await.unwrap();
        assert!(article.content.contains("../images/abcdef123456/123456abcdef"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        save_article(dir.path(), id, &article.meta.url, "Updated", None, None, None, None,
            &article.content, &no_data(), WriteToken::for_test()).await.unwrap();
        let stored = std::fs::read_to_string(&path).unwrap();
        assert!(!crate::md::test_frontmatter_values(&stored).contains_key("images"));
        assert!(!stored.contains("obsolete-metadata"));
        assert!(stored.contains("![alt](../images/abcdef123456/123456abcdef)"));
        // Metadata-only saves also omit the unsupported field.
        std::fs::write(&path, raw).unwrap();
        crate::persistence::entries::set_entry_tags(dir.path(), id, vec!["bookmark".into()], WriteToken::for_test()).await.unwrap();
        let stored = std::fs::read_to_string(&path).unwrap();
        assert!(!crate::md::test_frontmatter_values(&stored).contains_key("images"));
        assert!(read_article(dir.path(), id).await.unwrap().meta.is_bookmarked);
    }

    #[tokio::test]
    async fn resave_local_body_and_favicon_keeps_files_without_frontmatter_map() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let image_url = "https://example.com/image.png";
        let icon_url = "https://example.com/icon.png";
        let mut bytes = data(image_url, PNG_DATA);
        bytes.extend(data(icon_url, PNG_DATA));
        let article = save_article(lib, id, "https://example.com/a", "T", None, None, None,
            Some(icon_url), &format!("![alt]({image_url})"), &bytes, WriteToken::for_test()).await.unwrap();
        let image_dir = crate::library::entry_images_dir(lib, id);
        let orphan = image_dir.join("000000000000.png");
        std::fs::write(&orphan, b"orphan").unwrap();
        save_article(lib, id, "https://example.com/a", "T", None, None, None,
            article.favicon.as_deref(), &article.content, &no_data(), WriteToken::for_test()).await.unwrap();
        for url in [image_url, icon_url] {
            assert!(image_dir.join(format!("{}.png", crate::persistence::images::image_url_hash(url))).exists());
        }
        assert!(!orphan.exists());
        let stored = std::fs::read_to_string(crate::library::entry_path(lib, id)).unwrap();
        assert!(!stored.contains("images:"));
    }

    #[tokio::test]
    async fn save_article_gcs_images_no_longer_referenced() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let url_a = "https://example.com/a.png";
        let url_b = "https://example.com/b.svg";
        let svg = "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=";

        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, None,
            &format!("![a]({url_a})"), &data(url_a, PNG_DATA), WriteToken::for_test(),
        )
        .await
        .unwrap();
        let images_dir = crate::library::entry_images_dir(lib, id);
        assert_eq!(std::fs::read_dir(&images_dir).unwrap().count(), 1);

        // Re-save referencing a different image: the first file is swept.
        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, None,
            &format!("![b]({url_b})"), &data(url_b, svg), WriteToken::for_test(),
        )
        .await
        .unwrap();
        let files: Vec<_> = std::fs::read_dir(&images_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with(".svg"));
    }

    #[tokio::test]
    async fn save_article_keeps_remote_url_when_bytes_absent() {
        // No captured bytes: retain the remote URL without creating a local path.
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let url = "https://example.com/missing.png";
        save_article(
            lib, "abcdef123456", "https://example.com/a", "T", None, None, None, None,
            &format!("![x]({url})"), &no_data(), WriteToken::for_test(),
        )
        .await
        .unwrap();
        let article = read_article(lib, "abcdef123456").await.unwrap();
        assert_eq!(article.content, format!("![x]({url})"));
        // No file was written.
        let images_dir = crate::library::entry_images_dir(lib, "abcdef123456");
        assert!(std::fs::read_dir(&images_dir).map(|d| d.count()).unwrap_or(0) == 0);
    }

    #[tokio::test]
    async fn partial_image_write_failure_preserves_remote_url_and_successful_local_image() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let good = "https://example.com/good.png";
        let bad = "https://example.com/bad.png";
        let good_hash = crate::persistence::images::image_url_hash(good);
        let bad_hash = crate::persistence::images::image_url_hash(bad);
        // A directory at the destination makes atomic file replacement fail.
        std::fs::create_dir_all(crate::library::entry_image_path(lib, id, &format!("{bad_hash}.png"))).unwrap();
        let mut bytes = data(good, PNG_DATA);
        bytes.extend(data(bad, PNG_DATA));
        let body = format!("![bad]({bad}) ![good]({good}) ![again]({bad})");
        save_article(lib, id, "https://example.com/a", "T", None, None, None, None,
            &body, &bytes, WriteToken::for_test()).await.unwrap();
        let saved = read_article(lib, id).await.unwrap();
        assert_eq!(saved.content, format!("![bad]({bad}) ![good](../images/{id}/{good_hash}) ![again]({bad})"));
        assert!(crate::library::entry_image_path(lib, id, &format!("{good_hash}.png")).is_file());
    }

    #[tokio::test]
    async fn save_article_leaves_an_image_free_body_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let body = "# Heading\n\nProse with a [link](https://x.test/) only.\n";
        save_article(
            lib, "abcdef123456", "https://example.com/a", "T", None, None, None, None,
            body, &no_data(), WriteToken::for_test(),
        )
        .await
        .unwrap();
        let article = read_article(lib, "abcdef123456").await.unwrap();
        assert_eq!(article.content, body.trim_end());
        assert!(!crate::library::entry_images_dir(lib, "abcdef123456").exists());
    }

    #[tokio::test]
    async fn save_article_stores_the_favicon_and_frontmatter_holds_a_local_ref() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let favicon_url = "https://example.com/favicon.ico";

        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, Some(favicon_url),
            "body", &data(favicon_url, PNG_DATA), WriteToken::for_test(),
        )
        .await
        .unwrap();

        let article = read_article(lib, id).await.unwrap();
        let hash = crate::persistence::images::image_url_hash(favicon_url);

        // Stored value is a local ref — nothing for the main window to fetch.
        assert_eq!(
            article.favicon.as_deref(),
            Some(crate::library::entry_image_ref(id, &hash).as_str())
        );
        assert!(crate::library::entry_image_path(lib, id, &format!("{hash}.png")).exists());
    }

    #[tokio::test]
    async fn save_article_keeps_the_favicon_ref_when_bytes_are_absent() {
        // A failed favicon fetch degrades to a broken ref, never to a live URL.
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let favicon_url = "https://example.com/favicon.ico";

        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, Some(favicon_url),
            "body", &no_data(), WriteToken::for_test(),
        )
        .await
        .unwrap();

        let article = read_article(lib, id).await.unwrap();
        assert!(article.favicon.as_deref().unwrap().starts_with("../images/"));
        assert!(!article.favicon.as_deref().unwrap().contains("http"));
    }

    #[tokio::test]
    async fn save_article_does_not_re_hash_an_already_local_favicon() {
        // The re-save path: a second save is fed the ref it wrote last time.
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let favicon_url = "https://example.com/favicon.ico";

        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, Some(favicon_url),
            "body", &data(favicon_url, PNG_DATA), WriteToken::for_test(),
        )
        .await
        .unwrap();
        let first = read_article(lib, id).await.unwrap().favicon.unwrap();

        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, Some(&first),
            "body", &no_data(), WriteToken::for_test(),
        )
        .await
        .unwrap();

        let article = read_article(lib, id).await.unwrap();
        assert_eq!(article.favicon.as_deref(), Some(first.as_str()));
    }

    #[tokio::test]
    async fn save_article_gc_keeps_the_favicon_file() {
        // The favicon is referenced from frontmatter, not the body — the sweep
        // must still treat it as live.
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "abcdef123456";
        let favicon_url = "https://example.com/favicon.ico";
        let body_url = "https://example.com/p.png";
        let mut url_data = data(favicon_url, PNG_DATA);
        url_data.insert(body_url.to_string(), PNG_DATA.to_string());

        save_article(
            lib, id, "https://example.com/a", "T", None, None, None, Some(favicon_url),
            &format!("![a]({body_url})"), &url_data, WriteToken::for_test(),
        )
        .await
        .unwrap();

        let hash = crate::persistence::images::image_url_hash(favicon_url);
        assert!(crate::library::entry_image_path(lib, id, &format!("{hash}.png")).exists());
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
            &no_data(),
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
            &no_data(),
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
            &no_data(),
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
