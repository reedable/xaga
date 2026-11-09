//! Indexed merge/deletion steps, driven by the command queue or recovery's index writer.
use super::{Mutation, Phase, Progress, Tags, TagsError, Transition};
use crate::{
    embeddings::EmbeddingsDb,
    library_writer::{LibraryWriter, WriterError},
};

const BATCH_SIZE: usize = 1024;

async fn checkpoint(
    writer: &LibraryWriter,
    id: &str,
    revision: u64,
    phase: Phase,
    progress: Progress,
) -> Result<u64, WriterError> {
    writer
        .mutate_tags(Mutation::Checkpoint {
            id: id.into(),
            revision,
            phase,
            progress,
        })
        .await?;
    Ok(revision + 1)
}

/// One bounded step. True means another step is needed. Source files precede
/// cache removal; both pending checkpoints precede either mutation.
pub(crate) async fn step(
    writer: &LibraryWriter,
    db: &EmbeddingsDb,
    id: &str,
) -> Result<bool, WriterError> {
    if writer.is_shutting_down() {
        return Err(WriterError::ShuttingDown);
    }
    let store = Tags::read(writer.library_dir()).await?;
    let Some((mut revision, names, replacement_name, candidates, losing, survivor, files, index)) =
        store.transitions.iter().find_map(|t| match t {
            Transition::Delete {
                id: target,
                revision,
                files,
                index,
            } if target == id => Some((
                *revision,
                vec![store.tag(id).ok()?.name.clone()],
                None,
                vec![id.to_owned()],
                id.to_owned(),
                None,
                files.clone(),
                index.clone(),
            )),
            Transition::Merge {
                source_id,
                destination_id,
                survivor_id,
                destination_name,
                revision,
                files,
                index,
            } if source_id == id => {
                let losing = if survivor_id == source_id {
                    destination_id
                } else {
                    source_id
                };
                Some((
                    *revision,
                    vec![
                        store.tag(source_id).ok()?.name.clone(),
                        store.tag(destination_id).ok()?.name.clone(),
                    ],
                    Some(destination_name.clone()),
                    vec![source_id.clone(), destination_id.clone()],
                    losing.clone(),
                    Some(survivor_id.clone()),
                    files.clone(),
                    index.clone(),
                ))
            }
            _ => None,
        })
    else {
        return Ok(false);
    };
    let replay = !files.pending.is_empty() || !index.pending.is_empty();
    let pending = if !index.pending.is_empty() {
        index.pending.clone()
    } else if !files.pending.is_empty() {
        files.pending.clone()
    } else {
        db.tag_entry_id_batch_any(&candidates, files.after.as_deref(), BATCH_SIZE)
            .await
            .map_err(index_error)?
    };
    if pending.is_empty() {
        revision = checkpoint(
            writer,
            id,
            revision,
            Phase::Files,
            Progress {
                complete: true,
                ..Default::default()
            },
        )
        .await?;
        revision = checkpoint(
            writer,
            id,
            revision,
            Phase::Index,
            Progress {
                complete: true,
                ..Default::default()
            },
        )
        .await?;
        writer
            .mutate_tags(Mutation::Finish {
                id: id.into(),
                revision,
            })
            .await?;
        return Ok(false);
    }
    let next = if replay {
        files.after.clone()
    } else {
        pending.last().cloned()
    };
    // Discovery can append file work after a previous batch's file checkpoint.
    // Keep that work intact while replaying the independently persisted index batch.
    let remaining_files: Vec<_> = files
        .pending
        .iter()
        .filter(|entry| !pending.contains(entry))
        .cloned()
        .collect();
    let file_progress = Progress {
        after: files.after.clone(),
        pending: if replay && !index.pending.is_empty() {
            files.pending.clone()
        } else {
            pending.clone()
        },
        complete: false,
    };
    revision = checkpoint(writer, id, revision, Phase::Files, file_progress).await?;
    revision = checkpoint(
        writer,
        id,
        revision,
        Phase::Index,
        Progress {
            after: files.after,
            pending: pending.clone(),
            complete: false,
        },
    )
    .await?;
    use futures::{StreamExt, TryStreamExt};
    futures::stream::iter(pending.iter().cloned())
        .map(|entry| writer.patch_tag_labels(entry, names.clone(), replacement_name.clone()))
        .buffer_unordered(8)
        .try_collect::<Vec<_>>()
        .await?;
    db.patch_tag_batch(&losing, survivor.as_deref(), &pending)
        .await
        .map_err(index_error)?;
    let progress = Progress {
        after: next,
        ..Default::default()
    };
    revision = checkpoint(
        writer,
        id,
        revision,
        Phase::Files,
        Progress {
            pending: remaining_files,
            ..progress.clone()
        },
    )
    .await?;
    checkpoint(writer, id, revision, Phase::Index, progress).await?;
    Ok(true)
}

