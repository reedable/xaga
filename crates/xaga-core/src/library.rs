//! Xaga library layout: the structure of a library directory and the
//! library-level operations over it.
//!
//! Today this owns only the single-writer lock convention. It is the intended
//! home for the rest of the library-relative path knowledge (entries/, topics/,
//! feeds/, the index dir) that currently lives scattered across persistence,
//! recovery, and the app layers — to be consolidated here as a separate effort.

use std::path::Path;

use crate::fs::FileLock;

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
