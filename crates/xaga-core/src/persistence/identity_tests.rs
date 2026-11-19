use super::{articles, entries, identity, topics};
use crate::{
    atom, library,
    library_writer::{LibraryWriter, Tier, WriteToken},
    md,
};
use std::{collections::HashSet, path::Path};

fn put(path: &Path, raw: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, raw).unwrap();
}
fn writer(dir: &Path) -> LibraryWriter {
    LibraryWriter::start(dir.to_owned(), library::acquire_lock(dir).unwrap())
}
fn markdown(id: Option<&str>) -> String {
    format!(
        "---\n{}source: https://example.org/article\ntitle: Original\ncustom: {{keep: [one, two]}}\nimages: {{legacy: value}}\n# retained comment\nitems: [{{type: entry, id: member}}]\nannotations: [{{id: note, text: [passage], occurrence: [0], color: blue, reference: entry, created_at: '2026-01-01'}}]\n---\n\nExact body\r\n\n",
        id.map(|id| format!("id: {id}\n")).unwrap_or_default()
    )
}
fn feed(extra: &str) -> String {
    format!(
        r#"<feed xmlns="http://www.w3.org/2005/Atom"><id>publisher:original</id><title>Original</title><updated>2026-01-01T00:00:00Z</updated>{extra}</feed>"#
    )
}

#[tokio::test]
async fn read_only_infers_missing_identity_without_touching_sources() {
    let dir = tempfile::tempdir().unwrap();
    for path in [
        library::entry_path(dir.path(), "entry"),
        library::topic_path(dir.path(), "topic"),
    ] {
        put(&path, &markdown(None));
    }
    let article = articles::read_article(dir.path(), "entry").await.unwrap();
    assert_eq!(article.meta.id, "entry");
    assert_eq!(article.annotations[0].reference, "entry");
    assert_eq!(
        topics::read_topic(dir.path(), "topic").await.unwrap().id,
        "topic"
    );
    for path in [
        library::entry_path(dir.path(), "entry"),
        library::topic_path(dir.path(), "topic"),
    ] {
        assert_eq!(std::fs::read_to_string(path).unwrap(), markdown(None));
    }
    assert!(!library::index_path(dir.path()).parent().unwrap().exists());
}

#[tokio::test]
async fn identity_repair_preserves_all_other_fields_body_and_references() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::entry_path(dir.path(), "entry");
    put(&path, &markdown(None));
    let writer = writer(dir.path());
    let article = articles::read_article_with_writer(dir.path(), "entry", Some(&writer))
        .await
        .unwrap();
    assert_eq!(article.meta.id, "entry");
    assert_eq!(article.annotations[0].reference, "entry");
    let saved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        saved,
        markdown(None).replacen("---\n", "---\nid: \"entry\"\n", 1)
    );
    let before = saved.clone();
    articles::read_article_with_writer(dir.path(), "entry", Some(&writer))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    writer.shutdown().await;
}

#[tokio::test]
async fn matching_identity_is_retained_and_conflicts_block_reads_mutations_and_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let writer = writer(dir.path());
    let path = library::entry_path(dir.path(), "entry");
    put(&path, &markdown(Some("entry")));
    assert_eq!(
        entries::read_entry(dir.path(), "entry").await.unwrap().id,
        "entry"
    );
    let conflicting = markdown(Some("other"));
    put(&path, &conflicting);
    assert!(
        articles::read_article_with_writer(dir.path(), "entry", Some(&writer))
            .await
            .is_err()
    );
    assert_eq!(
        entries::read_entry(dir.path(), "entry")
            .await
            .unwrap_err()
            .code(),
        "entry:malformed"
    );
    assert!(entries::read_entry_tags(dir.path(), "entry").await.is_err());
    assert!(
        writer
            .set_entry_unread(Tier::Interactive, "entry".into(), true)
            .await
            .is_err()
    );
    assert!(
        writer
            .set_entry_color(Tier::Interactive, "entry".into(), Some("blue".into()))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), conflicting);
    let path = library::topic_path(dir.path(), "topic");
    put(&path, &conflicting);
    assert_eq!(
        topics::read_topic(dir.path(), "topic")
            .await
            .unwrap_err()
            .code(),
        "topic:malformed"
    );
    assert!(topics::read_topic(dir.path(), "topic").await.is_err());
    assert!(
        topics::read_topic_entities(dir.path(), "topic")
            .await
            .is_err()
    );
    assert!(
        writer
            .save_topic(
                Tier::Interactive,
                "topic".into(),
                "Changed".into(),
                "".into(),
                None,
                vec![]
            )
            .await
            .is_err()
    );
    assert!(
        writer
            .delete_topic(Tier::Interactive, "topic".into())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), conflicting);
    writer.shutdown().await;
}

