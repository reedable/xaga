use super::schema::EMBEDDING_DIM;
use super::*;

const TAG_A: &str = "46fa4a27-1ddf-448a-a5e6-95691a29c602";
const TAG_B: &str = "46fa4a27-1ddf-448a-a5e6-95691a29c603";

#[tokio::test]
async fn tag_browsing_intersects_direct_assignments_and_counts_each_entry_once() {
    use crate::types::EntityFilter;
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let mut both = sample("both", "entry", None);
    both.tags = vec![TAG_A.into(), TAG_B.into(), TAG_A.into()];
    both.is_unread = true;
    both.color = "blue".into();
    let mut one = sample("one", "entry", None);
    one.tags = vec![TAG_A.into()];
    one.is_unread = true;
    one.color.clear();
    let mut read = sample("read", "entry", None);
    read.tags = vec![TAG_A.into(), TAG_B.into()];
    read.is_unread = false;
    read.color.clear();
    let mut annotation = sample("annotation", "annotation", None);
    annotation.tags = vec![TAG_A.into(), TAG_B.into()];
    annotation.is_unread = true;
    let mut feed = sample("feed", "feed", None);
    feed.tags = vec![TAG_A.into(), TAG_B.into()];
    feed.is_unread = true;
    let mut empty = sample("empty", "entry", None);
    empty.tags.clear();
    empty.is_unread = false;
    db.add_batch(vec![both, one, read, annotation, feed, empty])
        .await
        .unwrap();
    db.ensure_indexes().await.unwrap();
    db.ensure_indexes().await.unwrap();
    let ids = vec![TAG_A.into(), TAG_B.into()];
    let list = db.list_tag_entities(&ids, EntityFilter::All).await.unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().any(|entry| entry.id == "both"));
    assert!(
        list.iter()
            .any(|entry| entry.id == "annotation" && entry.reference.is_some())
    );
    assert_eq!(db.count_tag_unread(&ids).await.unwrap(), 1);
    assert_eq!(
        db.list_tag_entities(&ids, EntityFilter::Unread)
            .await
            .unwrap()
            .len(),
        3
    );
    let saved = db
        .list_tag_entities(&ids, EntityFilter::Saved)
        .await
        .unwrap();
    assert_eq!(saved.len(), 2);
    assert!(saved.iter().any(|entity| entity.id == "annotation"));
    assert_eq!(
        db.list_tag_entities(&[], EntityFilter::All)
            .await
            .unwrap()
            .len(),
        5
    );
    assert_eq!(db.count_tag_unread(&[]).await.unwrap(), 2);
    for filter in [
        EntityFilter::All,
        EntityFilter::Unread,
        EntityFilter::Saved,
        EntityFilter::Bookmarked,
    ] {
        let mut tags: Vec<_> = db
            .list_tag_entities(&[], filter)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        let mut all: Vec<_> = db
            .filter_entities(filter)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        tags.sort();
        all.sort();
        assert_eq!(tags, all);
    }

    assert!(
        db.list_tag_entities(&[uuid::Uuid::new_v4().to_string()], EntityFilter::All)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        db.list_tag_entities(&["x') OR true --".into()], EntityFilter::All)
            .await
            .is_err()
    );
    let predicate = EmbeddingsDb::tag_predicate(&ids, EntityFilter::Unread).unwrap();
    let plan = db
        .table
        .query()
        .only_if(predicate)
        .explain_plan(false)
        .await
        .unwrap();
    println!("tag intersection plan: {plan}");
    assert!(plan.contains("ScalarIndex"), "{plan}");
    assert!(plan.contains("LabelList"), "{plan}");
    crate::embeddings::pipeline::apply_entry_tag(
        &db,
        &crate::tags::EntryTagChange {
            entry_id: "both".into(),
            tag_id: TAG_B.into(),
            assigned: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(db.count_tag_unread(&ids).await.unwrap(), 0);
    assert_eq!(
        db.list_tag_entities(&ids, EntityFilter::All)
            .await
            .unwrap()
            .len(),
        2
    );
    let readonly = EmbeddingsDb::open_readonly(dir.path()).await.unwrap();
    assert_eq!(
        readonly
            .list_tag_entities(&[], EntityFilter::All)
            .await
            .unwrap()
            .len(),
        5
    );
}

#[tokio::test]
#[ignore = "explicit bounded tag-query scaling measurement"]
async fn tag_browsing_scaling_curve() {
    use crate::types::EntityFilter;
    use std::time::Instant;
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let mut previous = 0;
    for total in [2000, 4000, 8000, 16000, 32000, 64000] {
        let rows = (previous..total)
            .map(|i| {
                let mut row = sample(&format!("scale-{i}"), "entry", None);
                row.tags = vec![TAG_A.into()];
                if i % 100 == 0 {
                    row.tags.push(TAG_B.into());
                }
                row.is_unread = i % 200 == 0;
                row
            })
            .collect();
        let build = Instant::now();
        db.add_batch(rows).await.unwrap();
        // Rebuild/optimize to compare fully indexed points, rather than tail scans.
        db.ensure_indexes().await.unwrap();
        db.table
            .optimize(lancedb::table::optimize::OptimizeAction::Index(
                Default::default(),
            ))
            .await
            .unwrap();
        let index_ms = build.elapsed().as_millis();
        let ids = vec![TAG_A.into(), TAG_B.into()];
        let plan = db
            .table
            .query()
            .only_if(EmbeddingsDb::tag_predicate(&ids, EntityFilter::Unread).unwrap())
            .explain_plan(false)
            .await
            .unwrap();
        assert!(plan.contains("LabelList"), "{plan}");
        let start = Instant::now();
        let count = db.count_tag_unread(&ids).await.unwrap();
        let count_us = start.elapsed().as_micros();
        let start = Instant::now();
        let sparse = db.list_tag_entities(&ids, EntityFilter::All).await.unwrap();
        let sparse_us = start.elapsed().as_micros();
        let start = Instant::now();
        let dense = db.list_tag_entities(&[], EntityFilter::All).await.unwrap();
        let dense_us = start.elapsed().as_micros();
        assert_eq!(count, total / 200);
        assert_eq!(sparse.len(), total / 100);
        assert_eq!(dense.len(), total);
        println!(
            "tag_scale rows={total} build_index_ms={index_ms} count_us={count_us} sparse_us={sparse_us} dense_us={dense_us}"
        );
        previous = total;
    }
}

#[tokio::test]
#[ignore = "explicit two-million-row annotation tagging measurement"]
async fn annotation_tag_scaling() {
    use std::time::Instant;
    let dir = tempfile::tempdir().unwrap();
    let writer = crate::library_writer::LibraryWriter::start(
        dir.path().into(),
        crate::library::acquire_lock(dir.path()).unwrap(),
    );
    let research = writer.create_tag("Research".into()).await.unwrap().tag;
    let work = writer.create_tag("Work".into()).await.unwrap().tag;
    let db = EmbeddingsDb::open_writer(dir.path(), &writer.lock())
        .await
        .unwrap();
    let started = Instant::now();
    for base in (0..2_000_000).step_by(25_000) {
        let rows = (base..base + 25_000)
            .map(|i| {
                let annotation = i % 10 == 0;
                let mut row = sample(
                    &format!("{}-{i:07}", if annotation { "ann" } else { "entry" }),
                    if annotation { "annotation" } else { "entry" },
                    None,
                );
                if annotation {
                    row.reference = if i < 10_240 {
                        "shared-parent".into()
                    } else {
                        format!("parent-{i:07}")
                    };
                }
                if (!annotation && i % 100 == 1) || (annotation && i < 10_240) {
                    row.tags = vec![research.id.clone(), work.id.clone()];
                }
                row.is_unread = i % 200 == 1;
                row
            })
            .collect();
        db.add_batch(rows).await.unwrap();
    }
    db.ensure_indexes().await.unwrap();
    db.table
        .optimize(lancedb::table::optimize::OptimizeAction::Index(
            Default::default(),
        ))
        .await
        .unwrap();
    let build_ms = started.elapsed().as_millis();
    let groups = vec![vec![research.id.clone()], vec![work.id.clone()]];
    let predicate =
        EmbeddingsDb::tag_group_predicate_for(&groups, crate::types::EntityFilter::All, None)
            .unwrap();
    let plan = db
        .table
        .query()
        .select(Select::columns(&EmbeddingsDb::SUMMARY_COLUMNS))
        .only_if(predicate)
        .explain_plan(false)
        .await
        .unwrap();
    assert!(plan.contains("LabelList"), "{plan}");
    let start = Instant::now();
    let results = db
        .list_tag_entity_groups(&groups, crate::types::EntityFilter::All)
        .await
        .unwrap();
    let list_ms = start.elapsed().as_millis();
    assert_eq!(results.len(), 21_024);
    let start = Instant::now();
    assert_eq!(db.count_tag_group_unread(&groups).await.unwrap(), 10_000);
    let count_ms = start.elapsed().as_millis();
    let start = Instant::now();
    assert_eq!(
        db.tag_entry_id_batch(&research.id, Some("entry-9999999"), 1024)
            .await
            .unwrap(),
        vec!["shared-parent"]
    );
    let candidates_ms = start.elapsed().as_millis();
    let path = crate::library::entry_path(dir.path(), "shared-parent");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let annotations = (0..1024).map(|i| format!("- id: ann-{:07}\n  text: [passage]\n  occurrence: [0]\n  color: blue\n  created_at: today\n  tags: [Research, Work]\n", i*10)).collect::<String>();
    std::fs::write(
        &path,
        format!(
            "---\nsource: https://example.org/a\nannotations:\n{annotations}---\n\nExact body\n"
        ),
    )
    .unwrap();
    let start = Instant::now();
    writer
        .patch_tag_labels(
            "shared-parent".into(),
            vec!["Research".into()],
            Some("Notes".into()),
        )
        .await
        .unwrap();
    let source_ms = start.elapsed().as_millis();
    let article = crate::persistence::articles::read_article(dir.path(), "shared-parent")
        .await
        .unwrap();
    assert_eq!(article.annotations.len(), 1024);
    assert!(
        article
            .annotations
            .iter()
            .all(|a| a.tags == vec!["Notes", "Work"])
    );
    let start = Instant::now();
    db.patch_tag_batch(&research.id, Some(&work.id), &["shared-parent".into()])
        .await
        .unwrap();
    let cache_ms = start.elapsed().as_millis();
    assert_eq!(
        db.get_entry("ann-0000000").await.unwrap().unwrap().tags,
        vec![work.id]
    );
    println!(
        "annotation_tag_scale rows=2000000 annotations=200000 shared_parent_annotations=1024 build_ms={build_ms} list_ms={list_ms} count_ms={count_ms} parent_candidates_ms={candidates_ms} source_patch_ms={source_ms} cache_patch_ms={cache_ms}\n{plan}"
    );
    writer.shutdown().await;
}

#[tokio::test]
async fn readonly_open_does_not_create_the_documents_table() {
    let dir = tempfile::tempdir().unwrap();

    assert!(EmbeddingsDb::open_readonly(dir.path()).await.is_err());
    assert!(!crate::library::index_path(dir.path()).exists());

    let connection = lancedb::connect(
        crate::library::index_path(dir.path())
            .to_str()
            .expect("UTF-8 test path"),
    )
    .execute()
    .await
    .unwrap();
    assert!(
        !connection
            .table_names()
            .execute()
            .await
            .unwrap()
            .contains(&"documents".to_string())
    );
}

fn sample(id: &str, kind: &str, vector: Option<Vec<f32>>) -> EmbeddingEntry {
    EmbeddingEntry {
        id: id.to_string(),
        kind: kind.parse().expect("valid kind"),
        source: "https://example.com/a".to_string(),
        title: "Title".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        published_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        checksum: "abc".to_string(),
        reference: String::new(),
        color: String::new(),
        summary: "a summary".to_string(),
        tags: vec![],
        topics: vec!["topic-1".to_string(), "topic-2".to_string()],
        entities: vec![],
        is_unread: true,
        is_pinned: true,
        vector,
    }
}

/// The schema round-trips a row both with a vector and with a null vector,
/// preserving the new `kind`/state/`summary`/`topics` columns. This guards the
/// Arrow null-validity buffer and column ordering before the single DB reset.
#[tokio::test]
async fn schema_round_trips_with_and_without_vector() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    db.upsert(sample(
        "with-vec",
        "entry",
        Some(vec![0.5f32; EMBEDDING_DIM as usize]),
    ))
    .await
    .unwrap();

    db.upsert(sample("no-vec", "entry", None)).await.unwrap();
    assert_eq!(db.count().await.unwrap(), 2);

    let got = db
        .get_entry("with-vec")
        .await
        .unwrap()
        .expect("row present");

    assert_eq!(got.kind, Kind::Entry);
    assert_eq!(got.summary, "a summary");

    assert_eq!(
        got.topics,
        vec!["topic-1".to_string(), "topic-2".to_string()]
    );

    assert!(got.is_unread && got.is_pinned && got.color.is_empty());

    let v = got.vector.expect("vector present");

    assert_eq!(v.len(), EMBEDDING_DIM as usize);
    assert_eq!(v[0], 0.5);

    let pending = db.get_entry("no-vec").await.unwrap().expect("row present");

    assert!(
        pending.vector.is_none(),
        "null-vector row must read back as None"
    );

    assert_eq!(
        pending.topics,
        vec!["topic-1".to_string(), "topic-2".to_string()]
    );

    assert!(pending.is_pinned);
}

