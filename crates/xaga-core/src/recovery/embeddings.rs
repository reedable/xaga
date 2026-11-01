//! The diff stage: walk the filesystem, compare against the index, and drive the repair. Each item
//! that needs embedding is streamed to the caller's `RepairSink`; the post-walk residue (feed embeds
//! + deletes) is returned. Pure computation — the reads are delegated to persistence
//! (`for_each_md`/`parse_document`) and embeddings (`list_entries_meta`); the *apply* is the sink's.

use std::collections::HashSet;
use std::path::Path;

use crate::embeddings::pipeline::annotation_checksum;
use crate::embeddings::row::{article_embed_text, checksum, feed_embed_text, topic_embed_text};
use crate::embeddings::{EmbeddingsDb, EmbeddingsError};
use crate::fs::WalkOutcome;
use crate::library;
use crate::md::{md_to_article, md_to_topic};
use crate::persistence::walk::for_each_md;
use crate::types::{Article, Feed, Kind};

use super::{RepairResidue, RepairSink};

/// Walks `{library_dir}/topics/` and `{library_dir}/entries/`, comparing each against LanceDB and
/// streaming the items that need embedding to `sink` (topics before articles, so they index first).
/// Returns the residue that can only be computed after the full walk: feed embeds and deletes.
pub(crate) async fn repair_item_embeddings<S: RepairSink>(
    library_dir: &Path,
    db: &EmbeddingsDb,
    sink: &mut S,
    writer: Option<&crate::library_writer::LibraryWriter>,
) -> Result<RepairResidue, EmbeddingsError> {
    let pins = crate::pins::Pins::read(library_dir)
        .await
        .map_err(std::io::Error::other)?;
    let pin_targets: HashSet<_> = pins.pins.into_iter().collect();
    let pinned_ids = db.pinned_ids(Kind::Entry).await?;
    let pinned_topics = db.pinned_ids(Kind::Topic).await?;
    let pinned_feeds = db.pinned_ids(Kind::Feed).await?;
    let db_meta = db.list_entries_meta().await?; // id → (checksum, reference, has_vector, kind, items, is_unread)

    // Embedding is needed when the row is absent, its content changed, or its vector is missing.
    let needs_embedding = |id: &str, cksum: &str| -> bool {
        match db_meta.get(id) {
            None => true,
            Some((stored, _, has_vector, _, _, _, _, _)) => stored != cksum || !has_vector,
        }
    };

    // The annotation rows need reconciling when the entry's recorded annotation membership
    // differs from the `.md`'s annotation set, OR when any annotation row's content diverged:
    // row absent, stored checksum ≠ the `.md`'s (a crash-stranded note edit), or vector still
    // pending (crash between row sync and embed). All diffed against `db_meta` (already loaded)
    // — no per-entry query. Recovery reconciles the checksum of every row it knows about;
    // there is no reason not to (the universal-backstop contract, crash_consistency.md).
    let annotations_differ = |article: &Article| -> bool {
        let membership_differs =
            db_meta
                .get(&article.meta.id)
                .is_some_and(|(_, _, _, _, recorded, _, _, _)| {
                    let recorded: HashSet<&str> = recorded.iter().map(|s| s.as_str()).collect();
                    let current: HashSet<&str> =
                        article.annotations.iter().map(|a| a.id.as_str()).collect();
                    recorded != current
                });
        membership_differs
            || article
                .annotations
                .iter()
                .any(|a| match db_meta.get(&a.id) {
                    None => true,
                    Some((stored, _, has_vector, ..)) => {
                        stored != &annotation_checksum(a) || !has_vector
                    }
                })
    };

    // Reconcile as we walk: record every on-disk id (for `to_delete` below) and stream only the
    // items that need embedding straight to the sink. Nothing is accumulated here — an object that
    // doesn't need work is dropped the moment the walk moves past it, freeing its content.
    let mut fs_ids: HashSet<String> = HashSet::new();

    // Topics first so they index before articles.
    let topics_outcome = for_each_md(
        &library::topics_dir(library_dir),
        &md_to_topic,
        |mut topic| {
            topic.is_pinned = pin_targets.contains(&crate::pins::PinTarget::Topic {
                id: topic.id.clone(),
            });
            fs_ids.insert(topic.id.clone());
            if needs_embedding(&topic.id, &checksum(&topic.title, topic_embed_text(&topic)))
                || pinned_topics.contains(&topic.id) != topic.is_pinned
            {
                sink.topic(topic);
            }
        },
    )
    .await;

    let mut tags = crate::tags::Tags::read(library_dir)
        .await
        .map_err(std::io::Error::other)?;
    let mut resolver = tags.resolver();
    let mut observed_transitions = HashSet::new();
    let mut entries_outcome = WalkOutcome::Enumerated;
    let mut entries = match tokio::fs::read_dir(library::entries_dir(library_dir)).await {
        Ok(entries) => Some(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => {
            entries_outcome = WalkOutcome::Unreadable;
            None
        }
    };
    while let Some(entries) = &mut entries {
        let path = match entries.next_entry().await {
            Ok(Some(entry)) => entry.path(),
            Ok(None) => break,
            Err(_) => {
                entries_outcome = WalkOutcome::Unreadable;
                break;
            }
        };
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let raw = match crate::fs::read_md_capped(&path).await {
            Ok(raw) => raw,
            Err(_) => {
                entries_outcome = WalkOutcome::Unreadable;
                continue;
            }
        };
        let Ok((fm, body)) = crate::md::parse_document(&raw) else {
            continue;
        };
        let mut article = md_to_article(crate::md::Markdown {
            id: crate::util::item_id(&path),
            fm,
            body,
        });
        let names = article.meta.tags.as_deref().unwrap_or_default();
        let observations = tags.observation_ids(names);
        if resolver.resolve(names).is_err()
            || observations
                .iter()
                .any(|id| !observed_transitions.contains(id))
        {
            let writer = writer.ok_or_else(|| {
                std::io::Error::other("unknown article labels require a library writer")
            })?;
            tags = writer
                .discover_tags(names.to_vec())
                .await
                .map_err(std::io::Error::other)?;
            resolver = tags.resolver();
            observed_transitions.extend(observations);
        }
        let resolved_tags = resolver.resolve(names).map_err(std::io::Error::other)?;
        article.meta.is_pinned = pin_targets.contains(&crate::pins::PinTarget::Entry {
            id: article.meta.id.clone(),
        });
        fs_ids.insert(article.meta.id.clone());
        fs_ids.extend(article.annotations.iter().map(|a| a.id.clone()));
        let needs_embed = needs_embedding(
            &article.meta.id,
            &checksum(&article.meta.title, article_embed_text(&article)),
        );
        let state_differs = db_meta.get(&article.meta.id).is_some_and(
            |(_, _, _, _, _, unread, color, stored_tags)| {
                *unread != article.meta.is_unread
                    || color.as_str() != article.meta.color.as_deref().unwrap_or("")
                    || stored_tags != &resolved_tags
            },
        );
        if needs_embed
            || state_differs
            || annotations_differ(&article)
            || pinned_ids.contains(&article.meta.id) != article.meta.is_pinned
        {
            sink.article(article);
        }
    }

    // Feeds: diff like items — re-embed a feed only when its row is absent, its checksum changed,
    // or its vector is pending. A feed row is stale when its feed is gone from disk.

    let empty = HashSet::new();

    let all_feeds = crate::persistence::feeds::list_feeds(library_dir, &empty, &empty)
        .await
        .unwrap_or_default();

    let feed_ids: HashSet<String> = all_feeds.iter().map(|f| f.id.clone()).collect();

    let to_embed_feeds: Vec<Feed> = all_feeds
        .into_iter()
        .filter(|f| {
            needs_embedding(&f.id, &checksum(&f.title, feed_embed_text(f)))
                || pinned_feeds.contains(&f.id) != f.is_pinned
        })
        .collect();

    // A delete is only legitimate when the source was actually read: an id absent from `fs_ids` /
    // `feed_ids` means "gone from disk" only if the walk that built those sets was complete. If
    // either item walk could not be fully observed (permission/IO failure — e.g. macOS TCC denying
    // a bundled app access to the library), withhold *all* deletes rather than prune the mirror on
    // a false "empty." The item walks proxy whole-library readability: entries/topics/feeds share
    // one library root, so an access denial hits them together. Embeds/upserts for whatever *was*
    // read still proceed, and deletes self-heal on the next recovery once the source is readable.
    let source_reliable =
        topics_outcome == WalkOutcome::Enumerated && entries_outcome == WalkOutcome::Enumerated;

    // Compare each cached identity with source identities, including annotations.
    // A surviving parent file does not authorize an annotation absent from it.
    let to_delete: Vec<String> = if source_reliable {
        db_meta
            .into_iter()
            .filter(|(id, (_, _, _, kind, _, _, _, _))| {
                if kind == Kind::Feed.as_str() {
                    !feed_ids.contains(id)
                } else {
                    !fs_ids.contains(id)
                }
            })
            .map(|(id, _)| id)
            .collect()
    } else {
        log::warn!(
            "recovery: library walk incomplete (topics={topics_outcome:?}, entries={entries_outcome:?}); \
             withholding all deletes to protect the {}-row embeddings mirror",
            db_meta.len()
        );
        Vec::new()
    };

    Ok(RepairResidue {
        to_embed_feeds,
        to_delete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::EmbeddingEntry;
    use crate::persistence::articles::{add_annotation, read_article, save_article};
    use crate::types::{Annotation, Topic};

    struct CollectingSink {
        articles: Vec<String>,
    }
    impl RepairSink for CollectingSink {
        fn topic(&mut self, _topic: Topic) {}
        fn article(&mut self, article: Article) {
            self.articles.push(article.meta.id.clone());
        }
    }

    fn annotation_row(annotation: &Annotation, checksum: &str) -> EmbeddingEntry {
        EmbeddingEntry {
            id: annotation.id.clone(),
            kind: Kind::Annotation,
            source: String::new(),
            title: annotation.title.clone(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            published_at: String::new(),
            updated_at: String::new(),
            checksum: checksum.to_string(),
            reference: annotation.reference.clone(),
            color: annotation.color.clone(),
            summary: String::new(),
            tags: vec![],
            topics: vec![],
            items: vec![],
            is_unread: false,
            is_pinned: false,
            vector: Some(vec![0.5f32; 384]),
        }
    }

    #[tokio::test]
    async fn recovery_deletes_orphan_annotation_without_deleting_its_parent_or_live_annotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = library::entry_path(dir.path(), "entry");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "---\nsource: https://example.org/a\ntitle: Saved\n---\n\nBody\n",
        )
        .unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let live = Annotation {
            id: "live".into(),
            title: "Passage".into(),
            reference: "entry".into(),
            text: vec!["Passage".into()],
            occurrence: vec![0],
            color: "blue".into(),
            content: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        add_annotation(
            dir.path(),
            "entry",
            live.clone(),
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        let article = read_article(dir.path(), "entry").await.unwrap();
        db.upsert(EmbeddingEntry::from_article(
            &article,
            Some(vec![0.1; 384]),
            Vec::new(),
        ))
        .await
        .unwrap();
        db.upsert(annotation_row(&live, &annotation_checksum(&live)))
            .await
            .unwrap();
        let mut deleted = live.clone();
        deleted.id = "deleted".into();
        db.upsert(annotation_row(&deleted, &annotation_checksum(&deleted)))
            .await
            .unwrap();

        let mut sink = CollectingSink { articles: vec![] };
        let residue = repair_item_embeddings(dir.path(), &db, &mut sink, None)
            .await
            .unwrap();
        assert_eq!(residue.to_delete, vec!["deleted"]);
    }

    #[tokio::test]
    async fn recovery_detects_color_changes_and_removal_without_content_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = library::entry_path(dir.path(), "entry");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "---\nsource: https://example.org/a\ntitle: Saved\ncolor: blue\n---\n\nBody\n",
        )
        .unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let article = read_article(dir.path(), "entry").await.unwrap();
        db.upsert(EmbeddingEntry::from_article(
            &article,
            Some(vec![0.1; 384]),
            Vec::new(),
        ))
        .await
        .unwrap();
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        for color in [Some("green"), None] {
            let current = std::fs::read_to_string(&path).unwrap();
            let updated =
                crate::md::patch_document(&current, &[("color", serde_json::json!(color))], None)
                    .unwrap();
            std::fs::write(&path, updated).unwrap();
            let mut sink = CollectingSink { articles: vec![] };
            repair_item_embeddings(dir.path(), &db, &mut sink, None)
                .await
                .unwrap();
            assert_eq!(sink.articles, ["entry"]);
            let article = read_article(dir.path(), "entry").await.unwrap();
            crate::embeddings::sync_entry_row(&article, &db, &events, &events, &annotations)
                .await
                .unwrap();
            let mut sink = CollectingSink { articles: vec![] };
            repair_item_embeddings(dir.path(), &db, &mut sink, None)
                .await
                .unwrap();
            assert!(sink.articles.is_empty());
            assert_eq!(
                db.get_entry("entry").await.unwrap().unwrap().color,
                color.unwrap_or("")
            );
        }
    }

    #[tokio::test]
    async fn recovery_detects_reading_state_drift_without_content_or_annotation_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = library::entry_path(dir.path(), "entry");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "---\nsource: https://example.org/a\ntitle: Saved\nis_unread: true\n---\n\nBody\n",
        )
        .unwrap();
        let article = read_article(dir.path(), "entry").await.unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let mut row = EmbeddingEntry::from_article(&article, Some(vec![0.1; 384]), Vec::new());
        row.is_unread = false;
        db.upsert(row).await.unwrap();
        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(dir.path(), &db, &mut sink, None)
            .await
            .unwrap();
        assert_eq!(sink.articles, vec!["entry"]);
        let (tx, _) = flume::unbounded();
        let (annotation_tx, _) = flume::unbounded();
        crate::embeddings::sync_entry_row(&article, &db, &tx, &tx, &annotation_tx)
            .await
            .unwrap();
        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(dir.path(), &db, &mut sink, None)
            .await
            .unwrap();
        assert!(sink.articles.is_empty());
    }

    /// A crash-stranded note edit: the `.md`'s annotation content diverged from the stored row
    /// while ids and the entry body are unchanged. The walk must stream the article (the embed
    /// pass heals it); once the stored checksum matches the `.md` again, the walk streams
    /// nothing.
    #[tokio::test]
    async fn stale_annotation_checksum_streams_the_article() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();

        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            "body",
            &std::collections::HashMap::new(),
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        add_annotation(
            lib,
            "abcdef123456",
            Annotation {
                id: "a1".to_string(),
                title: "highlighted".to_string(),
                reference: "abcdef123456".to_string(),
                text: vec!["highlighted".to_string()],
                occurrence: vec![0],
                color: "yellow".to_string(),
                content: "the edited note".to_string(),
                created_at: String::new(),
            },
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        let article = read_article(lib, "abcdef123456").await.unwrap();

        let db = EmbeddingsDb::open(lib).await.unwrap();
        // Entry row fully current: correct checksum, vector present, membership recorded.
        let mut entry_row =
            EmbeddingEntry::from_article(&article, Some(vec![0.5f32; 384]), Vec::new());
        entry_row.items = vec!["a1".to_string()];
        db.upsert(entry_row).await.unwrap();
        // Annotation row: vector present, membership matching — only the checksum is stale.
        db.upsert(annotation_row(&article.annotations[0], "stale"))
            .await
            .unwrap();

        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(lib, &db, &mut sink, None)
            .await
            .unwrap();
        assert_eq!(sink.articles, vec!["abcdef123456".to_string()]);

        // Healed row → clean walk streams nothing.
        db.upsert(annotation_row(
            &article.annotations[0],
            &annotation_checksum(&article.annotations[0]),
        ))
        .await
        .unwrap();
        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(lib, &db, &mut sink, None)
            .await
            .unwrap();
        assert_eq!(sink.articles, Vec::<String>::new());
    }

    #[tokio::test]
    async fn pin_only_drift_is_repaired_without_content_changes() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            "body",
            &Default::default(),
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        let article = read_article(lib, "abcdef123456").await.unwrap();
        let db = EmbeddingsDb::open(lib).await.unwrap();
        db.upsert(EmbeddingEntry::from_article(
            &article,
            Some(vec![0.5; 384]),
            Vec::new(),
        ))
        .await
        .unwrap();
        let writer = crate::library_writer::LibraryWriter::start(
            lib.into(),
            crate::library::acquire_lock(lib).unwrap(),
        );
        for pinned in [true, false] {
            writer
                .set_pin(
                    crate::pins::PinTarget::Entry {
                        id: article.meta.id.clone(),
                    },
                    pinned,
                )
                .await
                .unwrap();
            let mut sink = CollectingSink { articles: vec![] };
            repair_item_embeddings(lib, &db, &mut sink, None)
                .await
                .unwrap();
            assert_eq!(sink.articles, vec![article.meta.id.clone()]);
            let current = read_article(lib, &article.meta.id).await.unwrap();
            let (unread, _) = flume::unbounded();
            let (bookmarked, _) = flume::unbounded();
            let (annotations, _) = flume::unbounded();
            crate::embeddings::pipeline::sync_entry_row(
                &current,
                &db,
                &unread,
                &bookmarked,
                &annotations,
            )
            .await
            .unwrap();
            let row = db.get_entry(&article.meta.id).await.unwrap().unwrap();
            assert_eq!(row.is_pinned, pinned);
            assert_eq!(row.vector, Some(vec![0.5; 384]));
            let pins = db.filter_pinned_entries().await.unwrap();
            assert_eq!(pins.len(), usize::from(pinned));
            if pinned {
                assert!(pins[0].is_pinned);
            }
            let mut sink = CollectingSink { articles: vec![] };
            repair_item_embeddings(lib, &db, &mut sink, None)
                .await
                .unwrap();
            assert!(sink.articles.is_empty());
        }
        let other = tempfile::tempdir().unwrap();
        assert!(
            EmbeddingsDb::open(other.path())
                .await
                .unwrap()
                .filter_pinned_entries()
                .await
                .unwrap()
                .is_empty()
        );
        writer.shutdown().await;
    }

    /// The regression guard: when the entries directory cannot be read (e.g. macOS TCC denying a
    /// bundled app access to the library), recovery must NOT read that as "the library is empty"
    /// and delete every mirror row. An unreadable source withholds all deletes.
    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_library_withholds_deletes() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();

        // A populated DB whose backing `.md` is on disk (not deleted) but about to be unreadable.
        save_article(
            lib,
            "abcdef123456",
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            None,
            "body",
            &std::collections::HashMap::new(),
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        let article = read_article(lib, "abcdef123456").await.unwrap();

        let db = EmbeddingsDb::open(lib).await.unwrap();
        db.upsert(EmbeddingEntry::from_article(
            &article,
            Some(vec![0.5f32; 384]),
            Vec::new(),
        ))
        .await
        .unwrap();

        // Deny enumeration on the entries dir so the walk cannot observe the source.
        let entries = crate::library::entries_dir(lib);
        let original = std::fs::metadata(&entries).unwrap().permissions();
        std::fs::set_permissions(&entries, std::fs::Permissions::from_mode(0o000)).unwrap();

        // A root test runner bypasses chmod; without an enforced denial there is nothing to assert.
        if tokio::fs::read_dir(&entries).await.is_ok() {
            std::fs::set_permissions(&entries, original).unwrap();
            return;
        }

        let mut sink = CollectingSink { articles: vec![] };
        let residue = repair_item_embeddings(lib, &db, &mut sink, None)
            .await
            .unwrap();

        // Restore before asserting so the tempdir stays cleanable even on failure.
        std::fs::set_permissions(&entries, original).unwrap();

        assert!(
            residue.to_delete.is_empty(),
            "an unreadable library must not authorize deletes, got: {:?}",
            residue.to_delete
        );
    }
}

