//! The 0.17 layout transition. Pending format metadata is the migration journal.
use std::{fs, path::Path};

use crate::fs::FileLock;

pub(super) fn upgrade_required(dir: &Path) -> bool {
    match super::LibraryManifest::read(dir) {
        Ok(Some(manifest)) => {
            dir.join(super::MANIFEST_NAME).exists()
                || manifest
                    .format_version()
                    .is_none_or(|v| v < super::LibraryVersion::new(0, 17))
        }
        // Unknown or conflicting metadata cannot establish a safe browsing layout.
        _ => true,
    }
}

/// Refuse collisions instead of merging stores or selecting one of two copies.
/// A rename either leaves the old path or the new path, making replay idempotent.
fn relocate(dir: &Path, old: &str, new: &Path) -> Result<(), super::upgrade::Error> {
    let old = dir.join(old);
    let source = match fs::symlink_metadata(&old) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let expected_file = old.extension().is_some_and(|ext| ext == "yaml");
    if (expected_file && !source.file_type().is_file())
        || (!expected_file && !source.file_type().is_dir())
    {
        return Err(format!("unexpected layout source type: {}", old.display()).into());
    }
    match fs::symlink_metadata(new) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => {
            return Err(format!("layout destination already exists: {}", new.display()).into());
        }
    }
    let parent = new.parent().ok_or("missing layout parent")?;
    fs::create_dir_all(parent)?;
    fs::rename(&old, new)?;
    fs::File::open(parent)?.sync_all()?;
    fs::File::open(dir)?.sync_all()?;
    super::upgrade::checkpoint(new, "layout_relocated")?;
    Ok(())
}

pub(super) fn convert(dir: &Path, _lock: &FileLock) -> Result<(), super::upgrade::Error> {
    relocate(dir, "feeds", &super::feeds_dir(dir))?;
    relocate(dir, "favicons", &super::favicons_dir(dir))?;
    relocate(dir, "tags.yaml", &super::tags_path(dir))?;
    relocate(dir, "pins.yaml", &super::pins_path(dir))?;
    relocate(dir, "embeddings.lance", &super::index_path(dir))?;
    // Relocate the journal last. Both locations are recognized on retry, but
    // simultaneous manifests are refused by LibraryManifest::read.
    relocate(dir, "library.yaml", &super::metadata_manifest_path(dir))?;
    Ok(())
}

pub(super) fn rewrite_favicons(
    dir: &Path,
    lock: &FileLock,
    report: &mut super::UpgradeReport,
    progress: &impl Fn(super::UpgradeProgress),
) {
    for directory in [
        super::entries_dir(dir),
        super::topics_dir(dir),
        super::pdfs_dir(dir),
    ] {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                report.errors.push(format!(
                    "favicon references in {}: {error}",
                    directory.display()
                ));
                continue;
            }
        };
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    report.errors.push(format!(
                        "favicon references in {}: {error}",
                        directory.display()
                    ));
                    continue;
                }
            };
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            match rewrite_favicon(&path, lock) {
                Ok(true) => report.converted += 1,
                Ok(false) => {}
                Err(error) => report
                    .errors
                    .push(format!("favicon reference in {}: {error}", path.display())),
            }
            progress(super::UpgradeProgress::from_report(
                super::UpgradePhase::Converting,
                report,
            ));
        }
    }
}