#[test]
fn present_malformed_markdown_identity_is_not_treated_as_missing() {
    for id in ["null", "''", "[]", "{}", "true", "../escape", "123"] {
        assert!(
            md::parse_document(&markdown(Some(id))).is_err(),
            "accepted {id}"
        );
    }
    assert_eq!(
        md::parse_document(&markdown(Some("'123'")))
            .unwrap()
            .0
            .id
            .as_deref(),
        Some("123")
    );
}

#[tokio::test]
async fn repair_failure_retains_inferred_identity_and_never_claims_a_saved_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::entry_path(dir.path(), "entry");
    put(&path, &markdown(None));
    let writer = writer(dir.path());
    writer.shutdown().await;
    let article = articles::read_article_with_writer(dir.path(), "entry", Some(&writer))
        .await
        .unwrap();
    assert_eq!(article.meta.id, "entry");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), markdown(None));
    assert!(
        writer
            .repair_identity(path, identity::Record::Entry)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn concurrent_reads_and_writes_preserve_saved_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::entry_path(dir.path(), "entry");
    put(&path, &markdown(None));
    let writer = writer(dir.path());
    let (read, change, second_read) = tokio::join!(
        articles::read_article_with_writer(dir.path(), "entry", Some(&writer)),
        writer.set_entry_unread(Tier::Interactive, "entry".into(), true),
        articles::read_article_with_writer(dir.path(), "entry", Some(&writer)),
    );
    assert!(read.is_ok() && change.is_ok() && second_read.is_ok());
    let article = articles::read_article(dir.path(), "entry").await.unwrap();
    assert!(article.meta.is_unread);
    assert_eq!(article.annotations[0].id, "note");
    assert_eq!(
        md::parse_document(&std::fs::read_to_string(path).unwrap())
            .unwrap()
            .0
            .id
            .as_deref(),
        Some("entry")
    );
    writer.shutdown().await;
}

#[test]
fn atom_identity_resolves_namespace_uri_and_preserves_publisher_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::feed_path(dir.path(), "local");
    for extra in [
        format!(
            r#"<xaga:id xmlns:xaga="{}">local</xaga:id>"#,
            atom::XAGA_NAMESPACE
        ),
        format!(
            r#"<alternate:id xmlns:alternate="{}">local</alternate:id>"#,
            atom::XAGA_NAMESPACE
        ),
        r#"<encoded:id xmlns:encoded="urn:xaga:&#97;tom">local</encoded:id>"#.into(),
    ] {
        let mut atom = atom::parse_atom(&feed(&extra)).unwrap();
        assert_eq!(atom::resolve_local_id(&atom, &path).unwrap(), "local");
        assert!(!atom::ensure_local_id(&mut atom, &path).unwrap());
        let xml = String::from_utf8(atom::atom_to_xml(&atom).unwrap()).unwrap();
        let roundtrip = atom::parse_atom(&xml).unwrap();
        assert_eq!(roundtrip.id(), "publisher:original");
        assert_eq!(atom::resolve_local_id(&roundtrip, &path).unwrap(), "local");
    }
    for extra in [
        format!(r#"<x:id xmlns:x="{}"/>"#, atom::XAGA_NAMESPACE),
        format!(
            r#"<x:id xmlns:x="{}"><nested/></x:id>"#,
            atom::XAGA_NAMESPACE
        ),
        format!(
            r#"<x:id xmlns:x="{}">local</x:id><y:id xmlns:y="{}">local</y:id>"#,
            atom::XAGA_NAMESPACE,
            atom::XAGA_NAMESPACE
        ),
    ] {
        assert!(atom::parse_atom(&feed(&extra)).is_err());
    }
    let atom = atom::parse_atom(&feed(
        r#"<xaga:id xmlns:xaga="urn:foreign">foreign</xaga:id>"#,
    ))
    .unwrap();
    assert_eq!(atom::resolve_local_id(&atom, &path).unwrap(), "local");
}

