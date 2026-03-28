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
) -> Result<Vec<Entry>> {
    match source {
        ItemSource::FeedEntries { url } => {
            // log::debug!("list_items: FeedEntries");
            let mut entries = feeds::list_entries(library, url.as_deref(), item_filter)
                .await
                .map_err(ItemError::Feed)?;
            filter::annotate_unread_status(library, &mut entries).await;
            Ok(entries)
        }
    }
}

/// Fetch embedded items from `source`, preserving similarity rank, then filter and annotate.
pub async fn list_embedded_items(
    library: &Path,
    source: EmbeddedItemSource,
    item_filter: ItemFilter,
) -> Result<Vec<EmbeddedEntry>> {
    match source {
        EmbeddedItemSource::RelatedTo { id, limit, min_similarity, max_similarity } => {
            // Preserve similarity rank — filter only, no sort.
            // log::debug!("list_embedded_items: calling find_related");
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_related(&id, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(library, &mut entries).await;
            Ok(entries)
        }
        EmbeddedItemSource::RelatedToBookmarks { limit, min_similarity, max_similarity } => {
            let bookmark_ids: Vec<String> = crate::bookmarks::list_bookmarks(library)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.id)
                .collect();
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_related_to_many(&bookmark_ids, limit, min_similarity, max_similarity).await?;
            let mut entries = filter::filter_items(library, entries, &item_filter)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(library, &mut entries).await;
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
            let mut entries = filter::filter_items(library, entries, &item_filter)
                .await
                .map_err(ItemError::Io)?;
            filter::annotate_embedded_unread_status(library, &mut entries).await;
            Ok(entries)
        }
    }
}