#[tokio::test]
async fn projected_point_reads_decode_lists_without_materializing_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let mut row = sample(
        "topic-x",
        "topic",
        Some(vec![0.5f32; EMBEDDING_DIM as usize]),
    );
    row.entities = vec!["entry-1".to_string(), "annotation-1".to_string()];
    db.upsert(row).await.unwrap();

    assert_eq!(
        db.find_topic_ids_by_entity_id("topic-x").await.unwrap(),
        Some(vec!["topic-1".to_string(), "topic-2".to_string()])
    );
    assert_eq!(
        db.find_entity_ids_by_topic_id("topic-x").await.unwrap(),
        Some(vec!["entry-1".to_string(), "annotation-1".to_string()])
    );
    assert_eq!(
        db.find_topic_ids_by_entity_id("missing").await.unwrap(),
        None
    );

    for column in ["topics", "items"] {
        let batch = db
            .find_batch_by_id("topic-x", &[column])
            .await
            .unwrap()
            .expect("row present");
        assert_eq!(batch.num_columns(), 1);
        assert!(batch.column_by_name(column).is_some());
        assert!(batch.column_by_name("vector").is_none());
    }
}

#[tokio::test]
async fn all_entities_counts_entries_annotations_and_unread_entries() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let unread_entry = sample("unread-entry", "entry", None);
    let mut read_entry = sample("read-entry", "entry", None);
    read_entry.is_unread = false;
    let mut annotation = sample("annotation", "annotation", None);
    annotation.is_unread = false;
    let mut topic = sample("topic", "topic", None);
    topic.is_unread = false;

    for row in [unread_entry, read_entry, annotation, topic] {
        db.upsert(row).await.unwrap();
    }

    assert_eq!(
        db.all_entities_counts().await.unwrap(),
        AllEntitiesCounts {
            entry_count: 3,
            unread_count: 1,
        }
    );
}

