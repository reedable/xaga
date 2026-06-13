use thiserror::Error;

use crate::embeddings::{EmbeddingsDb, EmbeddingsError};
use crate::types::SavedItem;

#[derive(Debug, Error)]
pub enum SavedError {
    #[error("Embeddings error: {0}")]
    Embeddings(#[from] EmbeddingsError),
}

/// Returns all bookmarks and annotations merged into a single list, sorted by
/// `created_at` descending (newest first).
///
/// Bookmarks have `reference` and `color` absent; annotations have both set.
/// Both halves are read from the shared LanceDB read handle; the caller is
/// responsible for reloading it to the latest version before calling.
pub async fn list_saved(db: &EmbeddingsDb) -> Result<Vec<SavedItem>, SavedError> {
    let (bookmarks_result, annotations_result) =
        tokio::join!(db.bookmarked_entries(), db.list_annotations());

    let bookmarks = bookmarks_result?;
    let annotations = annotations_result?;

    let mut items: Vec<SavedItem> = bookmarks
        .into_iter()
        .map(|e| SavedItem {
            id: e.id,
            title: e.title,
            created_at: e.created_at,
            url: e.url,
            is_unread: e.is_unread,
            is_bookmarked: true,
            reference: None,
            color: None,
        })
        .chain(annotations.into_iter().map(|a| SavedItem {
            id: a.id,
            title: a.title,
            created_at: a.created_at,
            url: a.url,
            is_unread: false,
            is_bookmarked: false,
            reference: Some(a.reference),
            color: Some(a.color),
        }))
        .collect();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}
