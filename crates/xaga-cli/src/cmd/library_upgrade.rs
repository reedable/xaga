//! Explicit legacy-library conversion to the running CLI’s major/minor version.
//! All legacy-format knowledge stays in the CLI.
//! File writes deliberately duplicate the validated atomic persistence mechanics:
//! this offline converter holds the whole-library lock and has no concurrent writer.
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use noyalib::{ParserConfig, Value, policy::DenyAnchors};
use xaga_core::{embeddings::EmbeddingsDb, fs::FileLock, library};

// Matches the application boundary; duplicated for this disposable converter.
const MAX_MD_BYTES: u64 = 12 * 1024 * 1024;
// Bound SQL expression size and cache commits at the multi-million-entry target.
const CACHE_BATCH_SIZE: usize = 1024;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Default)]
pub(super) struct UpgradeReport {
    pub converted: usize,
    pub unchanged: usize,
    pub reconciled: usize,
    pub errors: Vec<String>,
}

struct Conversion {
    rendered: Option<String>,
    is_unread: bool,
}

/// Edit only migration-owned fields. Serializing the entire generic Value tree
/// would reinterpret plain IDs such as `29688640e516` as overflowing floats.
/// The CST keeps every unrelated scalar's original bytes, including nested IDs.
fn patch_migration_fields(
    yaml: &str,
    fields: &BTreeMap<String, Value>,
    keys: &[&str],
) -> Result<String, Error> {
    // The frontmatter slice excludes its final newline. Keep one so root
    // insertions cannot join the previous scalar or block-scalar content.
    let mut document = noyalib::cst::parse_document(&format!("{yaml}\n"))?;
    for key in keys {
        if let Some(value) = fields.get(*key) {
            // JSON fragments are valid YAML and quote tag strings explicitly.
            document.insert_entry("", key, &serde_json::to_string(value)?)?;
        } else if document.get(key).is_some() {
            document.remove(key)?;
        }
    }
    let rendered = document.to_string();
    let config = ParserConfig::strict().with_policy(DenyAnchors);
    let _: BTreeMap<String, Value> = noyalib::from_str_with_config(&rendered, &config)?;
    Ok(rendered)
}

/// Existing boolean state wins over the legacy marker; read state is omission.
fn convert(raw: &str) -> Result<Conversion, Error> {
    let trimmed = raw.trim_start();
    let prefix = &raw[..raw.len() - trimmed.len()];
    let rest = trimmed
        .strip_prefix("---\n")
        .ok_or("missing opening frontmatter delimiter")?;
    let end = rest
        .find("\n---\n")
        .ok_or("missing closing frontmatter delimiter")?;
    let config = ParserConfig::strict().with_policy(DenyAnchors);
    let mut fields: BTreeMap<String, Value> = noyalib::from_str_with_config(&rest[..end], &config)?;
    let tags: Vec<String> = match fields.get("tags") {
        Some(value) => noyalib::from_value(value)?,
        None => Vec::new(),
    };
    let legacy_unread = tags.iter().any(|tag| tag == "unread");
    let existing: Option<bool> = match fields.get("is_unread") {
        Some(Value::Bool(value)) => Some(*value),
        Some(Value::Null) | None => None,
        Some(_) => return Err("is_unread must be a boolean".into()),
    };
    let is_unread = existing.unwrap_or(legacy_unread);
    if !legacy_unread && existing != Some(false) {
        return Ok(Conversion {
            rendered: None,
            is_unread,
        });
    }
    if legacy_unread {
        fields.insert(
            "tags".into(),
            noyalib::to_value(
                &tags
                    .into_iter()
                    .filter(|tag| tag != "unread")
                    .collect::<Vec<_>>(),
            )?,
        );
    }
    if is_unread {
        fields.insert("is_unread".into(), Value::Bool(true));
    } else {
        fields.remove("is_unread");
    }
    // Apply the same unsupported-field removal rule as ordinary Markdown writes.
    fields.remove("images");
    let yaml = patch_migration_fields(&rest[..end], &fields, &["tags", "is_unread", "images"])?;
    let rendered = format!(
        "{prefix}---\n{}\n---\n{}",
        yaml.trim_end_matches('\n'),
        &rest[end + 5..]
    );
    // Validate the complete result using the same strict YAML boundary before writing.
    let output = rendered.trim_start().strip_prefix("---\n").unwrap();
    let output_end = output.find("\n---\n").ok_or("invalid output delimiters")?;
    let _: BTreeMap<String, Value> = noyalib::from_str_with_config(&output[..output_end], &config)?;
    Ok(Conversion {
        rendered: Some(rendered),
        is_unread,
    })
}

