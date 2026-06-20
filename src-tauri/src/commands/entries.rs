use tauri::State;
use xaga_core::embeddings::{EmbeddingsDb, EntityRef, Job, StatePatch, Tier};
use xaga_core::entries;
use xaga_core::types::{Entry, ItemFilter, Kind, SavedItem};

use crate::AppState;
use super::parse_item_filter;

#[tauri::command]
pub async fn read_entry(id: String, state: State<'_, AppState>) -> Result<entries::Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    entries::read_entry(&data_dir, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_entry_read(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let tags: Vec<String> = entries::read_entry_tags(&data_dir, &id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|t| t != "unread")
        .collect();
    let entry = entries::set_entry_tags(&data_dir, &id, tags).await.map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch { is_unread: Some(false), ..Default::default() },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub async fn mark_entry_unread(id: String, state: State<'_, AppState>) -> Result<Entry, String> {
    let data_dir = state.library.read().unwrap().clone();
    let mut tags = entries::read_entry_tags(&data_dir, &id).await.map_err(|e| e.to_string())?;
    if !tags.contains(&"unread".to_string()) {
        tags.push("unread".to_string());
    }
    let entry = entries::set_entry_tags(&data_dir, &id, tags).await.map_err(|e| e.to_string())?;
    state.enqueue(
        Tier::Interactive,
        Job::SyncState {
            entity: EntityRef::new(id, Kind::Entry),
            state: StatePatch { is_unread: Some(true), ..Default::default() },
        },
    );
    Ok(entry)
}

#[tauri::command]
pub fn is_cache_ready(state: State<'_, AppState>) -> bool {
    state.embeddings_read.lock().unwrap().is_some()
}

#[tauri::command]
pub async fn list_all_items(
    item_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SavedItem>, String> {
    let data_dir = state.library.read().unwrap().clone();
    let item_filter = parse_item_filter(item_filter)?;

    let (entries_result, annotations_result) = tokio::join!(
        entries::list_entries(&data_dir),
        async {
            let db = EmbeddingsDb::open(&data_dir).await?;
            db.list_annotations().await
        },
    );

    let all_entries = entries_result.map_err(|e| e.to_string())?;
    let annotations = annotations_result.unwrap_or_default();

    let mut items: Vec<SavedItem> = all_entries
        .into_iter()
        .filter(|e| match item_filter {
            ItemFilter::All => true,
            ItemFilter::Unread => e.is_unread,
            ItemFilter::Bookmarked | ItemFilter::Saved => e.is_bookmarked,
        })
        .map(|e| SavedItem {
            id: e.id,
            title: e.title,
            created_at: e.created_at,
            url: e.url,
            is_unread: e.is_unread,
            is_bookmarked: e.is_bookmarked,
            reference: None,
            color: None,
        })
        .chain(
            annotations
                .into_iter()
                .filter(|_| matches!(item_filter, ItemFilter::All | ItemFilter::Saved))
                .map(|a| SavedItem {
                    id: a.id,
                    title: a.title,
                    created_at: a.created_at,
                    url: a.url,
                    is_unread: false,
                    is_bookmarked: false,
                    reference: Some(a.reference),
                    color: Some(a.color),
                }),
        )
        .collect();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}