#[tokio::test]
async fn atom_writes_repair_missing_local_identity_and_reject_stale_conflicting_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::feed_path(dir.path(), "local");
    put(&path, &feed(""));
    let writer = writer(dir.path());
    let empty = HashSet::new();
    let feeds = super::feeds::list_feeds(dir.path(), &empty, &empty)
        .await
        .unwrap();
    assert_eq!(feeds[0].id, "local");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), feed(""));
    writer
        .rename_feed(
            Tier::Interactive,
            "local".into(),
            "Renamed".into(),
            empty.clone(),
            empty.clone(),
        )
        .await
        .unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    let atom = atom::parse_atom(&raw).unwrap();
    assert_eq!(atom.id(), "publisher:original");
    assert_eq!(atom::resolve_local_id(&atom, &path).unwrap(), "local");
    let conflict = feed(&format!(
        r#"<x:id xmlns:x="{}">other</x:id>"#,
        atom::XAGA_NAMESPACE
    ));
    put(&path, &conflict);
    assert!(
        writer
            .write_feed_atom(Tier::Regular, "local".into(), atom, true)
            .await
            .is_err()
    );
    assert!(
        writer
            .remove_feed(Tier::Interactive, "local".into())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), conflict);
    writer.shutdown().await;
}

#[tokio::test]
async fn atom_discovery_repairs_only_when_given_a_writer() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::feed_path(dir.path(), "local");
    put(&path, &feed(""));
    let empty = HashSet::new();
    super::feeds::list_feeds(dir.path(), &empty, &empty)
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), feed(""));
    let writer = writer(dir.path());
    super::feeds::list_feeds_with_writer(dir.path(), &empty, &empty, Some(&writer))
        .await
        .unwrap();
    let mut atom = atom::parse_atom(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(!atom::ensure_local_id(&mut atom, &path).unwrap());
    writer.shutdown().await;
}

#[tokio::test]
async fn pdf_companions_share_identity_validation_and_canonical_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let id = crate::util::url_hash12("https://example.org/article");
    let path = library::pdf_entry_path(dir.path(), &id);
    put(
        &path,
        "---\nsource: https://example.org/article\ntitle: PDF\n---\n",
    );
    assert_eq!(
        entries::read_entry(dir.path(), &id).await.unwrap().title,
        "PDF"
    );
    let writer = writer(dir.path());
    articles::read_article_with_writer(dir.path(), &id, Some(&writer))
        .await
        .unwrap();
    assert_eq!(
        super::pdfs::read_source_entry(dir.path(), &id)
            .unwrap()
            .unwrap()
            .id
            .as_deref(),
        Some(id.as_str())
    );
    let canonical = library::entry_path(dir.path(), &id);
    put(&canonical, &markdown(Some("other")));
    assert!(entries::read_entry(dir.path(), &id).await.is_err());
    let mut found = vec![];
    let outcome = super::walk::for_each_entry(
        dir.path(),
        &md::md_to_entry,
        |entry| found.push(entry),
        None,
    )
    .await;
    assert!(found.is_empty());
    assert_eq!(outcome, crate::fs::WalkOutcome::Unreadable);
    writer.shutdown().await;
}

#[tokio::test]
async fn direct_identity_repair_reports_io_failure_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::entry_path(dir.path(), "entry");
    std::fs::create_dir_all(&path).unwrap();
    assert!(
        super::documents::repair_identity(&path, WriteToken::for_test())
            .await
            .is_err()
    );
    assert!(path.is_dir());
}

#[tokio::test]
async fn namespace_shadowing_cannot_reclassify_foreign_identity_during_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::feed_path(dir.path(), "local");
    let xml = feed(&format!(
        r#"<a:id xmlns:a="{}">local</a:id><a:id xmlns:a="urn:foreign">foreign</a:id>"#,
        atom::XAGA_NAMESPACE
    ));
    let atom = atom::parse_atom(&xml).unwrap();
    assert_eq!(atom::resolve_local_id(&atom, &path).unwrap(), "local");
    let serialized = String::from_utf8(atom::atom_to_xml(&atom).unwrap()).unwrap();
    let roundtrip = atom::parse_atom(&serialized).unwrap();
    assert_eq!(atom::resolve_local_id(&roundtrip, &path).unwrap(), "local");
    assert!(
        roundtrip
            .namespaces
            .values()
            .any(|uri| uri == "urn:foreign")
    );
    assert!(
        roundtrip
            .extensions
            .values()
            .flat_map(|fields| fields.values().flatten())
            .any(|field| field.value.as_deref() == Some("foreign"))
    );
}

