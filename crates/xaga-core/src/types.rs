use serde::{Deserialize, Serialize};

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

// ── Traits ────────────────────────────────────────────────────────────────────

/// Minimal display contract shared by every item (articles, notes, …).
/// Sufficient for list views, sorting, and mixed-type collections.
pub trait Item {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn created_at(&self) -> &str;
    fn updated_at(&self) -> Option<&str>;
    /// The canonical URL this item was fetched from.
    /// `None` for authored items (topics, notes) — they are treated as already read
    /// and are not bookmarkable.
    fn url(&self) -> Option<&str> { None }
    /// ISO-8601 publish date, if known. `None` excludes the item from `Upcoming`
    /// and includes it in `All`.
    fn published_at(&self) -> Option<&str> { None }
    fn is_unread(&self) -> bool { false }
    fn is_bookmarked(&self) -> bool { false }
    /// Non-empty only for annotation rows; holds the parent entry ID.
    fn reference(&self) -> &str { "" }
}

// ── Kind ──────────────────────────────────────────────────────────────────────

/// The domain type discriminator for the four kinds the system distinguishes.
///
/// In the Rust layer the kind is encoded structurally by the distinct domain
/// structs (`Entry`, `Feed`, `Topic`, `Annotation`); this enum is the single
/// source of truth for the discriminator as an explicit value, needed only at
/// type-erasure boundaries — the LanceDB `kind` column and the embedded-result
/// wire payload. The serde representation matches `as_str()` and the
/// `CollectionItem` serde tags, so the value never needs translation between the
/// wire, the DB column, and `CollectionItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Entry,
    Annotation,
    Topic,
    Feed,
}

impl Kind {
    /// Canonical discriminator string stored in the DB `kind` column and sent on
    /// the wire. Kept in sync with the serde representation above.
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Entry => "entry",
            Kind::Annotation => "annotation",
            Kind::Topic => "topic",
            Kind::Feed => "feed",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(Kind::Entry),
            "annotation" => Ok(Kind::Annotation),
            "topic" => Ok(Kind::Topic),
            "feed" => Ok(Kind::Feed),
            other => Err(format!("unknown kind: {other}")),
        }
    }
}

// ── Entry ─────────────────────────────────────────────────────────────────────

/// Metadata for a feed entry. Mirrors the TypeScript `Entry` type.
///
/// Used as the list-view representation of a feed entry, and embedded
/// (via `#[serde(flatten)]`) inside `Article`.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub summary: Option<String>,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    /// Internal only — not sent to the frontend.
    #[serde(skip)]
    pub tags: Vec<String>,
}

impl Item for Entry {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
    fn url(&self) -> Option<&str> { Some(&self.url) }
    fn published_at(&self) -> Option<&str> { self.published_at.as_deref() }
    fn is_unread(&self) -> bool { self.is_unread }
    fn is_bookmarked(&self) -> bool { self.is_bookmarked }
}

// ── SavedItem ─────────────────────────────────────────────────────────────────

/// A unified record representing either a bookmark or an annotation.
/// Annotations have `reference` and `color` set; bookmarks do not.
#[derive(Debug, Serialize)]
pub struct SavedItem {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub url: String,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    /// Parent article ID — present for annotations, absent for bookmarks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Highlight color — present for annotations, absent for bookmarks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Item for SavedItem {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { None }
    fn url(&self) -> Option<&str> { Some(&self.url) }
    fn is_unread(&self) -> bool { self.is_unread }
    fn is_bookmarked(&self) -> bool { self.is_bookmarked }
    fn reference(&self) -> &str { self.reference.as_deref().unwrap_or("") }
}

// ── PartialAnnotation ─────────────────────────────────────────────────────────

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

// ── RankedItem ────────────────────────────────────────────────────────────────

/// A minimal similarity result: just an item ID and its cosine similarity score.
/// Mirrors the TypeScript `{ id: string } & Embedded` pattern.
#[derive(Debug, Serialize)]
pub struct RankedItem {
    pub id: String,
    pub similarity: f32,
}

// ── EmbeddedEntry ─────────────────────────────────────────────────────────────

/// An `Entry` returned from an embedding similarity query, carrying its score.
/// Mirrors the TypeScript `EmbeddedEntry` type (= `Entry & Embedded`).
/// For annotation rows, `reference` holds the parent article ID and `color`
/// holds the highlight color; both are omitted for regular entries.
#[derive(Debug, Serialize)]
pub struct EmbeddedEntry {
    #[serde(flatten)]
    pub entry: Entry,
    /// Domain discriminator minted from the row's `kind` column. The wire
    /// distillation the frontend brands into `__kind` — the only thing that
    /// separates a feed result from an entry result, since both are URL-bearing.
    pub kind: Kind,
    pub similarity: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reference: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub color: String,
}

impl Item for EmbeddedEntry {
    fn id(&self) -> &str { self.entry.id() }
    fn title(&self) -> &str { self.entry.title() }
    fn created_at(&self) -> &str { self.entry.created_at() }
    fn updated_at(&self) -> Option<&str> { self.entry.updated_at() }
    fn url(&self) -> Option<&str> { self.entry.url() }
    fn published_at(&self) -> Option<&str> { self.entry.published_at() }
    fn is_unread(&self) -> bool { self.entry.is_unread() }
    fn is_bookmarked(&self) -> bool { self.entry.is_bookmarked() }
    fn reference(&self) -> &str { &self.reference }
}