/// The `items` column (the topic-membership reverse-index base) round-trips, and an
/// empty `items` reads back empty — guards the new column's Arrow building and ordering.
#[tokio::test]
async fn entities_column_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let mut topic = sample("topic-x", "topic", None);

    topic.entities = vec!["m1".to_string(), "m2".to_string(), "m3".to_string()];
    db.upsert(topic).await.unwrap();
    db.upsert(sample("entry-x", "entry", None)).await.unwrap();

    let got = db
        .get_entry("topic-x")
        .await
        .unwrap()
        .expect("topic row present");

    assert_eq!(
        got.entities,
        vec!["m1".to_string(), "m2".to_string(), "m3".to_string()]
    );

    let entry = db
        .get_entry("entry-x")
        .await
        .unwrap()
        .expect("entry row present");

    assert!(
        entry.entities.is_empty(),
        "an entry with no members/annotations reads back empty"
    );
}

/// `reconcile_topic_membership` adds the topic to newly-listed members, removes it from
/// dropped members (via the topic-row `items` base), and tracks the base for the next diff.
#[tokio::test]
async fn reconcile_topic_membership_adds_removes_and_tracks_base() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    // Topic row with empty base; two entry rows with empty topics.
    let mut topic = sample("topic-1", "topic", None);

    topic.topics = vec![];
    db.upsert(topic).await.unwrap();

    for id in ["entry-1", "entry-2"] {
        let mut e = sample(id, "entry", None);
        e.topics = vec![];
        db.upsert(e).await.unwrap();
    }

    // First reconcile: both entries become members.
    crate::embeddings::pipeline::reconcile_topic_membership(
        &db,
        "topic-1",
        &["entry-1".to_string(), "entry-2".to_string()],
    )
    .await
    .unwrap();

    assert!(
        db.get_entry("entry-1")
            .await
            .unwrap()
            .unwrap()
            .topics
            .contains(&"topic-1".to_string())
    );

    assert!(
        db.get_entry("entry-2")
            .await
            .unwrap()
            .unwrap()
            .topics
            .contains(&"topic-1".to_string())
    );

    assert_eq!(
        db.get_entry("topic-1").await.unwrap().unwrap().entities,
        vec!["entry-1".to_string(), "entry-2".to_string()],
        "base tracks the propagated members"
    );

    // Second reconcile: entry-2 dropped from the topic.
    crate::embeddings::pipeline::reconcile_topic_membership(
        &db,
        "topic-1",
        &["entry-1".to_string()],
    )
    .await
    .unwrap();

    assert!(
        db.get_entry("entry-1")
            .await
            .unwrap()
            .unwrap()
            .topics
            .contains(&"topic-1".to_string())
    );

    assert!(
        !db.get_entry("entry-2")
            .await
            .unwrap()
            .unwrap()
            .topics
            .contains(&"topic-1".to_string()),
        "dropped member loses the topic"
    );

    assert_eq!(
        db.get_entry("topic-1").await.unwrap().unwrap().entities,
        vec!["entry-1".to_string()]
    );
}

