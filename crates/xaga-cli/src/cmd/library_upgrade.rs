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
// Match the service's periodic maintenance pattern, counting CLI batch operations.
const MAINTENANCE_BATCHES: usize = 100;

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
    color: Option<String>,
    tags: Vec<String>,
}

/// Significant CST tokens with byte ranges. Scalar tokens can include trailing
/// whitespace; leave that trivia and sibling comments outside migration edits.
fn migration_tokens(
    node: &noyalib::cst::GreenNode,
    source: &str,
    offset: usize,
    tokens: &mut Vec<(noyalib::cst::SyntaxKind, usize, usize)>,
) {
    use noyalib::cst::{GreenChild, SyntaxKind};
    let mut cursor = offset;
    for child in node.children() {
        match child {
            GreenChild::Node(node) => migration_tokens(node, source, cursor, tokens),
            GreenChild::Token { kind, .. }
                if !matches!(
                    kind,
                    SyntaxKind::Whitespace | SyntaxKind::Newline | SyntaxKind::Comment
                ) =>
            {
                let end = cursor + source[cursor..cursor + child.text_len()].trim_end().len();
                tokens.push((*kind, cursor, end));
            }
            _ => {}
        }
        cursor += child.text_len();
    }
}

/// Edit only migration-owned fields, retaining every unrelated source byte.
/// noyalib 0.0.15's path-shaped edits truncate indentless sequences and cannot
/// remove multiline values. Use typed parser spans and CST token boundaries,
/// and fully parse each edit before the next operation can access a lazy cache.
fn patch_migration_fields(
    yaml: &str,
    fields: &BTreeMap<String, Value>,
    keys: &[&str],
) -> Result<String, Error> {
    use noyalib::cst::SyntaxKind;
    let config = ParserConfig::strict().with_policy(DenyAnchors);
    let mut rendered = format!("{yaml}\n");
    for key in keys {
        let document = noyalib::cst::parse_document(&rendered)?;
        let spans: BTreeMap<String, noyalib::Spanned<Value>> =
            noyalib::from_str_with_config(&rendered, &config)?;
        let mut tokens = Vec::new();
        migration_tokens(document.syntax(), &rendered, 0, &mut tokens);
        if let Some(span) = spans.get(*key) {
            let start = span.start.index();
            let end = tokens
                .iter()
                .filter(|(_, token_start, token_end)| {
                    *token_start >= start && *token_end <= span.end.index()
                })
                .map(|(_, _, end)| *end)
                .last()
                .unwrap_or(start);
            if let Some(value) = fields.get(*key) {
                let fragment = serde_json::to_string(value)?;
                let on_own_line = rendered[..start]
                    .rsplit('\n')
                    .next()
                    .is_some_and(|prefix| prefix.trim().is_empty());
                let replacement = if on_own_line {
                    format!("  {fragment}")
                } else {
                    fragment
                };
                rendered.replace_range(start..end, &replacement);
            } else {
                let colon = tokens
                    .iter()
                    .rposition(|(kind, at, _)| *kind == SyntaxKind::ColonIndicator && *at < start)
                    .ok_or("migration field has no key separator")?;
                let (_, mut entry_start, key_end) = *tokens
                    .get(colon.wrapping_sub(1))
                    .ok_or("migration field has no key")?;
                let stored_key: String =
                    noyalib::from_str_with_config(&rendered[entry_start..key_end], &config)?;
                if stored_key != *key {
                    return Err("migration key span mismatch".into());
                }
                let mut entry_end = end;
                // Flow mappings need one adjoining comma removed with the entry.
                if let Some((SyntaxKind::Comma, _, comma_end)) =
                    tokens.iter().find(|(_, at, _)| *at >= end)
                {
                    entry_end = *comma_end;
                } else if let Some((SyntaxKind::Comma, comma_start, _)) =
                    tokens.iter().rev().find(|(_, _, at)| *at <= entry_start)
                {
                    entry_start = *comma_start;
                }
                rendered.replace_range(entry_start..entry_end, "");
                if spans.len() == 1
                    && tokens
                        .first()
                        .is_none_or(|(kind, _, _)| *kind != SyntaxKind::OpenBrace)
                {
                    rendered.push_str("{}\n");
                }
            }
        } else if let Some(value) = fields.get(*key) {
            let fragment = serde_json::to_string(value)?;
            if matches!(tokens.first(), Some((SyntaxKind::OpenBrace, _, _))) {
                let close = tokens
                    .iter()
                    .rposition(|(kind, _, _)| *kind == SyntaxKind::CloseBrace)
                    .ok_or("root mapping has no closing brace")?;
                let comma = if spans.is_empty()
                    || (close > 0 && tokens[close - 1].0 == SyntaxKind::Comma)
                {
                    ""
                } else {
                    ","
                };
                rendered.insert_str(
                    tokens[close].1,
                    &format!("{comma}{}: {fragment}", serde_json::to_string(key)?),
                );
            } else {
                let mut document = document;
                document.insert_entry("", key, &fragment)?;
                rendered = document.to_string();
            }
        }
        let _: BTreeMap<String, Value> = noyalib::from_str_with_config(&rendered, &config)?;
    }
    Ok(rendered)
}

