use tauri::State;
use xaga_core::embeddings::{EntityRef, Job, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::persistence::topics;
use xaga_core::query::entities::{self, EmbeddedEntitySource};
use xaga_core::types::{EntityFilter, Kind, ListEntity, RankedEntity, Reference, Topic};

use super::{parse_entity_filter, parse_entity_sort};
use crate::error::AppError;
use crate::{AppState, get_unread_ids};

#[tauri::command]
pub async fn save_topic(
    id: String,
    title: String,
    description: String,
    threshold: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Topic, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    // Edit metadata while preserving existing manual entities — the read+write happens
    // inside the writer's topic lane, so a concurrent entity change can't be clobbered.
    let topic = writer
        .edit_topic_meta(WriteTier::Interactive, id, title, description, threshold)
        .await
        .map_err(AppError::from)?;
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
) -> Result<Topic, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let topic = writer
        .create_topic(WriteTier::Interactive, title, description, threshold)
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::UpsertTopic {
            topic: topic.clone(),
        },
    );
    Ok(topic)
}

#[tauri::command]
pub async fn create_topic_with_member(
    title: String,
    description: String,
    threshold: Option<f32>,
    entity: Reference,
    state: State<'_, AppState>,
) -> Result<Topic, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let member_id = entity.id().to_string();
    let topic = writer
        .create_topic_with_member(
            WriteTier::Interactive,
            title,
            description,
            threshold,
            entity,
        )
        .await
        .map_err(AppError::from)?;
    for job in create_topic_with_member_jobs(&topic, member_id) {
        state.enqueue(Tier::Interactive, job);
    }
    Ok(topic)
}

fn create_topic_with_member_jobs(topic: &Topic, member_id: String) -> [Job; 2] {
    [
        Job::UpsertTopic {
            topic: topic.clone(),
        },
        Job::AddMember {
            topic_id: topic.id.clone(),
            member_id,
        },
    ]
}

#[tauri::command]
pub async fn set_topic_pinned(
    id: String,
    pinned: bool,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    super::pins::set_pin(xaga_core::pins::PinTarget::Topic { id }, pinned, app, state).await
}

#[tauri::command]
pub async fn list_topics(state: State<'_, AppState>) -> Result<Vec<Topic>, AppError> {
    let data_dir = state.library_path();
    let mut topics = topics::list_topics(&data_dir)
        .await
        .map_err(AppError::from)?;

    let unread = get_unread_ids(&state).await;

    for topic in &mut topics {
        let mut all_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Semantic matches via proven path — no content guard, title-only embedding is valid.
        if let Ok(embedded) = entities::list_embedded_entities(
            &data_dir,
            EmbeddedEntitySource::RelatedTo {
                id: topic.id.clone(),
                limit: None,
                min_similarity: topic.threshold.or(Some(0.8)),
                max_similarity: None,
            },
            EntityFilter::All,
        )
        .await
        {
            for e in &embedded {
                all_ids.insert(e.entry.id.clone());
            }
        }

        // Manual entities — both entries and annotations contribute to the count.
        for entity in &topic.members {
            all_ids.insert(entity.id().to_string());
        }

        topic.entry_count = all_ids.len();
        topic.unread_count = all_ids.iter().filter(|id| unread.contains(*id)).count();
    }

    Ok(topics)
}

#[tauri::command]
pub async fn read_topic(id: String, state: State<'_, AppState>) -> Result<Topic, AppError> {
    let data_dir = state.library_path();
    topics::read_topic(&data_dir, &id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn delete_topic(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let writer = crate::commands::require_writer(&state)?;
    writer
        .delete_topic(WriteTier::Interactive, id.clone())
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::Delete {
            entity: EntityRef::new(id, Kind::Topic),
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn add_topic_entity(
    topic_id: String,
    entity: Reference,
    state: State<'_, AppState>,
) -> Result<Topic, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let member_id = entity.id().to_string();
    let topic = writer
        .add_topic_entity(WriteTier::Interactive, topic_id.clone(), entity)
        .await
        .map_err(AppError::from)?;
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
pub async fn remove_topic_entity(
    topic_id: String,
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<Topic, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let topic = writer
        .remove_topic_entity(WriteTier::Interactive, topic_id.clone(), entity_id.clone())
        .await
        .map_err(AppError::from)?;
    state.enqueue(
        Tier::Interactive,
        Job::RemoveMember {
            topic_id,
            member_id: entity_id,
        },
    );
    Ok(topic)
}

#[tauri::command]
pub async fn list_topic_entities(
    id: String,
    entity_filter: Option<String>,
    sort: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ListEntity>, AppError> {
    let data_dir = state.library_path();
    let entity_filter = parse_entity_filter(entity_filter)?;
    let sort = parse_entity_sort(sort)?;
    xaga_core::query::topics::list_topic_entities(&data_dir, &id, entity_filter, sort)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn list_entry_topics(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    // Read the entry's topics from the LanceDB reverse index (the `topics` column,
    // maintained by the topic-membership reconcile) instead of scanning every topic file.
    let db = state.embeddings_read.lock().unwrap().clone();
    let Some(db) = db else {
        return Err(AppError::Message(
            "embeddings read handle unavailable".to_string(),
        ));
    };
    Ok(db
        .find_topic_ids_by_entity_id(&entry_id)
        .await
        .map_err(AppError::from)?
        .unwrap_or_default())
}

#[tauri::command]
pub async fn rank_topics_by_entity(
    entity_id: String,
    min_similarity: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Vec<RankedEntity>, AppError> {
    let data_dir = state.library_path();
    let entries = entities::list_embedded_entities(
        &data_dir,
        EmbeddedEntitySource::TopicsRelatedTo {
            id: entity_id,
            limit: None,
            min_similarity,
        },
        EntityFilter::All,
    )
    .await
    .map_err(AppError::from)?;

    Ok(entries
        .into_iter()
        .map(|e| RankedEntity {
            id: e.entry.id,
            similarity: e.similarity,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_with_member_indexes_topic_before_membership_delta() {
        let topic = Topic {
            id: "topic-1".into(),
            title: "Topic".into(),
            created_at: "2026-10-20T00:00:00Z".into(),
            updated_at: None,
            description: None,
            threshold: None,
            is_pinned: false,
            entry_count: 1,
            unread_count: 0,
            members: vec![],
        };

        let [upsert, membership] = create_topic_with_member_jobs(&topic, "entry-1".into());
        assert!(matches!(
            upsert,
            Job::UpsertTopic { topic: queued } if queued.id == "topic-1"
        ));
        assert!(matches!(
            membership,
            Job::AddMember { topic_id, member_id }
                if topic_id == "topic-1" && member_id == "entry-1"
        ));
    }
}