/// `add_topic_member` / `remove_topic_member` update just that one member's reverse-index
/// `topics` column and the topic-row `items` base — the precise O(1) live path. Re-adding
/// an existing member is a no-op.
#[tokio::test]
async fn single_member_add_remove_tracks_base() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let mut topic = sample("topic-1", "topic", None);

    topic.topics = vec![];
    topic.entities = vec![];
    db.upsert(topic).await.unwrap();

    let mut e = sample("entry-1", "entry", None);

    e.topics = vec![];
    db.upsert(e).await.unwrap();

    crate::embeddings::pipeline::add_topic_member(&db, "topic-1", "entry-1")
        .await
        .unwrap();

    assert!(
        db.get_entry("entry-1")
            .await
            .unwrap()
            .unwrap()
            .topics
            .contains(&"topic-1".to_string())
    );

    assert_eq!(
        db.get_entry("topic-1").await.unwrap().unwrap().entities,
        vec!["entry-1".to_string()]
    );

    // Idempotent re-add: base unchanged.
    crate::embeddings::pipeline::add_topic_member(&db, "topic-1", "entry-1")
        .await
        .unwrap();

    assert_eq!(
        db.get_entry("topic-1").await.unwrap().unwrap().entities,
        vec!["entry-1".to_string()]
    );

    crate::embeddings::pipeline::remove_topic_member(&db, "topic-1", "entry-1")
        .await
        .unwrap();

    assert!(
        !db.get_entry("entry-1")
            .await
            .unwrap()
            .unwrap()
            .topics
            .contains(&"topic-1".to_string())
    );

    assert!(
        db.get_entry("topic-1")
            .await
            .unwrap()
            .unwrap()
            .entities
            .is_empty()
    );
}

#[tokio::test]
async fn unread_summaries_and_sync_events_use_the_boolean_without_tags() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let mut row = sample("entry-1", "entry", None);
    row.tags.clear();
    row.is_unread = true;
    db.upsert(row).await.unwrap();
    assert!(db.list_entries().await.unwrap()[0].is_unread);
    let article = crate::types::Article {
        meta: db.list_entries().await.unwrap().remove(0),
        content: "Body".into(),
        annotations: vec![],
        favicon: None,
    };
    let mut initial = EmbeddingEntry::from_article(
        &article,
        Some(vec![0.1; EMBEDDING_DIM as usize]),
        Vec::new(),
    );
    initial.is_unread = false;
    db.upsert(initial).await.unwrap();
    let (unread_tx, unread_rx) = flume::unbounded();
    let (bookmark_tx, _) = flume::unbounded();
    let (annotation_tx, _) = flume::unbounded();
    crate::embeddings::sync_entry_row(&article, &db, &unread_tx, &bookmark_tx, &annotation_tx)
        .await
        .unwrap();
    assert_eq!(unread_rx.len(), 1);
    let row = db.get_entry("entry-1").await.unwrap().unwrap();
    assert!(row.is_unread);
    assert!(row.tags.is_empty());
    assert!(row.vector.is_some());
    crate::embeddings::sync_entry_row(&article, &db, &unread_tx, &bookmark_tx, &annotation_tx)
        .await
        .unwrap();
    assert_eq!(unread_rx.len(), 1);
    assert_eq!(db.all_entities_counts().await.unwrap().unread_count, 1);
    assert!(db.unread_ids().await.unwrap().contains("entry-1"));
}

