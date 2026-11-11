//! Xaga library layout: the structure of a library directory and the
//! library-level operations over it.
//!
//! Single source of truth for where things live inside a library — the subdir
//! names live here as consts and nowhere else, so persistence, recovery,
//! embeddings, and the app layers derive every path through these functions
//! rather than re-encoding the layout.

use std::path::{Path, PathBuf};

use crate::fs::FileLock;

mod layout;
mod manifest;
mod status;
pub use status::{LibraryReadOnlyCause, LibraryStatus};
mod upgrade;
pub use manifest::{LibraryFormatError, LibraryManifest, LibraryUpgrade, LibraryVersion};
pub use upgrade::{UpgradePhase, UpgradeProgress, UpgradeReport, upgrade_library};

const METADATA_SUBDIR: &str = "metadata";
const INTERNAL_SUBDIR: &str = ".xaga";

fn metadata_manifest_path(library_dir: &Path) -> PathBuf {
    library_dir.join(METADATA_SUBDIR).join(MANIFEST_NAME)
}

const MANIFEST_NAME: &str = "library.yaml";
fn manifest_path(library_dir: &Path) -> PathBuf {
    let legacy = library_dir.join(MANIFEST_NAME);
    if std::fs::symlink_metadata(&legacy).is_ok() {
        legacy
    } else {
        metadata_manifest_path(library_dir)
    }
}

// The completed manifest is already committed when the obsolete lock is retired.
// Cleanup failure must not turn a completed conversion into an unresumable error.
fn retire_legacy_lock(dir: &Path) {
    let result = std::fs::remove_file(dir.join(LOCK_FILE_NAME))
        .and_then(|_| std::fs::File::open(dir)?.sync_all());
    if let Err(error) = result {
        if error.kind() != std::io::ErrorKind::NotFound {
            log::warn!("could not retire obsolete library lock: {error}");
        }
    }
}

/// Select write mode, initializing a new empty directory at the app version.
/// Existing unversioned, mismatched and incomplete libraries remain read-only.
pub fn open(
    library_dir: &Path,
    app_version: LibraryVersion,
) -> Result<Option<FileLock>, LibraryFormatError> {
    // Only the library lock may exist in a new directory (including after a
    // previous initialization attempt). Never infer a format for existing data.
    let is_empty = || -> Result<bool, LibraryFormatError> {
        let entries = match std::fs::read_dir(library_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let entry = entry?;
            if (entry.file_name() == ".xaga" || entry.file_name() == "metadata")
                && entry.file_type()?.is_dir()
            {
                for internal in std::fs::read_dir(entry.path())? {
                    if internal?.file_name() != LOCK_FILE_NAME || entry.file_name() == "metadata" {
                        return Ok(false);
                    }
                }
                continue;
            }
            if entry.file_name() != LOCK_FILE_NAME {
                return Ok(false);
            }
        }
        Ok(true)
    };
    match LibraryManifest::read(library_dir)? {
        Some(manifest) if !manifest.matches_version(app_version) => return Ok(None),
        None if !is_empty()? => return Ok(None),
        _ => {}
    }
    let Some(lock) = acquire_lock(library_dir) else {
        return Ok(None);
    };
    // Recheck inside the write right: another opener or upgrade may have raced us.
    match LibraryManifest::read(library_dir)? {
        Some(manifest) if manifest.matches_version(app_version) => {
            if app_version >= LibraryVersion::new(0, 17) {
                retire_legacy_lock(library_dir);
            }
            Ok(Some(lock))
        }
        None if is_empty()? => {
            LibraryManifest::initialize(library_dir, app_version, &lock)?;
            if app_version >= LibraryVersion::new(0, 17) {
                retire_legacy_lock(library_dir);
            }
            Ok(Some(lock))
        }
        _ => Ok(None),
    }
}

// ── Layout ──────────────────────────────────────────────────────────────────────
// Subdir/index names live here once; `*_dir` and `*_path` both build from them.

