//! Domain model (Rust), echoing `src/domain.ts`.
//!
//! Pure: no persistence (`.md`/`.atom`), no cache (LanceDB row), no query/UI concern appears
//! here. The persistence layer transforms storage ↔ these types; the caching layer transforms
//! these ↔ `EmbeddingEntry`. Identity is structural shape AND the `__kind` brand together.

use serde::{Deserialize, Serialize};

// ── Item + capabilities (TS intersections → Rust supertraits) ────────────────────

pub trait Item {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn created_at(&self) -> &str;
    fn updated_at(&self) -> Option<&str>;
    /// Optional blurb. `None` for items without one.
    fn description(&self) -> Option<&str> {
        None
    }
    /// ISO-8601 publish date, if known.
    fn published_at(&self) -> Option<&str> {
        None
    }
}

pub trait WebResource: Item {
    fn url(&self) -> &str;
}
pub trait Document: Item {
    fn content(&self) -> &str;
}
/// Authoritative summary counts — independent of `members.len()` (live fetch can diverge).
pub trait Countable: Item {
    fn entry_count(&self) -> usize;
    fn unread_count(&self) -> usize;
}
pub trait Pinnable: Item {
    fn is_pinned(&self) -> bool;
}
pub trait Colorable: Item {
    fn color(&self) -> Option<&str>;
}

// `Collection<T>` has no Rust trait analogue (a generic container); its membership is the
// `members: Vec<Reference>` field on the collection entities (`Feed`, `Topic`), which also impl
// `Countable`.

// ── Kind (the discriminator; Rust's encoding of the TS `__kind` brand) ──────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Entry,
    Annotation,
    Topic,
    Feed,
    Note,
}

impl Kind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Kind::Entry => "entry",
            Kind::Annotation => "annotation",
            Kind::Topic => "topic",
            Kind::Feed => "feed",
            Kind::Note => "note",
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
            "note" => Ok(Kind::Note),
            other => Err(format!("unknown kind: {other}")),
        }
    }
}

// ── Reference (a branded pointer to a collection member) ────────────────────────

/// A typed, branded pointer to an item — the enum variant IS the brand (mirrors TS
/// `Reference<T>`). A value object, not an entity. Members of a `Collection`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "__kind", rename_all = "lowercase")]
pub enum Reference {
    Entry { id: String },
    Annotation { id: String },
    Note { id: String },
}

impl Reference {
    pub fn id(&self) -> &str {
        match self {
            Reference::Entry { id } | Reference::Annotation { id } | Reference::Note { id } => id,
        }
    }
}

impl From<&Reference> for Kind {
    fn from(r: &Reference) -> Self {
        match r {
            Reference::Entry { .. } => Kind::Entry,
            Reference::Annotation { .. } => Kind::Annotation,
            Reference::Note { .. } => Kind::Note,
        }
    }
}

// ── Entry ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    pub is_pinned: bool,
    /// The bookmark color (from `Colorable`); `None` when uncolored.
    pub color: Option<String>,
}

impl Item for Entry {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn updated_at(&self) -> Option<&str> {
        self.updated_at.as_deref()
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn published_at(&self) -> Option<&str> {
        self.published_at.as_deref()
    }
}
impl WebResource for Entry {
    fn url(&self) -> &str {
        &self.url
    }
}
impl Pinnable for Entry {
    fn is_pinned(&self) -> bool {
        self.is_pinned
    }
}
impl Colorable for Entry {
    fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }
}

// ── Feed ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Feed {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub is_pinned: bool,
    pub entry_count: usize,
    pub unread_count: usize,
    /// The feed's entries, as references (`Collection<Entry>` membership).
    pub members: Vec<Reference>,
}

impl Item for Feed {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn updated_at(&self) -> Option<&str> {
        self.updated_at.as_deref()
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
impl WebResource for Feed {
    fn url(&self) -> &str {
        &self.url
    }
}
impl Countable for Feed {
    fn entry_count(&self) -> usize {
        self.entry_count
    }
    fn unread_count(&self) -> usize {
        self.unread_count
    }
}
impl Pinnable for Feed {
    fn is_pinned(&self) -> bool {
        self.is_pinned
    }
}

// ── Topic ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub threshold: Option<f32>,
    pub is_pinned: bool,
    pub entry_count: usize,
    pub unread_count: usize,
    /// Curated members (`Collection<Entry | Annotation | Note>` membership).
    pub members: Vec<Reference>,
}

impl Item for Topic {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn updated_at(&self) -> Option<&str> {
        self.updated_at.as_deref()
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
impl Countable for Topic {
    fn entry_count(&self) -> usize {
        self.entry_count
    }
    fn unread_count(&self) -> usize {
        self.unread_count
    }
}
impl Pinnable for Topic {
    fn is_pinned(&self) -> bool {
        self.is_pinned
    }
}

// ── Article (a loaded, content-bearing Entry; kind stays "entry") ───────────────

#[derive(Debug, Clone, Serialize)]
pub struct Article {
    #[serde(flatten)]
    pub meta: Entry,
    pub content: String,
    pub annotations: Vec<Annotation>,
    pub favicon: Option<String>,
}

impl Item for Article {
    fn id(&self) -> &str {
        self.meta.id()
    }
    fn title(&self) -> &str {
        self.meta.title()
    }
    fn created_at(&self) -> &str {
        self.meta.created_at()
    }
    fn updated_at(&self) -> Option<&str> {
        self.meta.updated_at()
    }
    fn description(&self) -> Option<&str> {
        self.meta.description()
    }
    fn published_at(&self) -> Option<&str> {
        self.meta.published_at()
    }
}
impl WebResource for Article {
    fn url(&self) -> &str {
        self.meta.url()
    }
}
impl Colorable for Article {
    fn color(&self) -> Option<&str> {
        self.meta.color()
    }
}
impl Document for Article {
    fn content(&self) -> &str {
        &self.content
    }
}

// ── Note (content entity with no web presence; counterpart to Article) ──────────

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub content: String,
    pub is_bookmarked: bool,
    pub color: Option<String>,
}

impl Item for Note {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn updated_at(&self) -> Option<&str> {
        self.updated_at.as_deref()
    }
}
impl Document for Note {
    fn content(&self) -> &str {
        &self.content
    }
}
impl Colorable for Note {
    fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }
}

// ── Annotation (a Document on a span of an article) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    /// `title` (from `Item`) is the highlighted text; carried so it lists in mixed collections.
    pub title: String,
    /// The parent article id.
    pub reference: String,
    /// One text segment per block-level element covered by the selection.
    pub text: Vec<String>,
    /// 0-based occurrence index for each corresponding entry in `text`.
    pub occurrence: Vec<u32>,
    pub color: String,
    /// `content` (from `Document`) = the user's comment on the highlight (was `note`);
    /// `""` for a plain highlight.
    pub content: String,
    pub created_at: String,
}

impl Item for Annotation {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn updated_at(&self) -> Option<&str> {
        None
    }
}
impl Document for Annotation {
    fn content(&self) -> &str {
        &self.content
    }
}
impl Colorable for Annotation {
    fn color(&self) -> Option<&str> {
        Some(&self.color)
    }
}
