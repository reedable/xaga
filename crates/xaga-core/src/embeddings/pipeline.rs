use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use sha2::Digest;

use crate::types::Annotation;
use crate::util::Frontmatter;
use super::db::EmbeddingEntry;
use super::model::Embedder;
use super::{EmbeddingsDb, EmbeddingsError};

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
/// Each annotation row uses the composite ID `"{article_id}:{annotation.id}"`.
/// The title is the highlighted text; the note (if present) is the embedding body.
async fn embed_annotations(
    article_id: &str,
    fm: &Frontmatter,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let existing = db.annotation_meta_for_article(article_id).await?;

    let current_ids: HashSet<String> = fm
        .annotations
        .iter()
        .map(|a| format!("{article_id}:{}", a.id))
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
        let row_id = format!("{article_id}:{}", annotation.id);
        let checksum = annotation_checksum(annotation);

        if existing.get(&row_id).map_or(false, |s| s == &checksum) {
            log::debug!("embed_file: annotation {row_id} checksum unchanged — skipping");
            continue;
        }

        log::debug!("embed_file: embedding annotation {row_id}");
        let title = annotation.text.join(" ");
        let body = annotation.note.clone().unwrap_or_default();
        let vector = compute_vector(embedder, title.clone(), body).await?;

        db.upsert(EmbeddingEntry {
            id: row_id,
            source: fm.source.clone(),
            title,
            created_at: annotation.created_at.clone(),
            published_at: fm.published_at.clone().unwrap_or_default(),
            updated_at: fm.updated_at.clone().unwrap_or_default(),
            checksum,
            vector,
        })
        .await?;
    }

    Ok(())
}

/// Reads a `.md` file and brings LanceDB in sync with its current content:
///
/// 1. Re-embeds the article if its checksum (title + body/summary) changed.
/// 2. Reconciles annotation rows if the file has annotations.
///
/// Shared by both the Tauri embedding service and `xaga-cli embeddings recover`.
pub async fn embed_file(
    path: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(EmbeddingsError::Io)?;

    let (fm, body) = crate::util::parse_document(&raw)
        .ok_or_else(|| EmbeddingsError::Parse(path.display().to_string()))?;

    let id = crate::util::item_id(path);

    // ── Article ──────────────────────────────────────────────────────────────

    let checksum = fm.content_checksum(&body);
    let stored = db.get_checksum(&id).await?;

    log::debug!("embed_file: id={id} stored_checksum={stored:?} computed_checksum={checksum:?}");

    if stored.as_deref() != Some(checksum.as_str()) {
        if stored.is_some() {
            log::debug!("embed_file: id={id} checksum changed — re-embedding");
        } else {
            log::debug!("embed_file: id={id} not in db — embedding");
        }
        log::debug!("embed_file: embedding id={id} title={:?}", fm.title);
        let vector =
            compute_vector(embedder, fm.title.clone(), fm.embed_text(&body).to_string()).await?;
        db.upsert(fm.to_embedding_entry(id.clone(), &body, vector)).await?;
    } else {
        log::debug!("embed_file: id={id} checksum unchanged — skipping article re-embed");
    }

    // ── Annotations ──────────────────────────────────────────────────────────

    if !fm.annotations.is_empty() {
        embed_annotations(&id, &fm, embedder, db).await?;
    }

    Ok(())
}