fn read_capped(path: &Path) -> Result<String, Error> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_MD_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MD_BYTES {
        return Err("article exceeds the 12 MiB read limit".into());
    }
    Ok(String::from_utf8(bytes)?)
}

/// Sibling temp + fsync + rename + directory fsync, under the library lock.
fn write_atomic(path: &Path, raw: &str, _lock: &FileLock) -> Result<(), Error> {
    let parent = path.parent().ok_or("article has no parent directory")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(raw.as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

/// Update only existing cache rows, preserving vectors, memberships and all other
/// columns. Missing rows remain reconstructible by normal recovery.
async fn reconcile(
    table: &lancedb::Table,
    states: &[(String, bool)],
    _lock: &FileLock,
) -> Result<usize, Error> {
    let mut updated = 0;
    for unread in [true, false] {
        let ids: Vec<_> = states
            .iter()
            .filter(|(_, value)| *value == unread)
            .map(|(id, _)| format!("'{}'", id.replace('\'', "''")))
            .collect();
        if ids.is_empty() {
            continue;
        }
        let result = table.update()
            .only_if(format!("kind = 'entry' AND id IN ({}) AND (is_unread != {unread} OR array_contains(tags, 'unread'))", ids.join(",")))
            .column("is_unread", unread.to_string())
            .column("tags", "array_remove(tags, 'unread')")
            .execute().await?;
        updated += result.rows_updated as usize;
    }
    Ok(updated)
}

/// Transfer each reference durably before stripping its legacy marker. This
/// sequencing remains safe after any individual file or store commit is interrupted.
async fn convert_pins(dir: &Path, lock: &FileLock) -> Result<(), Error> {
    use xaga_core::pins::{PinTarget, Pins};
    let mut pins = Pins::read(dir).await?;
    for (folder, extension, kind) in [
        (library::entries_dir(dir), "md", "entry"),
        (library::topics_dir(dir), "md", "topic"),
        (library::feeds_dir(dir), "atom", "feed"),
    ] {
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().is_none_or(|ext| ext != extension) {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|id| id.to_str())
                .ok_or("non-UTF-8 target id")?
                .to_owned();
            let target = match kind {
                "entry" => PinTarget::Entry { id },
                "topic" => PinTarget::Topic { id },
                _ => PinTarget::Feed { id },
            };
            target.validate()?;
            if !fs::symlink_metadata(&path)?.file_type().is_file() {
                return Err(format!("{}: expected regular file", path.display()).into());
            }
            let converted = (|| -> Result<Option<String>, Error> {
                if kind == "feed" {
                    // Stored Atom feeds use the same read contract as runtime
                    // feed persistence; the Markdown cap does not apply to XML.
                    convert_feed_pin(&fs::read_to_string(&path)?)
                } else {
                    convert_markdown_pin(&read_capped(&path)?)
                }
            })()
            .map_err(|error| format!("{}: {error}", path.display()))?;
            if let Some(rendered) = converted {
                if !pins.contains(&target) {
                    pins.pins.push(target);
                    // The ordinary parser validates identities, duplicate membership and encoding.
                    let yaml = pins.serialize()?;
                    write_atomic(&library::pins_path(dir), &yaml, lock)?;
                }
                write_atomic(&path, &rendered, lock)?;
            }
        }
    }
    Ok(())
}

fn convert_feed_pin(raw: &str) -> Result<Option<String>, Error> {
    let mut atom = xaga_core::atom::parse_atom(raw)?;
    let before = atom.categories().len();
    atom.categories.retain(|category| {
        !(category.term() == "pinned" && category.scheme() == Some("https://xaga.app/ns/tag"))
    });
    if atom.categories().len() == before {
        return Ok(None);
    }
    Ok(Some(String::from_utf8(xaga_core::atom::atom_to_xml(
        &atom,
    )?)?))
}

fn convert_markdown_pin(raw: &str) -> Result<Option<String>, Error> {
    let trimmed = raw.trim_start();
    let prefix = &raw[..raw.len() - trimmed.len()];
    let rest = trimmed.strip_prefix("---\n").ok_or("missing frontmatter")?;
    let end = rest.find("\n---\n").ok_or("missing closing delimiter")?;
    let config = ParserConfig::strict().with_policy(DenyAnchors);
    let mut fields: BTreeMap<String, Value> = noyalib::from_str_with_config(&rest[..end], &config)?;
    let tags: Vec<String> = fields
        .get("tags")
        .map(noyalib::from_value)
        .transpose()?
        .unwrap_or_default();
    if !tags.iter().any(|tag| tag == "pinned") {
        return Ok(None);
    }
    fields.insert(
        "tags".into(),
        noyalib::to_value(
            &tags
                .into_iter()
                .filter(|tag| tag != "pinned")
                .collect::<Vec<_>>(),
        )?,
    );
    let yaml = patch_migration_fields(&rest[..end], &fields, &["tags"])?;
    Ok(Some(format!(
        "{prefix}---\n{}\n---\n{}",
        yaml.trim_end_matches('\n'),
        &rest[end + 5..]
    )))
}

pub(super) async fn upgrade_library(dir: &Path, lock: &FileLock) -> UpgradeReport {
    let mut report = UpgradeReport::default();
    let upgrade = match library::LibraryManifest::begin_upgrade(
        dir,
        crate::cmd::library::application_version(),
        lock,
    ) {
        Ok(Some(upgrade)) => upgrade,
        Ok(None) => return report,
        Err(error) => {
            report.errors.push(format!("library format: {error}"));
            return report;
        }
    };

    // Use the ordinary writer-open boundary to create an absent cache. Conversion
    // itself uses direct, narrowly-scoped cache updates owned entirely by this CLI.
    let table = async {
        let _db = EmbeddingsDb::open_writer(dir, lock).await?;
        let path = library::index_path(dir);
        let connection = lancedb::connect(path.to_str().ok_or("non-UTF-8 cache path")?)
            .execute()
            .await?;
        Ok::<_, Error>(connection.open_table("documents").execute().await?)
    }
    .await;
    let table = match table {
        Ok(table) => table,
        Err(error) => {
            report.errors.push(format!("open cache: {error}"));
            return report;
        }
    };
    if let Err(error) = convert_pins(dir, lock).await {
        report
            .errors
            .push(format!("pin conversion: {error}; rerun library upgrade"));
        return report;
    }
    let pins = match xaga_core::pins::Pins::read(dir).await {
        Ok(pins) => pins,
        Err(error) => {
            report.errors.push(format!("pin store: {error}"));
            return report;
        }
    };
    // Clear the disposable legacy projection before deriving the new membership.
    // No UI pin discovery reads this cache; retries always reconcile from pins.yaml.
    if let Err(error) = table
        .update()
        .only_if("is_pinned = true OR array_contains(tags, 'pinned')")
        .column("is_pinned", "false")
        .column("tags", "array_remove(tags, 'pinned')")
        .execute()
        .await
    {
        report.errors.push(format!("pin cache cleanup: {error}"));
    }
    for chunk in pins.pins.chunks(CACHE_BATCH_SIZE) {
        let targets: Vec<_> = chunk
            .iter()
            .filter_map(|target| {
                use xaga_core::pins::PinTarget;
                let (kind, id) = match target {
                    PinTarget::Entry { id } => ("entry", id),
                    PinTarget::Feed { id } => ("feed", id),
                    PinTarget::Topic { id } => ("topic", id),
                    PinTarget::Site { .. } => return None,
                };
                Some(format!(
                    "(kind = '{kind}' AND id = '{}')",
                    id.replace('\'', "''")
                ))
            })
            .collect();
        if targets.is_empty() {
            continue;
        }
        if let Err(error) = table
            .update()
            .only_if(targets.join(" OR "))
            .column("is_pinned", "true")
            .execute()
            .await
        {
            report
                .errors
                .push(format!("pin cache reconciliation: {error}"));
        }
    }
    let entries_dir = library::entries_dir(dir);
    let entries = match fs::read_dir(&entries_dir) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            report
                .errors
                .push(format!("{}: {error}", entries_dir.display()));
            return report;
        }
    };
    let mut states = Vec::with_capacity(CACHE_BATCH_SIZE);
    for entry in entries.into_iter().flatten() {
        let result = (|| -> Result<(), Error> {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "md") {
                return Ok(());
            }
            let process = (|| -> Result<(), Error> {
                if !entry.file_type()?.is_file() {
                    return Err("expected a regular article file".into());
                }
                let id = path
                    .file_stem()
                    .and_then(|id| id.to_str())
                    .ok_or("non-UTF-8 entry id")?;
                let conversion = convert(&read_capped(&path)?)?;
                if let Some(rendered) = conversion.rendered {
                    write_atomic(&path, &rendered, lock)?;
                    report.converted += 1;
                } else {
                    report.unchanged += 1;
                }
                states.push((id.to_owned(), conversion.is_unread));
                Ok(())
            })();
            process.map_err(|error| format!("{}: {error}", path.display()).into())
        })();
        if let Err(error) = result {
            report
                .errors
                .push(format!("file conversion: {error}; rerun library upgrade"));
        }
        if states.len() >= CACHE_BATCH_SIZE {
            reconcile_report(&table, &mut states, lock, &mut report).await;
        }
    }
    reconcile_report(&table, &mut states, lock, &mut report).await;
    // Remove reserved markers even from stale rows awaiting ordinary recovery.
    // This does not infer or change their boolean reading state.
    if let Err(error) = table
        .update()
        .only_if("array_contains(tags, 'unread')")
        .column("tags", "array_remove(tags, 'unread')")
        .execute()
        .await
    {
        report.errors.push(format!(
            "cache legacy-marker cleanup: {error}; rerun library upgrade"
        ));
    }
    if report.errors.is_empty() {
        if let Err(error) = upgrade.complete() {
            report
                .errors
                .push(format!("complete library upgrade: {error}"));
        }
    }
    report
}

