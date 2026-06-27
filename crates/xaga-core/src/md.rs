//! Intersection of Markdown and domain
//! File system concerns should be managed by `persistence` layer

use crate::{
    fm::{Frontmatter, split_document},
    types::{Annotation, Article, Entry, Topic},
};

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