/// Existing boolean state wins over the legacy marker; read state is omission.
#[cfg(test)]
fn convert(raw: &str) -> Result<Conversion, Error> {
    convert_legacy(raw, true)
}

fn convert_legacy(raw: &str, convert_reading_state: bool) -> Result<Conversion, Error> {
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
    let legacy_unread = convert_reading_state && tags.iter().any(|tag| tag == "unread");
    let legacy_bookmark = tags.iter().any(|tag| tag == "bookmark");
    let existing_color = match fields.get("color") {
        Some(Value::String(value)) if xaga_core::types::COLOR_NAMES.contains(&value.as_str()) => {
            Some(value.clone())
        }
        Some(Value::Null) | None => None,
        Some(_) => return Err("color must be an annotation palette identifier".into()),
    };
    let color = existing_color.or_else(|| legacy_bookmark.then(|| "blue".into()));
    let existing: Option<bool> = match fields.get("is_unread") {
        Some(Value::Bool(value)) => Some(*value),
        Some(Value::Null) | None => None,
        Some(_) => return Err("is_unread must be a boolean".into()),
    };
    let is_unread = existing.unwrap_or(legacy_unread);
    if !legacy_unread
        && !legacy_bookmark
        && existing != Some(false)
        && fields.get("color") != Some(&Value::Null)
    {
        return Ok(Conversion {
            rendered: None,
            is_unread,
            color,
            tags,
        });
    }
    if legacy_unread || legacy_bookmark {
        fields.insert(
            "tags".into(),
            noyalib::to_value(
                &tags
                    .iter()
                    .filter(|tag| {
                        (!convert_reading_state || tag.as_str() != "unread")
                            && tag.as_str() != "bookmark"
                    })
                    .collect::<Vec<_>>(),
            )?,
        );
    }
    if is_unread {
        fields.insert("is_unread".into(), Value::Bool(true));
    } else {
        fields.remove("is_unread");
    }
    if let Some(value) = &color {
        fields.insert("color".into(), Value::String(value.clone()));
    } else {
        fields.remove("color");
    }
    // Apply the same unsupported-field removal rule as ordinary Markdown writes.
    fields.remove("images");
    let yaml = patch_migration_fields(
        &rest[..end],
        &fields,
        &["tags", "is_unread", "color", "images"],
    )?;
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
        color,
        tags: tags
            .into_iter()
            .filter(|t| (!convert_reading_state || t != "unread") && t != "bookmark")
            .collect(),
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
type EntryState = (String, bool, Option<String>, Vec<String>);

async fn reconcile(
    table: &lancedb::Table,
    states: &[EntryState],
    registry: &xaga_core::tags::Tags,
    _lock: &FileLock,
) -> Result<usize, Error> {
    use arrow_array::builder::{ListBuilder, StringBuilder};
    use arrow_array::{
        ArrayRef, BooleanArray, RecordBatch, RecordBatchIterator, RecordBatchReader, StringArray,
    };
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    if states.is_empty() {
        return Ok(0);
    }
    let resolver = registry.resolver();
    let mut resolved = Vec::with_capacity(states.len());
    for (id, unread, color, names) in states {
        if color
            .as_deref()
            .is_some_and(|value| !xaga_core::types::COLOR_NAMES.contains(&value))
        {
            return Err("color must be an annotation palette identifier".into());
        }
        resolved.push((
            id.as_str(),
            *unread,
            color.as_deref().unwrap_or_default(),
            resolver.resolve(names)?,
        ));
    }
    // Partial-schema merge updates only the supplied columns, with inserts and
    // deletes disabled. Diverse tag sets still commit once per bounded batch.
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("is_unread", DataType::Boolean, false),
        Field::new("color", DataType::Utf8, false),
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        ),
    ]));
    let mut tags = ListBuilder::new(StringBuilder::new());
    for (_, _, _, ids) in &resolved {
        for id in ids {
            tags.values().append_value(id);
        }
        tags.append(true);
    }
    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(
            resolved.iter().map(|row| row.0).collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from(vec!["entry"; resolved.len()])),
        Arc::new(BooleanArray::from(
            resolved.iter().map(|row| row.1).collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from(
            resolved.iter().map(|row| row.2).collect::<Vec<_>>(),
        )),
        Arc::new(tags.finish()),
    ];
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    let reader: Box<dyn RecordBatchReader + Send> =
        Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
    let mut merge = table.merge_insert(&["id", "kind"]);
    merge.when_matched_update_all(None);
    let result = merge.execute(reader).await?;
    Ok(result.num_updated_rows as usize)
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
    let source_version = match library::LibraryManifest::read(dir) {
        Ok(manifest) => manifest.and_then(|m| m.format_version()),
        Err(e) => {
            report.errors.push(format!("library format: {e}"));
            return report;
        }
    };
    let legacy = source_version.is_none_or(|v| v < library::LibraryVersion::new(0, 13));
    let legacy_reading = source_version.is_none_or(|v| v < library::LibraryVersion::new(0, 10));
    let legacy_pins = source_version.is_none_or(|v| v < library::LibraryVersion::new(0, 11));
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
        let db = EmbeddingsDb::open_writer(dir, lock).await?;
        let path = library::index_path(dir);
        let connection = lancedb::connect(path.to_str().ok_or("non-UTF-8 cache path")?)
            .execute()
            .await?;
        Ok::<_, Error>((db, connection.open_table("documents").execute().await?))
    }
    .await;
    let (db, table) = match table {
        Ok(pair) => pair,
        Err(error) => {
            report.errors.push(format!("open cache: {error}"));
            return report;
        }
    };
    let remove_legacy_column = async {
        if table
            .schema()
            .await?
            .field_with_name("is_bookmarked")
            .is_ok()
        {
            for index in table.list_indices().await? {
                if index.columns.iter().any(|column| column == "is_bookmarked") {
                    table.drop_index(&index.name).await?;
                }
            }
            table.drop_columns(&["is_bookmarked"]).await?;
        }
        Ok::<_, Error>(())
    }
    .await;
    if let Err(error) = remove_legacy_column {
        report
            .errors
            .push(format!("bookmark schema conversion: {error}"));
        return report;
    }
    if let Err(error) = if legacy_pins {
        convert_pins(dir, lock).await
    } else {
        Ok(())
    } {
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
        .only_if(if legacy_pins {
            "is_pinned = true OR array_contains(tags, 'pinned')"
        } else {
            "is_pinned = true"
        })
        .column("is_pinned", "false")
        .column(
            "tags",
            if legacy_pins {
                "array_remove(tags, 'pinned')"
            } else {
                "tags"
            },
        )
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
    let mut registry = match xaga_core::tags::Tags::read(dir).await {
        Ok(registry) => registry,
        Err(e) => {
            report.errors.push(format!("tag store: {e}"));
            return report;
        }
    };
    if let Err(error) = maintain_cache(&db, &table).await {
        report
            .errors
            .push(format!("cache maintenance: {error}; rerun library upgrade"));
        return report;
    }
    let mut batches = 0usize;
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
                let raw = read_capped(&path)?;
                let conversion = if legacy {
                    convert_legacy(&raw, legacy_reading)?
                } else {
                    let (fm, _) = xaga_core::md::parse_document(&raw)?;
                    Conversion {
                        rendered: None,
                        is_unread: fm.is_unread.unwrap_or(false),
                        color: fm.color,
                        tags: fm.tags,
                    }
                };
                if let Some(rendered) = conversion.rendered {
                    write_atomic(&path, &rendered, lock)?;
                    report.converted += 1;
                } else {
                    report.unchanged += 1;
                }
                states.push((
                    id.to_owned(),
                    conversion.is_unread,
                    conversion.color,
                    conversion.tags,
                ));
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
            reconcile_report(&table, &mut states, &mut registry, dir, lock, &mut report).await;
            batches += 1;
            if batches % MAINTENANCE_BATCHES == 0 {
                if let Err(error) = maintain_cache(&db, &table).await {
                    report
                        .errors
                        .push(format!("cache maintenance: {error}; rerun library upgrade"));
                    return report;
                }
                #[cfg(test)]
                println!(
                    "upgrade_batch_checkpoint files={} cached_rows={} batches={batches}",
                    report.converted + report.unchanged,
                    report.reconciled
                );
            }
        }
    }
    reconcile_report(&table, &mut states, &mut registry, dir, lock, &mut report).await;
    // Remove reserved markers even from stale rows awaiting ordinary recovery.
    // This does not infer or change their boolean reading state.
    if legacy {
        if let Err(error) = table
            .update()
            .only_if("array_contains(tags, 'unread') OR array_contains(tags, 'bookmark')")
            .column(
                "tags",
                if legacy_reading {
                    "array_remove(array_remove(tags, 'unread'), 'bookmark')"
                } else {
                    "array_remove(tags, 'bookmark')"
                },
            )
            .execute()
            .await
        {
            report.errors.push(format!(
                "cache legacy-marker cleanup: {error}; rerun library upgrade"
            ));
        }
    }
    if report.errors.is_empty() {
        if let Err(error) = registry.write_upgrade(dir, lock).await {
            report.errors.push(format!("tag definitions: {error}"));
        }
        for column in ["color", "kind"] {
            if let Err(error) = table
                .create_index(&[column], lancedb::index::Index::Bitmap(Default::default()))
                .execute()
                .await
            {
                report
                    .errors
                    .push(format!("{column} index: {error}; rerun library upgrade"));
            }
        }
    }
    if report.errors.is_empty() {
        if let Err(error) = maintain_cache(&db, &table).await {
            report
                .errors
                .push(format!("cache maintenance: {error}; rerun library upgrade"));
        }
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

async fn maintain_cache(db: &EmbeddingsDb, table: &lancedb::Table) -> Result<(), Error> {
    db.maintain().await?;
    // The converter's direct table handle must follow versions pruned by core.
    table.checkout_latest().await?;
    Ok(())
}

async fn reconcile_report(
    table: &lancedb::Table,
    states: &mut Vec<EntryState>,
    registry: &mut xaga_core::tags::Tags,
    dir: &Path,
    lock: &FileLock,
    report: &mut UpgradeReport,
) {
    if states.is_empty() {
        return;
    }
    let mut resolver = registry.resolver();
    for (_, _, _, names) in states.iter() {
        if resolver.resolve(names).is_err() {
            if let Err(error) = registry.discover(names) {
                report
                    .errors
                    .push(format!("tag discovery: {error}; rerun library upgrade"));
                states.clear();
                return;
            }
            resolver = registry.resolver();
        }
    }
    if let Err(error) = registry.write_upgrade(dir, lock).await {
        report
            .errors
            .push(format!("tag definitions: {error}; rerun library upgrade"));
        states.clear();
        return;
    }
    match reconcile(table, states, registry, lock).await {
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
    fn bookmark_conversion_preserves_palette_colors_and_retries() {
        for color in ["red", "orange", "yellow", "green", "blue", "violet"] {
            let original = document(&format!("color: {color}\ntags: [research, bookmark]"));
            let conversion = convert(&original).unwrap();
            assert_eq!(conversion.color.as_deref(), Some(color));
            let rendered = conversion.rendered.unwrap();
            assert!(rendered.ends_with("---\n\nBody\n\n"));
            assert_eq!(values(&rendered)["custom"], values(&original)["custom"]);
            assert_eq!(
                values(&rendered)["tags"],
                noyalib::to_value(&["research"]).unwrap()
            );
            assert!(convert(&rendered).unwrap().rendered.is_none());
        }
        let unmarked = convert(&document("tags: [research]")).unwrap();
        assert!(unmarked.color.is_none());
        assert!(unmarked.rendered.is_none());
        assert!(convert(&document("color: invalid\ntags: [bookmark]")).is_err());
    }

    #[test]
    fn conversion_preserves_state_metadata_and_exact_body_and_is_idempotent() {
        for (fields, unread, changed) in [
            ("tags: [research, unread, bookmark]", true, true),
            ("tags: [research, bookmark]", false, true),
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
            ("is_unread: true\ntags: [research, bookmark]", true, true),
            ("is_unread: false\ntags: [research, bookmark]", false, true),
        ] {
            let original = document(fields);
            let converted = convert(&original).unwrap();
            assert_eq!(converted.is_unread, unread);
            assert_eq!(converted.rendered.is_some(), changed);
            let saved = converted.rendered.as_deref().unwrap_or(&original);
            let mut expected = values(&original);
            expected.insert("tags".into(), noyalib::to_value(&vec!["research"]).unwrap());
            expected.insert("color".into(), Value::String("blue".into()));
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
    fn block_tag_migrations_preserve_siblings_body_and_retry_safely() {
        let preserved = "# Keep this sibling comment\ncustom: {id: 29688640e516, ratio: 1.234567890123456789}\nfavicon: ../images/abcdef123456/112233445566\n";
        for indent in ["", "  "] {
            for tags in [
                vec!["bookmark"],
                vec!["pinned", "unread", "bookmark", "research"],
            ] {
                let sequence: String = tags
                    .iter()
                    .map(|tag| format!("{indent}- {tag}\n"))
                    .collect();
                for suffix in ["", preserved] {
                    let original =
                        format!("---\ntitle: Saved\ntags:\n{sequence}{suffix}---\n\nBody\r\n\n");
                    let pinned = convert_markdown_pin(&original).unwrap();
                    let after_pin = pinned.as_deref().unwrap_or(&original);
                    let converted = convert(after_pin).unwrap();
                    assert_eq!(converted.color.as_deref(), Some("blue"));
                    let saved = converted.rendered.unwrap();
                    assert!(saved.contains(suffix), "sibling changed: {saved}");
                    assert!(saved.ends_with("---\n\nBody\r\n\n"));
                    let expected: Vec<_> = tags
                        .iter()
                        .copied()
                        .filter(|tag| *tag == "research")
                        .collect();
                    assert_eq!(
                        values(&saved)["tags"],
                        noyalib::to_value(&expected).unwrap()
                    );
                    assert_eq!(converted.is_unread, tags.contains(&"unread"));
                    assert!(convert_markdown_pin(&saved).unwrap().is_none());
                    assert!(convert(&saved).unwrap().rendered.is_none());
                }
            }
        }
    }

    #[test]
    fn multiline_and_flow_metadata_removal_preserves_other_fields() {
        let cases = [
            "title: Saved\ntags:\n- unread\n- bookmark\n- research\nis_unread:\n  false\nimages:\n  one:\n    nested: unsupported\n# Keep sibling\ncustom: {id: 29688640e516}\n",
            "{images: {one: {nested: unsupported}}, tags: [unread, bookmark, research], is_unread: false, custom: {id: 29688640e516}}\n",
            "{tags: [unread, bookmark, research], is_unread: false, custom: {id: 29688640e516}, images: {one: {nested: unsupported}}}\n",
        ];
        for fields in cases {
            let raw = format!("---\n{fields}---\n\nExact body\r\n");
            let converted = convert(&raw).unwrap();
            let saved = converted.rendered.unwrap();
            assert!(!converted.is_unread);
            let result = values(&saved);
            assert!(!result.contains_key("images"));
            assert!(!result.contains_key("is_unread"));
            assert_eq!(result["tags"], noyalib::to_value(&["research"]).unwrap());
            assert_eq!(result["color"], Value::String("blue".into()));
            assert!(saved.contains("custom: {id: 29688640e516}"));
            assert!(saved.ends_with("---\n\nExact body\r\n"));
            if fields.contains("# Keep sibling") {
                assert!(saved.contains("# Keep sibling\n"));
            }
            assert!(convert(&saved).unwrap().rendered.is_none());
        }
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
            "is_unread: true\ncolor: blue\ntags: [research, pinned]",
        );
        let original = fs::read_to_string(&path).unwrap();
        let article = read_article(dir.path(), "entry").await.unwrap();
        let mut registry = xaga_core::tags::Tags::default();
        registry
            .discover(article.meta.tags.as_deref().unwrap_or_default())
            .unwrap();
        registry.write_upgrade(dir.path(), &lock).await.unwrap();
        let db = EmbeddingsDb::open_writer(dir.path(), &lock).await.unwrap();
        let (tx, _) = flume::unbounded::<Entry>();
        let (annotation_tx, _) = flume::unbounded::<ListItem>();
        xaga_core::embeddings::sync_entry_row(&article, &db, &tx, &tx, &annotation_tx)
            .await
            .unwrap();
        seed(dir.path(), "read-entry", "color: blue");
        let mut read_article = read_article(dir.path(), "read-entry").await.unwrap();
        read_article.annotations.clear();
        xaga_core::embeddings::sync_entry_row(&read_article, &db, &tx, &tx, &annotation_tx)
            .await
            .unwrap();
        let table = table(dir.path(), &lock).await;
        table
            .add_columns(
                lancedb::table::NewColumnTransform::SqlExpressions(vec![(
                    "is_bookmarked".into(),
                    "true".into(),
                )]),
                None,
            )
            .await
            .unwrap();
        table
            .create_index(
                &["is_bookmarked"],
                lancedb::index::Index::Bitmap(Default::default()),
            )
            .execute()
            .await
            .unwrap();

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
        // Refresh this independently opened handle after CLI schema conversion.
        table.checkout_latest().await.unwrap();
        assert!(
            table
                .schema()
                .await
                .unwrap()
                .field_with_name("is_bookmarked")
                .is_err()
        );
        assert!(
            table
                .list_indices()
                .await
                .unwrap()
                .iter()
                .all(|index| !index.columns.iter().any(|column| column == "is_bookmarked"))
        );
        let row = db.get_entry("entry").await.unwrap().unwrap();
        let read_row = db.get_entry("read-entry").await.unwrap().unwrap();
        assert!(!read_row.is_unread && !read_row.color.is_empty());
        assert!(read_row.tags.is_empty());
        assert!(row.is_unread && !row.color.is_empty() && row.is_pinned);
        assert_eq!(
            row.tags,
            xaga_core::tags::Tags::read(dir.path())
                .await
                .unwrap()
                .resolver()
                .resolve(&["research".into()])
                .unwrap()
        );
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
    #[tokio::test]
    async fn tag_upgrade_013_preserves_labels_and_retries_without_rekeying() {
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        library::LibraryManifest::begin_upgrade(
            dir.path(),
            library::LibraryVersion::new(0, 13),
            &lock,
        )
        .unwrap()
        .unwrap()
        .complete()
        .unwrap();
        let path = seed(
            dir.path(),
            "entry",
            "tags: [Research, research, unread, bookmark, pinned, straße, strasse]\nis_unread: true\ncolor: green",
        );
        let original = fs::read(&path).unwrap();
        let mut article = read_article(dir.path(), "entry").await.unwrap();
        article.meta.tags = None;
        let db = EmbeddingsDb::open_writer(dir.path(), &lock).await.unwrap();
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        xaga_core::embeddings::sync_entry_row(&article, &db, &events, &events, &annotations)
            .await
            .unwrap();
        let cache = table(dir.path(), &lock).await;
        cache
            .update()
            .only_if("id = 'entry'")
            .column("topics", "['keep-topic']")
            .column(
                "vector",
                format!(
                    "arrow_cast([{}], 'FixedSizeList(384, Float32)')",
                    vec!["0.5"; 384].join(",")
                ),
            )
            .execute()
            .await
            .unwrap();
        // A bad file leaves the manifest pending; valid-file work remains durable.
        let bad = seed(dir.path(), "bad", "tags: invalid");
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(!report.errors.is_empty());
        assert_eq!(fs::read(&path).unwrap(), original);
        let first = xaga_core::tags::Tags::read(dir.path()).await.unwrap();
        assert_eq!(first.tags.len(), 6);
        assert!(
            library::LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .migration_target()
                .is_some()
        );
        fs::write(&bad, document("tags: [Research]")).unwrap();
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(
            xaga_core::tags::Tags::read(dir.path()).await.unwrap(),
            first
        );
        db.reload().await.unwrap();
        let row = db.get_entry("entry").await.unwrap().unwrap();
        assert_eq!(
            row.tags,
            first
                .resolver()
                .resolve(
                    &[
                        "Research", "unread", "bookmark", "pinned", "straße", "strasse"
                    ]
                    .map(String::from)
                )
                .unwrap()
        );
        assert_eq!(row.vector, Some(vec![0.5; 384]));
        assert_eq!(row.topics, vec!["keep-topic"]);
        assert_eq!(row.color, "green");
        assert!(row.is_unread);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(upgrade_library(dir.path(), &lock).await.errors.is_empty());
    }

    #[tokio::test]
    async fn tag_upgrade_012_only_interprets_markers_owned_by_that_format() {
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        library::LibraryManifest::begin_upgrade(
            dir.path(),
            library::LibraryVersion::new(0, 12),
            &lock,
        )
        .unwrap()
        .unwrap()
        .complete()
        .unwrap();
        let path = seed(dir.path(), "entry", "tags: [unread, pinned, bookmark]");
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let (fm, _) = xaga_core::md::parse_document(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(fm.tags, vec!["unread", "pinned"]);
        assert_eq!(fm.color.as_deref(), Some("blue"));
        assert_ne!(fm.is_unread, Some(true));
        let registry = xaga_core::tags::Tags::read(dir.path()).await.unwrap();
        assert_eq!(registry.tags.len(), 2);
    }

    #[tokio::test]
    async fn tag_upgrade_crosses_batch_boundary_and_keeps_first_discovered_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        library::LibraryManifest::begin_upgrade(
            dir.path(),
            library::LibraryVersion::new(0, 13),
            &lock,
        )
        .unwrap()
        .unwrap()
        .complete()
        .unwrap();
        let cache = table(dir.path(), &lock).await;
        seed_cache_rows(&cache, 0, 1027).await;
        let initial_version = cache.version().await.unwrap();
        for i in 0..1027 {
            seed(
                dir.path(),
                &format!("{i:012x}"),
                &format!("tags: [Research, RESEARCH, tag-{i}]"),
            );
        }
        let report = upgrade_library(dir.path(), &lock).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let tags = xaga_core::tags::Tags::read(dir.path()).await.unwrap();
        assert_eq!(tags.tags.len(), 1028);
        let research = tags.tags.iter().find(|t| t.name == "Research").unwrap();
        for i in [0, 1023, 1024, 1026] {
            let db = EmbeddingsDb::open_writer(dir.path(), &lock).await.unwrap();
            let row = db.get_entry(&format!("{i:012x}")).await.unwrap().unwrap();
            assert_eq!(
                row.tags,
                tags.resolver()
                    .resolve(&["Research".into(), format!("tag-{i}")])
                    .unwrap()
            );
            assert_eq!(row.tags[0], research.id);
        }
        let cache = table(dir.path(), &lock).await;
        let cache_versions = cache.version().await.unwrap() - initial_version;
        assert!(
            cache_versions <= 32,
            "two batches plus scalar-index maintenance produced {cache_versions} versions; writes must not scale with distinct tag combinations"
        );
        assert!(
            library::LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .matches_version(crate::cmd::library::application_version())
        );
    }
    async fn seed_cache_rows(cache: &lancedb::Table, start: usize, end: usize) {
        use arrow_array::builder::{ListBuilder, StringBuilder};
        use arrow_array::{
            ArrayRef, BooleanArray, FixedSizeListArray, Float32Array, RecordBatch,
            RecordBatchIterator, StringArray,
        };
        use arrow_schema::DataType;
        use std::sync::Arc;
        let schema = cache.schema().await.unwrap();
        let count = end - start;
        let columns: Vec<ArrayRef> = schema
            .fields()
            .iter()
            .map(|field| match field.data_type() {
                DataType::Utf8 => {
                    let values: Vec<String> = (start..end)
                        .map(|i| match field.name().as_str() {
                            "id" => format!("{i:012x}"),
                            "kind" => "entry".into(),
                            "source" => "https://example.org/a".into(),
                            "title" => "Example".into(),
                            "checksum" => "preserved-checksum".into(),
                            "color" => "green".into(),
                            _ => String::new(),
                        })
                        .collect();
                    Arc::new(StringArray::from(values)) as ArrayRef
                }
                DataType::Boolean => {
                    Arc::new(BooleanArray::from(vec![field.name() == "is_unread"; count]))
                        as ArrayRef
                }
                DataType::List(_) => {
                    let mut values = ListBuilder::new(StringBuilder::new());
                    for _ in 0..count {
                        if field.name() == "topics" {
                            values.values().append_value("preserved-topic");
                        }
                        values.append(true);
                    }
                    Arc::new(values.finish()) as ArrayRef
                }
                DataType::FixedSizeList(item, width) => {
                    let values =
                        Float32Array::from_iter_values((0..count * *width as usize).map(|j| {
                            let n = (start as u32)
                                .wrapping_mul(*width as u32)
                                .wrapping_add(j as u32);
                            n.wrapping_mul(2654435761) as f32 / u32::MAX as f32
                        }));
                    Arc::new(FixedSizeListArray::new(
                        item.clone(),
                        *width,
                        Arc::new(values),
                        None,
                    )) as ArrayRef
                }
                other => arrow_array::new_null_array(other, count),
            })
            .collect();
        let batch = RecordBatch::try_new(schema.clone(), columns).unwrap();
        cache
            .add(
                Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema.clone()))
                    as Box<dyn arrow_array::RecordBatchReader + Send>,
            )
            .execute()
            .await
            .unwrap();
    }

    /// One scaling point. Run in separate processes for independent peak-memory
    /// measurements; fixture creation is excluded from migration elapsed time.
    #[tokio::test]
    #[ignore = "explicit migration scaling measurement; set XAGA_TAG_SCALE_ROWS"]
    async fn tag_upgrade_scaling_point() {
        use std::time::Instant;
        let rows: usize = std::env::var("XAGA_TAG_SCALE_ROWS")
            .unwrap_or_else(|_| "8192".into())
            .parse()
            .unwrap();
        assert!(rows > 0);
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let lock = library::acquire_lock(lib).unwrap();
        library::LibraryManifest::begin_upgrade(lib, library::LibraryVersion::new(0, 13), &lock)
            .unwrap()
            .unwrap()
            .complete()
            .unwrap();
        fs::create_dir_all(library::entries_dir(lib)).unwrap();
        let raw = document("tags: [Research, RESEARCH]\nis_unread: true\ncolor: green");
        let cache = table(lib, &lock).await;
        let setup = Instant::now();
        for start in (0..rows).step_by(32768) {
            let end = (start + 32768).min(rows);
            for i in start..end {
                fs::write(library::entry_path(lib, &format!("{i:012x}")), &raw).unwrap();
            }
            seed_cache_rows(&cache, start, end).await;
        }
        for column in ["id", "kind"] {
            cache
                .create_index(&[column], lancedb::index::Index::BTree(Default::default()))
                .execute()
                .await
                .unwrap();
        }
        println!(
            "tag_curve rows={rows} setup_ms={}",
            setup.elapsed().as_millis()
        );
        let initial_version = cache.version().await.unwrap();
        let started = Instant::now();
        let report = upgrade_library(lib, &lock).await;
        let migration_ms = started.elapsed().as_millis();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.unchanged, rows);
        assert_eq!(report.reconciled, rows);
        let registry = xaga_core::tags::Tags::read(lib).await.unwrap();
        assert_eq!(registry.tags.len(), 1);
        assert_eq!(registry.tags[0].name, "Research");
        let cache = table(lib, &lock).await;
        let commits = cache.version().await.unwrap() - initial_version;
        assert_eq!(cache.count_rows(Some(format!("kind = 'entry' AND array_contains(tags, '{}') AND array_length(tags) = 1 AND color = 'green' AND is_unread = true AND checksum = 'preserved-checksum' AND array_contains(topics, 'preserved-topic')", registry.tags[0].id))).await.unwrap(), rows);
        assert_eq!(
            fs::read_to_string(library::entry_path(lib, "000000000000")).unwrap(),
            raw
        );
        assert!(
            library::LibraryManifest::read(lib)
                .unwrap()
                .unwrap()
                .matches_version(crate::cmd::library::application_version())
        );
        let started = Instant::now();
        let retry = upgrade_library(lib, &lock).await;
        assert!(retry.errors.is_empty());
        assert_eq!(retry.reconciled, 0);
        assert_eq!(xaga_core::tags::Tags::read(lib).await.unwrap(), registry);
        println!(
            "tag_curve rows={rows} migration_ms={migration_ms} cache_commits={commits} completed_retry_ms={}",
            started.elapsed().as_millis()
        );
    }
}
