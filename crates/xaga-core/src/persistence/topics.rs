use std::path::Path;

use chrono::Utc;
use thiserror::Error;

use crate::{
    library_writer::WriteToken,
    md::parse_frontmatter,
    md::{Markdown, md_to_topic, parse_document},
    persistence::walk::for_each_md,
    types::{CollectionItem, Topic},
};

#[derive(Debug, Error)]
pub enum TopicsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Topic not found")]
    NotFound,
}

impl TopicsError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            TopicsError::Io(_) => "topic:io",
            TopicsError::NotFound => "topic:not-found",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            TopicsError::Io(_) => "Couldn't read or write this topic.",
            TopicsError::NotFound => "This topic could not be found.",
        }
    }
}

/// Encodes a [`Topic`] (plus its membership `items`) to its `.md` representation — the single topic
/// encoder. `items` is passed alongside the domain object because `Topic.members` (`Vec<Reference>`)
/// is a lossy view of the stored `Vec<CollectionItem>` (it drops the annotation parent `reference`);
/// the renderer uses `items`, not `topic.members`. `tags` is reconstructed from `topic.is_pinned`.
pub(crate) fn render_topic(topic: &Topic, items: &[CollectionItem]) -> String {
    #[derive(serde::Serialize)]
    struct Fm<'a> {
        id: &'a str,
        title: &'a str,
        created: &'a str,
        updated: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f32>,
        // A Topic's blurb is its `description`, stored as the `summary` frontmatter key (the same
        // key entries use). A Topic has no body content, so nothing is written after the `---`.
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<&'a str>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        items: &'a [CollectionItem],
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        tags: &'a [String],
    }

    let mut tags: Vec<String> = Vec::new();
    if topic.is_pinned {
        tags.push(crate::tags::PINNED.to_string());
    }

    let fm = Fm {
        id: &topic.id,
        title: &topic.title,
        created: &topic.created_at,
        updated: topic.updated_at.as_deref().unwrap_or(&topic.created_at),
        threshold: topic.threshold,
        summary: topic.description.as_deref().filter(|s| !s.is_empty()),
        items,
        tags: &tags,
    };

    let yaml = serde_yaml::to_string(&fm).expect("frontmatter serialization is infallible");
    format!("---\n{yaml}---\n")
}

/// Creates or updates a topic at `topics/{id}.md`.
/// Preserves the original `created_at` if the file already exists.
/// Always sets `updated_at` to the current UTC time.
pub(crate) async fn save_topic(
    library_dir: &Path,
    id: &str,
    title: &str,
    description: &str,
    threshold: Option<f32>,
    items: Vec<CollectionItem>,
    _token: WriteToken,
) -> Result<Topic, TopicsError> {
    let dir = crate::library::topics_dir(library_dir);
    tokio::fs::create_dir_all(&dir).await?;
    let path = crate::library::topic_path(library_dir, id);

    // Preserve created_at and any existing tags (e.g. "pinned") across the rewrite.
    let (created_at, tags) = if tokio::fs::try_exists(&path).await? {
        let raw = tokio::fs::read_to_string(&path).await?;
        match parse_frontmatter(&raw) {
            Some(fm) => (fm.created_at, fm.tags),
            None => (Utc::now().to_rfc3339(), vec![]),
        }
    } else {
        (Utc::now().to_rfc3339(), vec![])
    };

    let updated_at = Utc::now().to_rfc3339();

    let topic = Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at,
        updated_at: Some(updated_at),
        description: (!description.is_empty()).then(|| description.to_string()),
        threshold,
        is_pinned: crate::tags::is_pinned(&tags),
        entry_count: 0,
        unread_count: 0,
        members: items.iter().map(crate::types::Reference::from).collect(),
    };
    crate::fs::write_atomic_async(path.clone(), render_topic(&topic, &items).into_bytes()).await?;
    Ok(topic)
}

