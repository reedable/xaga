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

pub fn render_topic(
    id: &str,
    title: &str,
    created_at: &str,
    updated_at: &str,
    threshold: Option<f32>,
    items: &[CollectionItem],
    tags: &[String],
    description: &str,
) -> String {
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

    let fm = Fm {
        id,
        title,
        created: created_at,
        updated: updated_at,
        threshold,
        summary: (!description.is_empty()).then_some(description),
        items,
        tags,
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

    tokio::fs::write(
        &path,
        render_topic(
            id,
            title,
            &created_at,
            &updated_at,
            threshold,
            &items,
            &tags,
            description,
        )
        .as_bytes(),
    )
    .await?;

    let is_pinned = tags.iter().any(|t| t == "pinned");
    Ok(Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at,
        updated_at: Some(updated_at),
        description: (!description.is_empty()).then(|| description.to_string()),
        threshold,
        is_pinned,
        entry_count: 0,
        unread_count: 0,
        members: items.iter().map(crate::types::Reference::from).collect(),
    })
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

    let has = fm.tags.iter().any(|t| t == "pinned");
    if has == pinned {
        return Ok(false);
    }

    let mut tags = fm.tags.clone();
    if pinned {
        tags.push("pinned".to_string());
    } else {
        tags.retain(|t| t != "pinned");
    }
    let updated_at = fm.updated_at.clone().unwrap_or_else(|| fm.created_at.clone());
    // Legacy fallback: pre-migration topics keep their blurb in the body. Re-derive the
    // description so the rewrite moves it into the `summary` frontmatter key.
    let description = fm.summary.clone().unwrap_or(body);
    let new = render_topic(
        id,
        &fm.title,
        &fm.created_at,
        &updated_at,
        fm.threshold,
        &fm.items,
        &tags,
        &description,
    );
    tokio::fs::write(&path, new.as_bytes()).await?;
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

    tokio::fs::write(
        &path,
        render_topic(id, title, created_at, updated_at, threshold, &items, tags, description)
            .as_bytes(),
    )
    .await?;

    Ok(Topic {
        id: id.to_string(),
        title: title.to_string(),
        created_at: created_at.to_string(),
        updated_at: Some(updated_at.to_string()),
        description: (!description.is_empty()).then(|| description.to_string()),
        threshold,
        is_pinned: tags.iter().any(|t| t == "pinned"),
        entry_count: 0,
        unread_count: 0,
        members: items.iter().map(crate::types::Reference::from).collect(),
    })
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

/// Returns the topic for the given id.
pub async fn read_topic(data_dir: &Path, id: &str) -> Result<Topic, TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    let is_pinned = fm.tags.iter().any(|t| t == "pinned");
    Ok(Topic {
        id: crate::util::item_id(&path),
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
    })
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
            let is_pinned = fm.tags.iter().any(|t| t == "pinned");
            topics.push(Topic {
                id: crate::util::item_id(&path),
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

/// Returns the union of embedding-matched items and manually curated items for a topic,
/// with the given filter applied and ordered by `sort`. Deduplicates by ID.
pub async fn list_topic_items(
    data_dir: &Path,
    id: &str,
    item_filter: crate::types::ItemFilter,
    sort: crate::types::ItemSort,
) -> Result<Vec<crate::types::SavedItem>, TopicsError> {
    let path = topic_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, _body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    let threshold = fm.threshold.unwrap_or(0.8);

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
                    color: e.entry.color.clone(),
                },
            ));
        }
    }

    // 2. Manually curated entries via the proven filter path.
    let mut manual_entries: Vec<crate::types::Entry> = Vec::new();
    for item in &fm.items {
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
    for item in &fm.items {
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
