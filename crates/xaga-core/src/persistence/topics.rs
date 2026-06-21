use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use uuid::Uuid;

use crate::types::CollectionItem;

use super::frontmatter::{parse_document, parse_frontmatter};

pub use crate::types::Topic;

#[derive(Debug, Error)]
pub enum TopicsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Topic not found")]
    NotFound,
}

pub(crate) fn topic_path(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("topics").join(format!("{id}.md"))
}

/// Encodes a [`Topic`] (plus its membership `items`) to its `.md` representation — the single topic
/// encoder. `items` is passed alongside the domain object because `Topic.members` (`Vec<Reference>`)
/// is a lossy view of the stored `Vec<CollectionItem>` (it drops the annotation parent `reference`);
/// the renderer uses `items`, not `topic.members`. `tags` is reconstructed from `topic.is_pinned`.
pub fn render_topic(topic: &Topic, items: &[CollectionItem]) -> String {
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
pub async fn save_topic(
    data_dir: &Path,
    id: &str,
    title: &str,
    description: &str,
    threshold: Option<f32>,
    items: Vec<CollectionItem>,
) -> Result<Topic, TopicsError> {
    let dir = data_dir.join("topics");
    tokio::fs::create_dir_all(&dir).await?;
    let path = topic_path(data_dir, id);

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
    tokio::fs::write(&path, render_topic(&topic, &items).as_bytes()).await?;
    Ok(topic)
}

/// Adds or removes the `pinned` frontmatter tag on a topic's `.md`. Returns true if it changed.
/// Preserves the topic's timestamps (a pin toggle is not a content edit). The `is_pinned` LanceDB
/// column is updated by the watcher → `sync_row` re-read; membership is unchanged so the reconcile
/// sees no member delta. Mirrors `feeds::set_feed_pinned`.
pub async fn set_topic_pinned(data_dir: &Path, id: &str, pinned: bool) -> Result<bool, TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;

    if crate::tags::is_pinned(&fm.tags) == pinned {
        return Ok(false);
    }

    let updated_at = fm.updated_at.clone().unwrap_or_else(|| fm.created_at.clone());
    let items = fm.items.clone();
    // Decode to the domain Topic (which handles the legacy summary/body description fallback), then
    // flip the pinned flag and preserve the timestamp — a pin toggle is not a content edit.
    let mut topic = topic_from_document(id, fm, body);
    topic.is_pinned = pinned;
    topic.updated_at = Some(updated_at);
    tokio::fs::write(&path, render_topic(&topic, &items).as_bytes()).await?;
    Ok(true)
}

/// Creates or updates a topic with explicit timestamps — for migration use only.
/// Does not read the existing file or call `Utc::now()`.
pub async fn save_topic_with_timestamps(
    data_dir: &Path,
    id: &str,
    title: &str,
    description: &str,
    threshold: Option<f32>,
    items: Vec<CollectionItem>,
    tags: &[String],
    created_at: &str,
    updated_at: &str,
) -> Result<Topic, TopicsError> {
    let dir = data_dir.join("topics");
    tokio::fs::create_dir_all(&dir).await?;
    let path = topic_path(data_dir, id);

    let topic = Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at: created_at.to_string(),
        updated_at: Some(updated_at.to_string()),
        description: (!description.is_empty()).then(|| description.to_string()),
        threshold,
        is_pinned: crate::tags::is_pinned(tags),
        entry_count: 0,
        unread_count: 0,
        members: items.iter().map(crate::types::Reference::from).collect(),
    };
    tokio::fs::write(&path, render_topic(&topic, &items).as_bytes()).await?;
    Ok(topic)
}

/// Creates a new topic with a UUID as its ID.
pub async fn create_topic(
    data_dir: &Path,
    title: &str,
    description: &str,
    threshold: Option<f32>,
) -> Result<Topic, TopicsError> {
    let id = Uuid::new_v4().to_string();
    save_topic(data_dir, &id, title, description, threshold, vec![]).await
}

/// Builds a [`Topic`] from an already-parsed document (used by recovery, which parses once during
/// its scan and hands the domain object straight to indexing). Mirrors [`read_topic`].
pub fn topic_from_document(id: &str, fm: super::frontmatter::Frontmatter, body: String) -> Topic {
    let is_pinned = crate::tags::is_pinned(&fm.tags);
    Topic {
        id: id.to_string(),
        title: fm.title,
        created_at: fm.created_at,
        updated_at: fm.updated_at,
        // `summary` is the canonical home; fall back to the body for pre-migration topics.
        description: fm.summary.or_else(|| (!body.is_empty()).then_some(body)),
        threshold: fm.threshold,
        is_pinned,
        entry_count: 0,
        unread_count: 0,
        members: fm.items.iter().map(crate::types::Reference::from).collect(),
    }
}

/// Returns the topic for the given id.
pub async fn read_topic(data_dir: &Path, id: &str) -> Result<Topic, TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    Ok(topic_from_document(&crate::util::item_id(&path), fm, body))
}

/// Returns a topic's stored membership (the `.md` `items:` list) — the storage shape
/// (`CollectionItem`, carrying the annotation parent `reference`), not the domain `members`.
pub async fn read_topic_items(data_dir: &Path, id: &str) -> Result<Vec<CollectionItem>, TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, _body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    Ok(fm.items)
}

/// Deletes the topic file for the given id.
pub async fn delete_topic(data_dir: &Path, id: &str) -> Result<(), TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

/// Returns all topics, sorted by id.
pub async fn list_topics(data_dir: &Path) -> Result<Vec<Topic>, TopicsError> {
    let topics_dir = data_dir.join("topics");
    if !tokio::fs::try_exists(&topics_dir).await? {
        return Ok(vec![]);
    }

    let mut dir_entries = tokio::fs::read_dir(&topics_dir).await?;
    let mut topics: Vec<Topic> = Vec::new();

    while let Some(dir_entry) = dir_entries.next_entry().await? {
        let path = dir_entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let raw = tokio::fs::read_to_string(&path).await?;
        if let Some((fm, body)) = parse_document(&raw) {
            topics.push(topic_from_document(&crate::util::item_id(&path), fm, body));
        }
    }

    topics.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(topics)
}

/// Appends `item` to the topic's manual items list. No-op if already present.
pub async fn add_topic_item(
    data_dir: &Path,
    topic_id: &str,
    item: CollectionItem,
) -> Result<Topic, TopicsError> {
    let path = topic_path(data_dir, topic_id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    if fm.items.iter().any(|i| i.id() == item.id()) {
        return read_topic(data_dir, topic_id).await;
    }
    // Legacy fallback: pre-migration topics keep their blurb in the body.
    let description = fm.summary.unwrap_or(body);
    let mut items = fm.items;
    items.push(item);
    save_topic(data_dir, topic_id, &fm.title, &description, fm.threshold, items).await
}

/// Removes the item with the given ID from the topic's manual items list.
pub async fn remove_topic_item(
    data_dir: &Path,
    topic_id: &str,
    item_id: &str,
) -> Result<Topic, TopicsError> {
    let path = topic_path(data_dir, topic_id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    if !fm.items.iter().any(|i| i.id() == item_id) {
        return read_topic(data_dir, topic_id).await;
    }
    // Legacy fallback: pre-migration topics keep their blurb in the body.
    let description = fm.summary.unwrap_or(body);
    let mut items = fm.items;
    items.retain(|i| i.id() != item_id);
    save_topic(data_dir, topic_id, &fm.title, &description, fm.threshold, items).await
}
