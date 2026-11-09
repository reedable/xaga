//! The sole persistence boundary for Markdown: apply supplied fields to the stored document.
use super::entries::EntryError;
use crate::{
    library_writer::WriteToken,
    md::{parse_document, patch_document},
};
use std::path::Path;

// Only this module can construct bytes accepted by the Markdown filesystem writer.
pub(crate) struct ValidatedDocument(String);
impl ValidatedDocument {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Call under the library writer's per-file lane. Omitted fields and body survive.
/// An absent file starts as an empty document and receives the same patch.
/// Invalid/unreadable existing data aborts without writing.
pub(crate) async fn upsert(
    path: &Path,
    updates: &[(&str, serde_json::Value)],
    body: Option<&str>,
    _token: WriteToken,
) -> Result<String, EntryError> {
    let raw = match crate::fs::read_md_capped(path).await {
        Ok(raw) => raw,
        Err(crate::fs::ReadMdError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            "---\n{}\n---\n".to_owned()
        }
        Err(e) => return Err(e.into()),
    };
    let rendered = patch_document(&raw, updates, body)?;
    parse_document(&rendered)?;
    crate::fs::write_markdown_async(path.to_owned(), ValidatedDocument(rendered.clone())).await?;
    Ok(rendered)
}

fn yaml_tokens(
    node: &noyalib::cst::GreenNode,
    base: usize,
    output: &mut Vec<(noyalib::cst::SyntaxKind, usize, usize)>,
) {
    let mut offset = base;
    for child in node.children() {
        match child {
            noyalib::cst::GreenChild::Node(n) => yaml_tokens(n, offset, output),
            noyalib::cst::GreenChild::Token { kind, .. } => {
                output.push((*kind, offset, offset + child.text_len()))
            }
        }
        offset += child.text_len();
    }
}

fn annotation_mapping(
    node: &noyalib::cst::GreenNode,
    base: usize,
    id: (usize, usize),
) -> Option<(&noyalib::cst::GreenNode, usize)> {
    if id.0 < base || id.1 > base + node.text_len() {
        return None;
    }
    let mut offset = base;
    for child in node.children() {
        if let noyalib::cst::GreenChild::Node(inner) = child {
            if let Some(found) = annotation_mapping(inner, offset, id) {
                return Some(found);
            }
        }
        offset += child.text_len();
    }
    (node.kind() == noyalib::cst::SyntaxKind::BlockMapping).then_some((node, base))
}

/// Change only one annotation's tags, retaining unrelated source bytes.
/// The caller owns the parent Entry writer lane.
pub(super) async fn patch_annotation_tags(
    path: &Path,
    index: usize,
    tags: &[String],
    _token: WriteToken,
) -> Result<(), EntryError> {
    let raw = crate::fs::read_md_capped(path).await?;
    parse_document(&raw)?;
    let trimmed = raw.trim_start();
    let prefix = &raw[..raw.len() - trimmed.len()];
    let rest = trimmed
        .strip_prefix("---\n")
        .ok_or(crate::md::MdError::Delimiter)?;
    let end = rest.find("\n---\n").ok_or(crate::md::MdError::Delimiter)?;
    let edit = || -> Result<String, noyalib::Error> {
        let mut document = noyalib::cst::parse_document(&rest[..end])?;
        let parent = format!("annotations[{index}]");
        let field = format!("{parent}.tags");
        let value = serde_json::to_string(tags).expect("string sequence serializes");
        let parent_span = document
            .span_at(&parent)
            .ok_or_else(|| noyalib::Error::Parse("annotation mapping unavailable".into()))?;
        if document.source()[parent_span.0..parent_span.1]
            .trim_start()
            .starts_with('{')
        {
            if let Some((start, end)) = document.span_at(&field) {
                if !tags.is_empty() {
                    document.set(&field, &value)?;
                } else {
                    use noyalib::cst::SyntaxKind;
                    let mut leaves = Vec::new();
                    yaml_tokens(document.syntax(), 0, &mut leaves);
                    leaves.retain(|(_, a, b)| {
                        *a >= parent_span.0
                            && *b <= parent_span.1
                            && !document.source()[*a..*b].trim().is_empty()
                    });
                    let colon = leaves
                        .iter()
                        .rposition(|(kind, _, b)| {
                            *b <= start && *kind == SyntaxKind::ColonIndicator
                        })
                        .ok_or_else(|| noyalib::Error::Parse("tags key unavailable".into()))?;
                    let key_start = leaves[colon - 1].1;
                    let source = document.source();
                    let tail = &source[end..parent_span.1];
                    let after = tail.len() - tail.trim_start().len();
                    if tail[after..].starts_with(',') {
                        document.replace_span(key_start, end + after + 1, "")?;
                    } else {
                        let comma =
                            source[parent_span.0..key_start].rfind(',').ok_or_else(|| {
                                noyalib::Error::Parse("tags separator unavailable".into())
                            })? + parent_span.0;
                        document.replace_span(comma, end, "")?;
                    }
                }
            } else if !tags.is_empty() {
                let close = parent_span.0
                    + document.source()[parent_span.0..parent_span.1]
                        .rfind('}')
                        .ok_or_else(|| {
                            noyalib::Error::Parse("annotation mapping unavailable".into())
                        })?;
                let separator = if document.source()[parent_span.0..close]
                    .trim_end()
                    .ends_with(',')
                {
                    " "
                } else {
                    ", "
                };
                document.replace_span(close, close, &format!("{separator}tags: {value}"))?;
            }
        } else if document.span_at(&field).is_some() {
            // Locate the direct field in the Annotation mapping. A collection's
            // value span can include following comments, so edit only its tokens.
            let id_span = document
                .span_at(&format!("{parent}.id"))
                .ok_or_else(|| noyalib::Error::Parse("annotation ID unavailable".into()))?;
            let (mapping, base) = annotation_mapping(document.syntax(), 0, id_span)
                .ok_or_else(|| noyalib::Error::Parse("annotation mapping unavailable".into()))?;
            let mut offset = base;
            let mut range = None;
            for child in mapping.children() {
                if let noyalib::cst::GreenChild::Node(entry) = child {
                    if entry.kind() == noyalib::cst::SyntaxKind::MappingEntry {
                        let source = &document.source()[offset..offset + entry.text_len()];
                        if source
                            .split_once(':')
                            .and_then(|(key, _)| noyalib::from_str::<String>(key.trim()).ok())
                            .as_deref()
                            == Some("tags")
                        {
                            let mut leaves = Vec::new();
                            yaml_tokens(entry, offset, &mut leaves);
                            let end = leaves
                                .iter()
                                .rev()
                                .find(|(kind, _, _)| {
                                    !matches!(
                                        kind,
                                        noyalib::cst::SyntaxKind::Whitespace
                                            | noyalib::cst::SyntaxKind::Newline
                                            | noyalib::cst::SyntaxKind::Comment
                                    )
                                })
                                .map(|(_, _, end)| *end)
                                .ok_or_else(|| {
                                    noyalib::Error::Parse("tags value unavailable".into())
                                })?;
                            let end = document.source()[..end]
                                .trim_end_matches([' ', '\t', '\r', '\n'])
                                .len();
                            range = Some((offset, end));
                            break;
                        }
                    }
                }
                offset += child.text_len();
            }
            let (start, end) =
                range.ok_or_else(|| noyalib::Error::Parse("tags field unavailable".into()))?;
            let fragment = if tags.is_empty() {
                String::new()
            } else {
                format!("tags: {value}")
            };
            document.replace_span(start, end, &fragment)?;
        } else if !tags.is_empty() {
            let value = serde_json::to_string(tags).expect("string sequence serializes");
            // Insert after the last value in a block mapping. Keeping the
            // newline outside the mapping span avoids joining the next item.
            let source = document.source();
            let (id_start, _) = document
                .span_at(&format!("{parent}.id"))
                .ok_or_else(|| noyalib::Error::Parse("annotation has no ID".into()))?;
            let line_start = source[..id_start].rfind('\n').map_or(0, |i| i + 1);
            let line = &source[line_start..id_start];
            let indent = if let Some(dash) = line.find("- ") {
                dash + 2
            } else {
                line.len() - line.trim_start().len()
            };
            let (_, end) = document
                .span_at(&parent)
                .ok_or_else(|| noyalib::Error::Parse("annotation mapping unavailable".into()))?;
            let end = source[end..]
                .find('\n')
                .map_or(source.len(), |n| end + n + 1);
            let prefix = if end == source.len() && !source.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            document.replace_span(
                end,
                end,
                &format!("{prefix}{}tags: {value}\n", " ".repeat(indent)),
            )?;
        }
        Ok(format!(
            "{prefix}---\n{}{}",
            document.source(),
            &rest[end..]
        ))
    };
    let rendered = edit().map_err(|e| crate::md::MdError::Frontmatter(e.to_string()))?;
    let mut expected = crate::md::frontmatter_json(&raw)?;
    let target = expected
        .get_mut("annotations")
        .and_then(|a| a.get_mut(index))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or(EntryError::NotFound)?;
    if tags.is_empty() {
        target.remove("tags");
    } else {
        target.insert("tags".into(), serde_json::json!(tags));
    }
    if crate::md::frontmatter_json(&rendered)? != expected {
        return Err(crate::md::MdError::Frontmatter(
            "annotation tag patch changed unrelated fields".into(),
        )
        .into());
    }
    crate::fs::write_markdown_async(path.to_owned(), ValidatedDocument(rendered)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ORIGINAL: &str = "---\nsource: https://example.com/a\ntitle: Original\nsummary: Saved summary\ntags: [research]\ncolor: blue\nannotations:\n  - id: note\n    text: [selection]\n    occurrence: [0]\n    color: yellow\n    note: Irreplaceable\n    created_at: '2026-01-01'\ncustom: {nested: [one, two]}\n---\n\nOriginal body\n\n";

    #[tokio::test]
    async fn same_field_patch_creates_and_updates_a_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        let fields = [
            ("title", serde_json::json!("Initial")),
            ("summary", serde_json::json!("Original summary")),
            ("tags", serde_json::json!(["research"])),
        ];
        upsert(
            &path,
            &fields,
            Some("Original body"),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let (fm, body) = parse_document(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(fm.title, "Initial");
        assert_eq!(fm.summary.as_deref(), Some("Original summary"));
        assert_eq!(fm.tags, vec!["research"]);
        assert_eq!(body, "Original body");
        upsert(
            &path,
            &[("title", serde_json::json!("Updated"))],
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let (fm, body) = parse_document(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(fm.title, "Updated");
        assert_eq!(fm.summary.as_deref(), Some("Original summary"));
        assert_eq!(fm.tags, vec!["research"]);
        assert_eq!(body, "Original body");
    }

    #[tokio::test]
    async fn explicit_empty_values_are_stored_and_omitted_fields_survive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        std::fs::write(&path, ORIGINAL).unwrap();
        upsert(
            &path,
            &[
                ("summary", serde_json::json!("")),
                ("tags", serde_json::json!([])),
            ],
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        let (fm, body) = parse_document(&raw).unwrap();
        assert_eq!(fm.summary.as_deref(), Some(""));
        assert!(fm.tags.is_empty());
        assert_eq!(fm.annotations[0].content, "Irreplaceable");
        assert_eq!(body, "Original body");
        assert_eq!(
            crate::md::test_frontmatter_values(&raw)["custom"],
            crate::md::test_frontmatter_values(ORIGINAL)["custom"]
        );
        upsert(
            &path,
            &[("summary", serde_json::Value::Null)],
            Some(""),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        let (fm, body) = parse_document(&raw).unwrap();
        assert!(raw.contains("summary: null"));
        assert_eq!(fm.summary, None);
        assert_eq!(body, "");
        assert_eq!(fm.annotations[0].content, "Irreplaceable");
        upsert(
            &path,
            &[("annotations", serde_json::json!([]))],
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(parse_document(&raw).unwrap().0.annotations.is_empty());
        assert_eq!(
            crate::md::test_frontmatter_values(&raw)["custom"],
            crate::md::test_frontmatter_values(ORIGINAL)["custom"]
        );
    }

    #[tokio::test]
    async fn upsert_preserves_fields_not_supplied() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        std::fs::write(&path, ORIGINAL).unwrap();
        upsert(
            &path,
            &[("title", serde_json::json!("Updated"))],
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        let mut expected = crate::md::test_frontmatter_values(ORIGINAL);
        expected.insert("title".into(), noyalib::Value::String("Updated".into()));
        assert_eq!(crate::md::test_frontmatter_values(&saved), expected);
        assert!(saved.ends_with("---\n\nOriginal body\n\n"));
        // An omitted body or field cannot clear it, on a second save either.
        upsert(&path, &[], None, WriteToken::for_test())
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
    }

    #[tokio::test]
    async fn malformed_existing_data_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        let malformed = ORIGINAL.replace("tags: [research]", "tags: [broken");
        std::fs::write(&path, &malformed).unwrap();
        assert!(
            upsert(&path, &[], None, WriteToken::for_test())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[tokio::test]
    async fn generic_atomic_writer_cannot_bypass_markdown_preservation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        std::fs::write(&path, ORIGINAL).unwrap();
        assert!(crate::fs::write_atomic(&path, b"replacement").is_err());
        assert!(
            crate::fs::write_atomic_async(path.clone(), b"replacement".to_vec())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), ORIGINAL);
    }

    #[tokio::test]
    async fn invalid_patch_leaves_original_file_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        std::fs::write(&path, ORIGINAL).unwrap();
        assert!(
            upsert(
                &path,
                &[("tags", serde_json::json!(42))],
                None,
                WriteToken::for_test()
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), ORIGINAL);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_failure_leaves_original_file_intact() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        std::fs::write(&path, ORIGINAL).unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = upsert(
            &path,
            &[("title", serde_json::json!("Changed"))],
            None,
            WriteToken::for_test(),
        )
        .await;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), ORIGINAL);
    }

    #[tokio::test]
    async fn unreadable_target_is_not_treated_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.md");
        std::fs::create_dir(&path).unwrap();
        assert!(
            upsert(&path, &[], None, WriteToken::for_test())
                .await
                .is_err()
        );
        assert!(path.is_dir());
    }
}
