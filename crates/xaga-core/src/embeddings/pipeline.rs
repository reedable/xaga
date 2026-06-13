use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use sha2::Digest;

use crate::types::{Annotation, Entry, SavedItem};
use crate::util::Frontmatter;
use super::db::EmbeddingEntry;
use super::model::Embedder;
use super::{EmbeddingsDb, EmbeddingsError};

/// Constructs an `Entry` from a frontmatter reference without consuming it.
fn entry_from_fm_ref(id: &str, fm: &Frontmatter) -> Entry {
    Entry {
        id: id.to_string(),
        url: fm.source.clone(),
        title: fm.title.clone(),
        created_at: fm.created_at.clone(),
        published_at: fm.published_at.clone(),
        updated_at: fm.updated_at.clone(),
        summary: fm.summary.clone(),
        is_unread: fm.tags.contains(&"unread".to_string()),
        is_bookmarked: fm.tags.contains(&"bookmark".to_string()),
        tags: fm.tags.clone(),
    }
}

/// Compares old and new tag state; sends the entry to the appropriate channels
/// when is_unread or is_bookmarked actually changed.
fn emit_entry_changes(
    id: &str,
    fm: &Frontmatter,
    old_tags: Option<&[String]>,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) {
    let old_unread = old_tags.map_or(false, |t| t.contains(&"unread".to_string()));
    let old_bookmarked = old_tags.map_or(false, |t| t.contains(&"bookmark".to_string()));
    let new_unread = fm.tags.contains(&"unread".to_string());
    let new_bookmarked = fm.tags.contains(&"bookmark".to_string());
    if old_unread != new_unread || old_bookmarked != new_bookmarked {
        let entry = entry_from_fm_ref(id, fm);
        if old_unread != new_unread {
            let _ = unread_tx.send(entry.clone());
        }
        if old_bookmarked != new_bookmarked {
            let _ = bookmarked_tx.send(entry);
        }
    }
}

async fn compute_vector(
    embedder: &Arc<Embedder>,
    title: String,
    body: String,
) -> Result<Vec<f32>, EmbeddingsError> {
    let embedder_ref = Arc::clone(embedder);
    match tokio::task::spawn_blocking(move || embedder_ref.embed(&title, &body)).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(EmbeddingsError::Model(e.to_string())),
    }
}

/// SHA-256 checksum of an annotation's embeddable content.
fn annotation_checksum(annotation: &Annotation) -> String {
    let mut h = sha2::Sha256::new();
    h.update(annotation.text.join(" ").as_bytes());
    if let Some(note) = &annotation.note {
        h.update(b"\n\n");
        h.update(note.as_bytes());
    }
    hex::encode(h.finalize())
}

/// Reconciles annotation rows in LanceDB against the annotations currently in `fm`.
///
/// - Deletes rows for annotations removed from the file.
/// - Upserts rows for annotations that are new or whose checksum changed.
///
/// Each annotation row uses the annotation's own UUID as its DB ID.
/// The title is the highlighted text; the note (if present) is the embedding body.
/// The `reference` column holds the parent article ID; `color` holds the highlight color.
async fn embed_annotations(
    article_id: &str,
    fm: &Frontmatter,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    let existing = db.annotation_meta_for_article(article_id).await?;

    let current_ids: HashSet<String> = fm
        .annotations
        .iter()
        .map(|a| a.id.clone())
        .collect();

    let to_delete: Vec<String> = existing
        .keys()
        .filter(|id| !current_ids.contains(*id))
        .cloned()
        .collect();
    if !to_delete.is_empty() {
        log::debug!(
            "embed_file: removing {} stale annotation(s) for {article_id}",
            to_delete.len()
        );
        db.delete_by_ids(&to_delete).await?;
    }

    for annotation in &fm.annotations {
        let checksum = annotation_checksum(annotation);
        let title = annotation.text.join(" ");

        match existing.get(&annotation.id) {
            Some((stored_checksum, stored_color))
                if stored_checksum == &checksum && stored_color == &annotation.color =>
            {
                log::debug!("embed_file: annotation {} unchanged — skipping", annotation.id);
                continue;
            }
            Some((stored_checksum, stored_color)) if stored_checksum == &checksum => {
                // Color-only change: re-upsert with existing vector, no ML inference.
                log::debug!("embed_file: annotation {} color changed — upserting without re-embedding", annotation.id);
                let _ = stored_color; // suppress warning
                if let Some(mut existing_entry) = db.get_entry(&annotation.id).await? {
                    existing_entry.color = annotation.color.clone();
                    db.upsert(existing_entry).await?;
                    let _ = annotation_tx.send(SavedItem {
                        id: annotation.id.clone(),
                        title: title.clone(),
                        created_at: annotation.created_at.clone(),
                        url: fm.source.clone(),
                        is_unread: false,
                        is_bookmarked: false,
                        reference: Some(article_id.to_string()),
                        color: Some(annotation.color.clone()),
                    });
                }
                continue;
            }
            _ => {}
        }

        log::debug!("embed_file: embedding annotation {}", annotation.id);
        let body = annotation.note.clone().unwrap_or_default();
        let vector = compute_vector(embedder, title.clone(), body).await?;

        db.upsert(EmbeddingEntry {
            id: annotation.id.clone(),
            kind: "annotation".to_string(),
            source: fm.source.clone(),
            title: title.clone(),
            created_at: annotation.created_at.clone(),
            published_at: fm.published_at.clone().unwrap_or_default(),
            updated_at: fm.updated_at.clone().unwrap_or_default(),
            checksum,
            reference: article_id.to_string(),
            color: annotation.color.clone(),
            summary: String::new(),
            tags: vec![],
            topics: vec![],
            is_unread: false,
            is_bookmarked: false,
            is_pinned: false,
            vector: Some(vector),
        })
        .await?;

        let _ = annotation_tx.send(SavedItem {
            id: annotation.id.clone(),
            title,
            created_at: annotation.created_at.clone(),
            url: fm.source.clone(),
            is_unread: false,
            is_bookmarked: false,
            reference: Some(article_id.to_string()),
            color: Some(annotation.color.clone()),
        });
    }

    Ok(())
}