const ENTRIES_SUBDIR: &str = "entries";
const TOPICS_SUBDIR: &str = "topics";
const FEEDS_SUBDIR: &str = "feeds";
const IMAGES_SUBDIR: &str = "images";
const FAVICONS_SUBDIR: &str = "favicons";
const PDFS_SUBDIR: &str = "pdfs";
const TAGS_NAME: &str = "tags.yaml";

/// The library-scoped tag definitions and transition store.
pub fn tags_path(library_dir: &Path) -> PathBuf {
    library_dir.join(METADATA_SUBDIR).join(TAGS_NAME)
}

const PINS_NAME: &str = "pins.yaml";

/// The library-scoped ordered pin store.
pub fn pins_path(library_dir: &Path) -> PathBuf {
    library_dir.join(METADATA_SUBDIR).join(PINS_NAME)
}

const INDEX_NAME: &str = "embeddings.lance";

/// The directory holding entry `.md` files.
#[inline]
pub fn entries_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(ENTRIES_SUBDIR)
}

/// The directory holding stored PDFs and their dormant Markdown companions.
#[inline]
pub fn pdfs_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(PDFS_SUBDIR)
}

/// Path to a stored PDF (`<library>/pdfs/<hash12>.pdf`).
#[inline]
pub fn pdf_path(library_dir: &Path, hash12: &str) -> PathBuf {
    library_dir.join(format!("{PDFS_SUBDIR}/{hash12}.pdf"))
}

/// Path to a stored PDF's dormant Entry-format companion
/// (`<library>/pdfs/<hash12>.md`).
#[inline]
pub fn pdf_entry_path(library_dir: &Path, hash12: &str) -> PathBuf {
    library_dir.join(format!("{PDFS_SUBDIR}/{hash12}.md"))
}

/// The directory holding topic `.md` files.
#[inline]
pub fn topics_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(TOPICS_SUBDIR)
}

/// The directory holding feed `.atom` files.
#[inline]
pub fn feeds_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(METADATA_SUBDIR).join(FEEDS_SUBDIR)
}

/// The LanceDB embeddings index directory.
#[inline]
pub fn index_path(library_dir: &Path) -> PathBuf {
    library_dir.join(INTERNAL_SUBDIR).join(INDEX_NAME)
}

/// Path to a single entry's `.md` file (`<library>/entries/<hash12>.md`).
///
/// Built in one allocation — the combined `entries/<hash12>.md` tail is formatted
/// once and joined once, rather than materialising the intermediate `entries/`
/// directory. That extra allocation is avoided per call, which matters in the
/// per-entry recovery loops. (Embeds the `/` separator, which Rust's `Path`
/// accepts on every platform.)
#[inline]
pub fn entry_path(library_dir: &Path, hash12: &str) -> PathBuf {
    library_dir.join(format!("{ENTRIES_SUBDIR}/{hash12}.md"))
}

/// Path to a single topic's `.md` file (`<library>/topics/<id>.md`).
/// One-allocation, like [`entry_path`].
#[inline]
pub fn topic_path(library_dir: &Path, id: &str) -> PathBuf {
    library_dir.join(format!("{TOPICS_SUBDIR}/{id}.md"))
}

/// Path to a single feed's `.atom` file (`<library>/metadata/feeds/<id>.atom`).
/// One-allocation, like [`entry_path`].
#[inline]
pub fn feed_path(library_dir: &Path, id: &str) -> PathBuf {
    library_dir.join(format!("{METADATA_SUBDIR}/{FEEDS_SUBDIR}/{id}.atom"))
}

// ── Entry image assets ────────────────────────────────────────────────────────
// Images extracted from an entry's body at save time live in a per-entry
// directory under a sibling `images/` tree — deliberately NOT under `entries/`.
// A subdirectory inside `entries/` would be visited by `fs::for_each_file`, and
// one named `<id>.md/` would fail as EISDIR and degrade the walk to
// `WalkOutcome::Unreadable`, which suppresses recovery's absence-based deletes.
// A sibling `images/` is opened by no existing scan at all, so stray or orphan
// image files can never perturb entry enumeration.
//
// Entry deletion does not exist in the codebase yet. When it is added, it must
// delete the entry `.md` first, then `remove_dir_all` the image directory: that
// order leaves at most orphan images (benign, swept by GC) rather than a live
// `.md` whose references point at deleted bytes.

