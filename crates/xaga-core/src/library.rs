//! Xaga library layout: the structure of a library directory and the
//! library-level operations over it.
//!
//! Single source of truth for where things live inside a library — the subdir
//! names live here as consts and nowhere else, so persistence, recovery,
//! embeddings, and the app layers derive every path through these functions
//! rather than re-encoding the layout.

use std::path::{Path, PathBuf};

use crate::fs::FileLock;

// ── Layout ──────────────────────────────────────────────────────────────────────
// Subdir/index names live here once; `*_dir` and `*_path` both build from them.

const ENTRIES_SUBDIR: &str = "entries";
const TOPICS_SUBDIR: &str = "topics";
const FEEDS_SUBDIR: &str = "feeds";
const IMAGES_SUBDIR: &str = "images";
const INDEX_NAME: &str = "embeddings.lance";

/// The directory holding entry `.md` files.
#[inline]
pub fn entries_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(ENTRIES_SUBDIR)
}

/// The directory holding topic `.md` files.
#[inline]
pub fn topics_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(TOPICS_SUBDIR)
}

/// The directory holding feed `.atom` files.
#[inline]
pub fn feeds_dir(library_dir: &Path) -> PathBuf {
    library_dir.join(FEEDS_SUBDIR)
}

/// The LanceDB embeddings index directory.
#[inline]
pub fn index_path(library_dir: &Path) -> PathBuf {
    library_dir.join(INDEX_NAME)
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

/// Path to a single feed's `.atom` file (`<library>/feeds/<id>.atom`).
/// One-allocation, like [`entry_path`].
#[inline]
pub fn feed_path(library_dir: &Path, id: &str) -> PathBuf {
    library_dir.join(format!("{FEEDS_SUBDIR}/{id}.atom"))
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
/// `../images/<id>/<file_name>`, relative to the entry `.md` in `entries/`, so
/// the document resolves the image in any markdown editor. Built here and
/// nowhere else — the layout literal must not be re-encoded by callers.
#[inline]
pub fn entry_image_ref(id: &str, file_name: &str) -> String {
    format!("../{IMAGES_SUBDIR}/{id}/{file_name}")
}

/// Inverse of [`entry_image_ref`], and the read-path traversal guard. Parses a
/// body reference back to `(id, file_name)`, returning `None` unless it has the
/// exact `../images/<id>/<file_name>` shape and both components are safe single
/// path segments (no separators, `..`, or NUL). Anything else — an absolute
/// URL, a differently-shaped relative path, an attempted traversal — yields
/// `None` and is left untouched by the reader.
pub fn parse_entry_image_ref(reference: &str) -> Option<(&str, &str)> {
    let tail = reference.strip_prefix("../")?.strip_prefix(IMAGES_SUBDIR)?;
    let tail = tail.strip_prefix('/')?;
    let (id, file_name) = tail.split_once('/')?;
    if file_name.contains('/') {
        return None;
    }
    if crate::persistence::entries::is_safe_id(id)
        && crate::persistence::entries::is_safe_id(file_name)
    {
        Some((id, file_name))
    } else {
        None
    }
}

// ── Single-writer lock ──────────────────────────────────────────────────────────

/// Filename of the single-writer advisory lock at a library's root. Private and
/// single-sourced: it is the shared agreement that makes the GUI and CLI exclude
/// each other on the same library, so it must never be duplicated into a caller.
const LOCK_FILE_NAME: &str = "xaga.lock";

/// Acquires the single-writer lock for the library rooted at `library_dir`.
///
/// Returns `None` if another live process already holds it. The lock is released
/// automatically when the returned [`FileLock`] is dropped.
pub fn acquire_lock(library_dir: &Path) -> Option<FileLock> {
    FileLock::try_acquire(library_dir.join(LOCK_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_ref_round_trips_through_parse() {
        let reference = entry_image_ref("a3f2c1d0e9b8", "4f2a91c3e0b7.jpg");
        assert_eq!(reference, "../images/a3f2c1d0e9b8/4f2a91c3e0b7.jpg");
        assert_eq!(
            parse_entry_image_ref(&reference),
            Some(("a3f2c1d0e9b8", "4f2a91c3e0b7.jpg"))
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
}