impl std::ops::Deref for EmbeddedEntry {
    type Target = Entry;
    fn deref(&self) -> &Entry { &self.entry }
}

impl std::ops::DerefMut for EmbeddedEntry {
    fn deref_mut(&mut self) -> &mut Entry { &mut self.entry }
}

// ── EmbeddedTopic ─────────────────────────────────────────────────────────────

/// A `Topic` returned from an embedding similarity query, carrying its score.
/// Shape mirrors `Topic` — no `url`, no `is_unread`, no `is_bookmarked`.
#[derive(Debug, Serialize)]
pub struct EmbeddedTopic {
    #[serde(flatten)]
    pub topic: Topic,
    pub similarity: f32,
}

/// A `Feed` returned from an embedding similarity query, carrying its score and the
/// `kind` discriminator (minted into `__kind` at the TS boundary).
#[derive(Debug, Serialize)]
pub struct EmbeddedFeed {
    #[serde(flatten)]
    pub feed: Feed,
    pub kind: Kind,
    pub similarity: f32,
}

/// Untagged union of all possible embedded search result shapes.
/// Serializes by shape alone — the `kind` field on entry/feed variants is the
/// discriminator the frontend brands into `__kind`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum EmbeddedItem {
    Topic(EmbeddedTopic),
    Feed(EmbeddedFeed),
    Entry(EmbeddedEntry),
}

// ── Annotation ────────────────────────────────────────────────────────────────

/// A saved highlight or annotation on a span of text in an article.
/// A plain highlight has `note: None`; an annotation has `note: Some(...)`.
/// Mirrors the TypeScript `Annotation` type.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Annotation {
    pub id: String,
    /// The ID of the article this annotation belongs to.
    /// Populated at runtime from context; not stored in YAML frontmatter.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reference: String,
    /// One text segment per block-level element covered by the selection.
    /// Single-paragraph selections produce a one-element vec.
    pub text: Vec<String>,
    /// 0-based occurrence index for each corresponding entry in `text`.
    pub occurrence: Vec<u32>,
    /// Highlight color; currently always "yellow".
    pub color: String,
    /// Optional user note. Absent for plain highlights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

// ── Article ───────────────────────────────────────────────────────────────────

/// A fully loaded saved entry, including markdown content and annotations.
/// Mirrors the TypeScript `Article` type (= `Entry & Document`).
#[derive(Debug, Serialize)]
pub struct Article {
    #[serde(flatten)]
    pub meta: Entry,
    pub content: String,
    pub annotations: Vec<Annotation>,
    pub favicon: Option<String>,
}

impl Item for Article {
    fn id(&self) -> &str { self.meta.id() }
    fn title(&self) -> &str { self.meta.title() }
    fn created_at(&self) -> &str { self.meta.created_at() }
    fn updated_at(&self) -> Option<&str> { self.meta.updated_at() }
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A user-authored document with full content.
/// Mirrors the TypeScript `Document` type (= `Item & { content: string }`).
#[derive(Debug, Serialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub content: String,
}

impl Item for Document {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
}

// ── Topic ─────────────────────────────────────────────────────────────────────

/// Mirrors the TypeScript `Topic` type (= `Document & { threshold?: number }`).
#[derive(Debug, Serialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
    /// Storage only — never serialized to JSON sent to the frontend.
    #[serde(skip)]
    pub items: Vec<CollectionItem>,
    /// Storage only — frontmatter tags (e.g. "pinned"); not sent to the frontend.
    #[serde(skip)]
    pub tags: Vec<String>,
    /// Derived from the `pinned` frontmatter tag; sent to the frontend (the `tags` field is not).
    pub is_pinned: bool,
    pub entry_count: usize,
    pub unread_count: usize,
}

impl Item for Topic {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
}

// ── Note ─────────────────────────────────────────────────────────────────────

/// Mirrors the TypeScript `Note` type (= `Document`).
pub type Note = Document;

// ── Feed ──────────────────────────────────────────────────────────────────────

/// A subscribed RSS/Atom feed. Mirrors the TypeScript `Feed` type.
#[derive(Debug, Serialize)]
pub struct Feed {
    pub id: String,
    pub url: String,
    pub title: String,
    pub entry_count: usize,
    pub unread_count: usize,
    pub bookmarked_count: usize,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub is_pinned: bool,
}

impl Item for Feed {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
}

// ── Collection ────────────────────────────────────────────────────────────────

/// A single member of a user-managed Collection, in insertion order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CollectionItem {
    Entry { id: String },
    /// `reference` is the parent article's ID, stored so the annotation can be
    /// looked up from the article file without scanning the whole library or
    /// relying on LanceDB. Older entries that pre-date this field deserialize
    /// with `reference = ""` via the serde default.
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

/// A user-managed collection of entries and annotations.
/// Mirrors the TypeScript `Collection` type — Feed and Topic are special cases.
#[derive(Debug, Serialize)]
pub struct Collection {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    /// Storage only — never serialized to JSON sent to the frontend.
    #[serde(skip)]
    pub items: Vec<CollectionItem>,
    pub entry_count: usize,
    pub unread_count: usize,
}

impl Item for Collection {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
}
