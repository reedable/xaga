use super::*;

fn annotation(id: &str) -> Annotation {
    Annotation {
        tags: vec![],
        id: id.to_string(),
        title: "highlighted".to_string(),
        reference: String::new(),
        text: vec!["highlighted".to_string()],
        occurrence: vec![0],
        color: "yellow".to_string(),
        content: String::new(),
        created_at: String::new(),
    }
}

fn start_writer(dir: &std::path::Path) -> LibraryWriter {
    let lock = crate::library::acquire_lock(dir).expect("test dir lock");
    LibraryWriter::start(dir.to_path_buf(), lock)
}

async fn seed_article(w: &LibraryWriter, id: &str) {
    w.save_article(
        Tier::Interactive,
        id.to_string(),
        format!("https://example.com/{id}"),
        "Title".to_string(),
        None,
        None,
        None,
        None,
        "body".to_string(),
        std::collections::HashMap::new(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn reading_state_bookmark_and_annotation_mutations_share_the_entry_lane() {
    let dir = tempfile::tempdir().unwrap();
    let writer = start_writer(dir.path());
    seed_article(&writer, "saved").await;
    let (read, bookmark, annotation) = tokio::join!(
        writer.set_entry_unread(Tier::Interactive, "saved".into(), false),
        writer.set_entry_color(Tier::Interactive, "saved".into(), Some("blue".into())),
        writer.add_annotation(Tier::Interactive, "saved".into(), annotation("note")),
    );
    read.unwrap();
    bookmark.unwrap();
    annotation.unwrap();
    let saved = articles::read_article(dir.path(), "saved").await.unwrap();
    assert!(!saved.meta.is_unread && saved.meta.color.is_some());
    assert_eq!(saved.annotations[0].id, "note");
    let raw = std::fs::read_to_string(crate::library::entry_path(dir.path(), "saved")).unwrap();
    assert!(!crate::md::test_frontmatter_values(&raw).contains_key("is_unread"));
    writer.shutdown().await;
}

#[tokio::test]
async fn concurrent_pins_share_one_lane_and_survive_restart() {
    use crate::pins::{PinTarget, Pins};
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    let mut tasks = Vec::new();
    for i in 0..32 {
        let writer = w.clone();
        tasks.push(tokio::spawn(async move {
            writer
                .set_pin(
                    PinTarget::Entry {
                        id: format!("entry{i}"),
                    },
                    true,
                )
                .await
                .unwrap();
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    w.set_pin(
        PinTarget::Entry {
            id: "entry0".into(),
        },
        true,
    )
    .await
    .unwrap();
    assert_eq!(Pins::read(dir.path()).await.unwrap().pins.len(), 32);
    w.set_pin(
        PinTarget::Entry {
            id: "entry0".into(),
        },
        false,
    )
    .await
    .unwrap();
    w.shutdown().await;
    drop(w);
    assert_eq!(Pins::read(dir.path()).await.unwrap().pins.len(), 31);
    let other = tempfile::tempdir().unwrap();
    assert!(Pins::read(other.path()).await.unwrap().pins.is_empty());
}

#[tokio::test]
async fn download_and_user_edits_preserve_persisted_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    let url = "https://example.com/preservation";
    let id = crate::util::url_hash12(url);
    w.write_feed_entry(
        Tier::Interactive,
        url.into(),
        "Feed title".into(),
        None,
        None,
        Some("Feed summary".into()),
    )
    .await
    .unwrap();
    let path = crate::library::entry_path(dir.path(), &id);
    let original = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        original.replacen("---\n", "---\ncustom: {nested: [one, two]}\n", 1),
    )
    .unwrap();
    let mut note = annotation("keep");
    note.content = "Irreplaceable note".into();
    w.add_annotation(Tier::Interactive, id.clone(), note)
        .await
        .unwrap();
    w.set_entry_color(Tier::Interactive, id.clone(), Some("blue".into()))
        .await
        .unwrap();
    // Exactly the regressed download input: new body, omitted feed metadata.
    w.save_article(
        Tier::Interactive,
        id.clone(),
        url.into(),
        "Downloaded title".into(),
        None,
        None,
        None,
        None,
        "Downloaded body".into(),
        Default::default(),
    )
    .await
    .unwrap();
    let saved = articles::read_article(dir.path(), &id).await.unwrap();
    assert_eq!(saved.meta.description.as_deref(), Some("Feed summary"));
    assert_eq!(saved.content, "Downloaded body");
    assert!(saved.meta.color.is_some());
    assert!(saved.meta.is_unread);
    assert_eq!(saved.annotations.len(), 1);
    assert_eq!(saved.annotations[0].content, "Irreplaceable note");
    let before_feed = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        crate::md::test_frontmatter_values(&before_feed)["custom"],
        noyalib::from_str::<noyalib::Value>("{nested: [one, two]}").unwrap()
    );
    // A repeated feed discovery must not replace the downloaded/user data.
    w.write_feed_entry(
        Tier::Interactive,
        url.into(),
        "Stale title".into(),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before_feed);
    w.add_annotation(Tier::Interactive, id.clone(), annotation("temporary"))
        .await
        .unwrap();
    w.remove_annotation(Tier::Interactive, id.clone(), "temporary".into())
        .await
        .unwrap();
    let saved = articles::read_article(dir.path(), &id).await.unwrap();
    assert_eq!(saved.meta.description.as_deref(), Some("Feed summary"));
    assert_eq!(saved.annotations.len(), 1);
    assert_eq!(saved.annotations[0].content, "Irreplaceable note");
    assert_eq!(
        crate::md::test_frontmatter_values(&std::fs::read_to_string(&path).unwrap())["custom"],
        crate::md::test_frontmatter_values(&before_feed)["custom"]
    );
    w.shutdown().await;
}

#[tokio::test]
async fn pdf_replacement_preserves_markdown_and_html_save_removes_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    let url = "https://example.com/document";
    let id = crate::util::url_hash12(&crate::util::normalize_url(url).unwrap());
    w.save_article(
        Tier::Interactive,
        id.clone(),
        url.into(),
        "Title".into(),
        None,
        None,
        None,
        None,
        "original body".into(),
        Default::default(),
    )
    .await
    .unwrap();
    w.add_annotation(Tier::Interactive, id.clone(), annotation("kept"))
        .await
        .unwrap();
    let before = articles::read_article(dir.path(), &id).await.unwrap();
    w.save_pdf(url.into(), Some(b"%PDF-valid".to_vec()))
        .await
        .unwrap();
    let pdf_stem = crate::util::url_hash12(&crate::util::normalize_url(url).unwrap());
    // The existing Entry is the one Markdown identity record; saving PDF
    // bytes neither changes it nor creates a competing companion.
    assert!(!crate::library::pdf_entry_path(dir.path(), &pdf_stem).exists());
    let after = articles::read_article(dir.path(), &id).await.unwrap();
    assert_eq!(before.content, after.content);
    assert_eq!(after.annotations.len(), 1);
    assert!(
        w.save_pdf(url.into(), Some(b"invalid".to_vec()))
            .await
            .is_err()
    );
    assert_eq!(
        crate::persistence::pdfs::read(dir.path(), url)
            .unwrap()
            .unwrap()
            .1,
        b"%PDF-valid"
    );
    w.save_article(
        Tier::Interactive,
        id.clone(),
        url.into(),
        "Title".into(),
        None,
        None,
        None,
        None,
        "new HTML body".into(),
        Default::default(),
    )
    .await
    .unwrap();
    assert!(
        crate::persistence::pdfs::read(dir.path(), url)
            .unwrap()
            .is_none()
    );
    assert!(!crate::library::pdf_entry_path(dir.path(), &pdf_stem).exists());
    assert_eq!(
        articles::read_article(dir.path(), &id)
            .await
            .unwrap()
            .annotations
            .len(),
        1
    );
    w.shutdown().await;
}

/// N concurrent annotation adds on one article lose nothing because the lane serializes them.
#[tokio::test]
async fn concurrent_same_file_annotations_all_survive() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    seed_article(&w, "abcdef123456").await;

    let n = 20;
    let mut handles = Vec::new();
    for i in 0..n {
        let w = w.clone();
        handles.push(tokio::spawn(async move {
            w.add_annotation(
                Tier::Interactive,
                "abcdef123456".to_string(),
                annotation(&format!("ann-{i}")),
            )
            .await
        }));
    }
    for h in handles {
        h.await.unwrap().unwrap();
    }

    let article = articles::read_article(dir.path(), "abcdef123456")
        .await
        .unwrap();
    assert_eq!(
        article.annotations.len(),
        n,
        "every concurrent annotation must survive — none clobbered"
    );
}

/// Regression: the same entry carried by two feeds (same URL → same entry file) must
/// serialize on the ENTRY lane, not race on separate feed lanes. N concurrent
/// `write_feed_entry` for one URL create the file exactly once (one `Some`, rest
/// `None`); no unconditional double-write can clobber a user's state on that entry.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_feed_entry_creates_for_same_url_create_once() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());

    let url = "https://example.com/shared-across-feeds";
    let mut handles = Vec::new();
    for _ in 0..12 {
        let w = w.clone();
        handles.push(tokio::spawn(async move {
            w.write_feed_entry(
                Tier::Regular,
                url.to_string(),
                "Title".to_string(),
                None,
                None,
                None,
            )
            .await
        }));
    }

    let mut created = 0;
    for h in handles {
        if h.await.unwrap().unwrap().is_some() {
            created += 1;
        }
    }
    assert_eq!(
        created, 1,
        "exactly one concurrent create should win; rest are no-ops"
    );
}

/// Regression: bookmarking an entry that is created concurrently (a feed carrying the
/// same URL) must still end with the bookmark color present — the existing-vs-new
/// decision is inside the entry lane, so the marking is never dropped. Races
/// `bookmark_entry` against `write_feed_entry` for the same URL; whichever lands first,
/// the entry ends up bookmarked.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bookmark_racing_feed_create_keeps_the_color() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    let url = "https://example.com/in-a-feed-and-bookmarked";

    let (wa, wb) = (w.clone(), w.clone());
    let feed = tokio::spawn(async move {
        wa.write_feed_entry(Tier::Regular, url.to_string(), "T".into(), None, None, None)
            .await
    });
    let bookmark = tokio::spawn(async move {
        wb.bookmark_entry(
            Tier::Interactive,
            url.to_string(),
            "T".into(),
            "".into(),
            None,
            None,
            None,
            None,
        )
        .await
    });
    feed.await.unwrap().unwrap();
    bookmark.await.unwrap().unwrap();

    // Regardless of ordering, the entry exists AND carries the bookmark color.
    let id = crate::util::url_hash12(&crate::util::normalize_url(url).unwrap());
    let tags = entries::read_entry_tags(dir.path(), &id).await.unwrap();
    assert!(
        entries::read_entry(dir.path(), &id)
            .await
            .unwrap()
            .color
            .is_some(),
        "bookmark color must survive a concurrent feed create; tags = {tags:?}"
    );
}

