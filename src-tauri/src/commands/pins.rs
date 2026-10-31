use crate::{AppState, error::AppError};
use tauri::State;
use xaga_core::pins::{PinTarget, Pins};

#[tauri::command]
pub async fn list_pins(state: State<'_, AppState>) -> Result<Pins, AppError> {
    Ok(Pins::read(&state.library_path()).await?)
}

#[tauri::command]
pub async fn set_pin(
    target: PinTarget,
    pinned: bool,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    super::require_writer(&state)?
        .set_pin(target.clone(), pinned)
        .await?;
    let (id, kind) = match &target {
        PinTarget::Entry { id } => (id.clone(), xaga_core::types::Kind::Entry),
        PinTarget::Feed { id } => (id.clone(), xaga_core::types::Kind::Feed),
        PinTarget::Topic { id } => (id.clone(), xaga_core::types::Kind::Topic),
    };
    state.enqueue(
        xaga_core::embeddings::Tier::Interactive,
        xaga_core::embeddings::Job::SyncState {
            entity: xaga_core::embeddings::EntityRef::new(id, kind),
            state: xaga_core::embeddings::StatePatch {
                is_pinned: Some(pinned),
                ..Default::default()
            },
        },
    );
    crate::events::emit_domain(
        &app,
        xaga_core::events::DomainEvent::PinChanged { target, pinned },
    );
    Ok(())
}