fn rewrite_favicon(path: &Path, lock: &FileLock) -> Result<bool, super::upgrade::Error> {
    let raw = super::upgrade::read_capped(path)?;
    if !raw.contains("../favicons/") {
        return Ok(false);
    }
    let (fm, _) = crate::md::parse_document(&raw)?;
    let Some(hash) = fm
        .favicon
        .as_deref()
        .and_then(|value| value.strip_prefix("../favicons/"))
    else {
        return Ok(false);
    };
    if !crate::persistence::entries::is_safe_id(hash) {
        return Err("invalid legacy favicon reference".into());
    }
    let rendered = super::upgrade::patch_favicon(&raw, &super::favicon_ref(hash))?;
    super::upgrade::write_atomic(path, &rendered, lock)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{self, LibraryManifest, LibraryVersion};

    fn legacy(dir: &Path) {
        fs::write(dir.join("library.yaml"), "format_version: '0.16'\n").unwrap();
        for directory in ["feeds", "favicons", "embeddings.lance", "entries"] {
            fs::create_dir(dir.join(directory)).unwrap();
        }
        fs::write(dir.join("pins.yaml"), "pins: [{type: entry, id: e}]\n").unwrap();
        fs::write(dir.join("tags.yaml"), "tags:\n- id: 46fa4a27-1ddf-448a-a5e6-95691a29c602\n  name: Current\ntransitions:\n- type: rename\n  revision: 0\n  id: 46fa4a27-1ddf-448a-a5e6-95691a29c602\n  old_name: Previous\n  files:\n    complete: false\n    pending: [e]\n").unwrap();
        fs::write(dir.join("feeds/f.atom"), "feed bytes").unwrap();
        fs::write(dir.join("favicons/icon"), "icon bytes").unwrap();
        fs::write(dir.join("embeddings.lance/retained"), "database bytes").unwrap();
        fs::write(dir.join("entries/e.md"), "---\nsource: https://example.com/\ntitle: Article\nfavicon: ../favicons/icon\ncustom: keep\n---\n\nExact body.\n").unwrap();
    }

    #[tokio::test]
    async fn upgrade_moves_stores_and_database_and_preserves_document_bytes() {
        let dir = tempfile::tempdir().unwrap();
        legacy(dir.path());
        let original = fs::read_to_string(library::entry_path(dir.path(), "e")).unwrap();
        let pins = fs::read(dir.path().join("pins.yaml")).unwrap();
        let tags = fs::read(dir.path().join("tags.yaml")).unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let report =
            library::upgrade_library(dir.path(), &lock, LibraryVersion::new(0, 17), |_| {}).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(!upgrade_required(dir.path()));
        assert!(!dir.path().join("library.yaml").exists());
        assert_eq!(fs::read(library::pins_path(dir.path())).unwrap(), pins);
        assert_eq!(fs::read(library::tags_path(dir.path())).unwrap(), tags);
        assert!(!dir.path().join("pins.yaml").exists());
        assert!(!dir.path().join("tags.yaml").exists());
        assert_eq!(
            fs::read_to_string(library::index_path(dir.path()).join("retained")).unwrap(),
            "database bytes"
        );
        assert_eq!(
            fs::read_to_string(library::feeds_dir(dir.path()).join("f.atom")).unwrap(),
            "feed bytes"
        );
        let saved = fs::read_to_string(library::entry_path(dir.path(), "e")).unwrap();
        let (fm, _) = crate::md::parse_document(&saved).unwrap();
        assert_eq!(fm.favicon.as_deref(), Some("../metadata/favicons/icon"));
        let unrelated = |raw: &str| {
            raw.split_inclusive('\n')
                .filter(|line| !line.starts_with("favicon:"))
                .collect::<String>()
        };
        assert_eq!(unrelated(&saved), unrelated(&original));
        drop(lock);
        fs::remove_dir_all(dir.path().join(".xaga")).unwrap();
        let reopened = library::open(dir.path(), LibraryVersion::new(0, 17))
            .unwrap()
            .unwrap();
        assert!(
            library::index_path(dir.path())
                .try_exists()
                .is_ok_and(|exists| !exists)
        );
        assert!(
            LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .matches_version(LibraryVersion::new(0, 17))
        );
        assert!(library::acquire_lock(dir.path()).is_none());
        let db = crate::embeddings::EmbeddingsDb::open_writer(dir.path(), &reopened)
            .await
            .unwrap();
        assert_eq!(db.count().await.unwrap(), 0);
        assert!(library::index_path(dir.path()).is_dir());
        assert_eq!(
            fs::read_to_string(library::entry_path(dir.path(), "e")).unwrap(),
            saved
        );
        drop(db);
        drop(reopened);
    }

    #[tokio::test]
    async fn partial_move_retries_and_conflicts_preserve_both_copies() {
        let dir = tempfile::tempdir().unwrap();
        legacy(dir.path());
        let lock = library::acquire_lock(dir.path()).unwrap();
        let target = LibraryVersion::new(0, 17);
        let moved = library::feeds_dir(dir.path());
        super::super::upgrade::faults()
            .lock()
            .unwrap()
            .insert((moved, "layout_relocated".into()));
        let report = library::upgrade_library(dir.path(), &lock, target, |_| {}).await;
        assert!(!report.errors.is_empty());
        assert!(upgrade_required(dir.path()));
        assert_eq!(
            LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .migration_target(),
            Some(target)
        );
        fs::create_dir_all(library::favicons_dir(dir.path())).unwrap();
        fs::write(library::favicon_path(dir.path(), "conflict"), "retain").unwrap();
        let report = library::upgrade_library(dir.path(), &lock, target, |_| {}).await;
        assert!(!report.errors.is_empty());
        assert_eq!(
            fs::read_to_string(dir.path().join("favicons/icon")).unwrap(),
            "icon bytes"
        );
        assert_eq!(
            fs::read_to_string(library::favicon_path(dir.path(), "conflict")).unwrap(),
            "retain"
        );
        fs::remove_dir_all(library::favicons_dir(dir.path())).unwrap();
        let report = library::upgrade_library(dir.path(), &lock, target, |_| {}).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn legacy_writer_blocks_transition_and_transition_holds_both_locks() {
        let dir = tempfile::tempdir().unwrap();
        legacy(dir.path());
        let root = dir.path().join("xaga.lock");
        let old = FileLock::try_acquire(root.clone()).unwrap();
        assert!(library::acquire_lock(dir.path()).is_none());
        assert!(!dir.path().join(".xaga").exists());
        drop(old);
        let transition = library::acquire_lock(dir.path()).unwrap();
        assert!(FileLock::try_acquire(root).is_none());
        assert!(FileLock::try_acquire(dir.path().join(".xaga/xaga.lock")).is_none());
        drop(transition);
        assert!(library::acquire_lock(dir.path()).is_some());
    }
}

#[cfg(test)]
mod database_tests {
    use super::*;
    use crate::{
        embeddings::{EmbeddingsDb, db::EmbeddingEntry},
        library::{self, LibraryVersion},
    };

    #[tokio::test]
    async fn relocating_a_real_database_preserves_vectors_and_rows() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("library.yaml"), "format_version: '0.16'\n").unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let db = EmbeddingsDb::open_writer(dir.path(), &lock).await.unwrap();
        let (fm, body) = crate::md::parse_document(
            "---\nsource: https://example.com/\ntitle: Saved article\n---\n\nSaved body.\n",
        )
        .unwrap();
        let article = crate::persistence::articles::article_from_document("e", fm, body);
        let vector = vec![0.5; 384];
        db.upsert(EmbeddingEntry::from_article(
            &article,
            Some(vector.clone()),
            vec![],
        ))
        .await
        .unwrap();
        let before = db.get_entry("e").await.unwrap().unwrap();
        drop(db);
        fs::rename(
            library::index_path(dir.path()),
            dir.path().join("embeddings.lance"),
        )
        .unwrap();
        let report =
            library::upgrade_library(dir.path(), &lock, LibraryVersion::new(0, 17), |_| {}).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let db = EmbeddingsDb::open_readonly(dir.path()).await.unwrap();
        let after = db.get_entry("e").await.unwrap().unwrap();
        assert_eq!(after.title, before.title);
        assert_eq!(after.checksum, before.checksum);
        assert_eq!(after.vector, Some(vector));
    }

    #[tokio::test]
    async fn interruption_after_manifest_relocation_retains_source_and_resumes() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("library.yaml"), "format_version: '0.16'\n").unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let target = LibraryVersion::new(0, 17);
        super::super::upgrade::faults().lock().unwrap().insert((
            library::metadata_manifest_path(dir.path()),
            "layout_relocated".into(),
        ));
        let report = library::upgrade_library(dir.path(), &lock, target, |_| {}).await;
        assert!(!report.errors.is_empty());
        let manifest = library::LibraryManifest::read(dir.path()).unwrap().unwrap();
        assert_eq!(manifest.format_version(), Some(LibraryVersion::new(0, 16)));
        assert_eq!(manifest.migration_target(), Some(target));
        assert!(upgrade_required(dir.path()));
        let report = library::upgrade_library(dir.path(), &lock, target, |_| {}).await;
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(!upgrade_required(dir.path()));
    }

    #[test]
    fn conflicting_manifests_are_refused_without_changing_either() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("library.yaml");
        let new = library::metadata_manifest_path(dir.path());
        fs::create_dir_all(new.parent().unwrap()).unwrap();
        fs::write(&old, "format_version: '0.16'\n").unwrap();
        fs::write(&new, "format_version: '0.17'\n").unwrap();
        assert!(library::LibraryManifest::read(dir.path()).is_err());
        assert!(upgrade_required(dir.path()));
        assert_eq!(fs::read_to_string(old).unwrap(), "format_version: '0.16'\n");
        assert_eq!(fs::read_to_string(new).unwrap(), "format_version: '0.17'\n");
    }

    #[test]
    fn future_version_mismatches_do_not_require_the_layout_upgrade() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = library::metadata_manifest_path(dir.path());
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        fs::write(
            &manifest,
            "format_version: '0.17'\nmigration: {target_version: '0.18'}\n",
        )
        .unwrap();
        let status = library::LibraryStatus::read(dir.path(), LibraryVersion::new(0, 18), false);
        assert!(!status.layout_upgrade_required);
        assert!(status.can_upgrade());
    }
}