/// Writes to different files proceed independently (lanes, not a global queue).
#[tokio::test]
async fn different_files_write_in_parallel() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());

    let mut handles = Vec::new();
    for i in 0..8 {
        let w = w.clone();
        handles.push(tokio::spawn(async move {
            w.create_article(
                Tier::Regular,
                format!("https://example.com/item-{i}"),
                format!("Title {i}"),
                None,
                None,
                None,
                None,
                "body".to_string(),
                std::collections::HashMap::new(),
            )
            .await
        }));
    }
    for h in handles {
        h.await.unwrap().unwrap();
    }
}

/// Writes accepted before shutdown are durable after it — a graceful shutdown never
/// loses a committed write.
#[tokio::test]
async fn shutdown_preserves_committed_interactive_writes() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    seed_article(&w, "abcdef123456").await;

    for i in 0..5 {
        w.add_annotation(
            Tier::Interactive,
            "abcdef123456".to_string(),
            annotation(&format!("keep-{i}")),
        )
        .await
        .unwrap();
    }

    w.shutdown().await;

    let article = articles::read_article(dir.path(), "abcdef123456")
        .await
        .unwrap();
    assert_eq!(
        article.annotations.len(),
        5,
        "committed writes survive shutdown"
    );
}

/// After shutdown begins, new submits are refused synchronously with a typed error —
/// never accepted-then-abandoned (which would hang the caller). This is the exact
/// failure mode that motivated the shutdown flag + channel drain.
#[tokio::test]
async fn shutdown_refuses_new_writes_without_hanging() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    seed_article(&w, "abcdef123456").await;

    w.shutdown().await;

    let r = w
        .add_annotation(
            Tier::Interactive,
            "abcdef123456".to_string(),
            annotation("late"),
        )
        .await;
    assert!(
        matches!(r, Err(WriterError::ShuttingDown)),
        "a post-shutdown write must be refused, not stranded; got {r:?}"
    );
}

