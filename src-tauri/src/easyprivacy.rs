use std::io;
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager, path::BaseDirectory};

const LIST: &str = "easyprivacy.txt";
const BUNDLED_LIST: &str = "resources/easyprivacy/easyprivacy.txt";

pub(super) fn installed(root: &Path) -> io::Result<bool> {
    match std::fs::metadata(root.join(LIST)) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The EasyPrivacy path is not a file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[derive(Serialize)]
struct Installation<'a> {
    version: &'a str,
    commit: &'a str,
    installed_at: u64,
}

fn header<'a>(list: &'a str, prefix: &str) -> io::Result<&'a str> {
    list.lines()
        .take(16)
        .find_map(|line| line.strip_prefix(prefix))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("Missing {prefix}")))
}

/// The caller holds the account lock across the existence check and both writes.
pub(super) fn install_if_missing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    root: &Path,
) -> Result<(), String> {
    let bundled = if tauri::is_dev() {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_LIST)
    } else {
        app.path()
            .resolve(BUNDLED_LIST, BaseDirectory::Resource)
            .map_err(|error| error.to_string())?
    };
    install(root, &bundled).map_err(|error| format!("Could not install EasyPrivacy: {error}"))
}

fn install(root: &Path, bundled: &Path) -> io::Result<()> {
    let path = root.join(LIST);
    if installed(root)? {
        return Ok(());
    }

    let bytes = std::fs::read(bundled)?;
    let list = std::str::from_utf8(&bytes).map_err(io::Error::other)?;
    let version = header(list, "! Version: ")?;
    let commit = header(list, "! Commit: ")?;
    xaga_core::fs::write_atomic(&path, &bytes)?;
    let installed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    let metadata = Installation {
        version,
        commit,
        installed_at,
    };
    xaga_core::fs::write_atomic(
        &root.join("easyprivacy.json"),
        &serde_json::to_vec_pretty(&metadata)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_exact_snapshot_and_preserves_existing_file_and_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_LIST);
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        install(root, &bundled).unwrap();

        let list = std::fs::read_to_string(bundled).unwrap();
        let installed = root.join(LIST);
        assert_eq!(std::fs::read(&installed).unwrap(), list.as_bytes());
        let metadata_path = root.join("easyprivacy.json");
        let recorded = std::fs::read(&metadata_path).unwrap();
        let metadata: serde_json::Value = serde_json::from_slice(&recorded).unwrap();
        assert_eq!(metadata["version"], header(&list, "! Version: ").unwrap());
        assert_eq!(metadata["commit"], header(&list, "! Commit: ").unwrap());
        assert!(metadata["installed_at"].as_u64().unwrap() >= before);

        // A subsequent launch or enable reuses the file even if the bundled
        // resource has changed or is unavailable.
        std::fs::write(&installed, b"existing revision").unwrap();
        install(root, &root.join("missing.txt")).unwrap();
        assert_eq!(std::fs::read(installed).unwrap(), b"existing revision");
        assert_eq!(std::fs::read(metadata_path).unwrap(), recorded);
    }
}