/// `apply_state` write-through flips the boolean columns + tags from the carried values,
/// preserves the vector/topics, and emits a change event only for columns that changed.
#[tokio::test]
async fn apply_state_writes_through_and_emits() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let mut e = sample(
        "entry-1",
        "entry",
        Some(vec![0.1f32; EMBEDDING_DIM as usize]),
    );

    e.tags = vec!["research".to_string()];
    e.is_unread = true;
    e.color.clear();
    e.topics = vec!["topic-1".to_string()];
    db.upsert(e).await.unwrap();

    let (unread_tx, unread_rx) = flume::unbounded();
    let (bm_tx, bm_rx) = flume::unbounded();

    // Mark read (is_unread -> false) and bookmark (-> true) in one patch.
    crate::embeddings::pipeline::apply_state(
        &db,
        "entry-1",
        Some(false),
        Some(Some("blue".into())),
        None,
        &unread_tx,
        &bm_tx,
    )
    .await
    .unwrap();

    let row = db.get_entry("entry-1").await.unwrap().unwrap();
    assert!(!row.is_unread && !row.color.is_empty());
    assert!(!row.tags.contains(&"unread".to_string()));
    assert!(!row.tags.contains(&"bookmark".to_string()));
    assert!(row.tags.contains(&"research".to_string()));
    assert!(row.vector.is_some(), "vector preserved");
    assert_eq!(row.topics, vec!["topic-1".to_string()], "topics preserved");
    assert_eq!(unread_rx.len(), 1, "is_unread changed → one event");
    assert_eq!(bm_rx.len(), 1, "color changed → one event");

    // A no-op patch (same value) emits nothing.
    crate::embeddings::pipeline::apply_state(
        &db,
        "entry-1",
        Some(false),
        None,
        None,
        &unread_tx,
        &bm_tx,
    )
    .await
    .unwrap();

    assert_eq!(unread_rx.len(), 1, "no event when unchanged");
}

#[tokio::test]
async fn color_events_include_recolor_and_removal_and_preserve_cached_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    db.upsert(sample("marked", "entry", Some(vec![0.1; 384])))
        .await
        .unwrap();
    let (unread, unread_events) = flume::unbounded();
    let (colors, color_events) = flume::unbounded();
    for color in [Some("blue"), Some("green"), None] {
        crate::embeddings::pipeline::apply_state(
            &db,
            "marked",
            None,
            Some(color.map(str::to_owned)),
            None,
            &unread,
            &colors,
        )
        .await
        .unwrap();
        assert_eq!(color_events.recv().unwrap().color.as_deref(), color);
        crate::embeddings::pipeline::apply_state(
            &db,
            "marked",
            None,
            Some(color.map(str::to_owned)),
            None,
            &unread,
            &colors,
        )
        .await
        .unwrap();
        assert!(color_events.is_empty());
        let row = db.get_entry("marked").await.unwrap().unwrap();
        assert_eq!(row.color, color.unwrap_or(""));
        assert_eq!(row.topics, ["topic-1", "topic-2"]);
        assert!(row.vector.is_some());
        assert_eq!(
            db.filter_bookmarked().await.unwrap().len(),
            usize::from(color.is_some())
        );
    }
    assert!(unread_events.is_empty());
    assert!(
        db.table
            .schema()
            .await
            .unwrap()
            .field_with_name("is_bookmarked")
            .is_err()
    );
}

#[tokio::test]
async fn color_queries_survive_cache_rebuild_and_index_creation() {
    let dir = tempfile::tempdir().unwrap();
    let writer = crate::library_writer::LibraryWriter::start(
        dir.path().into(),
        crate::library::acquire_lock(dir.path()).unwrap(),
    );
    writer
        .discover_tags(vec!["research".into(), "bookmark".into()])
        .await
        .unwrap();
    std::fs::create_dir_all(
        crate::library::entry_path(dir.path(), "fixture")
            .parent()
            .unwrap(),
    )
    .unwrap();
    for color in ["", "red", "orange", "yellow", "green", "blue", "violet"] {
        let id = if color.is_empty() { "unmarked" } else { color };
        let marking = if color.is_empty() {
            String::new()
        } else {
            format!("color: {color}\n")
        };
        let annotation = if color.is_empty() {
            "annotations:\n  - id: highlight\n    text: [selection]\n    occurrence: [0]\n    color: yellow\n    created_at: today\n"
        } else {
            ""
        };
        // An organizational label named bookmark must not restore marked state.
        std::fs::write(crate::library::entry_path(dir.path(), id), format!("---\nsource: https://example.org/{id}\ntitle: {id}\ntags: [research, bookmark]\n{marking}{annotation}---\n\nBody\n")).unwrap();
    }
    let mut db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let (events, _) = flume::unbounded();
    let (annotations, _) = flume::unbounded();
    for pass in 0..2 {
        if pass == 1 {
            db.reset().await.unwrap();
        }
        for id in [
            "unmarked", "red", "orange", "yellow", "green", "blue", "violet",
        ] {
            let article = crate::persistence::articles::read_article(dir.path(), id)
                .await
                .unwrap();
            crate::embeddings::sync_entry_row(&article, &db, &events, &events, &annotations)
                .await
                .unwrap();
        }
        db.ensure_indexes().await.unwrap();
        assert_eq!(
            db.filter_entities(crate::types::EntityFilter::All)
                .await
                .unwrap()
                .len(),
            8
        );
        let marked = db
            .filter_entities(crate::types::EntityFilter::Bookmarked)
            .await
            .unwrap();
        assert_eq!(marked.len(), 6);
        assert!(
            marked
                .iter()
                .all(|entity| entity.color.is_some() && entity.reference.is_none())
        );
        assert_eq!(db.get_bookmarked_ids().await.unwrap().len(), 6);
        let saved = db
            .filter_entities(crate::types::EntityFilter::Saved)
            .await
            .unwrap();
        assert_eq!(saved.len(), 7);
        assert!(
            saved.iter().any(|entity| entity.id == "highlight"
                && entity.reference.as_deref() == Some("unmarked"))
        );
        assert!(
            db.list_entries()
                .await
                .unwrap()
                .iter()
                .find(|entry| entry.id == "unmarked")
                .unwrap()
                .color
                .is_none()
        );
    }
    writer.shutdown().await;
}

