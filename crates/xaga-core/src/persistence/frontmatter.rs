//! `.md` frontmatter parsing — the persistence layer's inbound boundary.
//!
//! Parses the `---`-delimited document format into the in-memory [`Frontmatter`] and the neutral
//! [`Content`](crate::content::Content) payload. Lives under `persistence` (not `util`) because it
//! is the file-format boundary; `util` keeps only format-agnostic helpers (url/hash).

use crate::content::Content;
use crate::types::{Annotation, CollectionItem, Kind};

/// The parsed header fields of a `.md` document.
pub struct Frontmatter {
    pub source: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub summary: Option<String>,
    pub annotations: Vec<Annotation>,
    pub threshold: Option<f32>,
    pub tags: Vec<String>,
    pub favicon: Option<String>,
    pub items: Vec<CollectionItem>,
}

/// Maps a parsed document into the neutral [`Content`] payload.
/// The kind is derived as topics carry no `source`; entries always do — matching the former
/// `Frontmatter::to_embedding_entry` discriminant.
pub fn content_from_frontmatter(id: String, fm: Frontmatter, body: String) -> Content {
    let kind = if fm.source.is_empty() {
        Kind::Topic
    } else {
        Kind::Entry
    };
    Content {
        id,
        kind,
        source: fm.source,
        title: fm.title,
        created_at: fm.created_at,
        published_at: fm.published_at,
        updated_at: fm.updated_at,
        summary: fm.summary,
        tags: fm.tags,
        annotations: fm.annotations,
        threshold: fm.threshold,
        favicon: fm.favicon,
        items: fm.items,
        body,
    }
}

/// Parses a `---`-delimited document directly into a [`Content`], given the item's id. Returns
/// `None` if the format is invalid. The single parse the producers and recovery use so the
/// indexing consumer never re-reads the file.
pub fn parse_content(id: String, raw: &str) -> Option<Content> {
    let (fm, body) = split_document(raw)?;
    Some(content_from_frontmatter(id, fm, body))
}

/// Parses only the header fields from a `---`-delimited document.
/// Returns `None` if the format is invalid.
pub fn parse_frontmatter(raw: &str) -> Option<Frontmatter> {
    Some(split_document(raw)?.0)
}

/// Parses both the header fields and the markdown body from a `---`-delimited document.
/// Returns `None` if the format is invalid.
pub fn parse_document(raw: &str) -> Option<(Frontmatter, String)> {
    split_document(raw)
}

/// The `.md` frontmatter storage shape for an annotation. Storage keeps `note`; the domain
/// `Annotation`'s `title`/`content` are derived here at the boundary (title = highlighted text,
/// content = note). Mapped to/from the domain type by the `From` impls below.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FrontmatterAnnotation {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reference: String,
    pub text: Vec<String>,
    pub occurrence: Vec<u32>,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

impl From<FrontmatterAnnotation> for Annotation {
    fn from(a: FrontmatterAnnotation) -> Self {
        Annotation {
            title: a.text.join(" "),
            content: a.note.unwrap_or_default(),
            id: a.id,
            reference: a.reference,
            text: a.text,
            occurrence: a.occurrence,
            color: a.color,
            created_at: a.created_at,
        }
    }
}

impl From<&Annotation> for FrontmatterAnnotation {
    fn from(a: &Annotation) -> Self {
        FrontmatterAnnotation {
            id: a.id.clone(),
            reference: a.reference.clone(),
            text: a.text.clone(),
            occurrence: a.occurrence.clone(),
            color: a.color.clone(),
            note: (!a.content.is_empty()).then(|| a.content.clone()),
            created_at: a.created_at.clone(),
        }
    }
}

#[derive(serde::Deserialize, Default)]
struct FrontmatterRaw {
    #[serde(default)]
    source: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    published: Option<String>,
    #[serde(default)]
    updated: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    annotations: Vec<FrontmatterAnnotation>,
    #[serde(default)]
    threshold: Option<f32>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    favicon: Option<String>,
    #[serde(default)]
    items: Vec<CollectionItem>,
}

fn split_document(raw: &str) -> Option<(Frontmatter, String)> {
    let raw = raw.trim_start();
    let rest = raw.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let yaml = &rest[..end];
    let body = rest[end + 5..].trim_matches('\n').to_string();

    let fm: FrontmatterRaw = serde_yaml::from_str(yaml).ok()?;

    Some((
        Frontmatter {
            source: fm.source,
            title: fm.title,
            created_at: fm.created,
            published_at: fm.published,
            updated_at: fm.updated,
            summary: fm.summary,
            annotations: fm.annotations.into_iter().map(Into::into).collect(),
            threshold: fm.threshold,
            tags: fm.tags,
            favicon: fm.favicon,
            items: fm.items,
        },
        body,
    ))
}
