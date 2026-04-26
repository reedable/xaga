use std::collections::HashSet;
use std::path::Path;

use thiserror::Error;

use crate::embeddings::EmbeddingsDb;
use crate::feeds;
use crate::filter;
use crate::types::{EmbeddedEntry, Entry, ItemFilter};

// Forwarding shim — all types have moved to `crate::types`.
pub use crate::types::{Document, Item};

// ── ItemSource ────────────────────────────────────────────────────────────────

/// Describes where to fetch the candidate items before filtering.
pub enum ItemSource {
    /// Items that are feed entries, optionally scoped to one feed URL.
    FeedEntries { url: Option<String> },
}

/// Describes the source for an embedding similarity query.
pub enum EmbeddedItemSource {
    /// Items related to a given item, ranked by embedding similarity.
    RelatedTo { id: String, limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
    /// Items related to the centroid of all bookmarked items.
    RelatedToBookmarks { limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
    /// Items related to the centroid of all topics.
    RelatedToTopics { limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
    /// Items related to the union of all annotation embeddings.
    RelatedToAnnotations { limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
    /// Items related to an arbitrary pre-computed query vector (e.g. from embedded search text).
    RelatedToVector { vector: Vec<f32>, limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ItemError {
    #[error(transparent)]
    Feed(#[from] feeds::FeedError),
    #[error(transparent)]
    Embeddings(#[from] crate::embeddings::EmbeddingsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ItemError>;

// ── Public API ────────────────────────────────────────────────────────────────

/// Fetch items from `source`, then filter and annotate them.
pub async fn list_items(
    library: &Path,
    source: ItemSource,
    item_filter: ItemFilter,
    unread: &HashSet<String>,
) -> Result<Vec<Entry>> {
    match source {
        ItemSource::FeedEntries { url } => {
            let mut entries = feeds::list_entries(library, url.as_deref(), item_filter, unread)
                .await
                .map_err(ItemError::Feed)?;
            filter::annotate_unread_status(unread, &mut entries);
            Ok(entries)
        }
    }
}

/// Fetch embedded items from `source`, preserving similarity rank, then filter and annotate.
pub async fn list_embedded_items(
    library: &Path,
    source: EmbeddedItemSource,
    item_filter: ItemFilter,
    unread: &HashSet<String>,
) -> Result<Vec<EmbeddedEntry>> {
    match source {
        EmbeddedItemSource::RelatedTo { id, limit, min_similarity, max_similarity } => {
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_related(&id, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter, unread)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(unread, &mut entries);
            Ok(entries)
        }
        EmbeddedItemSource::RelatedToBookmarks { limit, min_similarity, max_similarity } => {
            let unread = filter::unread_ids(library).await;
            let bookmark_ids: Vec<String> = crate::entries::list_entries(library, &unread)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|e| e.is_bookmarked)
                .map(|e| e.id)
                .collect();
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_related_to_many(&bookmark_ids, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter, &unread)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(&unread, &mut entries);
            Ok(entries)
        }
        EmbeddedItemSource::RelatedToTopics { limit, min_similarity, max_similarity } => {
            let topic_ids: Vec<String> = crate::topics::list_topics(library)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.id)
                .collect();
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_related_to_many(&topic_ids, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter, unread)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(unread, &mut entries);
            Ok(entries)
        }
        EmbeddedItemSource::RelatedToAnnotations { limit, min_similarity, max_similarity } => {
            let db = EmbeddingsDb::open(library).await?;
            let annotation_ids: Vec<String> = db
                .list_annotations()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|a| a.id)
                .collect();
            let entries = db.find_related_to_many(&annotation_ids, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter, unread)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(unread, &mut entries);
            Ok(entries)
        }
        EmbeddedItemSource::RelatedToVector { vector, limit, min_similarity, max_similarity } => {
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_by_vector(vector, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter, unread)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(unread, &mut entries);
            Ok(entries)
        }
    }
}