/// Explicit scale verification; isolated synthetic cache, no model download.
#[tokio::test]
#[ignore = "creates a two-million-row isolated cache for query-plan and mutation measurements"]
async fn color_queries_at_design_scale() {
    use std::time::Instant;
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    let mut blue = 0usize;
    let mut annotations = 0usize;
    let started = Instant::now();
    for start in (0..2_000_000).step_by(32_768) {
        let rows = (start..(start + 32_768).min(2_000_000))
            .map(|i| {
                let annotation = i % 10 == 0;
                let mut row = sample(
                    &format!("scale-{i}"),
                    if annotation { "annotation" } else { "entry" },
                    None,
                );
                row.is_pinned = false;
                row.topics.clear();
                row.color = if annotation {
                    annotations += 1;
                    "yellow".into()
                } else if i % 7 == 0 {
                    blue += 1;
                    "blue".into()
                } else {
                    String::new()
                };
                row
            })
            .collect();
        db.add_batch(rows).await.unwrap();
    }
    println!(
        "scale rows=2000000 load_ms={}",
        started.elapsed().as_millis()
    );
    let started = Instant::now();
    db.ensure_indexes().await.unwrap();
    println!("index_ms={}", started.elapsed().as_millis());
    let (events, _) = flume::unbounded();
    for (stage, color, marked_delta, blue_delta) in [
        ("initial", None, 0usize, 0usize),
        ("assigned", Some(Some("green".to_string())), 1, 0),
        ("recolored", Some(Some("blue".to_string())), 1, 1),
        ("removed", Some(None), 0, 0),
    ] {
        if let Some(color) = color {
            let started = Instant::now();
            crate::embeddings::pipeline::apply_state(
                &db,
                "scale-1",
                None,
                Some(color),
                None,
                &events,
                &events,
            )
            .await
            .unwrap();
            println!("{stage} mutation_ms={}", started.elapsed().as_millis());
        }
        for (name, predicate, expected) in [
            (
                "blue",
                "kind = 'entry' AND color = 'blue'",
                blue + blue_delta,
            ),
            (
                "marked",
                "kind = 'entry' AND color != ''",
                blue + marked_delta,
            ),
            (
                "saved",
                "(kind = 'entry' AND color != '') OR kind = 'annotation'",
                blue + marked_delta + annotations,
            ),
        ] {
            let query = db
                .table
                .query()
                .select(Select::columns(&EmbeddingsDb::SUMMARY_COLUMNS))
                .only_if(predicate);
            let plan = query.explain_plan(false).await.unwrap();
            println!("{stage} {name} plan={plan}");
            assert!(plan.contains("ScalarIndex"), "missing scalar index: {plan}");
            let started = Instant::now();
            let batches: Vec<RecordBatch> =
                query.execute().await.unwrap().try_collect().await.unwrap();
            let count: usize = batches.iter().map(RecordBatch::num_rows).sum();
            println!(
                "{stage} {name} rows={count} query_ms={}",
                started.elapsed().as_millis()
            );
            assert_eq!(count, expected);
        }
    }
}

/// The reconcile is member-type-agnostic: an annotation-kind member gets its `topics`
/// updated exactly like an entry (covers W2 — callers now pass annotation members too).
#[tokio::test]
async fn reconcile_handles_annotation_members() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let mut topic = sample("topic-a", "topic", None);
    topic.topics = vec![];
    db.upsert(topic).await.unwrap();
    let mut anno = sample("anno-1", "annotation", None);
    anno.topics = vec![];
    db.upsert(anno).await.unwrap();

    crate::embeddings::pipeline::reconcile_topic_membership(
        &db,
        "topic-a",
        &["anno-1".to_string()],
    )
    .await
    .unwrap();

    let got = db
        .get_entry("anno-1")
        .await
        .unwrap()
        .expect("annotation row present");

    assert_eq!(got.kind, Kind::Annotation);

    assert!(
        got.topics.contains(&"topic-a".to_string()),
        "annotation member gets the topic"
    );
}