fn index_error(error: crate::embeddings::EmbeddingsError) -> WriterError {
    TagsError::Invalid(error.to_string()).into()
}

pub(crate) async fn resume(writer: &LibraryWriter, db: &EmbeddingsDb) -> Result<(), WriterError> {
    let snapshot = Tags::read(writer.library_dir()).await?.snapshot();
    let ids = snapshot
        .deleting
        .into_iter()
        .chain(snapshot.merging.into_iter().map(|merge| merge.source_id));
    for id in ids {
        let mut batches = 0;
        loop {
            match step(writer, db, &id).await {
                Err(WriterError::Tags(TagsError::Stale)) => continue,
                result => {
                    if !result? {
                        break;
                    }
                }
            }
            batches += 1;
            if batches % 100 == 0 {
                db.maintain().await.map_err(index_error)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        embeddings::EmbeddingEntry,
        library_writer::Tier,
        persistence::{articles, entries},
    };
    use std::path::Path;

    fn entry(dir: &Path, id: &str, tags: &str) {
        let path = crate::library::entry_path(dir, id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("---\nsource: https://example.org/{id}\ntitle: Saved\ncustom: retained\nis_unread: true\ncolor: blue\ntags: [{tags}]\n---\n\nExact **body**\n")).unwrap();
    }
    fn writer(dir: &Path) -> LibraryWriter {
        LibraryWriter::start(dir.into(), crate::library::acquire_lock(dir).unwrap())
    }
    async fn index(writer: &LibraryWriter, entry: &str) -> EmbeddingsDb {
        let db = EmbeddingsDb::open_writer(writer.library_dir(), &writer.lock())
            .await
            .unwrap();
        let article = articles::read_article(writer.library_dir(), entry)
            .await
            .unwrap();
        let (_, ids) = writer.resolve_entry_tags(entry.into()).await.unwrap();
        let mut row = EmbeddingEntry::from_article(&article, Some(vec![0.5; 384]), ids);
        row.is_pinned = true;
        db.add_batch(vec![row]).await.unwrap();
        db
    }
    struct Sink;
    impl crate::recovery::RepairSink for Sink {
        fn topic(&mut self, _: crate::types::Topic) {}
        fn article(
            &mut self,
            _: crate::types::Article,
            _: Vec<String>,
            _: crate::tags::AnnotationTagIds,
        ) {
        }
    }
    async fn recover(writer: &LibraryWriter, db: &EmbeddingsDb) {
        crate::recovery::embeddings::repair_entity_embeddings(
            writer.library_dir(),
            db,
            &mut Sink,
            Some(writer),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn tag_delete_preserves_other_data_and_refuses_conflicting_operations() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "a", "Research, Other");
        let writer = writer(dir.path());
        let db = index(&writer, "a").await;
        let store = Tags::read(dir.path()).await.unwrap();
        let id = store.tags[0].id.clone();
        let other = store.tags[1].id.clone();
        let before = db.get_entry("a").await.unwrap().unwrap();
        let updates = writer.subscribe_to_tag_updates();
        let snapshot = writer.delete_tag(id.clone()).await.unwrap();
        assert_eq!(snapshot.deleting, vec![id.clone()]);
        assert_eq!(
            updates.recv_async().await.unwrap().unwrap().deleting,
            vec![id.clone()]
        );
        assert!(matches!(
            writer.rename_tag(id.clone(), "New".into()).await,
            Err(WriterError::Tags(TagsError::Busy))
        ));
        assert!(writer.delete_tag(id.clone()).await.is_err());
        assert!(writer.create_tag("RESEARCH".into()).await.is_err());
        assert!(
            writer
                .set_entry_tag("a".into(), id.clone(), true)
                .await
                .is_err()
        );
        let (result, read) = tokio::join!(
            step(&writer, &db, &id),
            writer.set_entry_unread(Tier::Interactive, "a".into(), false)
        );
        assert!(result.unwrap());
        read.unwrap();
        assert_eq!(
            entries::read_entry_tags(dir.path(), "a").await.unwrap(),
            vec!["Other"]
        );
        let after = db.get_entry("a").await.unwrap().unwrap();
        assert_eq!(after.tags, vec![other]);
        assert_eq!(after.vector, before.vector);
        assert_eq!(after.color, before.color);
        assert_eq!(after.is_pinned, before.is_pinned);
        assert_eq!(after.is_unread, before.is_unread); // deletion owns only the tags cache column
        let raw = std::fs::read_to_string(crate::library::entry_path(dir.path(), "a")).unwrap();
        assert!(raw.contains("custom: retained"));
        assert!(raw.ends_with("Exact **body**\n"));
        assert!(
            !entries::read_entry(dir.path(), "a")
                .await
                .unwrap()
                .is_unread
        );
        assert!(!step(&writer, &db, &id).await.unwrap());
        assert!(!step(&writer, &db, &id).await.unwrap());
        assert!(
            updates
                .recv_async()
                .await
                .unwrap()
                .unwrap()
                .deleting
                .is_empty()
        );
        let mut stale = before.tags;
        db.retain_current_tag_ids(&mut stale).await.unwrap();
        assert!(!stale.contains(&id));
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_delete_recovery_resumes_pending_files_even_after_cache_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "a", "Research, Other");
        entry(dir.path(), "external", "Research");
        let writer = writer(dir.path());
        let mut db = index(&writer, "a").await;
        let id = Tags::read(dir.path()).await.unwrap().tags[0].id.clone();
        writer.delete_tag(id.clone()).await.unwrap();
        // Encountering an unindexed source while reserved records that file as work.
        let (store, ids) = writer.resolve_entry_tags("external".into()).await.unwrap();
        assert!(ids.is_empty());
        assert!(
            matches!(&store.transitions[0], Transition::Delete { files, .. } if files.pending == vec!["external"])
        );
        // Persist an interrupted batch before either file or cache work completes.
        let revision = store.transitions[0].revision();
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision,
                phase: Phase::Files,
                progress: Progress {
                    pending: vec!["a".into(), "external".into(), "missing".into()],
                    ..Default::default()
                },
            })
            .await
            .unwrap();
        db.reset().await.unwrap();
        writer.shutdown().await;
        drop(db);
        drop(writer);
        let reopened = self::writer(dir.path());
        let db = EmbeddingsDb::open_writer(dir.path(), &reopened.lock())
            .await
            .unwrap();
        recover(&reopened, &db).await;
        let store = Tags::read(dir.path()).await.unwrap();
        assert!(!store.tags.iter().any(|tag| tag.name == "Research"));
        assert!(store.transitions.is_empty());
        assert_eq!(
            entries::read_entry_tags(dir.path(), "a").await.unwrap(),
            vec!["Other"]
        );
        assert!(
            entries::read_entry_tags(dir.path(), "external")
                .await
                .unwrap()
                .is_empty()
        );
        reopened.shutdown().await;
    }

    #[tokio::test]
    async fn tag_delete_missed_labels_create_new_identity_only_after_retirement() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "known", "Research");
        entry(dir.path(), "missed", "Research");
        let writer = writer(dir.path());
        let db = index(&writer, "known").await;
        let id = Tags::read(dir.path()).await.unwrap().tags[0].id.clone();
        writer.delete_tag(id.clone()).await.unwrap();
        resume(&writer, &db).await.unwrap();
        assert!(Tags::read(dir.path()).await.unwrap().tags.is_empty());
        assert_eq!(
            entries::read_entry_tags(dir.path(), "missed")
                .await
                .unwrap(),
            vec!["Research"]
        );
        recover(&writer, &db).await;
        let tags = Tags::read(dir.path()).await.unwrap();
        assert_eq!(tags.tags.len(), 1);
        assert_eq!(tags.tags[0].name, "Research");
        assert_ne!(tags.tags[0].id, id);
        // Old queued IDs cannot become the new definition or resurrect the old one.
        let mut stale = vec![id.clone(), tags.tags[0].id.clone()];
        db.retain_current_tag_ids(&mut stale).await.unwrap();
        assert_eq!(stale, vec![tags.tags[0].id.clone()]);
        let empty = writer.create_tag("Unassigned".into()).await.unwrap().tag;
        writer.delete_tag(empty.id.clone()).await.unwrap();
        assert!(!step(&writer, &db, &empty.id).await.unwrap());
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_delete_failed_file_batch_remains_durable_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "a", "Research");
        let writer = writer(dir.path());
        let db = index(&writer, "a").await;
        let id = Tags::read(dir.path()).await.unwrap().tags[0].id.clone();
        writer.delete_tag(id.clone()).await.unwrap();
        std::fs::write(
            crate::library::entry_path(dir.path(), "a"),
            "---\ntags: [\n---\nBody",
        )
        .unwrap();
        assert!(step(&writer, &db, &id).await.is_err());
        let store = Tags::read(dir.path()).await.unwrap();
        assert!(
            matches!(&store.transitions[0], Transition::Delete { files, index, .. } if files.pending == vec!["a"] && index.pending == vec!["a"])
        );
        assert_eq!(
            db.get_entry("a").await.unwrap().unwrap().tags,
            vec![id.clone()]
        );
        entry(dir.path(), "a", "Research, Added");
        resume(&writer, &db).await.unwrap();
        assert_eq!(
            entries::read_entry_tags(dir.path(), "a").await.unwrap(),
            vec!["Added"]
        );
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_delete_limits_index_batches_and_skips_missing_or_unrelated_sources() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "base", "Research");
        entry(dir.path(), "unrelated", "Other");
        let writer = writer(dir.path());
        let db = index(&writer, "base").await;
        let before = db.get_entry("base").await.unwrap().unwrap();
        let id = before.tags[0].clone();
        let article = articles::read_article(dir.path(), "base").await.unwrap();
        let mut rows = Vec::new();
        for i in 0..1030 {
            let mut row = EmbeddingEntry::from_article(&article, None, vec![id.clone()]);
            row.id = format!("missing-{i:04}");
            rows.push(row);
        }
        let mut row = EmbeddingEntry::from_article(&article, None, vec![id.clone()]);
        row.id = "unrelated".into();
        rows.push(row);
        db.add_batch(rows).await.unwrap();
        writer.delete_tag(id.clone()).await.unwrap();
        assert!(step(&writer, &db, &id).await.unwrap());
        assert_eq!(db.count_tag_unread(&[id.clone()]).await.unwrap(), 8);
        resume(&writer, &db).await.unwrap();
        assert_eq!(db.count_tag_unread(&[id]).await.unwrap(), 0);
        assert_eq!(
            entries::read_entry_tags(dir.path(), "unrelated")
                .await
                .unwrap(),
            vec!["Other"]
        );
        assert!(db.get_entry("unrelated").await.unwrap().is_some());
        writer.shutdown().await;
    }
}
