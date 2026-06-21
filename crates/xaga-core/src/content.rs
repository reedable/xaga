//! The parsed-content payload — the single parse result that crosses the queue.
//!
//! This is the neutral type that lets persistence and indexing stop intersecting: the
//! persistence layer produces a `Content` (from a `.md`/`.atom` parse or a write), the producer
//! enqueues it, and the indexing layer consumes it to build a LanceDB row and a vector — without
//! ever reading a file. Neither `persistence` nor `embeddings` depends on the other; both depend
//! only on this and on `domain`/`types`.
//!
//! `Content` carries everything both sides need: the row fields (via [`Content::content_checksum`]
//! and the `embeddings` row mapping), the embed text (via [`Content::embed_text`]), the annotations,
//! and the topic membership (`items`). It also reconstructs the UI-facing domain objects
//! ([`Content::to_entry`] / [`to_article`](Content::to_article) / [`to_topic`](Content::to_topic))
//! so a producer parses once and serves both the frontend and the queue.

use serde::{Deserialize, Serialize};

use crate::types::{Annotation, Article, CollectionItem, Entry, Kind, Reference, Topic};

/// One parsed library item: the row/vector inputs plus the body and membership. Carried in the
/// `Job` queue so the indexing consumer never re-reads the file. Annotations ride along (their
/// rows/vectors are derived here too); `items` is the topic membership (empty for entries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub id: String,
    /// `Entry` for articles/feed entries, `Topic` for topics. Annotations are derived rows, not a
    /// top-level `Content`.
    pub kind: Kind,
    /// The canonical URL (`source` frontmatter key). Empty for topics.
    pub source: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    /// The blurb (`summary` key); a topic's description is stored here too.
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub annotations: Vec<Annotation>,
    /// Topic similarity threshold; `None` for entries.
    pub threshold: Option<f32>,
    pub favicon: Option<String>,
    /// Topic membership (the `.md` `items:` list); empty for entries.
    pub items: Vec<CollectionItem>,
    pub body: String,
}

impl Content {
    /// The text used for embedding: summary when present, otherwise the full body.
    /// Mirrors the former `Frontmatter::embed_text`.
    pub fn embed_text(&self) -> &str {
        self.summary.as_deref().unwrap_or(&self.body)
    }

    /// SHA-256 checksum of the embeddable content (title + [`embed_text`](Content::embed_text)).
    /// Annotations are embedded as separate rows and are not included here.
    /// Mirrors the former `Frontmatter::content_checksum`.
    pub fn content_checksum(&self) -> String {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(self.title.as_bytes());
        h.update(b"\n\n");
        h.update(self.embed_text().as_bytes());
        hex::encode(h.finalize())
    }

    pub fn is_unread(&self) -> bool {
        self.tags.iter().any(|t| t == "unread")
    }
    pub fn is_bookmarked(&self) -> bool {
        self.tags.iter().any(|t| t == "bookmark")
    }
    pub fn is_pinned(&self) -> bool {
        self.tags.iter().any(|t| t == "pinned")
    }

    // ── Domain reconstruction (so a producer parses once and also serves the frontend) ──

    /// The UI-facing `Entry`. Mirrors `entries::entry_from_fm`.
    pub fn to_entry(&self) -> Entry {
        Entry {
            id: self.id.clone(),
            url: self.source.clone(),
            title: self.title.clone(),
            created_at: self.created_at.clone(),
            published_at: self.published_at.clone(),
            updated_at: self.updated_at.clone(),
            description: self.summary.clone(),
            is_unread: self.is_unread(),
            is_bookmarked: self.is_bookmarked(),
            color: None,
        }
    }

    /// The UI-facing `Article`. Annotations get their parent `reference` set, matching
    /// `articles::read_article`.
    pub fn to_article(&self) -> Article {
        let annotations = self
            .annotations
            .iter()
            .cloned()
            .map(|mut a| {
                a.reference = self.id.clone();
                a
            })
            .collect();
        Article {
            meta: self.to_entry(),
            content: self.body.clone(),
            annotations,
            favicon: self.favicon.clone(),
        }
    }

    /// The UI-facing `Topic`. `entry_count`/`unread_count` are 0 (computed by the query layer),
    /// matching `topics::read_topic`. `description` falls back to the body for pre-migration topics.
    pub fn to_topic(&self) -> Topic {
        Topic {
            id: self.id.clone(),
            title: self.title.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            description: self
                .summary
                .clone()
                .or_else(|| (!self.body.is_empty()).then(|| self.body.clone())),
            threshold: self.threshold,
            is_pinned: self.is_pinned(),
            entry_count: 0,
            unread_count: 0,
            members: self.items.iter().map(Reference::from).collect(),
        }
    }

    /// The topic's member ids (for the reverse-index reconcile), from `items`.
    pub fn member_ids(&self) -> Vec<String> {
        self.items.iter().map(|i| i.id().to_string()).collect()
    }
}
