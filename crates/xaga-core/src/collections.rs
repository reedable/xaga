use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use crate::types::{Collection, CollectionItem};

#[derive(Debug, Error)]
pub enum CollectionsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Collection not found")]
    NotFound,
    #[error("Parse error: {0}")]
    Parse(#[from] serde_yaml::Error),
}

/// Serde-friendly representation of the on-disk YAML file.
#[derive(Debug, Serialize, Deserialize)]
struct CollectionFile {
    id: String,
    title: String,
    created: String,
    updated: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    items: Vec<CollectionItem>,
}

fn collection_path(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("collections").join(format!("{id}.yaml"))
}

fn file_to_collection(f: CollectionFile, unread_ids: &std::collections::HashSet<String>) -> Collection {
    let unread_count = f.items.iter().filter(|i| {
        matches!(i, CollectionItem::Entry { id } if unread_ids.contains(id))
    }).count();
    let entry_count = f.items.len();
    Collection {
        id: f.id,
        title: f.title,
        created_at: f.created,
        updated_at: Some(f.updated),
        items: f.items,
        entry_count,
        unread_count,
    }
}

async fn read_file(path: &Path) -> Result<CollectionFile, CollectionsError> {
    if !tokio::fs::try_exists(path).await? {
        return Err(CollectionsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(path).await?;
    Ok(serde_yaml::from_str(&raw)?)
}

async fn write_file(path: &Path, f: &CollectionFile) -> Result<(), CollectionsError> {
    let yaml = serde_yaml::to_string(f)?;
    tokio::fs::write(path, yaml.as_bytes()).await?;
    Ok(())
}

pub async fn create_collection(data_dir: &Path, title: &str) -> Result<Collection, CollectionsError> {
    let id = Uuid::new_v4().to_string();
    save_collection(data_dir, &id, title, vec![]).await
}

pub async fn save_collection(
    data_dir: &Path,
    id: &str,
    title: &str,
    items: Vec<CollectionItem>,
) -> Result<Collection, CollectionsError> {
    let dir = data_dir.join("collections");
    tokio::fs::create_dir_all(&dir).await?;
    let path = collection_path(data_dir, id);

    let created = if tokio::fs::try_exists(&path).await? {
        read_file(&path).await.map(|f| f.created).unwrap_or_else(|_| Utc::now().to_rfc3339())
    } else {
        Utc::now().to_rfc3339()
    };

    let f = CollectionFile { id: id.to_string(), title: title.to_string(), created, updated: Utc::now().to_rfc3339(), items };
    write_file(&path, &f).await?;
    Ok(file_to_collection(f, &Default::default()))
}

pub async fn read_collection(data_dir: &Path, id: &str) -> Result<Collection, CollectionsError> {
    let f = read_file(&collection_path(data_dir, id)).await?;
    Ok(file_to_collection(f, &Default::default()))
}

pub async fn delete_collection(data_dir: &Path, id: &str) -> Result<(), CollectionsError> {
    let path = collection_path(data_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(CollectionsError::NotFound);
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

pub async fn list_collections(data_dir: &Path) -> Result<Vec<Collection>, CollectionsError> {
    let dir = data_dir.join("collections");
    if !tokio::fs::try_exists(&dir).await? {
        return Ok(vec![]);
    }
    let mut entries = tokio::fs::read_dir(&dir).await?;
    let mut collections = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let raw = tokio::fs::read_to_string(&path).await?;
        if let Ok(f) = serde_yaml::from_str::<CollectionFile>(&raw) {
            collections.push(file_to_collection(f, &Default::default()));
        }
    }
    collections.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(collections)
}

/// Appends `item` to the collection's list. No-op if an item with the same ID already exists.
pub async fn add_item(
    data_dir: &Path,
    collection_id: &str,
    item: CollectionItem,
) -> Result<Collection, CollectionsError> {
    let path = collection_path(data_dir, collection_id);
    let mut f = read_file(&path).await?;
    if !f.items.iter().any(|i| i.id() == item.id()) {
        f.items.push(item);
        f.updated = Utc::now().to_rfc3339();
        write_file(&path, &f).await?;
    }
    Ok(file_to_collection(f, &Default::default()))
}

/// Returns the IDs of all collections that contain the given entry ID.
pub async fn list_entry_collections(data_dir: &Path, entry_id: &str) -> Result<Vec<String>, CollectionsError> {
    let collections = list_collections(data_dir).await?;
    Ok(collections.into_iter()
        .filter(|c| c.items.iter().any(|i| i.id() == entry_id))
        .map(|c| c.id)
        .collect())
}

/// Returns the items of a collection as `SavedItem`s, with the filter applied to entries.
/// Annotations are included for `All` and `Saved` filters; excluded for `Unread` and `Bookmarked`.
/// Insertion order is preserved.
pub async fn list_collection_items(
    data_dir: &Path,
    id: &str,
    item_filter: crate::types::ItemFilter,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Vec<crate::types::SavedItem>, CollectionsError> {
    let collection = read_collection(data_dir, id).await?;

    let annotation_ids: HashSet<String> = collection.items.iter()
        .filter_map(|i| match i { CollectionItem::Annotation { id } => Some(id.clone()), _ => None })
        .collect();

    let annotation_map: HashMap<String, crate::types::AnnotationSummary> =
        if !annotation_ids.is_empty() {
            match crate::embeddings::EmbeddingsDb::open(data_dir).await {
                Ok(db) => match db.list_annotations().await {
                    Ok(anns) => anns.into_iter()
                        .filter(|a| annotation_ids.contains(&a.id))
                        .map(|a| (a.id.clone(), a))
                        .collect(),
                    Err(_) => HashMap::new(),
                },
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

    let include_annotations = matches!(
        item_filter,
        crate::types::ItemFilter::All | crate::types::ItemFilter::Saved
    );

    let mut items = Vec::new();
    for collection_item in &collection.items {
        match collection_item {
            CollectionItem::Entry { id } => {
                if let Ok(entry) = crate::entries::read_entry(data_dir, id).await {
                    let is_unread = unread.contains(&entry.id);
                    let is_bookmarked = bookmarked.contains(&entry.id);
                    let include = match item_filter {
                        crate::types::ItemFilter::All => true,
                        crate::types::ItemFilter::Unread => is_unread,
                        crate::types::ItemFilter::Bookmarked => is_bookmarked,
                        crate::types::ItemFilter::Saved => is_bookmarked,
                    };
                    if include {
                        items.push(crate::types::SavedItem {
                            id: entry.id,
                            title: entry.title,
                            created_at: entry.created_at,
                            url: entry.url,
                            reference: None,
                            color: None,
                        });
                    }
                }
            }
            CollectionItem::Annotation { id } => {
                if include_annotations {
                    if let Some(ann) = annotation_map.get(id) {
                        items.push(crate::types::SavedItem {
                            id: ann.id.clone(),
                            title: ann.title.clone(),
                            created_at: ann.created_at.clone(),
                            url: ann.url.clone(),
                            reference: Some(ann.reference.clone()),
                            color: Some(ann.color.clone()),
                        });
                    }
                }
            }
        }
    }

    Ok(items)
}

/// Removes the item with the given ID from the collection's list.
pub async fn remove_item(
    data_dir: &Path,
    collection_id: &str,
    item_id: &str,
) -> Result<Collection, CollectionsError> {
    let path = collection_path(data_dir, collection_id);
    let mut f = read_file(&path).await?;
    let before = f.items.len();
    f.items.retain(|i| i.id() != item_id);
    if f.items.len() != before {
        f.updated = Utc::now().to_rfc3339();
        write_file(&path, &f).await?;
    }
    Ok(file_to_collection(f, &Default::default()))
}