#[cfg(test)]
mod tag_tests {
    use super::*;
    use crate::{
        embeddings::{EmbeddingEntry, sync_entry_row},
        library_writer::LibraryWriter,
    };
    #[derive(Default)]
    struct Sink {
        articles: Vec<Article>,
    }
    impl RepairSink for Sink {
        fn topic(&mut self, _: crate::types::Topic) {}
        fn article(&mut self, article: Article) {
            self.articles.push(article);
        }
    }
    #[tokio::test]
    async fn tag_only_drift_discovers_definitions_and_preserves_vectors_on_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(library::entries_dir(dir.path()))
            .await
            .unwrap();
        let path = library::entry_path(dir.path(), "entry");
        let document = |tags: &str| {
            format!(
                "---\nsource: https://example.org/a\ntitle: Article\ntags: [{tags}]\ncustom: preserved\n---\n\nBody\n"
            )
        };
        tokio::fs::write(&path, document("Research")).await.unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let mut db = EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let mut sink = Sink::default();
        repair_item_embeddings(dir.path(), &db, &mut sink, Some(&writer))
            .await
            .unwrap();
        assert_eq!(sink.articles.len(), 1);
        assert_eq!(sink.articles[0].meta.tags, Some(vec!["Research".into()]));
        let mut row =
            EmbeddingEntry::from_article(&sink.articles[0], Some(vec![0.5; 384]), Vec::new());
        row.tags = crate::tags::Tags::read(dir.path())
            .await
            .unwrap()
            .resolver()
            .resolve(&["Research".into()])
            .unwrap();
        db.upsert(row).await.unwrap();
        sink.articles.clear();
        repair_item_embeddings(dir.path(), &db, &mut sink, Some(&writer))
            .await
            .unwrap();
        assert!(sink.articles.is_empty());
        tokio::fs::write(&path, document("Other")).await.unwrap();
        repair_item_embeddings(dir.path(), &db, &mut sink, Some(&writer))
            .await
            .unwrap();
        assert_eq!(sink.articles.len(), 1);
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        sync_entry_row(&sink.articles[0], &db, &events, &events, &annotations)
            .await
            .unwrap();
        let row = db.get_entry("entry").await.unwrap().unwrap();
        let definitions = crate::tags::Tags::read(dir.path()).await.unwrap();
        assert_eq!(
            row.tags,
            definitions.resolver().resolve(&["Other".into()]).unwrap()
        );
        assert_eq!(row.vector, Some(vec![0.5; 384]));
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            document("Other")
        );
        db.reset().await.unwrap();
        sink.articles.clear();
        repair_item_embeddings(dir.path(), &db, &mut sink, Some(&writer))
            .await
            .unwrap();
        sync_entry_row(&sink.articles[0], &db, &events, &events, &annotations)
            .await
            .unwrap();
        assert_eq!(db.get_entry("entry").await.unwrap().unwrap().tags, row.tags);
        assert_eq!(
            crate::tags::Tags::read(dir.path()).await.unwrap(),
            definitions
        );
        writer.shutdown().await;
    }
    #[tokio::test]
    async fn tag_unknown_discovery_requires_writer_and_does_not_write_on_readonly_path() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(library::entries_dir(dir.path()))
            .await
            .unwrap();
        tokio::fs::write(
            library::entry_path(dir.path(), "entry"),
            "---\nsource: https://example.org/a\ntags: [Unknown]\n---\nBody",
        )
        .await
        .unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        assert!(
            repair_item_embeddings(dir.path(), &db, &mut Sink::default(), None)
                .await
                .is_err()
        );
        assert!(!library::tags_path(dir.path()).exists());
    }
}
