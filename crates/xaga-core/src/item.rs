use std::path::Path;

use thiserror::Error;

use crate::embeddings::EmbeddingsDb;
use crate::feeds;
use crate::filter;
use crate::types::{EmbeddedEntry, EmbeddedFeed, EmbeddedItem, EmbeddedTopic, Feed, ItemFilter, Kind};

// Forwarding shim — all types have moved to `crate::types`.
pub use crate::types::{Document, Item};

// ── EmbeddedItemSource ──────────────────────────────────────────────────────────

/// Describes the source for an embedding similarity query.
pub enum EmbeddedItemSource {
    /// Items related to a given item, ranked by embedding similarity.
    RelatedTo { id: String, limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
    /// Items related to an arbitrary pre-computed query vector (e.g. from embedded search text).
    RelatedToVector { vector: Vec<f32>, limit: Option<usize>, min_similarity: Option<f32>, max_similarity: Option<f32> },
    /// Topics ranked by similarity to a given item.
    TopicsRelatedTo { id: String, limit: Option<usize>, min_similarity: Option<f32> },
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

/// Fetch embedded items from `source`, preserving similarity rank, then filter.
/// `is_unread` and `is_bookmarked` are read directly from LanceDB tags.
pub async fn list_embedded_items(
    library: &Path,
    source: EmbeddedItemSource,
    item_filter: ItemFilter,
) -> Result<Vec<EmbeddedEntry>> {
    match source {
        EmbeddedItemSource::RelatedTo { id, limit, min_similarity, max_similarity } => {
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_related(&id, limit, min_similarity, max_similarity, None).await?;
            filter::filter_items(entries, &item_filter)
                .await
                .map_err(ItemError::Io)
        }
        EmbeddedItemSource::RelatedToVector { vector, limit, min_similarity, max_similarity } => {
            let db = EmbeddingsDb::open(library).await?;
            let entries = db.find_by_vector(vector, limit, min_similarity, max_similarity).await?;
            filter::filter_items(entries, &item_filter)
                .await
                .map_err(ItemError::Io)
        }
        EmbeddedItemSource::TopicsRelatedTo { id, limit, min_similarity } => {
            let db = EmbeddingsDb::open(library).await?;
            db.find_related(&id, limit, min_similarity, None, Some(Kind::Topic))
                .await
                .map_err(ItemError::Embeddings)
        }
    }
}

/// Enriches raw LanceDB results: rows with no URL are topic rows.
/// For each, loads the full Topic from disk and wraps it as EmbeddedItem::Topic.
/// Falls back to EmbeddedItem::Entry for any row that can't be resolved as a topic.
pub async fn enrich_embedded(library: &Path, entries: Vec<EmbeddedEntry>) -> Vec<EmbeddedItem> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.kind == Kind::Feed {
            // Feed stats are deferred: counts returned as 0. The server-side lookup
            // (read the feed `.atom` membership + intersect the unread/bookmarked id-sets
            // via `feeds::feed_summary`) is TECH_DEBT.md "Search and counts" → embedded
            // feed results return counts = 0.
            result.push(EmbeddedItem::Feed(EmbeddedFeed {
                feed: Feed {
                    id: entry.entry.id.clone(),
                    url: entry.entry.url.clone(),
                    title: entry.entry.title.clone(),
                    description: None,
                    entry_count: 0,
                    unread_count: 0,
                    created_at: entry.entry.created_at.clone(),
                    updated_at: entry.entry.updated_at.clone(),
                    // Pinned state is not shown in search results; deferred like the counts above.
                    is_pinned: false,
                    members: vec![],
                },
                kind: entry.kind,
                similarity: entry.similarity,
            }));
            continue;
        }
        if entry.entry.url.is_empty() {
            if let Ok(topic) = crate::topics::read_topic(library, &entry.entry.id).await {
                result.push(EmbeddedItem::Topic(EmbeddedTopic { topic, similarity: entry.similarity }));
                continue;
            }
        }
        result.push(EmbeddedItem::Entry(entry));
    }
    result
}
