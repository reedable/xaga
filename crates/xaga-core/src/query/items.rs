use std::path::Path;

use thiserror::Error;

use crate::{
    embeddings::EmbeddingsDb,
    query::filter,
    types::{
        EmbeddedEntry, EmbeddedFeed, EmbeddedItem, EmbeddedTopic, Feed, ItemFilter, Kind, ListItem,
    },
};

// ── EmbeddedItemSource ──────────────────────────────────────────────────────────

/// Describes the source for an embedding similarity query.
pub enum EmbeddedItemSource {
    /// Items related to a given item, ranked by embedding similarity.
    RelatedTo {
        id: String,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    },
    /// Items related to an arbitrary pre-computed query vector (e.g. from embedded search text).
    RelatedToVector {
        vector: Vec<f32>,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    },
    /// Topics ranked by similarity to a given item.
    TopicsRelatedTo {
        id: String,
        limit: Option<usize>,
        min_similarity: Option<f32>,
    },
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ItemError {
    #[error(transparent)]
    Embeddings(#[from] crate::embeddings::EmbeddingsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub(crate) type Result<T> = std::result::Result<T, ItemError>;

// ── Public API ────────────────────────────────────────────────────────────────

/// Fetch embedded items from `source`, preserving similarity rank, then filter.
/// `is_unread` and `is_bookmarked` are read directly from LanceDB tags.
pub async fn list_embedded_items(
    library_dir: &Path,
    source: EmbeddedItemSource,
    item_filter: ItemFilter,
) -> Result<Vec<EmbeddedEntry>> {
    match source {
        EmbeddedItemSource::RelatedTo {
            id,
            limit,
            min_similarity,
            max_similarity,
        } => {
            let db = EmbeddingsDb::open(library_dir).await?;
            let entries = db
                .search_related(&id, limit, min_similarity, max_similarity, None)
                .await?;
            filter::filter_items(entries, &item_filter)
                .await
                .map_err(ItemError::Io)
        }
        EmbeddedItemSource::RelatedToVector {
            vector,
            limit,
            min_similarity,
            max_similarity,
        } => {
            let db = EmbeddingsDb::open(library_dir).await?;
            let entries = db
                .search_by_vector(vector, limit, min_similarity, max_similarity)
                .await?;
            filter::filter_items(entries, &item_filter)
                .await
                .map_err(ItemError::Io)
        }
        EmbeddedItemSource::TopicsRelatedTo {
            id,
            limit,
            min_similarity,
        } => {
            let db = EmbeddingsDb::open(library_dir).await?;
            db.search_related(&id, limit, min_similarity, None, Some(Kind::Topic))
                .await
                .map_err(ItemError::Embeddings)
        }
    }
}

/// Lists every item in the library — entry rows and annotation rows — read entirely from
/// LanceDB, filtered by `item_filter` and sorted by `created_at` descending. Returns the flat
/// `ListItem` the items list renders (annotations carry `reference`/`color`).
pub async fn list_all_items(library_dir: &Path, item_filter: ItemFilter) -> Result<Vec<ListItem>> {
    let db = EmbeddingsDb::open(library_dir).await?;

    let (entries_result, annotations_result) =
        tokio::join!(db.list_entries(), db.list_annotations());

    let list_entries = entries_result?;
    let annotations = annotations_result.unwrap_or_default();

    let mut items: Vec<ListItem> = list_entries
        .into_iter()
        .filter(|e| match item_filter {
            ItemFilter::All => true,
            ItemFilter::Unread => e.is_unread,
            ItemFilter::Bookmarked | ItemFilter::Saved => e.is_bookmarked,
        })
        .map(|e| ListItem {
            id: e.id,
            title: e.title,
            created_at: e.created_at,
            url: e.url,
            is_unread: e.is_unread,
            is_bookmarked: e.is_bookmarked,
            reference: None,
            color: None,
        })
        .chain(
            annotations
                .into_iter()
                .filter(|_| matches!(item_filter, ItemFilter::All | ItemFilter::Saved))
                .map(|a| ListItem {
                    id: a.id,
                    title: a.title,
                    created_at: a.created_at,
                    url: a.url,
                    is_unread: false,
                    is_bookmarked: false,
                    reference: Some(a.reference),
                    color: Some(a.color),
                }),
        )
        .collect();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

/// Enriches raw LanceDB results: rows with no URL are topic rows.
/// For each, loads the full Topic from disk and wraps it as EmbeddedItem::Topic.
/// Falls back to EmbeddedItem::Entry for any row that can't be resolved as a topic.
pub async fn enrich_embedded(library_dir: &Path, entries: Vec<EmbeddedEntry>) -> Vec<EmbeddedItem> {
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
            if let Ok(topic) =
                crate::persistence::topics::read_topic(library_dir, &entry.entry.id).await
            {
                result.push(EmbeddedItem::Topic(EmbeddedTopic {
                    topic,
                    kind: Kind::Topic,
                    similarity: entry.similarity,
                }));
                continue;
            }
        }
        result.push(EmbeddedItem::Entry(entry));
    }
    result
}
