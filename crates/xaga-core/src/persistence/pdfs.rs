//! Validated PDF documents associated with normalized source URLs.
use crate::{library_writer::WriteToken, persistence::entries::EntryError};
use std::{
    io::{Error, ErrorKind, Read},
    path::{Path, PathBuf},
};
pub const MAX_PDF_BYTES: u64 = 64 * 1024 * 1024;

pub fn validate(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_PDF_BYTES {
        return Err("PDF exceeds 64 MiB limit".into());
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err("unsupported download format".into());
    }
    Ok(())
}
fn invalid(message: &str) -> EntryError {
    Error::new(ErrorKind::InvalidData, message).into()
}
fn path(library: &Path, url: &str) -> Result<PathBuf, EntryError> {
    let url = crate::util::normalize_url(url)?;
    let parsed = url::Url::parse(&url)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid("PDF source must be HTTP(S)"));
    }
    Ok(library
        .join("pdfs")
        .join(format!("{}.pdf", crate::util::url_hash12(&url))))
}
fn check_directory(library: &Path) -> Result<bool, EntryError> {
    match std::fs::symlink_metadata(library.join("pdfs")) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(invalid("PDF directory is not a regular directory")),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
pub fn read(library: &Path, url: &str) -> Result<Option<(PathBuf, Vec<u8>)>, EntryError> {
    let path = path(library, url)?;
    if !check_directory(library)? {
        return Ok(None);
    }
    let m = match std::fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if !m.is_file() || m.len() > MAX_PDF_BYTES {
        return Err(invalid("invalid stored PDF file"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&path)?
        .take(MAX_PDF_BYTES + 1)
        .read_to_end(&mut bytes)?;
    validate(&bytes).map_err(|e| invalid(&e))?;
    Ok(Some((path, bytes)))
}
pub(crate) async fn save(
    library: &Path,
    url: &str,
    bytes: Option<Vec<u8>>,
    _token: WriteToken,
) -> Result<(), EntryError> {
    let path = path(library, url)?;
    if let Some(bytes) = bytes {
        validate(&bytes).map_err(|e| invalid(&e))?;
        if !check_directory(library)? {
            std::fs::create_dir_all(library.join("pdfs"))?;
            check_directory(library)?;
        }
        crate::fs::write_atomic_async(path, bytes).await?;
    } else if check_directory(library)? {
        match std::fs::remove_file(path) {
            Ok(()) => (),
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_oversized_and_symlink_files() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.org/paper";
        std::fs::create_dir(dir.path().join("pdfs")).unwrap();
        let file = path(dir.path(), url).unwrap();
        std::fs::write(&file, b"not PDF").unwrap();
        assert!(read(dir.path(), url).is_err());
        let handle = std::fs::File::create(&file).unwrap();
        handle.set_len(MAX_PDF_BYTES + 1).unwrap();
        assert!(read(dir.path(), url).is_err());
        std::fs::remove_file(&file).unwrap();
        #[cfg(unix)]
        {
            let outside = dir.path().join("outside.pdf");
            std::fs::write(&outside, b"%PDF-valid").unwrap();
            std::os::unix::fs::symlink(outside, &file).unwrap();
            assert!(read(dir.path(), url).is_err());
        }
        assert!(read(dir.path(), "file:///tmp/paper.pdf").is_err());
    }

    #[tokio::test]
    async fn validated_atomic_replacement_and_removal() {
        let dir = tempfile::tempdir().unwrap();
        let token = WriteToken::for_test();
        let url = "https://example.org/paper";
        assert!(read(dir.path(), url).unwrap().is_none());
        save(dir.path(), url, Some(b"%PDF-first".to_vec()), token)
            .await
            .unwrap();
        assert!(
            save(dir.path(), url, Some(b"bad".to_vec()), token)
                .await
                .is_err()
        );
        assert_eq!(read(dir.path(), url).unwrap().unwrap().1, b"%PDF-first");
        save(dir.path(), url, Some(b"%PDF-next".to_vec()), token)
            .await
            .unwrap();
        assert_eq!(read(dir.path(), url).unwrap().unwrap().1, b"%PDF-next");
        save(dir.path(), url, None, token).await.unwrap();
        assert!(read(dir.path(), url).unwrap().is_none());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_directory() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("pdfs")).unwrap();
        assert!(read(dir.path(), "https://example.org/paper").is_err());
    }
}