#[tokio::test]
async fn topic_discovery_and_flow_mapping_repair_retain_other_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::topic_path(dir.path(), "123");
    let original = "---\n{title: Flow, images: {old: value}, custom: [one, two]}\n---\n\nBody\n\n";
    put(&path, original);
    let writer = writer(dir.path());
    let topics = topics::list_topics_with_writer(dir.path(), Some(&writer))
        .await
        .unwrap();
    assert_eq!(topics[0].id, "123");
    let raw = std::fs::read_to_string(&path).unwrap();
    let mut expected = md::test_frontmatter_values(original);
    expected.insert("id".into(), noyalib::Value::String("123".into()));
    assert_eq!(md::test_frontmatter_values(&raw), expected);
    assert!(raw.ends_with("---\n\nBody\n\n"));
    writer.shutdown().await;
}

#[cfg(unix)]
#[tokio::test]
async fn failed_disk_repair_still_allows_inferred_identity_read() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = library::entry_path(dir.path(), "entry");
    put(&path, &markdown(None));
    let writer = writer(dir.path());
    let parent = path.parent().unwrap();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
    let read = articles::read_article_with_writer(dir.path(), "entry", Some(&writer)).await;
    let repair = writer
        .repair_identity(path.clone(), identity::Record::Entry)
        .await;
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(read.unwrap().meta.id, "entry");
    assert!(repair.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), markdown(None));
    writer.shutdown().await;
}

#[tokio::test]
async fn refresh_repairs_missing_local_id_preserving_owned_fields_and_publisher_id() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let incoming = feed(&format!(
        r#"<remote:id xmlns:remote="{}">attacker</remote:id>"#,
        atom::XAGA_NAMESPACE
    ))
    .replace("publisher:original", "publisher:updated");
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0u8; 4096];
        socket.read(&mut request).unwrap();
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/atom+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", incoming.len(), incoming).unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let path = library::feed_path(dir.path(), "local");
    put(
        &path,
        &feed(&format!(
            r#"<link rel="self" href="http://{address}/"/><owned:setting xmlns:owned="{}">retained</owned:setting>"#,
            atom::XAGA_NAMESPACE
        )),
    );
    let writer = writer(dir.path());
    let results = super::feeds::refresh_all_feeds(
        &reqwest::Client::new(),
        dir.path(),
        &writer,
        &HashSet::new(),
        &HashSet::new(),
        2,
        65536,
    )
    .await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].2.as_ref().unwrap().0.id, "local");
    let mut saved = atom::parse_atom(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved.id(), "publisher:updated");
    assert!(!atom::ensure_local_id(&mut saved, &path).unwrap());
    assert!(
        saved
            .extensions
            .values()
            .filter_map(|fields| fields.get("setting"))
            .flatten()
            .any(|field| field.value.as_deref() == Some("retained"))
    );
    writer.shutdown().await;
    server.join().unwrap();
}

#[tokio::test]
async fn html_save_retains_selected_companion_identity_and_later_pdf_preserves_article() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://example.org/article";
    let id = crate::util::url_hash12(url);
    let writer = writer(dir.path());
    writer
        .save_pdf(url.into(), Some(b"%PDF-first".to_vec()))
        .await
        .unwrap();
    let mut annotation = md::parse_document(&markdown(None))
        .unwrap()
        .0
        .annotations
        .remove(0);
    annotation.reference = id.clone();
    writer
        .add_annotation(Tier::Interactive, id.clone(), annotation)
        .await
        .unwrap();
    let path = library::pdf_entry_path(dir.path(), &id);
    let article = writer
        .save_article(
            Tier::Interactive,
            id.clone(),
            url.into(),
            "HTML".into(),
            None,
            None,
            None,
            None,
            "Saved HTML body".into(),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(article.meta.id, id);
    assert!(path.exists());
    assert!(!library::pdf_path(dir.path(), &id).exists());
    assert!(!library::entry_path(dir.path(), &id).exists());
    let before = std::fs::read_to_string(&path).unwrap();
    writer
        .save_pdf(url.into(), Some(b"%PDF-second".to_vec()))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    assert_eq!(
        super::pdfs::read_with_writer(dir.path(), url, Some(&writer))
            .await
            .unwrap()
            .unwrap()
            .1,
        b"%PDF-second"
    );
    let reopened = articles::read_article(dir.path(), &id).await.unwrap();
    assert_eq!(reopened.content, "Saved HTML body");
    assert_eq!(reopened.annotations[0].id, "note");
    assert_eq!(reopened.annotations[0].reference, id);
    assert_eq!(reopened.meta.id, id);
    writer.shutdown().await;
}

#[tokio::test]
async fn identity_repair_cannot_use_another_librarys_writer() {
    let owned = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let path = library::entry_path(other.path(), "entry");
    put(&path, &markdown(None));
    let writer = writer(owned.path());
    assert!(
        writer
            .repair_identity(path.clone(), identity::Record::Entry)
            .await
            .is_err()
    );
    let article = articles::read_article_with_writer(other.path(), "entry", Some(&writer))
        .await
        .unwrap();
    assert_eq!(article.meta.id, "entry");
    assert_eq!(std::fs::read_to_string(path).unwrap(), markdown(None));
    writer.shutdown().await;
}

