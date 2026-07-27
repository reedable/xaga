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

    let (created_at, existing_annotations, existing_tags) = if file_exists {
        let raw = crate::fs::read_md_capped(&path).await?;
        match parse_document(&raw) {
            Ok((fm, _)) => (fm.created_at, fm.annotations, fm.tags),
            Err(_) => (chrono::Utc::now().to_rfc3339(), vec![], vec![]),
        }
    } else {
        (chrono::Utc::now().to_rfc3339(), vec![], vec![])
    };

    // Rewrite every remote image reference to its ext-less local ref
    // `../images/<id>/<hash>`, writing files for the URLs whose bytes `url_data`
    // supplied. Images are written BEFORE the `.md` below, so a committed
    // reference with a file behind it always has durable bytes (see
    // docs/architecture/crash_consistency.md). The `hash → url` map is stored in
    // frontmatter — the only place the remote URL survives.
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
                if !extraction.url_map.iter().any(|(h, _)| h == &hash) {
                    extraction.url_map.push((hash.clone(), url.to_string()));
                }
                Some(crate::library::entry_image_ref(id, &hash))
            }
            None => Some(url.to_string()),
        },
        None => None,
    };

    crate::persistence::images::write_entry_images(library_dir, id, &extraction.images, token).await;

    let images: Vec<crate::types::ImageRef> = extraction
        .url_map
        .into_iter()
        .map(|(hash, url)| crate::types::ImageRef { hash, url })
        .collect();

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
        content: extraction.body,
        annotations: with_reference(existing_annotations, id),
        favicon,
        images,
    };
    crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;

    // Sweep image files this entry's body no longer references. Keeps files whose
    // hash the committed body still points at (matched by file stem, since the
    // ref is ext-less and the file is `<hash>.<ext>`), so a previously-fetched
    // image that is still referenced but not re-fetched this save is preserved.
    // Best-effort; never fails the save.
    let keep: std::collections::HashSet<String> = article
        .images
        .iter()
        .map(|img| img.hash.clone())
        .collect();
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
        // Frontmatter map retained the URL, and the file was written.
        assert_eq!(article.images.len(), 1);
        assert_eq!(article.images[0].url, url);
        let images_dir = crate::library::entry_images_dir(lib, "abcdef123456");
        assert_eq!(std::fs::read_dir(&images_dir).unwrap().count(), 1);
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
    async fn save_article_broken_ref_when_bytes_absent_but_url_mapped() {
        // A failed fetch: no bytes → no file, body has a broken local ref, and the
        // URL is retained in the frontmatter map for recovery.
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
        let hash = crate::persistence::images::image_url_hash(url);
        assert!(article
            .content
            .contains(&format!("../images/abcdef123456/{hash}")));
        assert_eq!(article.images.len(), 1);
        assert_eq!(article.images[0].url, url);
        // No file was written.
        let images_dir = crate::library::entry_images_dir(lib, "abcdef123456");
        assert!(std::fs::read_dir(&images_dir).map(|d| d.count()).unwrap_or(0) == 0);
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
        assert!(article.images.is_empty());
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
        // The URL survives only in the frontmatter map, so a lost file is recoverable.
        assert!(article.images.iter().any(|i| i.url == favicon_url));
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
        assert!(article.images.iter().any(|i| i.url == favicon_url));
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
    let images = fm.images.clone();
    Article {
        meta: md_to_entry(Markdown {
            id: article_id.to_string(),
            fm,
            body: String::new(),
        }),
        content: body,
        annotations: with_reference(annotations, article_id),
        favicon,
        images,
    }
}
