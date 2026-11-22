use tauri::{AppHandle, State};
use xaga_core::embeddings::{Job, Tier};
use xaga_core::library_writer::Tier as WriteTier;
use xaga_core::types::Entry;

use crate::AppState;
use crate::error::AppError;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BookmarkResult {
    entry: Entry,
    index_updated: bool,
}

#[tauri::command]
pub(crate) async fn add_bookmark(
    id: String,
    color: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BookmarkResult, AppError> {
    set_bookmark(id, Some(color), &state, &app).await
}

#[tauri::command]
pub(crate) async fn remove_bookmark(
    id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BookmarkResult, AppError> {
    set_bookmark(id, None, &state, &app).await
}

async fn set_bookmark(
    id: String,
    color: Option<String>,
    state: &AppState,
    app: &AppHandle,
) -> Result<BookmarkResult, AppError> {
    let writer = crate::commands::require_writer(state)?;
    let entry = writer
        .set_entry_color(WriteTier::Interactive, id.clone(), color)
        .await?;
    let (response, receiver) = tokio::sync::oneshot::channel();
    state.enqueue(
        Tier::Interactive,
        Job::SyncBookmark {
            id,
            color: entry.color.clone(),
            response,
        },
    );
    let publication_error = match receiver.await {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(error) => Some(error.to_string()),
    };
    let index_updated = publication_error.is_none();
    if let Some(error) = publication_error {
        log::warn!("Saved index update failed for Entry {}: {error}", entry.id);
        crate::events::emit_domain(
            app,
            xaga_core::events::DomainEvent::BookmarkChanged(entry.clone()),
        );
        crate::events::emit_error(
            app,
            "main",
            crate::error::SerializedError {
                diagnostic_id: None,
                code: "app:error",
                message: "The bookmark change was saved. Saved couldn’t be updated. Try the change again."
                    .into(),
                detail: String::new(),
            },
        );
    }
    Ok(BookmarkResult {
        entry,
        index_updated,
    })
}