/// A membership change larger than the chunk size is applied across multiple chunks: every
/// member ends in the right state and the topic-row `items` base tracks the propagated set,
/// for both the add and the remove direction.
#[tokio::test]
async fn reconcile_chunked_multi_chunk_add_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let mut topic = sample("topic-1", "topic", None);
    topic.topics = vec![];
    db.upsert(topic).await.unwrap();

    let members: Vec<String> = (0..5).map(|i| format!("entry-{i}")).collect();

    for id in &members {
        let mut e = sample(id, "entry", None);
        e.topics = vec![];
        db.upsert(e).await.unwrap();
    }

    // Add all 5 with chunk size 2 -> 3 chunks.
    crate::embeddings::pipeline::reconcile_topic_membership_chunked(&db, "topic-1", &members, 2)
        .await
        .unwrap();

    for id in &members {
        assert!(
            db.get_entry(id)
                .await
                .unwrap()
                .unwrap()
                .topics
                .contains(&"topic-1".to_string()),
            "{id} tagged after multi-chunk add"
        );
    }

    let mut base = db.get_entry("topic-1").await.unwrap().unwrap().entities;
    base.sort();
    assert_eq!(base, members, "base = all members after multi-chunk add");

    // Drop 3, keep 2 (removals span 2 chunks at chunk size 2).
    let keep = vec!["entry-0".to_string(), "entry-1".to_string()];
    crate::embeddings::pipeline::reconcile_topic_membership_chunked(&db, "topic-1", &keep, 2)
        .await
        .unwrap();
    for id in &keep {
        assert!(
            db.get_entry(id)
                .await
                .unwrap()
                .unwrap()
                .topics
                .contains(&"topic-1".to_string())
        );
    }
    for id in ["entry-2", "entry-3", "entry-4"] {
        assert!(
            !db.get_entry(id)
                .await
                .unwrap()
                .unwrap()
                .topics
                .contains(&"topic-1".to_string()),
            "{id} untagged after multi-chunk remove"
        );
    }
    let mut base = db.get_entry("topic-1").await.unwrap().unwrap().entities;
    base.sort();
    assert_eq!(base, keep, "base = kept members after multi-chunk remove");
}

/// Simulates a crash mid-fan-out: some members already carry the topic and the base is only
/// partially advanced. A re-run converges (all members tagged, base complete) and the
/// already-correct members are not double-tagged (the per-member idempotency skip).
#[tokio::test]
async fn reconcile_chunked_resumes_from_partial_state() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let members: Vec<String> = (0..4).map(|i| format!("entry-{i}")).collect();
    for id in &members {
        let mut e = sample(id, "entry", None);
        // Pretend chunk 1 (entry-0, entry-1) was written before the crash.
        e.topics = if id == "entry-0" || id == "entry-1" {
            vec!["topic-1".to_string()]
        } else {
            vec![]
        };
        db.upsert(e).await.unwrap();
    }
    // Base advanced for only entry-0 (crash between entry-1's add and its base checkpoint).
    let mut topic = sample("topic-1", "topic", None);
    topic.topics = vec![];
    topic.entities = vec!["entry-0".to_string()];
    db.upsert(topic).await.unwrap();

    // Re-run with the full membership, chunk size 2.
    crate::embeddings::pipeline::reconcile_topic_membership_chunked(&db, "topic-1", &members, 2)
        .await
        .unwrap();

    for id in &members {
        let topics = db.get_entry(id).await.unwrap().unwrap().topics;
        assert_eq!(
            topics.iter().filter(|t| t.as_str() == "topic-1").count(),
            1,
            "{id} ends tagged exactly once (no duplicate from the re-run)"
        );
    }
    let mut base = db.get_entry("topic-1").await.unwrap().unwrap().entities;
    base.sort();
    assert_eq!(base, members, "base converges to the full membership");
}

/// Null-vector rows must be excluded from both similarity paths: the `nearest_to`
/// ANN branch (via `vector IS NOT NULL`) and the full-scan branch (via `is_null`).
/// Also verifies lancedb 0.33 accepts `vector IS NOT NULL` on a FixedSizeList column.
#[tokio::test]
async fn null_vector_rows_excluded_from_similarity() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    db.upsert(sample(
        "embedded",
        "entry",
        Some(vec![1.0f32; EMBEDDING_DIM as usize]),
    ))
    .await
    .unwrap();
    db.upsert(sample("pending", "entry", None)).await.unwrap();

    // ANN path: nearest_to + `vector IS NOT NULL`.
    let ann = db
        .search_by_vector(vec![1.0f32; EMBEDDING_DIM as usize], None, None, None)
        .await
        .unwrap();
    let ann_ids: Vec<&str> = ann.iter().map(|e| e.entry.id.as_str()).collect();
    assert!(ann_ids.contains(&"embedded"), "embedded row should match");
    assert!(
        !ann_ids.contains(&"pending"),
        "null-vector row must be excluded from ANN search"
    );

    // Full-scan path: max_similarity set routes through search_scored.
    let scan = db
        .search_by_vector(
            vec![1.0f32; EMBEDDING_DIM as usize],
            None,
            Some(-1.0),
            Some(1.0),
        )
        .await
        .unwrap();
    let scan_ids: Vec<&str> = scan.iter().map(|e| e.entry.id.as_str()).collect();
    assert!(
        !scan_ids.contains(&"pending"),
        "null-vector row must be excluded from full scan"
    );
}

/// `ensure_indexes` builds the scalar indexes, is safe to call twice, and leaves
/// id lookups and similarity queries working.
#[tokio::test]
async fn ensure_indexes_idempotent_and_queryable() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();
    db.upsert(sample(
        "a",
        "entry",
        Some(vec![1.0f32; EMBEDDING_DIM as usize]),
    ))
    .await
    .unwrap();
    db.upsert(sample(
        "b",
        "entry",
        Some(vec![0.2f32; EMBEDDING_DIM as usize]),
    ))
    .await
    .unwrap();

    db.ensure_indexes().await.unwrap();
    // Second call is a no-op (indexes already present).
    db.ensure_indexes().await.unwrap();

    assert!(db.get_entry("a").await.unwrap().is_some());
    let related = db
        .search_by_vector(vec![1.0f32; EMBEDDING_DIM as usize], None, None, None)
        .await
        .unwrap();
    assert!(related.iter().any(|e| e.entry.id == "a"));
}

