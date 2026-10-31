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
    #[error("malformed frontmatter: {0}")]
    Malformed(#[from] crate::md::MdError),
    #[error("file too large (max {max_bytes} bytes)")]
    TooLarge { max_bytes: u64 },
}

impl From<crate::fs::ReadMdError> for TopicsError {
    fn from(e: crate::fs::ReadMdError) -> Self {
        match e {
            crate::fs::ReadMdError::TooLarge { max_bytes } => TopicsError::TooLarge { max_bytes },
            crate::fs::ReadMdError::Io(e) => TopicsError::Io(e),
        }
    }
}

impl TopicsError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            TopicsError::Io(_) => "topic:io",
            TopicsError::NotFound => "topic:not-found",
            TopicsError::Malformed(_) => "topic:malformed",
            TopicsError::TooLarge { .. } => "topic:too-large",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            TopicsError::Io(_) => "Couldn't read or write this topic.",
            TopicsError::NotFound => "This topic could not be found.",
            TopicsError::Malformed(_) => "This topic's file is malformed and couldn't be read.",
            TopicsError::TooLarge { .. } => "This topic's file is too large to open.",
        }
    }
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

    let existing = if tokio::fs::try_exists(&path).await? {
        let raw = crate::fs::read_md_capped(&path).await?;
        let fm = parse_frontmatter(&raw)?;
        Some((raw, fm))
    } else {
        None
    };
    let created_at = existing
        .as_ref()
        .map(|(_, fm)| fm.created_at.clone())
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let is_pinned = crate::pins::Pins::read(library_dir)
        .await
        .map_err(std::io::Error::other)?
        .contains(&crate::pins::PinTarget::Topic { id: id.into() });

    let updated_at = Utc::now().to_rfc3339();

    let topic = Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at,
        updated_at: Some(updated_at),
        description: Some(description.to_string()),
        threshold,
        is_pinned,
        entry_count: 0,
        unread_count: 0,
        members: items.iter().map(crate::types::Reference::from).collect(),
    };
    let mut updates = vec![
        ("title", serde_json::json!(title)),
        ("updated", serde_json::json!(topic.updated_at)),
        ("summary", serde_json::json!(description)),
        ("threshold", serde_json::json!(threshold)),
        ("items", serde_json::json!(items)),
    ];
    if existing.is_none() {
        updates.push(("id", serde_json::json!(id)));
        updates.push(("created", serde_json::json!(topic.created_at)));
    }
    super::documents::upsert(&path, &updates, None, _token)
        .await
        .map_err(document_error)?;
    Ok(topic)
}

/// Returns the topic for the given id.
pub async fn read_topic(library_dir: &Path, id: &str) -> Result<Topic, TopicsError> {
    let path = crate::library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
    let mut topic = md_to_topic(Markdown {
        id: crate::util::item_id(&path),
        fm,
        body,
    });
    topic.is_pinned = crate::pins::Pins::read(library_dir)
        .await
        .map_err(std::io::Error::other)?
        .contains(&crate::pins::PinTarget::Topic { id: id.into() });
    Ok(topic)
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, _body) = parse_document(&raw)?;
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

    let pins = crate::pins::Pins::read(library_dir)
        .await
        .map_err(std::io::Error::other)?;
    let pin_targets: std::collections::HashSet<_> = pins.pins.into_iter().collect();
    for topic in &mut topics {
        topic.is_pinned = pin_targets.contains(&crate::pins::PinTarget::Topic {
            id: topic.id.clone(),
        });
    }
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
    if fm.items.iter().any(|i| i.id() == item.id()) {
        return read_topic(library_dir, topic_id).await;
    }
    // When `summary` is absent, the body supplies the description.
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
    let Some(pos) = fm.items.iter().position(|i| i.id() == old_id) else {
        return read_topic(library_dir, topic_id).await;
    };
    // When `summary` is absent, the body supplies the description.
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
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
    if !fm.items.iter().any(|i| i.id() == item_id) {
        return read_topic(library_dir, topic_id).await;
    }
    // When `summary` is absent, the body supplies the description.
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

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = "---\ntitle: Original\ncreated: '2026-01-01T00:00:00Z'\ntags: [pinned, research]\n# User metadata\ncustom: {keep: true}\n---\n\nUser-authored body\n\n";

    #[tokio::test]
    async fn editing_topic_with_block_items_preserves_members() {
        let dir = tempfile::tempdir().unwrap();
        let path = crate::library::topic_path(dir.path(), "topic");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let raw = "---\nid: topic\ntitle: Original\ncreated: '2026-01-01'\nupdated: '2026-01-02'\nthreshold: 0.8\nitems:\n  - type: entry\n    id: article\n  - type: annotation\n    id: highlight\n    reference: article\n---\n";
        for raw in [
            raw.to_string(),
            raw.replace("  - type:", "- type:")
                .replace("    id:", "  id:")
                .replace("    reference:", "  reference:"),
        ] {
            std::fs::write(&path, &raw).unwrap();
            let fm = parse_frontmatter(&raw).unwrap();
            let expected_items = serde_json::to_value(&fm.items).unwrap();
            save_topic(
                dir.path(),
                "topic",
                "Updated",
                "",
                fm.threshold,
                fm.items,
                WriteToken::for_test(),
            )
            .await
            .unwrap();
            let saved = read_topic(dir.path(), "topic").await.unwrap();
            assert_eq!(saved.title, "Updated");
            assert_eq!(saved.members.len(), 2);
            let stored = std::fs::read_to_string(&path).unwrap();
            assert_eq!(
                serde_json::to_value(parse_frontmatter(&stored).unwrap().items).unwrap(),
                expected_items
            );
        }
    }

    #[tokio::test]
    async fn topic_updates_preserve_user_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = crate::library::topic_path(dir.path(), "topic");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, ORIGINAL).unwrap();
        save_topic(
            dir.path(),
            "topic",
            "Changed",
            "Description",
            None,
            vec![],
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            parse_frontmatter(&saved).unwrap().tags,
            vec!["pinned", "research"]
        );

        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            crate::md::test_frontmatter_values(&saved)["custom"],
            crate::md::test_frontmatter_values(ORIGINAL)["custom"]
        );
        assert!(saved.ends_with("---\n\nUser-authored body\n\n"));
        let fm = parse_frontmatter(&saved).unwrap();
        assert_eq!(fm.tags, vec!["pinned", "research"]);
        assert_eq!(fm.created_at, "2026-01-01T00:00:00Z");
        assert_eq!(fm.summary.as_deref(), Some("Description"));
    }

    #[tokio::test]
    async fn malformed_topic_refuses_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = crate::library::topic_path(dir.path(), "topic");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let malformed = ORIGINAL.replace("tags: [pinned, research]", "tags: {invalid: true}");
        std::fs::write(&path, &malformed).unwrap();
        let result = save_topic(
            dir.path(),
            "topic",
            "Changed",
            "",
            None,
            vec![],
            WriteToken::for_test(),
        )
        .await;
        assert!(matches!(result, Err(TopicsError::Malformed(_))));
        assert_eq!(std::fs::read_to_string(path).unwrap(), malformed);
    }
}

fn document_error(error: super::entries::EntryError) -> TopicsError {
    use super::entries::EntryError;
    match error {
        EntryError::Io(e) => TopicsError::Io(e),
        EntryError::Malformed(e) => TopicsError::Malformed(e),
        EntryError::TooLarge { max_bytes } => TopicsError::TooLarge { max_bytes },
        EntryError::NotFound => TopicsError::NotFound,
        other => TopicsError::Io(std::io::Error::other(other)),
    }
}
