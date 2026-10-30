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
) -> Result<RepairResidue, EmbeddingsError> {
    let db_meta = db.list_entries_meta().await?; // id → (checksum, reference, has_vector, kind, items, is_unread)

    // Embedding is needed when the row is absent, its content changed, or its vector is missing.
    let needs_embedding = |id: &str, cksum: &str| -> bool {
        match db_meta.get(id) {
            None => true,
            Some((stored, _, has_vector, _, _, _)) => stored != cksum || !has_vector,
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
                .is_some_and(|(_, _, _, _, recorded, _)| {
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
    let topics_outcome = for_each_md(&library::topics_dir(library_dir), &md_to_topic, |topic| {
        fs_ids.insert(topic.id.clone());
        if needs_embedding(&topic.id, &checksum(&topic.title, topic_embed_text(&topic))) {
            sink.topic(topic);
        }
    })
    .await;

    let entries_outcome = for_each_md(
        &library::entries_dir(library_dir),
        &md_to_article,
        |article| {
            fs_ids.insert(article.meta.id.clone());
            let needs_embed = needs_embedding(
                &article.meta.id,
                &checksum(&article.meta.title, article_embed_text(&article)),
            );
            // Stream when content, reading state, or annotations drifted from the `.md`
            // (membership, content, or a pending vector) — the consumers re-check each and do only
            // the work required (`sync_entry_row` for rows, `embed_annotations` for vectors).
            let state_differs = db_meta
                .get(&article.meta.id)
                .is_some_and(|(_, _, _, _, _, unread)| *unread != article.meta.is_unread);
            if needs_embed || state_differs || annotations_differ(&article) {
                sink.article(article);
            }
        },
    )
    .await;

    // Feeds: diff like items — re-embed a feed only when its row is absent, its checksum changed,
    // or its vector is pending. A feed row is stale when its feed is gone from disk.

    let empty = HashSet::new();

    let all_feeds = crate::persistence::feeds::list_feeds(library_dir, &empty, &empty)
        .await
        .unwrap_or_default();

    let feed_ids: HashSet<String> = all_feeds.iter().map(|f| f.id.clone()).collect();

    let to_embed_feeds: Vec<Feed> = all_feeds
        .into_iter()
        .filter(|f| needs_embedding(&f.id, &checksum(&f.title, feed_embed_text(f))))
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

    // Build to_delete: in LanceDB but no longer on disk. Item rows: the `.md` is gone (annotation
    // rows carry a non-empty `reference` to their parent article — check that). Feed rows: the feed
    // is gone from disk.
    let to_delete: Vec<String> = if source_reliable {
        db_meta
            .into_iter()
            .filter(|(id, (_, reference, _, kind, _, _))| {
                if kind == Kind::Feed.as_str() {
                    return !feed_ids.contains(id);
                }

                let file_id = if reference.is_empty() {
                    id.as_str()
                } else {
                    reference.as_str()
                };

                !fs_ids.contains(file_id)
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
            is_bookmarked: false,
            is_pinned: false,
            vector: Some(vec![0.5f32; 384]),
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
        let mut row = EmbeddingEntry::from_article(&article, Some(vec![0.1; 384]));
        row.is_unread = false;
        db.upsert(row).await.unwrap();
        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(dir.path(), &db, &mut sink)
            .await
            .unwrap();
        assert_eq!(sink.articles, vec!["entry"]);
        let (tx, _) = flume::unbounded();
        let (annotation_tx, _) = flume::unbounded();
        crate::embeddings::sync_entry_row(&article, &db, &tx, &tx, &annotation_tx)
            .await
            .unwrap();
        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(dir.path(), &db, &mut sink)
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
        let mut entry_row = EmbeddingEntry::from_article(&article, Some(vec![0.5f32; 384]));
        entry_row.items = vec!["a1".to_string()];
        db.upsert(entry_row).await.unwrap();
        // Annotation row: vector present, membership matching — only the checksum is stale.
        db.upsert(annotation_row(&article.annotations[0], "stale"))
            .await
            .unwrap();

        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(lib, &db, &mut sink).await.unwrap();
        assert_eq!(sink.articles, vec!["abcdef123456".to_string()]);

        // Healed row → clean walk streams nothing.
        db.upsert(annotation_row(
            &article.annotations[0],
            &annotation_checksum(&article.annotations[0]),
        ))
        .await
        .unwrap();
        let mut sink = CollectingSink { articles: vec![] };
        repair_item_embeddings(lib, &db, &mut sink).await.unwrap();
        assert_eq!(sink.articles, Vec::<String>::new());
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
        let residue = repair_item_embeddings(lib, &db, &mut sink).await.unwrap();

        // Restore before asserting so the tempdir stays cleanable even on failure.
        std::fs::set_permissions(&entries, original).unwrap();

        assert!(
            residue.to_delete.is_empty(),
            "an unreadable library must not authorize deletes, got: {:?}",
            residue.to_delete
        );
    }
}
