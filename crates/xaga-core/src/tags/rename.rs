//! Bounded, restartable readable-name conversion. Registry checkpoints never own article data.
use super::{Mutation, Phase, Progress, Tags, TagsError, Transition};
use crate::library_writer::{LibraryWriter, WriterError};

const BATCH_SIZE: usize = 1024;
const FILE_CONCURRENCY: usize = 8;

pub(crate) async fn resume(writer: &LibraryWriter) -> Result<(), WriterError> {
    let store = Tags::read(writer.library_dir()).await?;
    let mut tasks = writer.rename_tasks.lock().unwrap();
    tasks.retain(|_, task| !task.is_finished());
    if writer.is_shutting_down() {
        return Err(WriterError::ShuttingDown);
    }
    for id in store.snapshot().renaming {
        let generation = writer
            .rename_generations
            .lock()
            .unwrap()
            .get(&id)
            .copied()
            .unwrap_or_default();
        let task_key = format!("{id}:{generation}");
        if tasks.contains_key(&task_key) {
            continue;
        }
        let worker = writer.clone();
        let task_id = id.clone();
        let task = tokio::spawn(async move {
            let Ok(_permit) = worker.rename_permits.clone().acquire_owned().await else {
                return;
            };
            if let Err(error) = run(&worker, &task_id).await {
                if !worker.is_shutting_down() {
                    let _ = worker.tag_updates.send(Err(error));
                }
            }
        });
        tasks.insert(task_key, task);
    }
    Ok(())
}

fn state(store: &Tags, id: &str) -> Option<(u64, String, String, Progress)> {
    store.transitions.iter().find_map(|transition| {
        if let Transition::Rename {
            id: target,
            old_name,
            revision,
            files,
        } = transition
        {
            if target == id {
                let name = store.tags.iter().find(|tag| tag.id == id)?.name.clone();
                return Some((*revision, old_name.clone(), name, files.clone()));
            }
        }
        None
    })
}

async fn checkpoint(
    writer: &LibraryWriter,
    id: &str,
    revision: u64,
    progress: Progress,
) -> Result<u64, WriterError> {
    let store = writer
        .mutate_tags(Mutation::Checkpoint {
            id: id.into(),
            revision,
            phase: Phase::Files,
            progress,
        })
        .await?;
    Ok(state(&store, id).ok_or(TagsError::Stale)?.0)
}

