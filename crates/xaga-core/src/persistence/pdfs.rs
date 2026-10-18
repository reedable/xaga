//! Validated PDF documents associated with normalized source URLs.

use crate::{library_writer::WriteToken, persistence::entries::EntryError};
use chrono::Utc;
use std::{
    io::{Error, ErrorKind, Read},
    path::{Path, PathBuf},
};

pub const MAX_PDF_BYTES: u64 = 64 * 1024 * 1024;

struct PdfPaths {
    normalized_url: String,
    stem: String,
    pdf: PathBuf,
    companion: PathBuf,
}

pub fn validate(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_PDF_BYTES {
        return Err("PDF exceeds 64 MiB limit".into());
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err("unsupported download format".into());
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> EntryError {
    Error::new(ErrorKind::InvalidData, message.into()).into()
}

fn pdf_paths(library: &Path, url: &str) -> Result<PdfPaths, EntryError> {
    let normalized_url = crate::util::normalize_url(url)?;
    let parsed = url::Url::parse(&normalized_url)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid("PDF source must be HTTP(S)"));
    }
    let stem = crate::util::url_hash12(&normalized_url);
    Ok(PdfPaths {
        pdf: crate::library::pdf_path(library, &stem),
        companion: crate::library::pdf_entry_path(library, &stem),
        normalized_url,
        stem,
    })
}

fn check_directory(library: &Path) -> Result<bool, EntryError> {
    match std::fs::symlink_metadata(crate::library::pdfs_dir(library)) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(invalid("PDF directory is not a regular directory")),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn valid_stem(stem: &str) -> bool {
    stem.len() == 12
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_capped_regular_file(path: &Path, max_bytes: u64) -> Result<Option<Vec<u8>>, EntryError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid("stored PDF resource is not a regular file"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(invalid("stored PDF resource exceeds its size limit"));
    }
    Ok(Some(bytes))
}

/// Reads and validates a dormant PDF companion by its URL-derived filename stem.
///
/// The stored normalized URL must hash back to the requested stem. This API is
/// stem-based so future migration can inventory `pdfs/` without knowing the URL.
pub fn read_source_entry(
    library: &Path,
    stem: &str,
) -> Result<Option<crate::md::Frontmatter>, EntryError> {
    if !valid_stem(stem) {
        return Err(invalid("invalid PDF companion filename stem"));
    }
    if !check_directory(library)? {
        return Ok(None);
    }
    let path = crate::library::pdf_entry_path(library, stem);
    let Some(bytes) = read_capped_regular_file(&path, crate::fs::MAX_MD_BYTES)? else {
        return Ok(None);
    };
    let raw = String::from_utf8(bytes)
        .map_err(|error| invalid(format!("PDF companion is not UTF-8: {error}")))?;
    let (frontmatter, body) = crate::md::parse_document(&raw)?;
    if !body.is_empty() {
        return Err(invalid("PDF identity companion must have an empty body"));
    }
    let normalized = crate::util::normalize_url(&frontmatter.source)?;
    let parsed = url::Url::parse(&normalized)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid("PDF companion must contain an HTTP(S) source"));
    }
    if normalized != frontmatter.source {
        return Err(invalid("PDF companion source is not normalized"));
    }
    if crate::util::url_hash12(&frontmatter.source) != stem {
        return Err(invalid("PDF companion source does not match its filename"));
    }
    Ok(Some(frontmatter))
}

pub fn read(library: &Path, url: &str) -> Result<Option<(PathBuf, Vec<u8>)>, EntryError> {
    let paths = pdf_paths(library, url)?;
    if !check_directory(library)? {
        return Ok(None);
    }
    let Some(bytes) = read_capped_regular_file(&paths.pdf, MAX_PDF_BYTES)? else {
        return Ok(None);
    };
    validate(&bytes).map_err(invalid)?;
    Ok(Some((paths.pdf, bytes)))
}

