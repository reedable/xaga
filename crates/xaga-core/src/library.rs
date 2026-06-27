//! Xaga library layout: the structure of a library directory and the
//! library-level operations over it.
//!
//! Today this owns only the single-writer lock convention. It is the intended
//! home for the rest of the library-relative path knowledge (entries/, topics/,
//! feeds/, the index dir) that currently lives scattered across persistence,
//! recovery, and the app layers — to be consolidated here as a separate effort.

use std::path::{Path, PathBuf};

use crate::fs::FileLock;

// ── Layout: where each area of a library lives ──────────────────────────────────
// Single source of truth for the library's directory structure. Persistence,
// recovery, embeddings, and the app layers all derive paths through these rather
// than re-encoding the subdir names.

/// The directory holding entry `.md` files.
pub fn entries_dir(library_dir: &Path) -> PathBuf {
    library_dir.join("entries")
}

/// The directory holding topic `.md` files.
pub fn topics_dir(library_dir: &Path) -> PathBuf {
    library_dir.join("topics")
}

/// The directory holding feed `.atom` files.
pub fn feeds_dir(library_dir: &Path) -> PathBuf {
    library_dir.join("feeds")
}

/// The LanceDB embeddings index directory.
pub fn index_path(library_dir: &Path) -> PathBuf {
    library_dir.join("embeddings.lance")
}

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
