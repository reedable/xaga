//! Derived types: query shapes + UI filter/sort + on-disk storage shapes. The domain model
//! lives in `crate::domain`; this module re-exports it and adds the shapes built around it.

use serde::Serialize;

pub use crate::domain::{
    Annotation, Article, COLOR_NAMES, Colorable, Countable, Document, Entity, Entry, Feed, Kind,
    ManagedEntity, Note, Pinnable, Reference, Topic, WebResource,
};

// ── EntityFilter ────────────────────────────────────────────────────────────────

/// Mirrors the shared variants of the TypeScript `Filter` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityFilter {
    /// All entities. Default.
    #[default]
    All,
    /// Entities with no saved `.md` body.
    Unread,
    /// Only color-marked Entries.
    Bookmarked,
    /// Color-marked Entries and annotation entities.
    Saved,
}

impl std::str::FromStr for EntityFilter {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Self::All),
            "unread" => Ok(Self::Unread),
            "bookmarked" => Ok(Self::Bookmarked),
            "saved" => Ok(Self::Saved),
            other => Err(format!("unknown entity filter: {other}")),
        }
    }
}

// ── EntitySort ──────────────────────────────────────────────────────────────────

/// Ordering for a topic's entity list. Mirrors the TypeScript sort options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntitySort {
    /// Unified similarity ranking, descending. Default.
    #[default]
    Relevance,
    /// Most-recent timestamp, descending.
    Timestamp,
}

impl std::str::FromStr for EntitySort {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "relevance" => Ok(Self::Relevance),
            "timestamp" => Ok(Self::Timestamp),
            other => Err(format!("unknown entity sort: {other}")),
        }
    }
}

// ── ListEntity (flat record for `list_all_entities`) ────────────────────────────────

/// A flat record in the entities list returned by `list_all_entities`: an entry (any state) or an
/// annotation row. The TS adapter splits it into the domain `Entry | PartialAnnotation` union
/// (annotations set reference/color).
#[derive(Debug, Serialize)]
pub struct ListEntity {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub url: String,
    pub is_unread: bool,
    pub is_pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

// ── SiteSummary (derived from Entry URLs) ────────────────────────────────────

/// Summary of a derived Collection, without a managed entity record or member list.
/// The frontend maps `hostname` to `Site.id`; the native payload retains hostname naming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiteSummary {
    pub hostname: String,
    pub entry_count: usize,
    pub unread_count: usize,
}

impl Entity for ListEntity {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
}
impl ManagedEntity for ListEntity {
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn updated_at(&self) -> Option<&str> {
        None
    }
}

// ── PartialAnnotation ───────────────────────────────────────────────────────────

/// An annotation record as stored in the embeddings DB — only what LanceDB holds:
/// no `text[]`, `occurrence[]`, or `note` (those live in the article's `.md`).
#[derive(Debug, Serialize)]
pub struct PartialAnnotation {
    pub id: String,
    /// The ID of the article this annotation belongs to.
    pub reference: String,
    /// The highlighted text segments joined into a single string.
    pub title: String,
    pub color: String,
    pub created_at: String,
    /// The canonical URL of the parent article.
    pub url: String,
}

// ── RankedEntity ──────────────────────────────────────────────────────────────────

/// A minimal similarity result: an entity ID and its cosine similarity score.
#[derive(Debug, Serialize)]
pub struct RankedEntity {
    pub id: String,
    pub similarity: f32,
}

// ── Embedded* (similarity query results) ─────────────────────────────────────────

/// An `Entry` returned from an embedding similarity query, carrying its score. For annotation
/// rows, `reference` holds the parent article ID; the highlight color rides on the flattened
/// `Entry.color`.
#[derive(Debug, Serialize)]
pub struct EmbeddedEntry {
    #[serde(flatten)]
    pub entry: Entry,
    /// Domain discriminator minted from the row's `kind` column; the frontend brands it `__kind`.
    pub kind: Kind,
    pub similarity: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reference: String,
}

impl Entity for EmbeddedEntry {
    fn id(&self) -> &str {
        self.entry.id()
    }
    fn title(&self) -> &str {
        self.entry.title()
    }
}
impl ManagedEntity for EmbeddedEntry {
    fn created_at(&self) -> &str {
        self.entry.created_at()
    }
    fn updated_at(&self) -> Option<&str> {
        self.entry.updated_at()
    }
    fn description(&self) -> Option<&str> {
        self.entry.description()
    }
    fn published_at(&self) -> Option<&str> {
        self.entry.published_at()
    }
}

impl std::ops::Deref for EmbeddedEntry {
    type Target = Entry;
    fn deref(&self) -> &Entry {
        &self.entry
    }
}

impl std::ops::DerefMut for EmbeddedEntry {
    fn deref_mut(&mut self) -> &mut Entry {
        &mut self.entry
    }
}

/// A `Topic` returned from an embedding similarity query, carrying its score + `kind`.
#[derive(Debug, Serialize)]
pub struct EmbeddedTopic {
    #[serde(flatten)]
    pub topic: Topic,
    pub kind: Kind,
    pub similarity: f32,
}

/// A `Feed` returned from an embedding similarity query, carrying its score + `kind`.
#[derive(Debug, Serialize)]
pub struct EmbeddedFeed {
    #[serde(flatten)]
    pub feed: Feed,
    pub kind: Kind,
    pub similarity: f32,
}

/// Untagged union of all embedded search result shapes.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EmbeddedEntity {
    Topic(EmbeddedTopic),
    Feed(EmbeddedFeed),
    Entry(EmbeddedEntry),
}
