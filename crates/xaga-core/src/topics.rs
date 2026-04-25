use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use uuid::Uuid;

use crate::util::{parse_document, parse_frontmatter};

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

fn render_topic(id: &str, title: &str, created_at: &str, updated_at: &str, threshold: Option<f32>, content: &str) -> String {
    let threshold_line = threshold
        .map(|t| format!("threshold: {:.2}\n", t))
        .unwrap_or_default();
    format!("---\nid: {id}\ntitle: {title}\ncreated: {created_at}\nupdated: {updated_at}\n{threshold_line}---\n\n{content}\n")
}

/// Creates or updates a topic at `topics/{id}.md`.
/// Preserves the original `created_at` if the file already exists.
/// Always sets `updated_at` to the current UTC time.
pub async fn save_topic(
    data_dir: &Path,
    id: &str,
    title: &str,
    content: &str,
    threshold: Option<f32>,
) -> Result<Topic, TopicsError> {
    let dir = data_dir.join("topics");
    tokio::fs::create_dir_all(&dir).await?;
    let path = topic_path(data_dir, id);

    let created_at = if tokio::fs::try_exists(&path).await? {
        let raw = tokio::fs::read_to_string(&path).await?;
        parse_frontmatter(&raw)
            .map(|fm| fm.created_at)
            .unwrap_or_else(|| Utc::now().to_rfc3339())
    } else {
        Utc::now().to_rfc3339()
    };

    let updated_at = Utc::now().to_rfc3339();

    tokio::fs::write(&path, render_topic(id, title, &created_at, &updated_at, threshold, content).as_bytes()).await?;

    Ok(Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at,
        updated_at: Some(updated_at),
        content: content.to_string(),
        threshold,
        entry_count: 0,
        unread_count: 0,
    })
}

/// Creates a new topic with a UUID as its ID.
pub async fn create_topic(
    data_dir: &Path,
    title: &str,
    content: &str,
    threshold: Option<f32>,
) -> Result<Topic, TopicsError> {
    let id = Uuid::new_v4().to_string();
    save_topic(data_dir, &id, title, content, threshold).await
}

/// Returns the topic for the given id.
pub async fn read_topic(data_dir: &Path, id: &str) -> Result<Topic, TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    Ok(Topic {
        id: crate::util::item_id(&path),
        title: fm.title,
        created_at: fm.created_at,
        updated_at: fm.updated_at,
        content: body,
        threshold: fm.threshold,
        entry_count: 0,
        unread_count: 0,
    })
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
            topics.push(Topic {
                id: crate::util::item_id(&path),
                title: fm.title,
                created_at: fm.created_at,
                updated_at: fm.updated_at,
                content: body,
                threshold: fm.threshold,
                entry_count: 0,
                unread_count: 0,
            });
        }
    }

    topics.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(topics)
}
