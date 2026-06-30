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
