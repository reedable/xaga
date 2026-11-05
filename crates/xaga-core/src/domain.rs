//! Domain model (Rust), echoing `src/domain.ts`.
//!
//! Pure: no persistence (`.md`/`.atom`), no cache (LanceDB row), no query/UI concern appears
//! here. The persistence layer transforms storage ↔ these types; the caching layer transforms
//! these ↔ `EmbeddingEntry`. Identity is structural shape AND the `__kind` brand together.

use serde::{Deserialize, Serialize};

/// Palette identifiers shared by article markings and annotation swatches.
pub const COLOR_NAMES: [&str; 6] = ["red", "orange", "yellow", "green", "blue", "violet"];

// ── Entities + capabilities (TS intersections → Rust supertraits) ───────────────

pub trait Entity {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
}

/// An entity with an independent managed record and lifecycle.
pub trait ManagedEntity: Entity {
    fn created_at(&self) -> &str;
    fn updated_at(&self) -> Option<&str>;
    /// Optional blurb. `None` for entities without one.
    fn description(&self) -> Option<&str> {
        None
    }
    /// ISO-8601 publish date, if known.
    fn published_at(&self) -> Option<&str> {
        None
    }
}

pub trait WebResource: ManagedEntity {
    fn url(&self) -> &str;
}
pub trait Document: ManagedEntity {
    fn content(&self) -> &str;
}
/// Authoritative summary counts — independent of `members.len()` (live fetch can diverge).
pub trait Countable {
    fn entry_count(&self) -> usize;
    fn unread_count(&self) -> usize;
}
pub trait Pinnable: ManagedEntity {
    fn is_pinned(&self) -> bool;
}
pub trait Colorable: ManagedEntity {
    fn color(&self) -> Option<&str>;
}

// Collection covers Feed, Topic, and the derived Site grouping. ManagedCollection<T>
// describes Feed and Topic: independent entities with a managed lifecycle and typed members.
// No shared Rust trait is needed: their `members: Vec<Reference>` fields and `Countable` impls
// carry the managed shape; `types::SiteSummary` carries derived Site counts and hostname.

// ── Kind (persisted entity discriminator; mirrors entity brands in TS) ──────────
// Site's UI brand is assigned at the platform boundary, not stored as an entity Kind.

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

/// A typed, branded pointer to an entity — the enum variant IS the brand (mirrors TS
/// `Reference<T>`). A value object, not an entity. Members of a `ManagedCollection`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "__kind", rename_all = "lowercase")]
pub enum Reference {
    Entry {
        id: String,
    },
    Annotation {
        id: String,
        /// Containing article, when known, so the annotation can be located.
        #[serde(default, rename = "articleId", skip_serializing_if = "Option::is_none")]
        article_id: Option<String>,
    },
    Note {
        id: String,
    },
}

impl Reference {
    pub fn id(&self) -> &str {
        match self {
            Reference::Entry { id } | Reference::Annotation { id, .. } | Reference::Note { id } => {
                id
            }
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
    /// Readable file labels. None means this summary has no assignment snapshot;
    /// Some(empty) explicitly carries no assignments. Never indexed IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub is_unread: bool,
    pub is_pinned: bool,
    /// The bookmark color (from `Colorable`); `None` when uncolored.
    pub color: Option<String>,
}

impl Entity for Entry {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
}
impl ManagedEntity for Entry {
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
    /// The feed's entries, as references (`ManagedCollection<Entry>` membership).
    pub members: Vec<Reference>,
}

impl Entity for Feed {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
}
impl ManagedEntity for Feed {
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
    /// Curated members (`ManagedCollection<Entry | Annotation | Note>` membership).
    pub members: Vec<Reference>,
}

impl Entity for Topic {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
}
impl ManagedEntity for Topic {
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

impl Entity for Article {
    fn id(&self) -> &str {
        self.meta.id()
    }
    fn title(&self) -> &str {
        self.meta.title()
    }
}
impl ManagedEntity for Article {
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
    pub color: Option<String>,
}

impl Entity for Note {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
}
impl ManagedEntity for Note {
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
    /// `title` (from `Entity`) is the highlighted text; carried so it lists in mixed collections.
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

impl Entity for Annotation {
    fn id(&self) -> &str {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
}
impl ManagedEntity for Annotation {
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