/// Adds or removes the `pinned` frontmatter tag on a topic's `.md`. Returns true if it changed.
/// Preserves the topic's timestamps (a pin toggle is not a content edit). The `is_pinned` LanceDB
/// column is updated by the watcher → `sync_row` re-read; membership is unchanged so the reconcile
/// sees no member delta. Mirrors `feeds::set_feed_pinned`.
pub(crate) async fn set_topic_pinned(
    library_dir: &Path,
    id: &str,
    pinned: bool,
    _token: WriteToken,
) -> Result<bool, TopicsError> {
    let path = crate::library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;

    if crate::tags::is_pinned(&fm.tags) == pinned {
        return Ok(false);
    }

    let updated_at = fm
        .updated_at
        .clone()
        .unwrap_or_else(|| fm.created_at.clone());
    let items = fm.items.clone();
    // Decode to the domain Topic (which handles the legacy summary/body description fallback), then
    // flip the pinned flag and preserve the timestamp — a pin toggle is not a content edit.
    let mut topic = md_to_topic(Markdown {
        id: id.to_string(),
        fm,
        body,
    });
    topic.is_pinned = pinned;
    topic.updated_at = Some(updated_at);
    crate::fs::write_atomic_async(path.clone(), render_topic(&topic, &items).into_bytes()).await?;
    Ok(true)
}

/// Returns the topic for the given id.
pub async fn read_topic(library_dir: &Path, id: &str) -> Result<Topic, TopicsError> {
    let path = crate::library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    Ok(md_to_topic(Markdown {
        id: crate::util::item_id(&path),
        fm,
        body,
    }))
}

/// Returns a topic's stored membership (the `.md` `items:` list) — the storage shape
/// (`CollectionItem`, carrying the annotation parent `reference`), not the domain `members`.
pub async fn read_topic_items(
    library_dir: &Path,
    id: &str,
) -> Result<Vec<CollectionItem>, TopicsError> {
    let path = crate::library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, _body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    Ok(fm.items)
}

/// Deletes the topic file for the given id.
pub(crate) async fn delete_topic(
    library_dir: &Path,
    id: &str,
    _token: WriteToken,
) -> Result<(), TopicsError> {
    let path = crate::library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

/// Returns all topics, sorted by id.
pub async fn list_topics(library_dir: &Path) -> Result<Vec<Topic>, TopicsError> {
    let topics_dir = crate::library::topics_dir(library_dir);
    let mut topics: Vec<Topic> = Vec::new();

    // for_each_md handles the missing dir and skips any file that fails to read or parse.
    for_each_md(&topics_dir, &md_to_topic, |t| topics.push(t)).await;

    topics.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(topics)
}

/// Appends `item` to the topic's manual items list. No-op if already present.
pub(crate) async fn add_topic_item(
    library_dir: &Path,
    topic_id: &str,
    item: CollectionItem,
    token: WriteToken,
) -> Result<Topic, TopicsError> {
    let path = crate::library::topic_path(library_dir, topic_id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    if fm.items.iter().any(|i| i.id() == item.id()) {
        return read_topic(library_dir, topic_id).await;
    }
    // Legacy fallback: pre-migration topics keep their blurb in the body.
    let description = fm.summary.unwrap_or(body);
    let mut items = fm.items;
    items.push(item);
    save_topic(
        library_dir,
        topic_id,
        &fm.title,
        &description,
        fm.threshold,
        items,
        token,
    )
    .await
}

/// Swaps a member's id in place — same list position, same member type. Used when an
/// annotation is replaced under a new id (a color change is delete/create); position is
/// preserved because membership is ordered. No-op if `old_id` is not a member.
pub(crate) async fn replace_topic_item(
    library_dir: &Path,
    topic_id: &str,
    old_id: &str,
    new_item: CollectionItem,
    token: WriteToken,
) -> Result<Topic, TopicsError> {
    let path = crate::library::topic_path(library_dir, topic_id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    let Some(pos) = fm.items.iter().position(|i| i.id() == old_id) else {
        return read_topic(library_dir, topic_id).await;
    };
    // Legacy fallback: pre-migration topics keep their blurb in the body.
    let description = fm.summary.unwrap_or(body);
    let mut items = fm.items;
    items[pos] = new_item;
    save_topic(
        library_dir,
        topic_id,
        &fm.title,
        &description,
        fm.threshold,
        items,
        token,
    )
    .await
}

/// Removes the item with the given ID from the topic's manual items list.
pub(crate) async fn remove_topic_item(
    library_dir: &Path,
    topic_id: &str,
    item_id: &str,
    token: WriteToken,
) -> Result<Topic, TopicsError> {
    let path = crate::library::topic_path(library_dir, topic_id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    if !fm.items.iter().any(|i| i.id() == item_id) {
        return read_topic(library_dir, topic_id).await;
    }
    // Legacy fallback: pre-migration topics keep their blurb in the body.
    let description = fm.summary.unwrap_or(body);
    let mut items = fm.items;
    items.retain(|i| i.id() != item_id);
    save_topic(
        library_dir,
        topic_id,
        &fm.title,
        &description,
        fm.threshold,
        items,
        token,
    )
    .await
}