#[tokio::test]
async fn url_lookup_and_assignment_reads_repair_identity_without_changing_assignments() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://example.org/article";
    let id = crate::util::url_hash12(url);
    let path = library::entry_path(dir.path(), &id);
    let writer = writer(dir.path());
    put(&path, &markdown(None));
    assert_eq!(
        articles::find_article_with_writer(dir.path(), url, Some(&writer))
            .await
            .unwrap()
            .unwrap()
            .meta
            .id,
        id
    );
    assert_eq!(
        md::parse_document(&std::fs::read_to_string(&path).unwrap())
            .unwrap()
            .0
            .id
            .as_deref(),
        Some(id.as_str())
    );
    put(&path, &markdown(None));
    assert!(
        entries::read_entry_tags_with_writer(dir.path(), &id, Some(&writer))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        md::parse_document(&std::fs::read_to_string(&path).unwrap())
            .unwrap()
            .0
            .id
            .as_deref(),
        Some(id.as_str())
    );
    put(&path, &markdown(None));
    assert!(
        articles::read_annotation_tags_with_writer(dir.path(), &id, "note", Some(&writer))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        md::parse_document(&std::fs::read_to_string(&path).unwrap())
            .unwrap()
            .0
            .id
            .as_deref(),
        Some(id.as_str())
    );
    writer.shutdown().await;
}

#[tokio::test]
async fn identity_only_serializer_fallback_preserves_unsupported_and_explicit_empty_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::entry_path(dir.path(), "entry");
    let raw = "---\n# Before a flow mapping\n{title: Flow, images: {old: kept}, is_unread: false, color: null}\n---\n\nBody\r\n\n";
    put(&path, raw);
    let writer = writer(dir.path());
    articles::read_article_with_writer(dir.path(), "entry", Some(&writer))
        .await
        .unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    let mut expected = md::test_frontmatter_values(raw);
    expected.insert("id".into(), noyalib::Value::String("entry".into()));
    assert_eq!(md::test_frontmatter_values(&saved), expected);
    assert!(saved.ends_with("---\n\nBody\r\n\n"));
    writer.shutdown().await;
}

#[test]
fn namespace_attribute_values_roundtrip_without_changing_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let path = library::feed_path(dir.path(), "local");
    let xml = feed(&format!(
        r#"<x:id xmlns:x="{}">local</x:id><foreign:field xmlns:foreign="urn:foreign?a=1&amp;b=&quot;quoted&quot;">kept</foreign:field>"#,
        atom::XAGA_NAMESPACE
    ));
    let atom = atom::parse_atom(&xml).unwrap();
    let uri = r#"urn:foreign?a=1&b="quoted""#;
    assert!(
        atom.namespaces.values().any(|value| value == uri),
        "namespaces: {:?}",
        atom.namespaces
    );
    let xml = String::from_utf8(atom::atom_to_xml(&atom).unwrap()).unwrap();
    let roundtrip = atom::parse_atom(&xml).unwrap();
    assert!(roundtrip.namespaces.values().any(|value| value == uri));
    assert_eq!(atom::resolve_local_id(&roundtrip, &path).unwrap(), "local");
}

#[test]
fn saved_atom_identity_retains_exact_characters_in_text_and_cdata() {
    let dir = tempfile::tempdir().unwrap();
    let id = " local&identity ";
    let path = library::feed_path(dir.path(), id);
    for text in [" local&amp;identity ", "<![CDATA[ local&identity ]]>"] {
        let xml = feed(&format!(
            r#"<x:id xmlns:x="{}">{text}</x:id>"#,
            atom::XAGA_NAMESPACE
        ));
        let atom = atom::parse_atom(&xml).unwrap();
        assert_eq!(atom::resolve_local_id(&atom, &path).unwrap(), id);
        let xml = String::from_utf8(atom::atom_to_xml(&atom).unwrap()).unwrap();
        assert_eq!(
            atom::resolve_local_id(&atom::parse_atom(&xml).unwrap(), &path).unwrap(),
            id
        );
    }
    let nested = feed(&format!(
        r#"<x:id xmlns:x="{}"><nested/>local</x:id>"#,
        atom::XAGA_NAMESPACE
    ));
    assert!(atom::parse_atom(&nested).is_err());
}