#[tokio::test]
async fn create_topic_with_member_persists_entry_and_annotation_members() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    seed_article(&w, "abcdef123456").await;

    let entry_topic = w
        .create_topic_with_member(
            Tier::Interactive,
            "Entry topic".into(),
            "Description".into(),
            Some(0.7),
            Reference::Entry {
                id: "abcdef123456".into(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        entry_topic.members.as_slice(),
        [crate::types::Reference::Entry { id }] if id == "abcdef123456"
    ));
    assert_eq!(
        topics::read_topic_entities(dir.path(), &entry_topic.id)
            .await
            .unwrap()
            .len(),
        1
    );

    w.add_annotation(
        Tier::Interactive,
        "abcdef123456".into(),
        annotation("annotation-1"),
    )
    .await
    .unwrap();
    let annotation_topic = w
        .create_topic_with_member(
            Tier::Interactive,
            "Annotation topic".into(),
            String::new(),
            None,
            Reference::Annotation {
                id: "annotation-1".into(),
                article_id: Some("abcdef123456".into()),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        annotation_topic.members.as_slice(),
        [Reference::Annotation { id, article_id: Some(article_id) }]
            if id == "annotation-1" && article_id == "abcdef123456"
    ));
    w.shutdown().await;
}

#[tokio::test]
async fn create_topic_with_missing_member_leaves_no_topic() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());

    let result = w
        .create_topic_with_member(
            Tier::Interactive,
            "Orphan".into(),
            String::new(),
            None,
            Reference::Entry {
                id: "missing".into(),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(WriterError::Entry(EntryError::NotFound))
    ));
    assert!(topics::list_topics(dir.path()).await.unwrap().is_empty());

    for unresolved in [
        Reference::Annotation {
            id: "annotation".into(),
            article_id: None,
        },
        Reference::Annotation {
            id: "annotation".into(),
            article_id: Some(String::new()),
        },
        Reference::Note { id: "note".into() },
    ] {
        let result = w
            .create_topic_with_member(
                Tier::Interactive,
                "Unresolved".into(),
                String::new(),
                None,
                unresolved,
            )
            .await;
        assert!(matches!(
            result,
            Err(WriterError::Entry(EntryError::NotFound))
        ));
        assert!(topics::list_topics(dir.path()).await.unwrap().is_empty());
    }

    seed_article(&w, "abcdef123456").await;
    let result = w
        .create_topic_with_member(
            Tier::Interactive,
            "Orphan annotation".into(),
            String::new(),
            None,
            Reference::Annotation {
                id: "missing-annotation".into(),
                article_id: Some("abcdef123456".into()),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(WriterError::Entry(EntryError::NotFound))
    ));
    assert!(topics::list_topics(dir.path()).await.unwrap().is_empty());
    w.shutdown().await;
}