#[allow(clippy::too_many_arguments)]
async fn batch(
    writer: &LibraryWriter,
    id: &str,
    revision: u64,
    old: &str,
    new: &str,
    pending: Vec<String>,
    before: Option<String>,
    after: Option<String>,
) -> Result<u64, WriterError> {
    use futures::{StreamExt, TryStreamExt};
    let revision = checkpoint(
        writer,
        id,
        revision,
        Progress {
            pending: pending.clone(),
            after: before,
            complete: false,
        },
    )
    .await?;
    futures::stream::iter(pending)
        .map(|entry| writer.patch_tag_name(entry, old.into(), Some(new.into())))
        .buffer_unordered(FILE_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?;
    checkpoint(
        writer,
        id,
        revision,
        Progress {
            after,
            ..Default::default()
        },
    )
    .await
}

async fn run(writer: &LibraryWriter, id: &str) -> Result<(), WriterError> {
    // Missing index assignments are outside the rename; recovery may register their old labels anew.
    let db = if tokio::fs::try_exists(crate::library::index_path(writer.library_dir()))
        .await
        .map_err(TagsError::from)?
    {
        Some(
            crate::embeddings::EmbeddingsDb::open_readonly(writer.library_dir())
                .await
                .map_err(|error| TagsError::Invalid(error.to_string()))?,
        )
    } else {
        None
    };
    loop {
        if writer.is_shutting_down() {
            return Err(WriterError::ShuttingDown);
        }
        let store = Tags::read(writer.library_dir()).await?;
        let Some((mut revision, old, new, progress)) = state(&store, id) else {
            return Ok(());
        };
        let pass = async {
            let mut after = progress.after;
            if !progress.pending.is_empty() {
                revision = batch(
                    writer,
                    id,
                    revision,
                    &old,
                    &new,
                    progress.pending,
                    after.clone(),
                    after.clone(),
                )
                .await?;
            }
            if let Some(db) = &db {
                db.reload()
                    .await
                    .map_err(|error| TagsError::Invalid(error.to_string()))?;
                loop {
                    if writer.is_shutting_down() {
                        return Err(WriterError::ShuttingDown);
                    }
                    let ids = db
                        .tag_entry_id_batch(id, after.as_deref(), BATCH_SIZE)
                        .await
                        .map_err(|error| TagsError::Invalid(error.to_string()))?;
                    if ids.is_empty() {
                        break;
                    }
                    let next = ids.last().cloned();
                    revision =
                        batch(writer, id, revision, &old, &new, ids, after, next.clone()).await?;
                    after = next;
                }
            }
            revision = checkpoint(
                writer,
                id,
                revision,
                Progress {
                    after,
                    pending: vec![],
                    complete: true,
                },
            )
            .await?;
            writer
                .mutate_tags(Mutation::Finish {
                    id: id.into(),
                    revision,
                })
                .await?;
            Ok(())
        }
        .await;
        match pass {
            Err(WriterError::Tags(TagsError::Stale)) => continue,
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library_writer::Tier;
    use crate::persistence::entries;
    use std::{path::Path, time::Instant};

    fn entry(dir: &Path, id: &str, names: &str) {
        let path = crate::library::entry_path(dir, id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("---\nsource: https://example.org/{id}\ntitle: Saved\ncustom: retained\ncolor: blue\ntags: [{names}]\n---\n\nExact **body**\n")).unwrap();
    }
    fn writer(dir: &Path) -> LibraryWriter {
        LibraryWriter::start(dir.into(), crate::library::acquire_lock(dir).unwrap())
    }
    async fn finish(
        writer: &LibraryWriter,
        updates: &flume::Receiver<Result<super::super::TagSnapshot, WriterError>>,
    ) {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                let update = updates.recv_async().await.unwrap().unwrap();
                let current = Tags::read(writer.library_dir()).await.unwrap().snapshot();
                if update.renaming.is_empty() && current.renaming.is_empty() {
                    break;
                }
            }
        })
        .await
        .unwrap();
    }

    struct Sink;
    impl crate::recovery::RepairSink for Sink {
        fn topic(&mut self, _: crate::types::Topic) {}
        fn article(&mut self, _: crate::types::Article, _: Vec<String>) {}
    }
    async fn recover(writer: &LibraryWriter, db: &crate::embeddings::EmbeddingsDb) {
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
    async fn tag_rename_keeps_index_ids_and_concurrent_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "article", "Research, Other");
        let writer = writer(dir.path());
        let registry = writer
            .discover_tags(vec!["Research".into(), "Other".into(), "Added".into()])
            .await
            .unwrap();
        let id = registry.tags[0].id.clone();
        let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let article = crate::persistence::articles::read_article(dir.path(), "article")
            .await
            .unwrap();
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        crate::embeddings::sync_entry_row(&article, &db, &events, &events, &annotations)
            .await
            .unwrap();
        let before = db.get_entry("article").await.unwrap().unwrap();
        let updates = writer.subscribe_to_tag_updates();
        let (rename, other_rename, added, unread) = tokio::join!(
            writer.rename_tag(id.clone(), "New name".into()),
            writer.rename_tag(registry.tags[1].id.clone(), "Other name".into()),
            writer.set_entry_tag("article".into(), registry.tags[2].id.clone(), true),
            writer.set_entry_unread(Tier::Interactive, "article".into(), true),
        );
        assert_eq!(rename.unwrap().tags[0].id, id);
        other_rename.unwrap();
        added.unwrap();
        unread.unwrap();
        finish(&writer, &updates).await;
        let names = entries::read_entry_tags(dir.path(), "article")
            .await
            .unwrap();
        assert!(names.contains(&"New name".into()));
        assert!(names.contains(&"Other name".into()));
        assert!(names.contains(&"Added".into()));
        let raw =
            std::fs::read_to_string(crate::library::entry_path(dir.path(), "article")).unwrap();
        assert!(raw.contains("custom: retained"));
        assert!(raw.ends_with("Exact **body**\n"));
        assert!(
            entries::read_entry(dir.path(), "article")
                .await
                .unwrap()
                .is_unread
        );
        recover(&writer, &db).await;
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        let (current_store, current_ids) =
            writer.resolve_entry_tags("article".into()).await.unwrap();
        assert_eq!(current_store.tags.len(), 3);
        assert!(current_ids.contains(&id));
        // A genuinely reintroduced old spelling gets a new ID, but a queued
        // pre-rename snapshot still carries its original assignments.
        let reintroduced = writer.create_tag("Research".into()).await.unwrap();
        assert!(reintroduced.created);
        assert_ne!(reintroduced.tag.id, id);
        crate::embeddings::pipeline::sync_entry_row_with_tag_changes(
            &article,
            &db,
            &events,
            &events,
            &annotations,
            None,
            Some(&before.tags),
        )
        .await
        .unwrap();
        let after = db.get_entry("article").await.unwrap().unwrap();
        assert_eq!(before.tags, after.tags);
        assert_eq!(before.checksum, after.checksum);
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_rename_retries_persisted_pending_case_changes_and_refuses_collisions() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "a", "Research, research");
        let writer = writer(dir.path());
        let registry = writer
            .discover_tags(vec!["Research".into(), "Other".into()])
            .await
            .unwrap();
        let id = registry.tags[0].id.clone();
        assert!(matches!(
            writer.rename_tag(id.clone(), " OTHER ".into()).await,
            Err(WriterError::Tags(TagsError::Collision))
        ));
        assert!(matches!(
            writer.rename_tag(id.clone(), "  ".into()).await,
            Err(WriterError::Tags(TagsError::InvalidName))
        ));
        writer
            .mutate_tags(Mutation::Rename {
                id: id.clone(),
                name: "RESEARCH".into(),
            })
            .await
            .unwrap();
        assert!(matches!(
            writer.rename_tag(id.clone(), "Again".into()).await,
            Err(WriterError::Tags(TagsError::Busy))
        ));
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 0,
                phase: Phase::Files,
                progress: Progress {
                    pending: vec!["a".into(), "deleted".into()],
                    ..Default::default()
                },
            })
            .await
            .unwrap();
        writer.shutdown().await;
        drop(writer);
        let reopened = self::writer(dir.path());
        let updates = reopened.subscribe_to_tag_updates();
        reopened.resume_tag_renames().await.unwrap();
        reopened.resume_tag_renames().await.unwrap();
        finish(&reopened, &updates).await;
        let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &reopened.lock())
            .await
            .unwrap();
        recover(&reopened, &db).await;
        assert_eq!(
            entries::read_entry_tags(dir.path(), "a").await.unwrap(),
            vec!["RESEARCH", "RESEARCH"]
        );
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        let new = reopened.create_tag("Research".into()).await.unwrap();
        assert!(!new.created);
        assert_eq!(new.tag.id, id);
        reopened.shutdown().await;
    }

    #[tokio::test]
    async fn tag_rename_failure_preserves_pending_indexed_batch_until_retry() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "good", "Old");
        let writer = writer(dir.path());
        let tag = writer.create_tag("Old".into()).await.unwrap().tag;
        let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let mut article = crate::persistence::articles::read_article(dir.path(), "good")
            .await
            .unwrap();
        article.meta.id = "broken".into();
        db.add_batch(vec![crate::embeddings::EmbeddingEntry::from_article(
            &article,
            None,
            vec![tag.id.clone()],
        )])
        .await
        .unwrap();
        let path = crate::library::entry_path(dir.path(), "broken");
        std::fs::write(&path, "---\ntags: [\n---\nBody").unwrap();
        writer
            .mutate_tags(Mutation::Rename {
                id: tag.id.clone(),
                name: "New".into(),
            })
            .await
            .unwrap();
        assert!(run(&writer, &tag.id).await.is_err());
        let store = Tags::read(dir.path()).await.unwrap();
        assert_eq!(state(&store, &tag.id).unwrap().3.pending, vec!["broken"]);
        std::fs::write(&path, "---\ntags: [Old]\n---\nBody").unwrap();
        run(&writer, &tag.id).await.unwrap();
        assert_eq!(
            entries::read_entry_tags(dir.path(), "broken")
                .await
                .unwrap(),
            vec!["New"]
        );
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        recover(&writer, &db).await;
        let recreated = Tags::read(dir.path())
            .await
            .unwrap()
            .tags
            .into_iter()
            .find(|tag| tag.name == "Old")
            .unwrap();
        assert_ne!(recreated.id, tag.id);
        assert_eq!(
            entries::read_entry_tags(dir.path(), "good").await.unwrap(),
            vec!["Old"]
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_rename_uses_indexed_candidates_and_recovery_recreates_old_tag() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..4 {
            entry(dir.path(), &format!("known-{i}"), "Old");
        }
        entry(dir.path(), "unindexed", "Old");
        entry(dir.path(), "false-positive", "Other");
        let broken = crate::library::entry_path(dir.path(), "broken");
        std::fs::write(&broken, "---\ntags: [\n---\nBody").unwrap();
        let writer = writer(dir.path());
        let registry = writer
            .discover_tags(vec!["Old".into(), "Other".into()])
            .await
            .unwrap();
        let id = registry.tags[0].id.clone();
        let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let mut article = crate::persistence::articles::read_article(dir.path(), "known-0")
            .await
            .unwrap();
        let rows = [
            "known-0",
            "known-1",
            "known-2",
            "known-3",
            "false-positive",
            "missing",
        ]
        .into_iter()
        .map(|entry| {
            article.meta.id = entry.into();
            crate::embeddings::EmbeddingEntry::from_article(&article, None, vec![id.clone()])
        })
        .collect();
        db.add_batch(rows).await.unwrap();
        db.ensure_indexes().await.unwrap();
        writer
            .mutate_tags(Mutation::Rename {
                id: id.clone(),
                name: "New".into(),
            })
            .await
            .unwrap();
        run(&writer, &id).await.unwrap();
        for i in 0..4 {
            assert_eq!(
                entries::read_entry_tags(dir.path(), &format!("known-{i}"))
                    .await
                    .unwrap(),
                vec!["New"]
            );
        }
        assert_eq!(
            entries::read_entry_tags(dir.path(), "unindexed")
                .await
                .unwrap(),
            vec!["Old"]
        );
        assert_eq!(
            entries::read_entry_tags(dir.path(), "false-positive")
                .await
                .unwrap(),
            vec!["Other"]
        );
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        assert_eq!(
            db.tag_entry_id_batch(&id, None, 2).await.unwrap(),
            vec!["false-positive", "known-0"]
        );
        assert_eq!(
            db.tag_entry_id_batch(&id, Some("known-0"), 2)
                .await
                .unwrap(),
            vec!["known-1", "known-2"]
        );
        recover(&writer, &db).await;
        assert_eq!(
            entries::read_entry_tags(dir.path(), "unindexed")
                .await
                .unwrap(),
            vec!["Old"]
        );
        let store = Tags::read(dir.path()).await.unwrap();
        let recreated = store.tags.iter().find(|tag| tag.name == "Old").unwrap();
        assert_ne!(recreated.id, id);
        assert_eq!(
            store.tags.iter().find(|tag| tag.name == "New").unwrap().id,
            id
        );
        assert!(store.transitions.is_empty());
        std::fs::remove_file(broken).unwrap();
        // Rename never changes assignment IDs, including stale index rows.
        assert_eq!(
            db.tag_entry_id_batch(&id, None, 1024).await.unwrap().len(),
            6
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_rename_can_repeat_without_recovery_or_restart() {
        let dir = tempfile::tempdir().unwrap();
        entry(dir.path(), "article", "First");
        let writer = writer(dir.path());
        let tag = writer.create_tag("First".into()).await.unwrap().tag;
        let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let article = crate::persistence::articles::read_article(dir.path(), "article")
            .await
            .unwrap();
        db.add_batch(vec![crate::embeddings::EmbeddingEntry::from_article(
            &article,
            None,
            vec![tag.id.clone()],
        )])
        .await
        .unwrap();
        let updates = writer.subscribe_to_tag_updates();
        for i in 0..10 {
            writer
                .rename_tag(tag.id.clone(), format!("Name {i}"))
                .await
                .unwrap();
            finish(&writer, &updates).await;
            let store = Tags::read(dir.path()).await.unwrap();
            assert_eq!(store.tags[0].id, tag.id);
            assert_eq!(store.tags[0].name, format!("Name {i}"));
            assert!(store.transitions.is_empty());
            assert_eq!(
                entries::read_entry_tags(dir.path(), "article")
                    .await
                    .unwrap(),
                vec![format!("Name {i}")]
            );
        }
        writer.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "explicit sparse index-first rename scaling measurement"]
    async fn tag_rename_scaling_curve() {
        for total in [2000, 4000, 8000] {
            let dir = tempfile::tempdir().unwrap();
            for i in 0..total {
                entry(
                    dir.path(),
                    &format!("entry-{i:06}"),
                    if i < 4 { "Old, Other" } else { "Other" },
                );
            }
            let writer = writer(dir.path());
            let registry = writer
                .discover_tags(
                    (0..10_000)
                        .map(|i| format!("Definition {i}"))
                        .chain(["Old".into(), "Other".into()])
                        .collect(),
                )
                .await
                .unwrap();
            let old = registry.tags.iter().find(|tag| tag.name == "Old").unwrap();
            let other = registry
                .tags
                .iter()
                .find(|tag| tag.name == "Other")
                .unwrap();
            let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &writer.lock())
                .await
                .unwrap();
            let mut article =
                crate::persistence::articles::read_article(dir.path(), "entry-000000")
                    .await
                    .unwrap();
            for start in (0..total).step_by(1024) {
                let rows = (start..total.min(start + 1024))
                    .map(|i| {
                        article.meta.id = format!("entry-{i:06}");
                        let ids = if i < 4 {
                            vec![old.id.clone(), other.id.clone()]
                        } else {
                            vec![other.id.clone()]
                        };
                        crate::embeddings::EmbeddingEntry::from_article(&article, None, ids)
                    })
                    .collect();
                db.add_batch(rows).await.unwrap();
            }
            db.ensure_indexes().await.unwrap();
            writer
                .mutate_tags(Mutation::Rename {
                    id: old.id.clone(),
                    name: "New".into(),
                })
                .await
                .unwrap();
            let started = Instant::now();
            run(&writer, &old.id).await.unwrap();
            println!(
                "rename_index_scale files={total} affected=4 elapsed_ms={}",
                started.elapsed().as_millis()
            );
            for i in 0..4 {
                assert!(
                    entries::read_entry_tags(dir.path(), &format!("entry-{i:06}"))
                        .await
                        .unwrap()
                        .contains(&"New".into())
                );
            }
            assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
            writer.shutdown().await;
        }
    }
}
