//! One-time 0.9 → 0.10 conversion. All legacy-format knowledge stays in the CLI.
//! File writes deliberately duplicate the validated atomic persistence mechanics:
//! this offline converter holds the whole-library lock and has no concurrent writer.
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use noyalib::{ParserConfig, SerializerConfig, Value, policy::DenyAnchors};
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
    let yaml =
        noyalib::to_string_with_config(&fields, &SerializerConfig::new().block_scalars(false))?;
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

pub(super) async fn upgrade_library(dir: &Path, lock: &FileLock) -> UpgradeReport {
    let mut report = UpgradeReport::default();
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
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        let db = EmbeddingsDb::open_readonly(dir.path()).await.unwrap();
        let row = db.get_entry("entry").await.unwrap().unwrap();
        let read_row = db.get_entry("read-entry").await.unwrap().unwrap();
        assert!(!read_row.is_unread && read_row.is_bookmarked);
        assert_eq!(read_row.tags, vec!["bookmark"]);
        assert!(row.is_unread && row.is_bookmarked && row.is_pinned);
        assert_eq!(row.tags, vec!["research", "bookmark", "pinned"]);
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
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
    }
}
