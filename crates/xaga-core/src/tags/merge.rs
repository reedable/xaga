//! Merge acceptance runs on the indexing service so populations share one write order.
use super::{Mutation, TagSnapshot, TagsError};
use crate::{
    embeddings::EmbeddingsDb,
    library_writer::{LibraryWriter, WriterError},
};

pub(crate) async fn begin(
    writer: &LibraryWriter,
    db: &EmbeddingsDb,
    source: String,
    destination: String,
) -> Result<TagSnapshot, WriterError> {
    // No other cache job can run between these counts and the durable choice.
    let source_count = db.count_tag_entries(&source).await.map_err(index_error)?;
    let destination_count = db
        .count_tag_entries(&destination)
        .await
        .map_err(index_error)?;
    Ok(writer
        .mutate_tags(Mutation::Merge {
            source,
            destination,
            source_count,
            destination_count,
        })
        .await?
        .snapshot())
}

fn index_error(error: crate::embeddings::EmbeddingsError) -> WriterError {
    TagsError::Invalid(error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        embeddings::EmbeddingEntry,
        persistence::{articles, entries},
        tags::{Phase, Progress, Tags, Transition, conversion},
        types::EntityFilter,
    };
    use std::path::Path;

    fn entry(dir: &Path, id: &str, names: &str) {
        let path = crate::library::entry_path(dir, id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("---\nsource: https://example.org/{id}\ntitle: Saved\ncustom: retained\nis_unread: true\ncolor: blue\ntags: [{names}]\n---\n\nExact **body**\n")).unwrap();
    }
    fn writer(dir: &Path) -> LibraryWriter {
        LibraryWriter::start(dir.into(), crate::library::acquire_lock(dir).unwrap())
    }
    async fn fixture(
        source_wins: bool,
    ) -> (
        tempfile::TempDir,
        LibraryWriter,
        EmbeddingsDb,
        String,
        String,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let writer = writer(dir.path());
        let source = writer.create_tag("Research".into()).await.unwrap().tag.id;
        let destination = writer.create_tag("Work".into()).await.unwrap().tag.id;
        writer.create_tag("Other".into()).await.unwrap();
        let db = EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let mut rows = Vec::new();
        for (id, names) in [
            ("source", "Research, Other"),
            ("destination", "Work"),
            ("both", "Research, Work, RESEARCH"),
        ] {
            entry(dir.path(), id, names);
            let article = articles::read_article(dir.path(), id).await.unwrap();
            let (_, tags) = writer.resolve_entry_tags(id.into()).await.unwrap();
            let mut row = EmbeddingEntry::from_article(&article, Some(vec![0.25; 384]), tags);
            row.is_pinned = true;
            rows.push(row);
        }
        if source_wins {
            entry(dir.path(), "extra", "Research");
            let article = articles::read_article(dir.path(), "extra").await.unwrap();
            rows.push(EmbeddingEntry::from_article(
                &article,
                None,
                vec![source.clone()],
            ));
        }
        db.add_batch(rows).await.unwrap();
        (dir, writer, db, source, destination)
    }
    struct Sink;
    impl crate::recovery::RepairSink for Sink {
        fn topic(&mut self, _: crate::types::Topic) {}
        fn article(&mut self, _: crate::types::Article, _: Vec<String>) {}
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
    async fn tag_merge_preserves_union_and_destination_name_with_either_survivor() {
        for source_wins in [false, true] {
            let (dir, writer, db, source, destination) = fixture(source_wins).await;
            let before = db.get_entry("both").await.unwrap().unwrap();
            let snapshot = begin(&writer, &db, source.clone(), destination.clone())
                .await
                .unwrap();
            let merge = &snapshot.merging[0];
            let survivor = if source_wins { &source } else { &destination };
            assert_eq!(&merge.survivor_id, survivor);
            assert_eq!(
                snapshot
                    .tags
                    .iter()
                    .filter(|tag| tag.name == "Work")
                    .count(),
                1
            );
            assert!(snapshot.tags.iter().all(|tag| tag.name != "Research"));
            let group = vec![source.clone(), destination.clone()];
            assert_eq!(
                db.list_tag_entry_groups(&[group.clone()], EntityFilter::All)
                    .await
                    .unwrap()
                    .len(),
                if source_wins { 4 } else { 3 }
            );
            assert_eq!(
                db.count_tag_group_unread(&[group]).await.unwrap(),
                if source_wins { 4 } else { 3 }
            );
            assert!(
                writer
                    .rename_tag(source.clone(), "Changed".into())
                    .await
                    .is_err()
            );
            assert!(writer.delete_tag(destination.clone()).await.is_err());
            assert!(
                begin(&writer, &db, source.clone(), destination.clone())
                    .await
                    .is_err()
            );
            entry(dir.path(), "new", "Other");
            let change = writer
                .set_entry_tag("new".into(), merge.losing_id().into(), true)
                .await
                .unwrap();
            assert_eq!(&change.tag_id, survivor);
            assert_eq!(
                entries::read_entry_tags(dir.path(), "new").await.unwrap(),
                vec!["Other", "Work"]
            );
            conversion::resume(&writer, &db).await.unwrap();
            for id in ["source", "destination", "both"] {
                let names = entries::read_entry_tags(dir.path(), id).await.unwrap();
                assert_eq!(
                    names.iter().filter(|name| name.as_str() == "Work").count(),
                    1
                );
                assert!(
                    names
                        .iter()
                        .all(|name| !name.eq_ignore_ascii_case("Research"))
                );
                let raw =
                    std::fs::read_to_string(crate::library::entry_path(dir.path(), id)).unwrap();
                assert!(raw.contains("custom: retained"));
                assert!(raw.ends_with("Exact **body**\n"));
            }
            let after = db.get_entry("both").await.unwrap().unwrap();
            assert_eq!(after.tags, vec![survivor.clone()]);
            assert_eq!(after.vector, before.vector);
            assert_eq!(after.color, before.color);
            assert_eq!(after.is_unread, before.is_unread);
            assert_eq!(after.is_pinned, before.is_pinned);
            let store = Tags::read(dir.path()).await.unwrap();
            assert!(store.transitions.is_empty());
            assert_eq!(
                store
                    .tags
                    .iter()
                    .find(|tag| &tag.id == survivor)
                    .unwrap()
                    .name,
                "Work"
            );
            assert!(!conversion::step(&writer, &db, &source).await.unwrap());
            writer.shutdown().await;
        }
    }

    #[tokio::test]
    async fn tag_merge_pending_files_survive_cache_rebuild_and_discovery_reserves_old_labels() {
        let (dir, writer, mut db, source, destination) = fixture(true).await;
        begin(&writer, &db, source.clone(), destination.clone())
            .await
            .unwrap();
        entry(dir.path(), "external", "Research, Work");
        let (store, ids) = writer.resolve_entry_tags("external".into()).await.unwrap();
        assert_eq!(ids, vec![source.clone()]);
        let revision = store.transitions[0].revision();
        assert!(
            matches!(&store.transitions[0], Transition::Merge { files, .. } if files.pending == vec!["external"])
        );
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: source.clone(),
                revision,
                phase: Phase::Files,
                progress: Progress {
                    pending: vec![
                        "source".into(),
                        "destination".into(),
                        "both".into(),
                        "extra".into(),
                        "external".into(),
                        "missing".into(),
                    ],
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
        assert!(store.transitions.is_empty());
        assert_eq!(
            store.tags.iter().find(|tag| tag.name == "Work").unwrap().id,
            source
        );
        assert!(
            !store
                .tags
                .iter()
                .any(|tag| tag.name == "Research" || tag.id == destination)
        );
        assert_eq!(
            entries::read_entry_tags(dir.path(), "external")
                .await
                .unwrap(),
            vec!["Work"]
        );
        reopened.shutdown().await;
    }

    #[tokio::test]
    async fn tag_merge_failed_batch_retries_and_missed_label_creates_new_identity_after_retirement()
    {
        let (dir, writer, db, source, destination) = fixture(false).await;
        entry(dir.path(), "missed", "Research");
        begin(&writer, &db, source.clone(), destination.clone())
            .await
            .unwrap();
        std::fs::write(
            crate::library::entry_path(dir.path(), "source"),
            "---\ntags: [\n---\nBody",
        )
        .unwrap();
        assert!(conversion::step(&writer, &db, &source).await.is_err());
        let store = Tags::read(dir.path()).await.unwrap();
        assert!(
            matches!(&store.transitions[0], Transition::Merge { files, index, .. } if !files.pending.is_empty() && files.pending == index.pending)
        );
        assert_eq!(db.count_tag_entries(&source).await.unwrap(), 2);
        entry(dir.path(), "source", "Research, Other");
        conversion::resume(&writer, &db).await.unwrap();
        assert!(
            !Tags::read(dir.path())
                .await
                .unwrap()
                .tags
                .iter()
                .any(|tag| tag.name == "Research")
        );
        recover(&writer, &db).await;
        let store = Tags::read(dir.path()).await.unwrap();
        let recreated = store
            .tags
            .iter()
            .find(|tag| tag.name == "Research")
            .unwrap();
        assert_ne!(recreated.id, source);
        assert_ne!(recreated.id, destination);
        assert_eq!(
            entries::read_entry_tags(dir.path(), "missed")
                .await
                .unwrap(),
            vec!["Research"]
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_merge_resumes_after_index_commit_and_rejects_stale_checkpoints() {
        let (dir, writer, mut db, source, destination) = fixture(false).await;
        begin(&writer, &db, source.clone(), destination.clone())
            .await
            .unwrap();
        let pending = vec!["source".into(), "both".into()];
        for (revision, phase) in [(0, Phase::Files), (1, Phase::Index)] {
            writer
                .mutate_tags(Mutation::Checkpoint {
                    id: source.clone(),
                    revision,
                    phase,
                    progress: Progress {
                        pending: pending.clone(),
                        ..Default::default()
                    },
                })
                .await
                .unwrap();
        }
        for id in &pending {
            writer
                .patch_tag_labels(
                    id.clone(),
                    vec!["Research".into(), "Work".into()],
                    Some("Work".into()),
                )
                .await
                .unwrap();
        }
        db.patch_tag_batch(&source, Some(&destination), &pending)
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: source.clone(),
                revision: 2,
                phase: Phase::Files,
                progress: Progress::default(),
            })
            .await
            .unwrap();
        // Discovery interrupts the old worker and adds a file missing from the index.
        entry(dir.path(), "external", "Research");
        writer.resolve_entry_tags("external".into()).await.unwrap();
        assert!(matches!(
            writer
                .mutate_tags(Mutation::Finish {
                    id: source.clone(),
                    revision: 3
                })
                .await,
            Err(WriterError::Tags(TagsError::Stale))
        ));
        // The file and index pending lists now differ. Both must survive a missing cache.
        entry(dir.path(), "source", "Research");
        db.reset().await.unwrap();
        conversion::resume(&writer, &db).await.unwrap();
        assert_eq!(
            entries::read_entry_tags(dir.path(), "source")
                .await
                .unwrap(),
            vec!["Work"]
        );
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        assert_eq!(
            entries::read_entry_tags(dir.path(), "external")
                .await
                .unwrap(),
            vec!["Work"]
        );
        assert_eq!(db.count_tag_entries(&source).await.unwrap(), 0);
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_merge_waits_for_existing_transitions_and_handles_unassigned_ties() {
        let (dir, writer, db, source, destination) = fixture(false).await;
        writer
            .mutate_tags(Mutation::Rename {
                id: destination.clone(),
                name: "Weekend".into(),
            })
            .await
            .unwrap();
        assert!(matches!(
            begin(&writer, &db, source.clone(), destination.clone()).await,
            Err(WriterError::Tags(TagsError::Busy))
        ));
        writer.wait_for_indexed_renames().await.unwrap();
        begin(&writer, &db, source.clone(), destination)
            .await
            .unwrap();
        conversion::resume(&writer, &db).await.unwrap();
        let a = writer.create_tag("Empty A".into()).await.unwrap().tag;
        let b = writer.create_tag("Empty B".into()).await.unwrap().tag;
        let status = begin(&writer, &db, a.id.clone(), b.id.clone())
            .await
            .unwrap();
        assert_eq!(status.merging[0].survivor_id, b.id);
        assert!(!conversion::step(&writer, &db, &a.id).await.unwrap());
        assert!(Tags::read(dir.path()).await.unwrap().transitions.is_empty());
        writer.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "explicit two-million-row merge batch measurement"]
    async fn tag_merge_at_design_scale() {
        use std::time::Instant;
        let total: usize = std::env::var("XAGA_TAG_MERGE_SCALE_ROWS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(2_000_000);
        assert!(total >= 4096 && total % 4 == 0);
        let dir = tempfile::tempdir().unwrap();
        let writer = writer(dir.path());
        let source = writer.create_tag("Research".into()).await.unwrap().tag.id;
        let destination = writer.create_tag("Work".into()).await.unwrap().tag.id;
        let db = EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        entry(dir.path(), "template", "Research");
        let mut article = articles::read_article(dir.path(), "template")
            .await
            .unwrap();
        let build = Instant::now();
        let mut rows = Vec::new();
        for i in 0..total {
            let id = format!("entry-{i:07}");
            let (names, tags) = match i % 4 {
                0 | 1 => ("Research", vec![source.clone()]),
                2 => ("Work", vec![destination.clone()]),
                _ => ("Research, Work", vec![source.clone(), destination.clone()]),
            };
            if i < 3072 {
                entry(dir.path(), &id, names);
            }
            article.meta.id = id;
            rows.push(EmbeddingEntry::from_article(&article, None, tags));
            if rows.len() == 8192 || i + 1 == total {
                db.add_batch(std::mem::take(&mut rows)).await.unwrap();
                if (i + 1) % (8192 * 50) == 0 {
                    db.maintain().await.unwrap();
                    eprintln!(
                        "merge_scale built={} elapsed_s={:.2}",
                        i + 1,
                        build.elapsed().as_secs_f64()
                    );
                }
            }
        }
        db.maintain().await.unwrap();
        eprintln!(
            "merge_scale total={total} build_s={:.3}",
            build.elapsed().as_secs_f64()
        );
        let start = Instant::now();
        let snapshot = begin(&writer, &db, source.clone(), destination.clone())
            .await
            .unwrap();
        eprintln!(
            "merge_scale total={total} acceptance_ms={}",
            start.elapsed().as_millis()
        );
        assert_eq!(snapshot.merging[0].survivor_id, source);
        for batch in 0..3 {
            let start = Instant::now();
            assert!(conversion::step(&writer, &db, &source).await.unwrap());
            eprintln!(
                "merge_scale total={total} batch={batch} entries=1024 elapsed_ms={}",
                start.elapsed().as_millis()
            );
            let remaining = db.count_tag_entries(&destination).await.unwrap();
            assert_eq!(remaining, (total / 2 - (batch + 1) * 512) as u64);
        }
        assert_eq!(
            entries::read_entry_tags(dir.path(), "entry-0000000")
                .await
                .unwrap(),
            vec!["Work"]
        );
        assert_eq!(
            entries::read_entry_tags(dir.path(), "entry-0000003")
                .await
                .unwrap(),
            vec!["Work"]
        );
        writer.shutdown().await;
    }
}
