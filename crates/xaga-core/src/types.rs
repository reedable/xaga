//! Derived types: query/wire DTOs + UI filter/sort + on-disk storage shapes. The domain model
//! lives in `crate::domain`; this module re-exports it and adds the shapes built around it.

use serde::{Deserialize, Serialize};

pub use crate::domain::{
    Annotation, Article, Bookmarkable, Colorable, Countable, Document, Entry, Feed, Item, Kind,
    Note, Pinnable, Reference, Topic, WebResource,
};

// ── ItemFilter ────────────────────────────────────────────────────────────────

/// Mirrors the TypeScript `ItemFilter` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemFilter {
    /// All items. Default.
    #[default]
    All,
    /// Items with no saved `.md` body.
    Unread,
    /// Only items whose tags contain "bookmark".
    Bookmarked,
    /// Items that are bookmarked or have at least one annotation.
    Saved,
}

impl std::str::FromStr for ItemFilter {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Self::All),
            "unread" => Ok(Self::Unread),
            "bookmarked" => Ok(Self::Bookmarked),
            "saved" => Ok(Self::Saved),
            other => Err(format!("unknown item filter: {other}")),
        }
    }
}

// ── ItemSort ──────────────────────────────────────────────────────────────────

/// Ordering for a topic's item list. Mirrors the TypeScript sort options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemSort {
    /// Unified similarity ranking, descending. Default.
    #[default]
    Relevance,
    /// Most-recent timestamp, descending.
    Timestamp,
}

impl std::str::FromStr for ItemSort {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "relevance" => Ok(Self::Relevance),
            "timestamp" => Ok(Self::Timestamp),
            other => Err(format!("unknown item sort: {other}")),
        }
    }
}

// ── CollectionItem (on-disk storage shape) ──────────────────────────────────────

/// A member of a topic's `.md` `items:` list, in insertion order. This is the on-disk storage
/// shape (serde tag `type`); the persistence layer maps it to the domain `Reference`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CollectionItem {
    Entry { id: String },
    /// `reference` is the parent article's ID, stored so the annotation can be looked up from the
    /// article file without scanning the library. Older entries deserialize with `reference = ""`.
    Annotation { id: String, #[serde(default)] reference: String },
}

impl CollectionItem {
    pub fn id(&self) -> &str {
        match self {
            Self::Entry { id } | Self::Annotation { id, .. } => id,
        }
    }
}

impl From<&CollectionItem> for Kind {
    fn from(item: &CollectionItem) -> Self {
        match item {
            CollectionItem::Entry { .. } => Kind::Entry,
            CollectionItem::Annotation { .. } => Kind::Annotation,
        }
    }
}

/// Maps the on-disk membership shape to the domain `Reference` (storage → domain at the boundary).
impl From<&CollectionItem> for Reference {
    fn from(item: &CollectionItem) -> Self {
        match item {
            CollectionItem::Entry { id } => Reference::Entry { id: id.clone() },
            CollectionItem::Annotation { id, .. } => Reference::Annotation { id: id.clone() },
        }
    }
}

// ── SavedItem (flat wire DTO for `list_all_items`) ──────────────────────────────

/// A flat wire record for the saved-items list: a bookmark or an annotation row. The TS adapter
/// splits it into the domain `Entry | PartialAnnotation` union (annotations set reference/color).
#[derive(Debug, Serialize)]
pub struct SavedItem {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub url: String,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Item for SavedItem {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { None }
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

// ── RankedItem ──────────────────────────────────────────────────────────────────

/// A minimal similarity result: an item ID and its cosine similarity score.
#[derive(Debug, Serialize)]
pub struct RankedItem {
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

impl Item for EmbeddedEntry {
    fn id(&self) -> &str { self.entry.id() }
    fn title(&self) -> &str { self.entry.title() }
    fn created_at(&self) -> &str { self.entry.created_at() }
    fn updated_at(&self) -> Option<&str> { self.entry.updated_at() }
    fn description(&self) -> Option<&str> { self.entry.description() }
    fn published_at(&self) -> Option<&str> { self.entry.published_at() }
}

impl std::ops::Deref for EmbeddedEntry {
    type Target = Entry;
    fn deref(&self) -> &Entry { &self.entry }
}

impl std::ops::DerefMut for EmbeddedEntry {
    fn deref_mut(&mut self) -> &mut Entry { &mut self.entry }
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
pub enum EmbeddedItem {
    Topic(EmbeddedTopic),
    Feed(EmbeddedFeed),
    Entry(EmbeddedEntry),
}
