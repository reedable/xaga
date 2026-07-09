use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, StatePatch, Tier};
use xaga_core::persistence::topics;
use xaga_core::query::items::{self, EmbeddedItemSource};
use xaga_core::types::{CollectionItem, ItemFilter, Kind, ListItem, RankedItem, Topic};

use super::{parse_item_filter, parse_item_sort};
use crate::{AppState, get_unread_ids};

#[tauri::command]
pub async fn save_topic(
    id: String,
    title: String,
    description: String,
    threshold: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    // Preserve existing manual items when editing from the UI
    let existing_items = topics::read_topic_items(&data_dir, &id)
        .await
        .unwrap_or_default();
    let topic = topics::save_topic(
        &data_dir,
        &id,
        &title,
        &description,
        threshold,
        existing_items,
    )
    .await
    .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::UpsertTopic {
            topic: topic.clone(),
        },
    );
    Ok(topic)
}

#[tauri::command]
pub async fn create_topic(
    title: String,
    description: String,
    threshold: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    let topic = topics::create_topic(&data_dir, &title, &description, threshold)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::UpsertTopic {
            topic: topic.clone(),
        },
    );
    Ok(topic)
}

#[tauri::command]
pub async fn set_topic_pinned(
    id: String,
    pinned: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::set_topic_pinned(&data_dir, &id, pinned)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Topic),
            state: StatePatch {
                is_pinned: Some(pinned),
                ..Default::default()
            },
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn list_topics(state: State<'_, AppState>) -> Result<Vec<Topic>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let mut topics = topics::list_topics(&data_dir)
        .await
        .map_err(|e| e.to_string())?;

    let unread = get_unread_ids(&state).await;

    for topic in &mut topics {
        let mut all_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Semantic matches via proven path — no content guard, title-only embedding is valid.
        if let Ok(embedded) = items::list_embedded_items(
            &data_dir,
            EmbeddedItemSource::RelatedTo {
                id: topic.id.clone(),
                limit: None,
                min_similarity: topic.threshold.or(Some(0.8)),
                max_similarity: None,
            },
            ItemFilter::All,
        )
        .await
        {
            for e in &embedded {
                all_ids.insert(e.entry.id.clone());
            }
        }

        // Manual items — both entries and annotations contribute to the count.
        for item in &topic.members {
            all_ids.insert(item.id().to_string());
        }

        topic.entry_count = all_ids.len();
        topic.unread_count = all_ids.iter().filter(|id| unread.contains(*id)).count();
    }

    Ok(topics)
}

#[tauri::command]
pub async fn read_topic(id: String, state: State<'_, AppState>) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::read_topic(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_topic(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let data_dir = state.library.read().unwrap().clone();
    topics::delete_topic(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::Delete {
            entity: EntityRef::new(id, Kind::Topic),
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn add_topic_item(
    topic_id: String,
    item: CollectionItem,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    let member_id = item.id().to_string();
    let topic = topics::add_topic_item(&data_dir, &topic_id, item)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::AddMember {
            topic_id,
            member_id,
        },
    );
    Ok(topic)
}

#[tauri::command]
pub async fn remove_topic_item(
    topic_id: String,
    item_id: String,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    let data_dir = state.library.read().unwrap().clone();
    let topic = topics::remove_topic_item(&data_dir, &topic_id, &item_id)
        .await
        .map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::RemoveMember {
            topic_id,
            member_id: item_id,
        },
    );
    Ok(topic)
}

#[tauri::command]
pub async fn list_topic_items(
    id: String,
    item_filter: Option<String>,
    sort: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ListItem>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;
    let sort = parse_item_sort(sort)?;
    xaga_core::query::topics::list_topic_items(&data_dir, &id, item_filter, sort)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_entry_topics(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // Phase C: read the entry's topics from the LanceDB reverse index (the `topics` column,
    // maintained by the topic-membership reconcile) instead of scanning every topic file.
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Err("embeddings read handle unavailable".to_string());
    };
    Ok(db
        .get_entry(&entry_id)
        .await
        .map_err(|e| e.to_string())?
        .map(|e| e.topics)
        .unwrap_or_default())
}

#[tauri::command]
pub async fn rank_topics_by_item(
    item_id: String,
    min_similarity: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Vec<RankedItem>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let entries = items::list_embedded_items(
        &data_dir,
        EmbeddedItemSource::TopicsRelatedTo {
            id: item_id,
            limit: None,
            min_similarity,
        },
        ItemFilter::All,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(entries
        .into_iter()
        .map(|e| RankedItem {
            id: e.entry.id,
            similarity: e.similarity,
        })
        .collect())
}
