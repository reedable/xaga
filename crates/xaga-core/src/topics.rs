use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use uuid::Uuid;

use crate::types::CollectionItem;
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

fn render_topic(
    id: &str,
    title: &str,
    created_at: &str,
    updated_at: &str,
    threshold: Option<f32>,
    items: &[CollectionItem],
    content: &str,
) -> String {
    #[derive(serde::Serialize)]
    struct Fm<'a> {
        id: &'a str,
        title: &'a str,
        created: &'a str,
        updated: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f32>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        items: &'a [CollectionItem],
    }

    let fm = Fm {
        id,
        title,
        created: created_at,
        updated: updated_at,
        threshold,
        items,
    };

    let yaml = serde_yaml::to_string(&fm).expect("frontmatter serialization is infallible");
    format!("---\n{yaml}---\n\n{content}\n")
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
    items: Vec<CollectionItem>,
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

    tokio::fs::write(
        &path,
        render_topic(
            id,
            title,
            &created_at,
            &updated_at,
            threshold,
            &items,
            content,
        )
        .as_bytes(),
    )
    .await?;

    Ok(Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at,
        updated_at: Some(updated_at),
        content: content.to_string(),
        threshold,
        items,
        entry_count: 0,
        unread_count: 0,
    })
}

/// Creates or updates a topic with explicit timestamps — for migration use only.
/// Does not read the existing file or call `Utc::now()`.
pub async fn save_topic_with_timestamps(
    data_dir: &Path,
    id: &str,
    title: &str,
    content: &str,
    threshold: Option<f32>,
    items: Vec<CollectionItem>,
    created_at: &str,
    updated_at: &str,
) -> Result<Topic, TopicsError> {
    let dir = data_dir.join("topics");
    tokio::fs::create_dir_all(&dir).await?;
    let path = topic_path(data_dir, id);

    tokio::fs::write(
        &path,
        render_topic(id, title, created_at, updated_at, threshold, &items, content).as_bytes(),
    )
    .await?;

    Ok(Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at: created_at.to_string(),
        updated_at: Some(updated_at.to_string()),
        content: content.to_string(),
        threshold,
        items,
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
    save_topic(data_dir, &id, title, content, threshold, vec![]).await
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
        items: fm.items,
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
                items: fm.items,
                entry_count: 0,
                unread_count: 0,
            });
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
    let topic = read_topic(data_dir, topic_id).await?;
    if topic.items.iter().any(|i| i.id() == item.id()) {
        return Ok(topic);
    }
    let title = topic.title;
    let content = topic.content;
    let threshold = topic.threshold;
    let mut items = topic.items;
    items.push(item);
    save_topic(data_dir, topic_id, &title, &content, threshold, items).await
}

/// Removes the item with the given ID from the topic's manual items list.
pub async fn remove_topic_item(
    data_dir: &Path,
    topic_id: &str,
    item_id: &str,
) -> Result<Topic, TopicsError> {
    let topic = read_topic(data_dir, topic_id).await?;
    if !topic.items.iter().any(|i| i.id() == item_id) {
        return Ok(topic);
    }
    let title = topic.title;
    let content = topic.content;
    let threshold = topic.threshold;
    let mut items = topic.items;
    items.retain(|i| i.id() != item_id);
    save_topic(data_dir, topic_id, &title, &content, threshold, items).await
}

/// Returns the IDs of all topics whose manual items list contains the given entry/annotation ID.
pub async fn list_entry_topics(
    data_dir: &Path,
    entry_id: &str,
) -> Result<Vec<String>, TopicsError> {
    let topics = list_topics(data_dir).await?;
    Ok(topics
        .into_iter()
        .filter(|t| t.items.iter().any(|i| i.id() == entry_id))
        .map(|t| t.id)
        .collect())
}

