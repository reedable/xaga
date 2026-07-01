//! The domain-object → `EmbeddingEntry` mapping — the indexing layer's inbound boundary.
//!
//! Lives here (not in persistence) so persistence has no dependency on `embeddings::db`. The
//! indexing consumer builds a LanceDB row directly from the domain object (`Article`/`Topic`) it
//! received on the queue — the object is carried end-to-end, never flattened into an intermediate
//! and never re-read from a file.
//!
//! The `tags` column is reconstructed from the object's state booleans (`is_unread` /
//! `is_bookmarked` / `is_pinned`). The row only ever uses those three state tags (see
//! `BatchSummaryColumns::summary_at` in `db.rs`); arbitrary file tags are not consulted by the
//! index, so this is behavior-preserving for queries and counts.

use sha2::Digest;

use crate::types::{Article, Feed, Kind, Topic};

use super::db::EmbeddingEntry;

/// SHA-256 checksum of the embeddable content (title + embed text). Annotations are embedded as
/// separate rows and are not included here. Shared by the row build and the embed-gate so both
/// agree on when a re-embed is needed.
pub(crate) fn checksum(title: &str, embed_text: &str) -> String {
    let mut h = sha2::Sha256::new();
    h.update(title.as_bytes());
    h.update(b"\n\n");
    h.update(embed_text.as_bytes());
    hex::encode(h.finalize())
}

/// The text embedded for an entry: its summary when present, otherwise the article body.
pub(crate) fn article_embed_text(article: &Article) -> &str {
    article
        .meta
        .description
        .as_deref()
        .unwrap_or(&article.content)
}

/// The text embedded for a topic: its description (the `summary` key); empty when absent.
pub(crate) fn topic_embed_text(topic: &Topic) -> &str {
    topic.description.as_deref().unwrap_or("")
}

/// The text embedded for a feed: its description (the `atom:subtitle`, mapped to the domain
/// `description` by persistence); empty when absent.
pub(crate) fn feed_embed_text(feed: &Feed) -> &str {
    feed.description.as_deref().unwrap_or("")
}

impl EmbeddingEntry {
    /// Builds an entry row from an `Article`. `topics`/`items` are reconcile-managed (written empty
    /// here, carried forward by the sync/embed paths). For entry rows `items` holds the annotation
    /// membership, filled by `sync_annotations` after the annotation rows are written.
    pub(crate) fn from_article(article: &Article, vector: Option<Vec<f32>>) -> EmbeddingEntry {
        let meta = &article.meta;
        let mut tags = Vec::new();
        if meta.is_unread {
            tags.push(crate::tags::UNREAD.to_string());
        }
        if meta.is_bookmarked {
            tags.push(crate::tags::BOOKMARK.to_string());
        }
        EmbeddingEntry {
            id: meta.id.clone(),
            kind: Kind::Entry,
            source: meta.url.clone(),
            title: meta.title.clone(),
            created_at: meta.created_at.clone(),
            published_at: meta.published_at.clone().unwrap_or_default(),
            updated_at: meta.updated_at.clone().unwrap_or_default(),
            checksum: checksum(&meta.title, article_embed_text(article)),
            reference: String::new(),
            color: String::new(),
            summary: meta.description.clone().unwrap_or_default(),
            tags,
            topics: vec![],
            items: vec![],
            is_unread: meta.is_unread,
            is_bookmarked: meta.is_bookmarked,
            is_pinned: false,
            vector,
        }
    }

    /// Builds a feed row from a domain `Feed`. Feed rows carry no created/updated timestamps.
    pub(crate) fn from_feed(feed: &Feed, vector: Option<Vec<f32>>) -> EmbeddingEntry {
        let mut tags = Vec::new();
        if feed.is_pinned {
            tags.push(crate::tags::PINNED.to_string());
        }
        EmbeddingEntry {
            id: feed.id.clone(),
            kind: Kind::Feed,
            source: feed.url.clone(),
            title: feed.title.clone(),
            created_at: String::new(),
            published_at: String::new(),
            updated_at: String::new(),
            checksum: checksum(&feed.title, feed_embed_text(feed)),
            reference: String::new(),
            color: String::new(),
            summary: feed.description.clone().unwrap_or_default(),
            tags,
            topics: vec![],
            items: vec![],
            is_unread: false,
            is_bookmarked: false,
            is_pinned: feed.is_pinned,
            vector,
        }
    }

    /// Builds a topic row from a `Topic`. `topics`/`items` are reconcile-managed (written empty
    /// here, carried forward by the sync/embed paths).
    pub(crate) fn from_topic(topic: &Topic, vector: Option<Vec<f32>>) -> EmbeddingEntry {
        let mut tags = Vec::new();
        if topic.is_pinned {
            tags.push(crate::tags::PINNED.to_string());
        }
        EmbeddingEntry {
            id: topic.id.clone(),
            kind: Kind::Topic,
            source: String::new(),
            title: topic.title.clone(),
            created_at: topic.created_at.clone(),
            published_at: String::new(),
            updated_at: topic.updated_at.clone().unwrap_or_default(),
            checksum: checksum(&topic.title, topic_embed_text(topic)),
            reference: String::new(),
            color: String::new(),
            summary: topic.description.clone().unwrap_or_default(),
            tags,
            topics: vec![],
            items: vec![],
            is_unread: false,
            is_bookmarked: false,
            is_pinned: topic.is_pinned,
            vector,
        }
    }
}