fn remove_if_exists(path: &Path) -> Result<(), EntryError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) async fn save(
    library: &Path,
    url: &str,
    bytes: Option<Vec<u8>>,
    token: WriteToken,
) -> Result<(), EntryError> {
    let paths = pdf_paths(library, url)?;
    if let Some(bytes) = bytes {
        validate(&bytes).map_err(invalid)?;
        if !check_directory(library)? {
            std::fs::create_dir_all(crate::library::pdfs_dir(library))?;
            check_directory(library)?;
        }

        // One normalized URL has one Markdown identity record. A document in
        // entries/ takes precedence because it may already contain feed metadata
        // and user-authored state. Only otherwise create a dormant PDF companion.
        let entry_path = crate::library::entry_path(library, &paths.stem);
        if !tokio::fs::try_exists(&entry_path).await? {
            if !tokio::fs::try_exists(&paths.companion).await? {
                let updates = [
                    ("source", serde_json::json!(paths.normalized_url)),
                    ("title", serde_json::json!(paths.normalized_url)),
                    ("aliases", serde_json::json!([paths.normalized_url])),
                    ("created", serde_json::json!(Utc::now().to_rfc3339())),
                    ("tags", serde_json::json!([])),
                ];
                super::documents::upsert(&paths.companion, &updates, None, token).await?;
            } else {
                // Refuse to place bytes behind malformed or mismatched identity.
                read_source_entry(library, &paths.stem)?;
            }
        }

        // Identity lands first. A hard kill may leave a harmless companion-only
        // state, but never newly committed PDF bytes without reversible identity.
        crate::fs::write_atomic_async(paths.pdf, bytes).await?;
    } else if check_directory(library)? {
        // Bytes disappear first. A hard kill may leave a harmless companion-only state.
        remove_if_exists(&paths.pdf)?;
        remove_if_exists(&paths.companion)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stem(url: &str) -> String {
        crate::util::url_hash12(&crate::util::normalize_url(url).unwrap())
    }

    fn pdf_path(library: &Path, url: &str) -> PathBuf {
        crate::library::pdf_path(library, &stem(url))
    }

    fn companion_path(library: &Path, url: &str) -> PathBuf {
        crate::library::pdf_entry_path(library, &stem(url))
    }

    #[test]
    fn rejects_invalid_oversized_and_symlink_pdf_files() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        std::fs::create_dir(crate::library::pdfs_dir(dir.path())).unwrap();
        let file = pdf_path(dir.path(), url);
        std::fs::write(&file, b"not PDF").unwrap();
        assert!(read(dir.path(), url).is_err());
        let handle = std::fs::File::create(&file).unwrap();
        handle.set_len(MAX_PDF_BYTES + 1).unwrap();
        assert!(read(dir.path(), url).is_err());
        std::fs::remove_file(&file).unwrap();
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            let target = outside.path().join("outside.pdf");
            std::fs::write(&target, b"%PDF-valid").unwrap();
            std::os::unix::fs::symlink(target, &file).unwrap();
            assert!(read(dir.path(), url).is_err());
        }
        assert!(read(dir.path(), "file:///tmp/paper.pdf").is_err());
    }

    #[tokio::test]
    async fn save_writes_entry_format_companion_and_removal_deletes_both() {
        let dir = tempfile::tempdir().unwrap();
        let url = "HTTPS://Example.ORG:443/paper#section";
        let normalized = crate::util::normalize_url(url).unwrap();
        save(
            dir.path(),
            url,
            Some(b"%PDF-first".to_vec()),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(read(dir.path(), url).unwrap().unwrap().1, b"%PDF-first");
        let raw = std::fs::read_to_string(companion_path(dir.path(), url)).unwrap();
        let (frontmatter, body) = crate::md::parse_document(&raw).unwrap();
        assert_eq!(frontmatter.source, normalized);
        assert_eq!(frontmatter.title, normalized);
        assert!(frontmatter.tags.is_empty());
        assert!(body.is_empty());
        assert_eq!(
            read_source_entry(dir.path(), &stem(url))
                .unwrap()
                .unwrap()
                .source,
            normalized
        );

        save(dir.path(), url, None, WriteToken::for_test())
            .await
            .unwrap();
        assert!(read(dir.path(), url).unwrap().is_none());
        assert!(read_source_entry(dir.path(), &stem(url)).unwrap().is_none());
    }

    #[tokio::test]
    async fn existing_entry_is_the_identity_and_prevents_a_duplicate_companion() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        let entry = crate::library::entry_path(dir.path(), &stem(url));
        std::fs::create_dir_all(crate::library::entries_dir(dir.path())).unwrap();
        let original = "---\nsource: https://example.org/paper\ntitle: Authored\ncreated: '2026-01-01'\ntags: [bookmark]\nannotations: []\n---\n\nBody\n";
        std::fs::write(&entry, original).unwrap();
        save(
            dir.path(),
            url,
            Some(b"%PDF-valid".to_vec()),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read_to_string(entry).unwrap(), original);
        assert!(!companion_path(dir.path(), url).exists());
        assert!(read(dir.path(), url).unwrap().is_some());
    }

    #[tokio::test]
    async fn invalid_replacement_preserves_pdf_and_companion() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        save(
            dir.path(),
            url,
            Some(b"%PDF-first".to_vec()),
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let before = std::fs::read(companion_path(dir.path(), url)).unwrap();
        assert!(
            save(
                dir.path(),
                url,
                Some(b"invalid".to_vec()),
                WriteToken::for_test()
            )
            .await
            .is_err()
        );
        assert_eq!(read(dir.path(), url).unwrap().unwrap().1, b"%PDF-first");
        assert_eq!(
            std::fs::read(companion_path(dir.path(), url)).unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn pdf_commit_failure_leaves_valid_companion_only_state() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        std::fs::create_dir_all(pdf_path(dir.path(), url)).unwrap();
        assert!(
            save(
                dir.path(),
                url,
                Some(b"%PDF-valid".to_vec()),
                WriteToken::for_test()
            )
            .await
            .is_err()
        );
        assert_eq!(
            read_source_entry(dir.path(), &stem(url))
                .unwrap()
                .unwrap()
                .source,
            url
        );
    }

    #[tokio::test]
    async fn malformed_existing_companion_does_not_replace_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        std::fs::create_dir(crate::library::pdfs_dir(dir.path())).unwrap();
        std::fs::write(pdf_path(dir.path(), url), b"%PDF-existing").unwrap();
        std::fs::write(companion_path(dir.path(), url), "not markdown").unwrap();
        assert!(
            save(
                dir.path(),
                url,
                Some(b"%PDF-replacement".to_vec()),
                WriteToken::for_test()
            )
            .await
            .is_err()
        );
        assert_eq!(read(dir.path(), url).unwrap().unwrap().1, b"%PDF-existing");
    }

    #[test]
    fn companion_read_validates_url_filename_body_and_stem() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        std::fs::create_dir(crate::library::pdfs_dir(dir.path())).unwrap();
        let path = companion_path(dir.path(), url);
        for raw in [
            "---\nsource: https://other.example/paper\ntitle: T\n---\n",
            "---\nsource: file:///tmp/paper.pdf\ntitle: T\n---\n",
            "---\nsource: https://example.org/paper\ntitle: T\n---\nbody\n",
        ] {
            std::fs::write(&path, raw).unwrap();
            assert!(read_source_entry(dir.path(), &stem(url)).is_err());
        }
        assert!(read_source_entry(dir.path(), "../not-a-stem").is_err());
    }

    #[test]
    fn legacy_pdf_without_companion_remains_readable() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/legacy";
        std::fs::create_dir(crate::library::pdfs_dir(dir.path())).unwrap();
        std::fs::write(pdf_path(dir.path(), url), b"%PDF-legacy").unwrap();
        assert_eq!(read(dir.path(), url).unwrap().unwrap().1, b"%PDF-legacy");
        assert!(read_source_entry(dir.path(), &stem(url)).unwrap().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_directory_and_companion() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), crate::library::pdfs_dir(dir.path())).unwrap();
        assert!(read(dir.path(), "https://example.org/paper").is_err());

        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        std::fs::create_dir(crate::library::pdfs_dir(dir.path())).unwrap();
        let target = outside.path().join("entry.md");
        std::fs::write(&target, format!("---\nsource: {url}\ntitle: T\n---\n")).unwrap();
        std::os::unix::fs::symlink(target, companion_path(dir.path(), url)).unwrap();
        assert!(read_source_entry(dir.path(), &stem(url)).is_err());
    }
}
