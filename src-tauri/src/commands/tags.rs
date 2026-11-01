use crate::{AppState, error::AppError};
use tauri::State;
use xaga_core::{
    library_writer::LibraryWriter,
    tags::{Tag, Tags},
};

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> Result<Vec<Tag>, AppError> {
    list_in_library(&state.library_path()).await
}

async fn list_in_library(dir: &std::path::Path) -> Result<Vec<Tag>, AppError> {
    Ok(Tags::read(dir).await?.tags)
}

#[tauri::command]
pub async fn create_tag(
    name: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Tag, AppError> {
    create_in_library(name, state.writer(), |tag| {
        crate::events::emit_domain(&app, xaga_core::events::DomainEvent::TagCreated(tag));
    })
    .await
}

async fn create_in_library(
    name: String,
    writer: Option<LibraryWriter>,
    on_created: impl FnOnce(Tag),
) -> Result<Tag, AppError> {
    let writer = writer.ok_or(AppError::ReadOnly)?;
    let result = writer.create_tag(name).await?;
    if result.created {
        on_created(result.tag.clone());
    }
    Ok(result.tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tauri::Listener;

    fn writer(dir: &std::path::Path) -> LibraryWriter {
        LibraryWriter::start(dir.into(), xaga_core::library::acquire_lock(dir).unwrap())
    }

    #[tokio::test]
    async fn tag_creation_is_committed_before_notification_and_duplicates_do_not_notify() {
        let dir = tempfile::tempdir().unwrap();
        let writer = writer(dir.path());
        let events = Arc::new(Mutex::new(Vec::new()));
        let app = tauri::test::mock_builder()
            .build(crate::app_context())
            .unwrap();
        let received = events.clone();
        app.listen("tag:created", move |event| {
            received
                .lock()
                .unwrap()
                .push(serde_json::from_str::<Tag>(event.payload()).unwrap());
        });
        let create = |name: &str| {
            let app = app.handle().clone();
            let path = dir.path().to_path_buf();
            create_in_library(name.to_owned(), Some(writer.clone()), move |tag| {
                assert!(
                    std::fs::read_to_string(xaga_core::library::tags_path(&path))
                        .unwrap()
                        .contains(&tag.id)
                );
                crate::events::emit_domain(&app, xaga_core::events::DomainEvent::TagCreated(tag));
            })
        };
        let (a, b, c) = tokio::join!(create("Research"), create("RESEARCH"), create(" research "));
        let a = a.unwrap();
        assert_eq!(a, b.unwrap());
        assert_eq!(a, c.unwrap());
        assert_eq!(events.lock().unwrap().len(), 1);
        assert_eq!(list_in_library(dir.path()).await.unwrap(), vec![a.clone()]);
        let before = std::fs::read(xaga_core::library::tags_path(dir.path())).unwrap();
        assert!(create(" \n ").await.is_err());
        assert_eq!(events.lock().unwrap().len(), 1);
        assert_eq!(
            std::fs::read(xaga_core::library::tags_path(dir.path())).unwrap(),
            before
        );
        let path = xaga_core::library::tags_path(dir.path());
        std::fs::write(&path, "tags: invalid").unwrap();
        assert!(create("New").await.is_err());
        assert_eq!(events.lock().unwrap().len(), 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "tags: invalid");
        std::fs::write(&path, before).unwrap();
        writer.shutdown().await;
        drop(writer);
        let reopened = self::writer(dir.path());
        let same = create_in_library(a.name.clone(), Some(reopened.clone()), |_| {
            panic!("duplicate event after restart")
        })
        .await
        .unwrap();
        assert_eq!(same, a);
        assert_eq!(list_in_library(dir.path()).await.unwrap(), vec![a]);
        reopened.shutdown().await;
    }

    #[tokio::test]
    async fn tag_commands_preserve_readonly_isolation_and_external_registry_edits() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        assert!(list_in_library(first.path()).await.unwrap().is_empty());
        let refused = create_in_library("Research".into(), None, |_| {
            panic!("read-only notification")
        })
        .await
        .unwrap_err();
        assert_eq!(refused.code(), "library:read-only");
        assert!(super::super::is_write_refused("create_tag", true));
        assert!(!super::super::is_write_refused("list_tags", true));
        assert!(!xaga_core::library::tags_path(first.path()).exists());
        let first_writer = writer(first.path());
        let second_writer = writer(second.path());
        let a = create_in_library("straße".into(), Some(first_writer.clone()), |_| {})
            .await
            .unwrap();
        let b = create_in_library("strasse".into(), Some(first_writer.clone()), |_| {})
            .await
            .unwrap();
        assert_ne!(a.id, b.id);
        let isolated = create_in_library("straße".into(), Some(second_writer.clone()), |_| {})
            .await
            .unwrap();
        assert_ne!(a.id, isolated.id);
        first_writer.shutdown().await;
        let path = xaga_core::library::tags_path(first.path());
        let raw = std::fs::read_to_string(&path)
            .unwrap()
            .replace("straße", "External");
        std::fs::write(&path, &raw).unwrap();
        let tags = list_in_library(first.path()).await.unwrap();
        assert_eq!(
            tags.iter().find(|tag| tag.id == a.id).unwrap().name,
            "External"
        );
        assert_eq!(
            list_in_library(second.path()).await.unwrap(),
            vec![isolated]
        );
        std::fs::write(&path, "tags: invalid").unwrap();
        assert!(list_in_library(first.path()).await.is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "tags: invalid");
        second_writer.shutdown().await;
    }
}