/// Many writes submitted concurrently with shutdown must each resolve — Ok (drained) or ShuttingDown
/// (refused) — with no hang and no clobber; the on-disk annotation count equals the
/// number that reported Ok (accepted writes are consistent, none lost or torn).
/// Runs on a multi-thread runtime so the submits and the shutdown genuinely race.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_submit_and_shutdown_never_hangs_and_stays_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let w = start_writer(dir.path());
    seed_article(&w, "abcdef123456").await;

    let mut interactive = Vec::new();
    for i in 0..10 {
        let w = w.clone();
        interactive.push(tokio::spawn(async move {
            w.add_annotation(
                Tier::Interactive,
                "abcdef123456".to_string(),
                annotation(&format!("keep-{i}")),
            )
            .await
        }));
    }
    let mut regular = Vec::new();
    for i in 0..10 {
        let w = w.clone();
        regular.push(tokio::spawn(async move {
            w.create_article(
                Tier::Regular,
                format!("https://example.com/feed-{i}"),
                "Feed entry".to_string(),
                None,
                None,
                None,
                None,
                "body".to_string(),
                std::collections::HashMap::new(),
            )
            .await
        }));
    }

    w.shutdown().await;

    // Every result is Ok or a clean ShuttingDown — never a hang, never another error.
    let mut ok_interactive = 0usize;
    for h in interactive {
        match h.await.unwrap() {
            Ok(_) => ok_interactive += 1,
            Err(WriterError::ShuttingDown) => {}
            Err(e) => panic!("unexpected interactive error: {e}"),
        }
    }
    for h in regular {
        match h.await.unwrap() {
            Ok(_) | Err(WriterError::ShuttingDown) => {}
            Err(e) => panic!("unexpected regular error: {e}"),
        }
    }

    // The file holds one annotation per write that returned Ok — no
    // accepted write was lost, no refused write landed.
    let article = articles::read_article(dir.path(), "abcdef123456")
        .await
        .unwrap();
    assert_eq!(
        article.annotations.len(),
        ok_interactive,
        "annotation count must match the number of accepted interactive writes"
    );
}
