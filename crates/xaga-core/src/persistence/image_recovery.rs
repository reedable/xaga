//! Apply recovered image references under the entry writer lane. Recovery never
//! replaces a whole stale article or changes its read state/PDF representation.
use super::{
    articles::article_from_document,
    entries::{EntryError, find_entry_path},
    images,
};
use crate::{library_writer::WriteToken, md::parse_document, types::Article};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePatch {
    pub start: usize,
    pub end: usize,
    pub original: String,
    pub url: String,
    pub css: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRecovery {
    pub summary: bool,
    pub expected: String,
    pub patches: Vec<ImagePatch>,
}

pub(crate) async fn apply(
    library: &Path,
    id: &str,
    recovery: ImageRecovery,
    data: HashMap<String, String>,
    token: WriteToken,
) -> Result<Article, EntryError> {
    if !super::entries::is_safe_id(id) {
        return Err(EntryError::InvalidId);
    }
    let path = find_entry_path(library, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
    let source = if recovery.summary {
        fm.summary.as_deref().unwrap_or("")
    } else {
        &body
    };
    // Do not overwrite a changed field, nor save orphan resources for that stale snapshot.
    if source != recovery.expected {
        return super::articles::project_pin(library, article_from_document(id, fm, body)).await;
    }
    let mut patches = recovery.patches;
    patches.sort_by_key(|p| p.start);
    let mut previous_end = 0;
    for p in &patches {
        if p.start < previous_end || source.get(p.start..p.end) != Some(p.original.as_str()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid image reference span",
            )
            .into());
        }
        previous_end = p.end;
    }
    let mut output = source.to_owned();
    let mut total = 0;
    let mut resources = HashMap::new();
    for p in patches.into_iter().rev() {
        let local = if let Some(local) = resources.get(&p.url) {
            local
        } else {
            let Some((hash, Some(image))) = images::extract_favicon(&p.url, &data) else {
                continue;
            };
            if total + image.bytes.len() > images::MAX_ENTRY_IMAGE_BYTES {
                continue;
            }
            total += image.bytes.len();
            let landed = images::write_entry_images(library, id, &[image], token).await;
            if landed.is_empty() {
                continue;
            }
            resources
                .entry(p.url.clone())
                .or_insert_with(|| crate::library::entry_image_ref(id, &hash))
        };
        let replacement = if p.css {
            format!("url({local})")
        } else {
            local.clone()
        };
        output.replace_range(p.start..p.end, &replacement);
    }
    if output == source {
        return super::articles::project_pin(library, article_from_document(id, fm, body)).await;
    }
    // Preserve unrelated frontmatter and the exact body on summary-only updates.
    let saved = if recovery.summary {
        super::documents::upsert(
            &path,
            &[("summary", serde_json::json!(output))],
            None,
            token,
        )
        .await?
    } else {
        super::documents::upsert(&path, &[], Some(&output), token).await?
    };
    let keep = images::referenced_image_hashes(id, &saved);
    images::gc_entry_images(library, id, &keep, token).await;
    let (fm, body) = parse_document(&saved)?;
    super::articles::project_pin(library, article_from_document(id, fm, body)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    const DATA: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC";
    const URL: &str = "https://example.org/comic.png";
    fn recovery(summary: bool, source: &str) -> ImageRecovery {
        let start = source.find(URL).unwrap();
        ImageRecovery {
            summary,
            expected: source.into(),
            patches: vec![ImagePatch {
                start,
                end: start + URL.len(),
                original: URL.into(),
                url: URL.into(),
                css: false,
            }],
        }
    }
    async fn fixture(lib: &Path, summary: &str) -> std::path::PathBuf {
        let path = crate::library::entry_path(lib, "entry1");
        tokio::fs::create_dir_all(path.parent().unwrap())
            .await
            .unwrap();
        let content = format!(
            "---\nsource: https://example.org/article\ntitle: User title\ncustom: preserve\ntags: [bookmark]\nsummary: {}\n---\n\nExact body.\n",
            serde_json::to_string(summary).unwrap()
        );
        tokio::fs::write(&path, content).await.unwrap();
        path
    }
    #[tokio::test]
    async fn summary_recovery_preserves_body_metadata_and_alternative_content() {
        let tmp = tempfile::tempdir().unwrap();
        let source = format!("<img src=\"{URL}\" alt=\"Comic\" title=\"Additional joke\">");
        let path = fixture(tmp.path(), &source).await;
        let before = tokio::fs::read_to_string(&path).await.unwrap();
        let (_, old_body) = parse_document(&before).unwrap();
        apply(
            tmp.path(),
            "entry1",
            recovery(true, &source),
            HashMap::from([(URL.into(), DATA.into())]),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let saved = tokio::fs::read_to_string(&path).await.unwrap();
        let (fm, body) = parse_document(&saved).unwrap();
        assert_eq!(body, old_body);
        assert!(saved.contains("custom: preserve"));
        assert!(saved.contains("User title"));
        assert!(saved.contains("bookmark"));
        let reference = crate::library::entry_image_ref("entry1", &images::image_url_hash(URL));
        assert_eq!(fm.summary.unwrap(), source.replace(URL, &reference));
        assert!(images::read_entry_images(tmp.path(), "entry1", &[reference]).await[0].is_some());
    }
    #[tokio::test]
    async fn stale_field_and_failed_capture_leave_original_file_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let source = format!("![Comic]({URL})");
        let path = fixture(tmp.path(), &source).await;
        let before = tokio::fs::read(&path).await.unwrap();
        apply(
            tmp.path(),
            "entry1",
            recovery(true, &source),
            HashMap::new(),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(tokio::fs::read(&path).await.unwrap(), before);
        let stale = recovery(true, &format!("old {source}"));
        apply(
            tmp.path(),
            "entry1",
            stale,
            HashMap::from([(URL.into(), DATA.into())]),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(tokio::fs::read(&path).await.unwrap(), before);
        assert!(!crate::library::entry_images_dir(tmp.path(), "entry1").exists());
    }
    #[tokio::test]
    async fn failed_resource_write_does_not_commit_a_local_reference() {
        let tmp = tempfile::tempdir().unwrap();
        let source = format!("![Comic]({URL})");
        let path = fixture(tmp.path(), &source).await;
        let before = tokio::fs::read(&path).await.unwrap();
        tokio::fs::write(tmp.path().join("images"), b"not a directory")
            .await
            .unwrap();
        apply(
            tmp.path(),
            "entry1",
            recovery(true, &source),
            HashMap::from([(URL.into(), DATA.into())]),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(tokio::fs::read(&path).await.unwrap(), before);
    }
}