/// Returns the union of embedding-matched items and manually curated items for a topic,
/// with the given filter applied and ordered by `sort`. Deduplicates by ID.
pub async fn list_topic_items(
    data_dir: &Path,
    id: &str,
    item_filter: crate::types::ItemFilter,
    sort: crate::types::ItemSort,
) -> Result<Vec<crate::types::SavedItem>, TopicsError> {
    let topic = read_topic(data_dir, id).await?;
    let threshold = topic.threshold.unwrap_or(0.8);

    // Ungated similarity lookup: every embedded item ranked by similarity to the topic.
    // The explicit low floor (-1.0) disables the default 0.5 cutoff so that even weakly
    // related manually-curated items still receive a score for relevance ranking.
    let all_embedded = crate::item::list_embedded_items(
        data_dir,
        crate::item::EmbeddedItemSource::RelatedTo {
            id: id.to_string(),
            limit: None,
            min_similarity: Some(-1.0),
            max_similarity: None,
        },
        crate::types::ItemFilter::All,
    )
    .await
    .unwrap_or_default();

    // id -> similarity, used to score manually curated items.
    let similarity_by_id: std::collections::HashMap<String, f32> = all_embedded
        .iter()
        .map(|e| (e.entry.id.clone(), e.similarity))
        .collect();

    let mut scored: Vec<(Option<f32>, crate::types::SavedItem)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1. Semantic matches at or above the topic threshold, with the requested filter applied.
    let semantic: Vec<_> = all_embedded
        .into_iter()
        .filter(|e| e.similarity >= threshold)
        .collect();
    for e in crate::filter::filter_items(semantic, &item_filter)
        .await
        .unwrap_or_default()
    {
        if seen.insert(e.entry.id.clone()) {
            scored.push((
                Some(e.similarity),
                crate::types::SavedItem {
                    id: e.entry.id.clone(),
                    title: e.entry.title.clone(),
                    created_at: e.entry.created_at.clone(),
                    url: e.entry.url.clone(),
                    is_unread: e.entry.is_unread,
                    is_bookmarked: e.entry.is_bookmarked,
                    reference: if e.reference.is_empty() { None } else { Some(e.reference.clone()) },
                    color: if e.color.is_empty() { None } else { Some(e.color.clone()) },
                },
            ));
        }
    }

    // 2. Manually curated entries via the proven filter path.
    let mut manual_entries: Vec<crate::types::Entry> = Vec::new();
    for item in &topic.items {
        if let CollectionItem::Entry { id } = item {
            if !seen.contains(id) {
                if let Ok(entry) = crate::entries::read_entry(data_dir, id).await {
                    manual_entries.push(entry);
                }
            }
        }
    }
    for entry in crate::filter::filter_items(manual_entries, &item_filter)
        .await
        .unwrap_or_default()
    {
        seen.insert(entry.id.clone());
        let similarity = similarity_by_id.get(&entry.id).copied();
        scored.push((
            similarity,
            crate::types::SavedItem {
                id: entry.id,
                title: entry.title,
                created_at: entry.created_at,
                url: entry.url,
                is_unread: entry.is_unread,
                is_bookmarked: entry.is_bookmarked,
                reference: None,
                color: None,
            },
        ));
    }

    // 3. Manually curated annotations.
    // Annotations have no unread/bookmarked state; filter::filter_items excludes them
    // from Unread and Bookmarked via is_unread()==false and is_bookmarked()==false,
    // and includes them in Saved via !reference().is_empty(). Replicated here since
    // Annotation does not implement Item.
    let include_annotations = matches!(
        item_filter,
        crate::types::ItemFilter::All | crate::types::ItemFilter::Saved
    );
    for item in &topic.items {
        if let CollectionItem::Annotation { id, reference } = item {
            if include_annotations && !seen.contains(id) && !reference.is_empty() {
                if let Ok(article) = crate::articles::read_article(data_dir, reference).await {
                    if let Some(ann) = article.annotations.iter().find(|a| &a.id == id) {
                        seen.insert(ann.id.clone());
                        let similarity = similarity_by_id.get(&ann.id).copied();
                        scored.push((
                            similarity,
                            crate::types::SavedItem {
                                id: ann.id.clone(),
                                title: ann.text.join(" "),
                                created_at: ann.created_at.clone(),
                                url: article.meta.url.clone(),
                                is_unread: false,
                                is_bookmarked: false,
                                reference: Some(reference.clone()),
                                color: Some(ann.color.clone()),
                            },
                        ));
                    }
                }
            }
        }
    }

    // Order the unified set.
    match sort {
        crate::types::ItemSort::Relevance => {
            // Highest similarity first; unscored (unembedded) items last.
            scored.sort_by(|a, b| match (a.0, b.0) {
                (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
            Ok(scored.into_iter().map(|(_, item)| item).collect())
        }
        crate::types::ItemSort::Timestamp => {
            let mut items: Vec<crate::types::SavedItem> =
                scored.into_iter().map(|(_, item)| item).collect();
            crate::filter::sort_items(&mut items);
            Ok(items)
        }
    }
}