async fn reconcile_report(
    table: &lancedb::Table,
    states: &mut Vec<(String, bool)>,
    lock: &FileLock,
    report: &mut UpgradeReport,
) {
    if states.is_empty() {
        return;
    }
    match reconcile(table, states, lock).await {
        Ok(count) => report.reconciled += count,
        Err(error) => report.errors.push(format!(
            "cache reconciliation for batch starting with {}: {error}; rerun library upgrade",
            states[0].0
        )),
    }
    states.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use xaga_core::{
        persistence::articles::read_article,
        types::{Entry, ListItem},
    };

    fn values(raw: &str) -> BTreeMap<String, Value> {
        let yaml = raw
            .trim_start()
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---\n")
            .unwrap()
            .0;
        noyalib::from_str(yaml).unwrap()
    }

    fn document(fields: &str) -> String {
        format!(
            "---\nsource: https://example.org/a\ntitle: Saved\ncreated: yesterday\nupdated: then\ncustom: {{nested: [one, two]}}\nannotations:\n  - id: note\n    text: [selection]\n    occurrence: [0]\n    color: yellow\n    note: Irreplaceable\n    created_at: then\n{fields}\n---\n\nBody\n\n"
        )
    }

    fn seed(dir: &Path, id: &str, fields: &str) -> std::path::PathBuf {
        let path = library::entry_path(dir, id);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, document(fields)).unwrap();
        path
    }

    async fn table(dir: &Path, lock: &FileLock) -> lancedb::Table {
        EmbeddingsDb::open_writer(dir, lock).await.unwrap();
        lancedb::connect(library::index_path(dir).to_str().unwrap())
            .execute()
            .await
            .unwrap()
            .open_table("documents")
            .execute()
            .await
            .unwrap()
    }

    #[test]
    fn conversion_preserves_state_metadata_and_exact_body_and_is_idempotent() {
        for (fields, unread, changed) in [
            ("tags: [research, unread, bookmark]", true, true),
            ("tags: [research, bookmark]", false, false),
            (
                "is_unread: true\ntags: [research, unread, bookmark]",
                true,
                true,
            ),
            (
                "is_unread: false\ntags: [research, unread, bookmark]",
                false,
                true,
            ),
            ("is_unread: true\ntags: [research, bookmark]", true, false),
            ("is_unread: false\ntags: [research, bookmark]", false, true),
        ] {
            let original = document(fields);
            let converted = convert(&original).unwrap();
            assert_eq!(converted.is_unread, unread);
            assert_eq!(converted.rendered.is_some(), changed);
            let saved = converted.rendered.as_deref().unwrap_or(&original);
            let mut expected = values(&original);
            expected.insert(
                "tags".into(),
                noyalib::to_value(&vec!["research", "bookmark"]).unwrap(),
            );
            if unread {
                expected.insert("is_unread".into(), Value::Bool(true));
            } else {
                expected.remove("is_unread");
            }
            assert_eq!(values(saved), expected);
            assert!(saved.ends_with("---\n\nBody\n\n"));
            assert!(convert(saved).unwrap().rendered.is_none());
        }
        let absent = document("custom_state: retained");
        assert!(convert(&absent).unwrap().rendered.is_none());
        let false_without_tags = convert(&document("is_unread: false"))
            .unwrap()
            .rendered
            .unwrap();
        assert!(!values(&false_without_tags).contains_key("tags"));
        assert!(!values(&false_without_tags).contains_key("is_unread"));
        let unsupported = document("tags: [unread]\nimages: {old: unsupported}");
        assert!(!values(&convert(&unsupported).unwrap().rendered.unwrap()).contains_key("images"));
    }

    #[tokio::test]
    async fn upgrade_preserves_site_pins_without_creating_site_cache_rows() {
        use xaga_core::pins::{PinTarget, Pins};
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let pins = Pins {
            pins: vec![PinTarget::Site {
                hostname: "example.com".into(),
            }],
        };
        fs::write(library::pins_path(dir.path()), pins.serialize().unwrap()).unwrap();
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!(Pins::read(dir.path()).await.unwrap(), pins);
        assert_eq!(
            table(dir.path(), &lock)
                .await
                .count_rows(None)
                .await
                .unwrap(),
            0
        );
    }

    #[test]
    fn migrations_preserve_numeric_looking_ids_and_unrelated_scalar_bytes() {
        // Real damaged reference, plus finite exponent and leading-zero IDs.
        // Compare source bytes: generic Value comparisons would already have
        // lost the original lexical ID before checking the output.
        let unrelated = r#"items:
  - type: annotation
    id: '62eeae91-660b-40d8-8980-1d447491796e'
    reference: 29688640e516
  - type: entry
    id: 12345678e001
  - type: entry
    id: 000123456789
custom:
  quoted_id: "29688640e516"
  ratio: 1.234567890123456789
  flag: true
  note: |-
    Keep this text exactly.
"#;
        for tags in [
            "[pinned, research]",
            "[unread, research]",
            "[pinned, unread, research]",
        ] {
            let raw =
                format!("---\ntitle: Example\ntags: {tags}\n{unrelated}---\n\nExact body\r\n");
            let pinned = convert_markdown_pin(&raw).unwrap();
            let after_pin = pinned.as_deref().unwrap_or(&raw);
            let reading = convert(after_pin).unwrap();
            let saved = reading.rendered.as_deref().unwrap_or(after_pin);
            assert!(
                saved.contains(unrelated),
                "unrelated fields changed: {saved}"
            );
            assert!(saved.ends_with("---\n\nExact body\r\n"));
            assert!(!saved.contains(".inf"));
            assert!(convert_markdown_pin(saved).unwrap().is_none());
            assert!(convert(saved).unwrap().rendered.is_none());
        }
    }

    #[tokio::test]
    async fn pin_conversion_retries_preserve_transferred_and_unresolved_references() {
        use xaga_core::pins::{PinTarget, Pins};
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let article = seed(
            dir.path(),
            "article",
            "tags: [pinned, research, bookmark]\nis_unread: true",
        );
        let topic = library::topic_path(dir.path(), "topic");
        fs::create_dir_all(topic.parent().unwrap()).unwrap();
        fs::write(&topic, document("tags: [pinned, research]")).unwrap();
        let feed = library::feed_path(dir.path(), "feed");
        fs::create_dir_all(feed.parent().unwrap()).unwrap();
        fs::write(&feed, r#"<feed xmlns="http://www.w3.org/2005/Atom"><id>feed</id><title>Kept title</title><updated>2026-01-01T00:00:00Z</updated><category term="pinned" scheme="https://xaga.app/ns/tag"/><category term="pinned"/><entry><id>entry</id><title>Kept entry</title><updated>2026-01-01T00:00:00Z</updated></entry></feed>"#).unwrap();
        // Simulates interruption after central membership committed but before marker removal.
        let initial = Pins {
            pins: vec![
                PinTarget::Entry {
                    id: "unresolved".into(),
                },
                PinTarget::Entry {
                    id: "article".into(),
                },
            ],
        };
        write_atomic(
            &library::pins_path(dir.path()),
            &noyalib::to_string(&initial).unwrap(),
            &lock,
        )
        .unwrap();
        convert_pins(dir.path(), &lock).await.unwrap();
        let pins = Pins::read(dir.path()).await.unwrap();
        assert_eq!(
            pins.pins,
            vec![
                PinTarget::Entry {
                    id: "unresolved".into()
                },
                PinTarget::Entry {
                    id: "article".into()
                },
                PinTarget::Topic { id: "topic".into() },
                PinTarget::Feed { id: "feed".into() }
            ]
        );
        let saved = fs::read_to_string(&article).unwrap();
        assert_eq!(
            values(&saved)["tags"],
            noyalib::to_value(&vec!["research", "bookmark"]).unwrap()
        );
        assert!(saved.ends_with("---\n\nBody\n\n"));
        assert_eq!(
            values(&saved)["annotations"],
            values(&document(""))["annotations"]
        );
        let atom = xaga_core::atom::parse_atom(&fs::read_to_string(&feed).unwrap()).unwrap();
        assert_eq!(atom.title().value, "Kept title");
        assert_eq!(atom.entries().len(), 1);
        assert_eq!(atom.categories().len(), 1); // Publisher category survives.
        let before = [article, topic, feed].map(|path| fs::read(path).unwrap());
        convert_pins(dir.path(), &lock).await.unwrap();
        assert_eq!(Pins::read(dir.path()).await.unwrap(), pins);
        assert_eq!(
            before,
            [
                library::entry_path(dir.path(), "article"),
                library::topic_path(dir.path(), "topic"),
                library::feed_path(dir.path(), "feed")
            ]
            .map(|path| fs::read(path).unwrap())
        );
    }

    #[tokio::test]
    async fn pin_conversion_reads_large_atom_and_preserves_feed_data() {
        use xaga_core::pins::{PinTarget, Pins};
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let path = library::feed_path(dir.path(), "large");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let content = "a".repeat(MAX_MD_BYTES as usize + 1024);
        let raw = format!(
            r#"<feed xmlns="http://www.w3.org/2005/Atom"><id>large</id><title>Large feed</title><updated>2026-01-01T00:00:00Z</updated><link rel="self" href="https://example.org/feed"/><category term="pinned" scheme="https://xaga.app/ns/tag"/><category term="pinned"/><entry><id>entry</id><title>Kept title</title><updated>2026-01-01T00:00:00Z</updated><content type="text">{content}</content></entry></feed>"#
        );
        assert!(raw.len() as u64 > MAX_MD_BYTES);
        fs::write(&path, &raw).unwrap();
        let mut expected = xaga_core::atom::parse_atom(&raw).unwrap();
        expected.categories.remove(0); // Only the app-local marker is removed.
        convert_pins(dir.path(), &lock).await.unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        let actual = xaga_core::atom::parse_atom(&saved).unwrap();
        assert!(
            actual == expected,
            "feed metadata, entries and content must survive conversion"
        );
        assert!(
            Pins::read(dir.path())
                .await
                .unwrap()
                .contains(&PinTarget::Feed { id: "large".into() })
        );
        convert_pins(dir.path(), &lock).await.unwrap();
        assert!(
            fs::read_to_string(&path).unwrap() == saved,
            "retry must not rewrite the converted feed"
        );
    }

    #[tokio::test]
    async fn pin_conversion_keeps_markdown_limit_and_reports_the_source_path() {
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let path = seed(dir.path(), "oversized", "tags: [pinned]");
        let mut raw = fs::read_to_string(&path).unwrap();
        raw.push_str(&"a".repeat(MAX_MD_BYTES as usize));
        fs::write(&path, &raw).unwrap();
        let error = convert_pins(dir.path(), &lock)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(path.to_str().unwrap()), "{error}");
        assert!(error.contains("12 MiB"), "{error}");
        assert!(
            fs::read_to_string(&path).unwrap() == raw,
            "failed conversion must preserve the file"
        );
        assert!(!library::pins_path(dir.path()).exists());
    }

    #[tokio::test]
    async fn pin_conversion_parse_errors_report_the_source_path() {
        for (folder, name, raw) in [
            ("feeds", "bad.atom", "<feed><broken"),
            ("entries", "bad.md", "missing frontmatter"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let lock = library::acquire_lock(dir.path()).unwrap();
            let path = dir.path().join(folder).join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, raw).unwrap();
            let error = convert_pins(dir.path(), &lock)
                .await
                .unwrap_err()
                .to_string();
            assert!(error.contains(path.to_str().unwrap()), "{error}");
            assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        }
    }

    #[test]
    fn malformed_files_and_anchors_are_refused() {
        for raw in [
            "body",
            "---\ntags: [unfinished\n---\nbody",
            "---\ncustom: &secret [a]\nother: *secret\n---\nBody",
            "---\nis_unread: invalid\n---\nBody",
            "---\ntags: not-a-list\n---\nBody",
        ] {
            assert!(convert(raw).is_err(), "{raw}");
        }
    }

    #[tokio::test]
    async fn retry_repairs_existing_cache_preserves_other_columns_and_leaves_files_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let path = seed(
            dir.path(),
            "entry",
            "is_unread: true\ntags: [research, bookmark, pinned]",
        );
        let original = fs::read_to_string(&path).unwrap();
        let article = read_article(dir.path(), "entry").await.unwrap();
        let db = EmbeddingsDb::open_writer(dir.path(), &lock).await.unwrap();
        let (tx, _) = flume::unbounded::<Entry>();
        let (annotation_tx, _) = flume::unbounded::<ListItem>();
        xaga_core::embeddings::sync_entry_row(&article, &db, &tx, &tx, &annotation_tx)
            .await
            .unwrap();
        seed(dir.path(), "read-entry", "tags: [bookmark]");
        let mut read_article = read_article(dir.path(), "read-entry").await.unwrap();
        read_article.annotations.clear();
        xaga_core::embeddings::sync_entry_row(&read_article, &db, &tx, &tx, &annotation_tx)
            .await
            .unwrap();
        let table = table(dir.path(), &lock).await;
        table
            .update()
            .only_if("id = 'read-entry'")
            .column("is_unread", "true")
            .column("tags", "['unread', 'bookmark']")
            .execute()
            .await
            .unwrap();
        table
            .update()
            .only_if("id = 'entry'")
            .column("is_unread", "false")
            .column("tags", "['research', 'unread', 'bookmark', 'pinned']")
            .column("topics", "['topic']")
            .column(
                "vector",
                format!(
                    "arrow_cast([{}], 'FixedSizeList(384, Float32)')",
                    vec!["0.1"; 384].join(",")
                ),
            )
            .execute()
            .await
            .unwrap();
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!(report.converted, 0);
        assert_eq!(report.unchanged, 2);
        assert_eq!(report.reconciled, 2);
        assert_eq!(
            values(&fs::read_to_string(&path).unwrap())["custom"],
            values(&original)["custom"]
        );
        let db = EmbeddingsDb::open_readonly(dir.path()).await.unwrap();
        let row = db.get_entry("entry").await.unwrap().unwrap();
        let read_row = db.get_entry("read-entry").await.unwrap().unwrap();
        assert!(!read_row.is_unread && read_row.is_bookmarked);
        assert_eq!(read_row.tags, vec!["bookmark"]);
        assert!(row.is_unread && row.is_bookmarked && row.is_pinned);
        assert_eq!(row.tags, vec!["research", "bookmark"]);
        assert_eq!(db.filter_pinned_entries().await.unwrap()[0].id, "entry");
        assert_eq!(row.topics, vec!["topic"]);
        assert_eq!(row.items, vec!["note"]);
        assert_eq!(row.vector, Some(vec![0.1; 384]));
        let annotation = db.get_entry("note").await.unwrap().unwrap();
        assert_eq!(annotation.title, "selection");
        assert_eq!(annotation.reference, "entry");
        assert_eq!(annotation.color, "yellow");
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!(report.reconciled, 0);
        let manifest = library::LibraryManifest::read(dir.path()).unwrap().unwrap();
        assert_eq!(
            manifest.format_version(),
            Some(crate::cmd::library::application_version())
        );
        assert_eq!(manifest.migration_target(), None);
    }

    #[tokio::test]
    async fn conversion_reports_errors_retries_and_keeps_libraries_isolated() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let good = seed(first.path(), "good", "tags: [research, unread]");
        let bad = seed(first.path(), "bad", "is_unread: invalid");
        let other = seed(second.path(), "good", "tags: [research, unread]");
        let other_before = fs::read_to_string(&other).unwrap();
        let bad_before = fs::read_to_string(&bad).unwrap();
        let lock = library::acquire_lock(first.path()).unwrap();
        assert!(library::acquire_lock(first.path()).is_none());
        let report = upgrade_library(first.path(), &lock).await;
        assert_eq!(report.converted, 1);
        assert_eq!(report.errors.len(), 1, "{report:?}");
        assert!(
            library::LibraryManifest::read(first.path())
                .unwrap()
                .unwrap()
                .migration_target()
                .is_some()
        );

        assert!(report.errors[0].contains("bad.md"));
        assert_eq!(fs::read_to_string(&bad).unwrap(), bad_before);
        assert_eq!(fs::read_to_string(&other).unwrap(), other_before);
        assert!(
            values(&fs::read_to_string(&good).unwrap()).get("is_unread")
                == Some(&Value::Bool(true))
        );
        seed(first.path(), "bad", "tags: [unread]");
        let report = upgrade_library(first.path(), &lock).await;
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!(report.converted, 1);
        assert_eq!(report.unchanged, 1);
        assert!(
            library::LibraryManifest::read(first.path())
                .unwrap()
                .unwrap()
                .matches_version(crate::cmd::library::application_version())
        );
    }

    #[tokio::test]
    async fn failed_cache_open_is_an_error_and_leaves_source_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = seed(dir.path(), "entry", "tags: [unread]");
        let original = fs::read_to_string(&path).unwrap();
        fs::write(library::index_path(dir.path()), "not a database directory").unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("cache"));
        let manifest = library::LibraryManifest::read(dir.path()).unwrap().unwrap();
        assert_eq!(manifest.format_version(), None);
        assert_eq!(
            manifest.migration_target(),
            Some(crate::cmd::library::application_version())
        );

        assert_eq!(fs::read_to_string(&path).unwrap(), original);
    }
}