/// The list-read API returns only the right rows: list_entries excludes feed and
/// annotation rows, filter_bookmarked filters on state, list_entries_by_ids fetches a
/// set, and projected reads still carry summary + state.
#[tokio::test]
async fn read_api_filters_by_kind_and_state() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    let mut e1 = sample("e1", "entry", Some(vec![0.1f32; EMBEDDING_DIM as usize]));
    e1.color = "blue".into();
    e1.tags = vec!["bookmark".to_string()];
    e1.summary = "s1".to_string();
    let e2 = sample("e2", "entry", Some(vec![0.2f32; EMBEDDING_DIM as usize]));
    let feed = sample("feed1", "feed", Some(vec![0.3f32; EMBEDDING_DIM as usize]));
    let mut ann = sample(
        "ann1",
        "annotation",
        Some(vec![0.4f32; EMBEDDING_DIM as usize]),
    );
    ann.reference = "e1".to_string();

    db.upsert(e1).await.unwrap();
    db.upsert(e2).await.unwrap();
    db.upsert(feed).await.unwrap();
    db.upsert(ann).await.unwrap();

    let mut all: Vec<String> = db
        .list_entries()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    all.sort();
    assert_eq!(all, vec!["e1".to_string(), "e2".to_string()]);

    let bm: Vec<String> = db
        .filter_bookmarked()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(bm, vec!["e1".to_string()]);

    let by_id: Vec<String> = db
        .list_entries_by_ids(&["e2".to_string()])
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(by_id, vec!["e2".to_string()]);

    let e1_read = db.list_entries_by_ids(&["e1".to_string()]).await.unwrap();
    assert_eq!(e1_read[0].description.as_deref(), Some("s1"));
    assert!(e1_read[0].color.is_some());
}

/// `get_embedding_status` counts per kind and distinguishes embedded from pending
/// (null-vector) rows; kinds with no rows are omitted.
#[tokio::test]
async fn embedding_status_counts_by_kind_and_vector() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    db.upsert(sample(
        "e1",
        "entry",
        Some(vec![0.1f32; EMBEDDING_DIM as usize]),
    ))
    .await
    .unwrap();
    db.upsert(sample("e2", "entry", None)).await.unwrap(); // pending
    db.upsert(sample(
        "f1",
        "feed",
        Some(vec![0.2f32; EMBEDDING_DIM as usize]),
    ))
    .await
    .unwrap();

    let status = db.get_embedding_status().await.unwrap();

    let entry = status
        .iter()
        .find(|s| s.kind == "entry")
        .expect("entry kind");
    assert_eq!(entry.total, 2);
    assert_eq!(entry.embedded, 1);

    let feed = status.iter().find(|s| s.kind == "feed").expect("feed kind");
    assert_eq!(feed.total, 1);
    assert_eq!(feed.embedded, 1);

    assert!(
        status
            .iter()
            .all(|s| s.kind != "annotation" && s.kind != "topic")
    );
}

/// `add_batch` inserts many rows in one call, round-tripping mixed Some/None
/// vectors, and the null-vector rows stay excluded from similarity.
#[tokio::test]
async fn add_batch_inserts_many_rows_with_mixed_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let db = EmbeddingsDb::open(dir.path()).await.unwrap();

    db.add_batch(vec![
        sample("a", "entry", Some(vec![0.1f32; EMBEDDING_DIM as usize])),
        sample("b", "entry", None),
        sample("c", "topic", Some(vec![0.3f32; EMBEDDING_DIM as usize])),
    ])
    .await
    .unwrap();

    assert_eq!(db.count().await.unwrap(), 3);
    assert!(db.get_entry("a").await.unwrap().unwrap().vector.is_some());
    assert!(db.get_entry("b").await.unwrap().unwrap().vector.is_none());
    assert_eq!(db.get_entry("c").await.unwrap().unwrap().kind, Kind::Topic);

    let related = db
        .search_by_vector(vec![0.1f32; EMBEDDING_DIM as usize], None, None, None)
        .await
        .unwrap();
    assert!(related.iter().any(|e| e.entry.id == "a"));
    assert!(!related.iter().any(|e| e.entry.id == "b"));
}

/// A second handle is version-pinned and sees another handle's commits after `reload`.
#[tokio::test]
async fn read_handle_visibility_across_connections() {
    let dir = tempfile::tempdir().unwrap();
    let writer = EmbeddingsDb::open(dir.path()).await.unwrap();
    let reader = EmbeddingsDb::open(dir.path()).await.unwrap();

    writer
        .add_batch(vec![sample("x", "entry", None)])
        .await
        .unwrap();

    let before = reader.count().await.unwrap();
    reader.reload().await.unwrap();
    let after = reader.count().await.unwrap();

    // After reload, the reader sees the writer's commit.
    assert_eq!(after, 1, "reader must see writer's commit after reload");
    // Before reload, the reader remains pinned to its open version.
    assert_eq!(
        before, 0,
        "handle is pinned to its open-version until reloaded"
    );
}
