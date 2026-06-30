//! Intersection of Markdown and domain
//! File system concerns should be managed by `persistence` layer

use crate::types::{Annotation, Article, CollectionItem, Entry, Topic};

/// Represends the .md storage format
pub(crate) struct Markdown {
    pub id: String,
    pub fm: Frontmatter,
    pub body: String,
}

/// Parses both the header fields and the markdown body from a `---`-delimited document.
/// Returns `None` if the format is invalid.
pub(crate) fn parse_document(raw: &str) -> Option<(Frontmatter, String)> {
    split_document(raw)
}

/// Converts .md storage format to Topic domain object
pub(crate) fn md_to_topic(md: Markdown) -> Topic {
    let is_pinned = crate::tags::is_pinned(&md.fm.tags);

    Topic {
        id: md.id.to_string(),
        title: md.fm.title,
        created_at: md.fm.created_at,
        updated_at: md.fm.updated_at,
        description: md.fm.summary,
        threshold: md.fm.threshold,
        is_pinned,
        entry_count: 0,
        unread_count: 0,
        members: md
            .fm
            .items
            .iter()
            .map(crate::types::Reference::from)
            .collect(),
    }
}

/// Converts .md storage format to Article domain object
pub(crate) fn md_to_article(md: Markdown) -> Article {
    let article_id = md.id.clone();
    let annotations = md.fm.annotations.clone();
    let content = md.body.clone();
    let favicon = md.fm.favicon.clone();

    Article {
        meta: md_to_entry(md),
        annotations: with_reference(annotations, &article_id),
        content,
        favicon,
    }
}

/// Enriches annotations vector with their reference id (article_id)
fn with_reference(mut annotations: Vec<Annotation>, article_id: &str) -> Vec<Annotation> {
    for a in &mut annotations {
        a.reference = article_id.to_string();
    }

    annotations
}

/// Converts .md storage format to Entry domain object
pub(crate) fn md_to_entry(md: Markdown) -> Entry {
    let is_unread = crate::tags::is_unread(&md.fm.tags);
    let is_bookmarked = crate::tags::is_bookmarked(&md.fm.tags);

    Entry {
        id: md.id,
        url: md.fm.source,
        title: md.fm.title,
        created_at: md.fm.created_at,
        published_at: md.fm.published_at,
        updated_at: md.fm.updated_at,
        description: md.fm.summary,
        is_unread,
        is_bookmarked,
        color: None,
    }
}

// ── Frontmatter: the `---`-delimited header parsing (the `.md` inbound boundary) ──

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

/// Parses only the header fields from a `---`-delimited document.
/// Returns `None` if the format is invalid.
pub(crate) fn parse_frontmatter(raw: &str) -> Option<Frontmatter> {
    Some(split_document(raw)?.0)
}

/// The `.md` frontmatter storage shape for an annotation. Storage keeps `note`; the domain
/// `Annotation`'s `title`/`content` are derived here at the boundary (title = highlighted text,
/// content = note). Mapped to/from the domain type by the `From` impls below.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct FrontmatterAnnotation {
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

pub(crate) fn split_document(raw: &str) -> Option<(Frontmatter, String)> {
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
