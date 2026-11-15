use crate::AppState;
use crate::error::AppError;
use tauri::State;
use xaga_core::embeddings::{EmbeddingsDb, Job, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::types::{Article, PartialAnnotation};

#[tauri::command]
pub(crate) async fn add_annotation(
    article_id: String,
    id: String,
    text: Vec<String>,
    occurrence: Vec<u32>,
    color: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<Article, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let annotation = xaga_core::types::Annotation {
        tags: vec![],
        id,
        title: text.join(" "),
        reference: article_id.clone(),
        text,
        occurrence,
        color,
        content: note.unwrap_or_default(),
        created_at: String::new(),
    };
    let article = writer
        .add_annotation(WriteTier::Interactive, article_id.clone(), annotation)
        .await
        .map_err(AppError::from)?;
    // Insert/refresh the annotation row now (embedder-free, immediately listable), then defer the
    // ONNX embed so the new annotation gets a vector — without it, search_related/rank_topics_by_entity
    // raise "id not found". embed_entry_vector is checksum-gated, so it skips the unchanged article
    // body and only fills the pending annotation vector via embed_annotations. Tier mirrors the
    // live-edit embed_follow() (Interactive -> Regular). Both jobs carry the domain object.
    state.enqueue_annotations(article.clone()).await;
    state.enqueue(
        Tier::Regular,
        Job::EmbedEntry {
            article: article.clone(),
        },
    );
    Ok(article)
}

/// Replaces an annotation under a new id — the write path for a color change, which is
/// modeled as delete/create (color is part of the annotation's identity). The delete/create
/// lands in one atomic `.md` write; a crash after it strands nothing silent, because the
/// changed id set is exactly what the recovery and sync membership diffs detect. The cost
/// (one annotation re-embed) is attributed to the annotation that changed.
#[tauri::command]
pub(crate) async fn replace_annotation(
    window: tauri::Window,
    article_id: String,
    old_id: String,
    id: String,
    text: Vec<String>,
    occurrence: Vec<u32>,
    color: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<Article, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let annotation = xaga_core::types::Annotation {
        tags: vec![],
        id: id.clone(),
        title: text.join(" "),
        reference: article_id.clone(),
        text,
        occurrence,
        color,
        content: note.unwrap_or_default(),
        created_at: String::new(),
    };
    let article = writer
        .replace_annotation(
            WriteTier::Interactive,
            article_id.clone(),
            old_id.clone(),
            annotation,
        )
        .await
        .map_err(AppError::from)?;

    // Row reconcile first: SyncAnnotations deletes the old id's row (gone from the `.md`) and
    // inserts the new one with a pending vector; the deferred EmbedEntry fills it. Enqueued
    // before the member jobs below so the new row exists when AddMember writes its `topics`.
    // Tier mirrors add_annotation.
    state.enqueue_annotations(article.clone()).await;
    state.enqueue(
        Tier::Regular,
        Job::EmbedEntry {
            article: article.clone(),
        },
    );

    // Migrate topic membership old id -> new id: swap in place in each topic `.md` (position
    // and type preserved), then move the reverse index. Topics are read from the LanceDB
    // reverse index, mirroring remove_annotation — best-effort; recovery's unconditional
    // reverse-index rebuild corrects any miss.
    let topic_ids: Vec<String> = {
        let db = state.embeddings_read.lock().unwrap().clone();
        match db {
            Some(db) => db
                .find_topic_ids_by_entity_id(&old_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => Vec::new(),
        }
    };
    for topic_id in topic_ids {
        let _ = writer
            .replace_topic_entity(
                WriteTier::Interactive,
                topic_id.clone(),
                old_id.clone(),
                xaga_core::types::Reference::Annotation {
                    id: id.clone(),
                    article_id: Some(article_id.clone()),
                },
            )
            .await;
        state.enqueue(
            Tier::Interactive,
            Job::RemoveMember {
                topic_id: topic_id.clone(),
                member_id: old_id.clone(),
            },
        );
        state.enqueue(
            Tier::Interactive,
            Job::AddMember {
                topic_id,
                member_id: id.clone(),
            },
        );
    }

    let replacement = article
        .annotations
        .iter()
        .find(|annotation| annotation.id == id)
        .map(|annotation| xaga_core::types::ListEntity {
            id: annotation.id.clone(),
            title: annotation.title.clone(),
            created_at: annotation.created_at.clone(),
            url: article.meta.url.clone(),
            is_unread: false,
            is_pinned: false,
            reference: Some(annotation.reference.clone()),
            color: Some(annotation.color.clone()),
        })
        .ok_or_else(|| AppError::Message("replacement annotation missing".into()))?;
    crate::events::emit_domain(
        &window,
        xaga_core::events::DomainEvent::AnnotationReplaced(
            xaga_core::events::AnnotationReplacement {
                old_id: old_id.clone(),
                entity: replacement,
            },
        ),
    );

    Ok(article)
}

#[tauri::command]
pub(crate) async fn remove_annotation(
    window: tauri::Window,
    article_id: String,
    annotation_id: String,
    state: State<'_, AppState>,
) -> Result<Article, AppError> {
    let writer = crate::commands::require_writer(&state)?;
    let article = writer
        .remove_annotation(
            WriteTier::Interactive,
            article_id.clone(),
            annotation_id.clone(),
        )
        .await
        .map_err(AppError::from)?;

    // Evict the deleted annotation from any topic entities lists immediately, so topic entity
    // lists don't serve stale data before LanceDB catches up. Read the annotation's topics
    // from the LanceDB reverse index (the `topics` column, maintained by the topic-membership
    // reconcile) instead of scanning every topic file — mirrors `list_entry_topics`. Best-effort:
    // a missing read handle or read error just skips the eviction (the reconcile still corrects it).
    let topic_ids: Vec<String> = {
        let db = state.embeddings_read.lock().unwrap().clone();
        match db {
            Some(db) => db
                .find_topic_ids_by_entity_id(&annotation_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default(),
            None => Vec::new(),
        }
    };
    for topic_id in topic_ids {
        let _ = writer
            .remove_topic_entity(
                WriteTier::Interactive,
                topic_id.clone(),
                annotation_id.clone(),
            )
            .await;
        state.enqueue(
            Tier::Interactive,
            Job::RemoveMember {
                topic_id,
                member_id: annotation_id.clone(),
            },
        );
    }

    state.enqueue_annotations(article.clone()).await;

    crate::events::emit_domain(
        &window,
        xaga_core::events::DomainEvent::AnnotationRemoved {
            id: annotation_id.clone(),
        },
    );

    Ok(article)
}

#[tauri::command]
pub(crate) async fn list_annotations(
    state: State<'_, AppState>,
) -> Result<Vec<PartialAnnotation>, AppError> {
    let data_dir = state.library_path();
    let db = EmbeddingsDb::open_readonly(&data_dir)
        .await
        .map_err(AppError::from)?;
    db.list_annotations().await.map_err(AppError::from)
}