/// Fast path: brings the row's metadata, tags, and denormalized state in sync with
/// the `.md` file *without* running the embedder. The vector is preserved when the
/// content is unchanged, or left `None` (pending) when the row is new or its content
/// changed — `None` ⟺ still needs embedding. Annotations are reconciled separately by
/// [`embed_vector`].
pub async fn sync_row(
    path: &Path,
    db: &EmbeddingsDb,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) -> Result<(), EmbeddingsError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(EmbeddingsError::Io)?;
    let (fm, body) = crate::util::parse_document(&raw)
        .ok_or_else(|| EmbeddingsError::Parse(path.display().to_string()))?;
    let id = crate::util::item_id(path);

    let existing = db.get_entry(&id).await?;
    let checksum = fm.content_checksum(&body);
    let content_unchanged =
        existing.as_ref().map(|e| e.checksum.as_str()) == Some(checksum.as_str());

    // Preserve the existing vector only when content is unchanged; otherwise leave it
    // pending so the embed pass recomputes it.
    let vector = if content_unchanged {
        existing.as_ref().and_then(|e| e.vector.clone())
    } else {
        None
    };

    // Write only when something the row stores actually changed (state derives from tags).
    let changed = match &existing {
        None => true,
        Some(e) => {
            !content_unchanged
                || e.tags != fm.tags
                || e.topics != fm.topics
                || e.summary != fm.summary.clone().unwrap_or_default()
        }
    };

    if changed {
        let old_tags = existing.as_ref().map(|e| e.tags.clone());
        db.upsert(fm.to_embedding_entry(id.clone(), &body, vector)).await?;
        emit_entry_changes(&id, &fm, old_tags.as_deref(), unread_tx, bookmarked_tx);
    }

    Ok(())
}

/// Slow path: computes and stores the embedding vector for `id` when it is missing or
/// stale, then reconciles annotation rows. Runs the embedder.
pub async fn embed_vector(
    path: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(EmbeddingsError::Io)?;
    let (fm, body) = crate::util::parse_document(&raw)
        .ok_or_else(|| EmbeddingsError::Parse(path.display().to_string()))?;
    let id = crate::util::item_id(path);

    let existing = db.get_entry(&id).await?;
    let checksum = fm.content_checksum(&body);
    let needs_vector = match &existing {
        None => true,
        Some(e) => e.vector.is_none() || e.checksum != checksum,
    };

    if needs_vector {
        log::debug!("embed_vector: embedding id={id} title={:?}", fm.title);
        let vector =
            compute_vector(embedder, fm.title.clone(), fm.embed_text(&body).to_string()).await?;
        db.upsert(fm.to_embedding_entry(id.clone(), &body, Some(vector))).await?;
    }

    // Always reconcile annotation rows — even when fm.annotations is empty — so that
    // removing the last annotation from an article deletes its stale LanceDB row.
    // No-op when both the file and the DB have zero annotations for this article.
    embed_annotations(&id, &fm, embedder, db, annotation_tx).await?;

    Ok(())
}

/// Synchronous full sync: metadata/state (`sync_row`) then vector + annotations
/// (`embed_vector`) in one pass. Used by `xaga-cli embeddings recover`, which wants a
/// finished index rather than the service's deferred two-phase behaviour.
pub async fn embed_file(
    path: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    sync_row(path, db, unread_tx, bookmarked_tx).await?;
    embed_vector(path, embedder, db, annotation_tx).await?;
    Ok(())
}
