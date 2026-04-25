use serde::Serialize;

// ── ItemFilter ────────────────────────────────────────────────────────────────

/// Mirrors the TypeScript `ItemFilter` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemFilter {
    /// All items. Default.
    #[default]
    All,
    /// Items with no saved `.md` body.
    Unread,
    /// Only items listed in `bookmarks.html`.
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
}

// ── Entry ─────────────────────────────────────────────────────────────────────

/// Metadata for a feed entry. Mirrors the TypeScript `Entry` type.
///
/// Used as the list-view representation of a feed entry, and embedded
/// (via `#[serde(flatten)]`) inside `Article`.
#[derive(Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub summary: Option<String>,
    pub is_unread: bool,
}

impl Item for Entry {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
    fn url(&self) -> Option<&str> { Some(&self.url) }
    fn published_at(&self) -> Option<&str> { self.published_at.as_deref() }
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
    /// Parent article ID — present for annotations, absent for bookmarks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Highlight color — present for annotations, absent for bookmarks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

// ── AnnotationSummary ─────────────────────────────────────────────────────────

/// A lightweight annotation record sourced from the embeddings DB.
/// Contains only what LanceDB stores — no `text[]`, `occurrence[]`, or `note`.
#[derive(Debug, Serialize)]
pub struct AnnotationSummary {
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

// ── EmbeddedEntry ─────────────────────────────────────────────────────────────

/// An `Entry` returned from an embedding similarity query, carrying its score.
/// Mirrors the TypeScript `EmbeddedEntry` type (= `Entry & Embedded`).
/// For annotation rows, `reference` holds the parent article ID and `color`
/// holds the highlight color; both are omitted for regular entries.
#[derive(Debug, Serialize)]
pub struct EmbeddedEntry {
    #[serde(flatten)]
    pub entry: Entry,
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
}

impl std::ops::Deref for EmbeddedEntry {
    type Target = Entry;
    fn deref(&self) -> &Entry { &self.entry }
}

impl std::ops::DerefMut for EmbeddedEntry {
    fn deref_mut(&mut self) -> &mut Entry { &mut self.entry }
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
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl Item for Feed {
    fn id(&self) -> &str { &self.id }
    fn title(&self) -> &str { &self.title }
    fn created_at(&self) -> &str { &self.created_at }
    fn updated_at(&self) -> Option<&str> { self.updated_at.as_deref() }
}