/// The directory holding all per-entry image subdirectories.
#[inline]
pub fn images_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(IMAGES_SUBDIR)
}

/// The directory holding shared favicon resources.
#[inline]
pub fn favicons_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(METADATA_SUBDIR).join(FAVICONS_SUBDIR)
}

/// The canonical extensionless file for a normalized favicon URL identity.
#[inline]
pub fn favicon_path(library_dir: &Path, hash: &str) -> PathBuf {
    library_dir.join(format!("{METADATA_SUBDIR}/{FAVICONS_SUBDIR}/{hash}"))
}

/// The reference stored in article frontmatter for a shared favicon.
#[inline]
pub fn favicon_ref(hash: &str) -> String {
    format!("../{METADATA_SUBDIR}/{FAVICONS_SUBDIR}/{hash}")
}

/// Parses a shared favicon reference and rejects traversal or foreign paths.
pub fn parse_favicon_ref(reference: &str) -> Option<&str> {
    let hash = reference
        .strip_prefix("../")?
        .strip_prefix(METADATA_SUBDIR)?
        .strip_prefix('/')?
        .strip_prefix(FAVICONS_SUBDIR)?
        .strip_prefix('/')?;
    crate::persistence::entries::is_safe_id(hash).then_some(hash)
}

/// The directory holding one entry's extracted images
/// (`<library>/images/<id>`). One-allocation, like [`entry_path`].
#[inline]
pub fn entry_images_dir(library_dir: &Path, id: &str) -> PathBuf {
    library_dir.join(format!("{IMAGES_SUBDIR}/{id}"))
}

/// Path to a single stored image (`<library>/images/<id>/<file_name>`).
/// One-allocation, like [`entry_path`].
#[inline]
pub fn entry_image_path(library_dir: &Path, id: &str, file_name: &str) -> PathBuf {
    library_dir.join(format!("{IMAGES_SUBDIR}/{id}/{file_name}"))
}

/// The reference form written into an entry's markdown body:
/// `../images/<id>/<hash>`, relative to the entry `.md` in `entries/`.
///
/// Extension-agnostic on purpose: the `hash` is the url-hash, while the actual
/// file on disk is `<hash>.<ext>` (the extension comes from the fetched media
/// type, unknown for a failed fetch). Keeping the body ref ext-less means a
/// failed image and a fetched one share the same ref shape, and recovery can
/// drop in `<hash>.<ext>` later without rewriting the body. The file is located
/// from this hash at read time. Built here and nowhere else.
#[inline]
pub fn entry_image_ref(id: &str, hash: &str) -> String {
    format!("../{IMAGES_SUBDIR}/{id}/{hash}")
}

/// Inverse of [`entry_image_ref`], and the read-path traversal guard. Parses a
/// body reference back to `(id, hash)`, returning `None` unless it has the exact
/// `../images/<id>/<hash>` shape and both components are safe single path
/// segments (no separators, `..`, or NUL). Anything else — an absolute URL, a
/// differently-shaped relative path, an attempted traversal — yields `None` and
/// is left untouched by the reader.
pub fn parse_entry_image_ref(reference: &str) -> Option<(&str, &str)> {
    let tail = reference.strip_prefix("../")?.strip_prefix(IMAGES_SUBDIR)?;
    let tail = tail.strip_prefix('/')?;
    let (id, hash) = tail.split_once('/')?;
    if hash.contains('/') {
        return None;
    }
    if crate::persistence::entries::is_safe_id(id) && crate::persistence::entries::is_safe_id(hash)
    {
        Some((id, hash))
    } else {
        None
    }
}

