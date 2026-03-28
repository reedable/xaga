use std::path::Path;
use std::sync::Arc;

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

/// Reads a `.md` file, computes its checksum, and — if not already in the
/// database — embeds it with `embedder` and upserts the result.
///
/// Shared by both the Tauri Reconciler and the `xaga-cli embeddings recover`
/// command so neither duplicates this logic.
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
    let checksum = fm.content_checksum(&body);
    let stored = db.get_checksum(&id).await?;

    log::debug!("embed_file: id={id} stored_checksum={stored:?} computed_checksum={checksum:?}");

    if stored.as_deref() == Some(checksum.as_str()) {
        log::debug!("embed_file: id={id} checksum unchanged — skipping");
        return Ok(());
    }

    if stored.is_some() {
        log::debug!("embed_file: id={id} checksum changed — re-embedding");
    } else {
        log::debug!("embed_file: id={id} not in db — embedding");
    }

    log::debug!("embed_file: embedding id={id} title={:?}", fm.title);

    let vector =
        compute_vector(embedder, fm.title.clone(), fm.embed_text(&body).to_string()).await?;

    db.upsert(fm.to_embedding_entry(id, &body, vector)).await
}
