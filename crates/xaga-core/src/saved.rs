use std::path::Path;

use thiserror::Error;

use crate::bookmarks::{self, BookmarksError};
use crate::embeddings::{EmbeddingsDb, EmbeddingsError};
use crate::types::SavedItem;

#[derive(Debug, Error)]
pub enum SavedError {
    #[error("Bookmarks error: {0}")]
    Bookmarks(#[from] BookmarksError),
    #[error("Embeddings error: {0}")]
    Embeddings(#[from] EmbeddingsError),
}

/// Returns all bookmarks and annotations merged into a single list, sorted by
/// `created_at` descending (newest first).
///
/// Bookmarks have `reference` and `color` absent; annotations have both set.
pub async fn list_saved(data_dir: &Path) -> Result<Vec<SavedItem>, SavedError> {
    let (bookmarks_result, annotations_result) = tokio::join!(
        bookmarks::list_bookmarks(data_dir),
        async {
            let db = EmbeddingsDb::open(data_dir).await?;
            db.list_annotations().await
        },
    );

    let bookmarks = bookmarks_result?;
    let annotations = annotations_result?;

    let mut items: Vec<SavedItem> = bookmarks
        .into_iter()
        .map(|e| SavedItem {
            id: e.id,
            title: e.title,
            created_at: e.created_at,
            url: e.url,
            reference: None,
            color: None,
        })
        .chain(annotations.into_iter().map(|a| SavedItem {
            id: a.id,
            title: a.title,
            created_at: a.created_at,
            url: a.url,
            reference: Some(a.reference),
            color: Some(a.color),
        }))
        .collect();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}
