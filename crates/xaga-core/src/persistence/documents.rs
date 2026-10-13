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

#[cfg(test)]
mod tests {
    use super::*;
    const ORIGINAL: &str = "---\nsource: https://example.com/a\ntitle: Original\nsummary: Saved summary\ntags: [research, bookmark]\nannotations:\n  - id: note\n    text: [selection]\n    occurrence: [0]\n    color: yellow\n    note: Irreplaceable\n    created_at: '2026-01-01'\ncustom: {nested: [one, two]}\n---\n\nOriginal body\n\n";

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
        assert_eq!(crate::md::test_frontmatter_values(&raw)["custom"], crate::md::test_frontmatter_values(ORIGINAL)["custom"]);
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
        assert_eq!(crate::md::test_frontmatter_values(&raw)["custom"], crate::md::test_frontmatter_values(ORIGINAL)["custom"]);
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
        let malformed = ORIGINAL.replace("tags: [research, bookmark]", "tags: [broken");
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