// ── Single-writer lock ──────────────────────────────────────────────────────────

/// Filename of the single-writer advisory lock inside a library's `.xaga/` directory. Private and
/// single-sourced: it is the shared agreement that makes the GUI and CLI exclude
/// each other on the same library, so it must never be duplicated into a caller.
const LOCK_FILE_NAME: &str = "xaga.lock";

/// Acquires the single-writer lock for the library rooted at `library_dir`.
///
/// Returns `None` if another live process already holds it. The lock is released
/// automatically when the returned [`FileLock`] is dropped.
pub fn acquire_lock(library_dir: &Path) -> Option<FileLock> {
    let legacy_path = library_dir.join(LOCK_FILE_NAME);
    let legacy = if legacy_path.exists()
        || library_dir.join(MANIFEST_NAME).exists()
        || (!metadata_manifest_path(library_dir).exists()
            && std::fs::read_dir(library_dir)
                .ok()?
                .any(|entry| entry.is_ok_and(|entry| entry.file_name() != ".xaga")))
    {
        Some(FileLock::try_acquire(legacy_path)?)
    } else {
        None
    };
    std::fs::create_dir_all(library_dir.join(INTERNAL_SUBDIR)).ok()?;
    let mut lock = FileLock::try_acquire(library_dir.join(INTERNAL_SUBDIR).join(LOCK_FILE_NAME))?;
    lock.retain(legacy);
    Some(lock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_app_and_cli_writers_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let app_lock = acquire_lock(dir.path()).expect("first writer acquires lock");

        assert!(acquire_lock(dir.path()).is_none());

        drop(app_lock);
        assert!(acquire_lock(dir.path()).is_some());
    }

    #[test]
    fn image_ref_round_trips_through_parse() {
        // The ref is extension-agnostic: it carries the url-hash, not a filename.
        let reference = entry_image_ref("a3f2c1d0e9b8", "4f2a91c3e0b7");
        assert_eq!(reference, "../images/a3f2c1d0e9b8/4f2a91c3e0b7");
        assert_eq!(
            parse_entry_image_ref(&reference),
            Some(("a3f2c1d0e9b8", "4f2a91c3e0b7"))
        );
    }

    #[test]
    fn parse_rejects_traversal_and_foreign_shapes() {
        // Each of these must yield None — the reader leaves them untouched
        // rather than reading a file, so none can escape the images tree.
        let rejected = [
            "../images/../../entries/x.md",     // parent-escape in the id slot
            "../images/a3f2c1/../../../etc/pw", // parent-escape in the file slot
            "../images/a3f2c1",                 // missing file component
            "../images/a3f2c1/",                // empty file component
            "../images//x.jpg",                 // empty id component
            "../images/a/b/c.jpg",              // extra path segment
            "images/a3f2c1/x.jpg",              // missing ../ prefix
            "../imagesx/a3f2c1/x.jpg",          // prefix is a different subdir
            "https://evil.test/x.jpg",          // absolute URL
            "data:image/png;base64,AAAA",       // data URI
            "/images/a3f2c1/x.jpg",             // root-relative
        ];
        for reference in rejected {
            assert_eq!(
                parse_entry_image_ref(reference),
                None,
                "expected None for {reference:?}"
            );
        }
    }

    #[test]
    fn parse_rejects_backslash_and_nul_components() {
        assert_eq!(parse_entry_image_ref("../images/a\\b/x.jpg"), None);
        assert_eq!(parse_entry_image_ref("../images/a3f2c1/x\0.jpg"), None);
    }

    #[test]
    fn shared_favicon_ref_round_trips_and_rejects_traversal() {
        let reference = favicon_ref("4f2a91c3e0b7");
        assert_eq!(parse_favicon_ref(&reference), Some("4f2a91c3e0b7"));
        assert_eq!(
            parse_favicon_ref("../metadata/favicons/../../entries/x.md"),
            None
        );
        assert_eq!(parse_favicon_ref("../metadata/favicons/a/b"), None);
    }
}
