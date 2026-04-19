use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use fs2::FileExt;

/// A file-based exclusive lock that grants the holder write access to the
/// embeddings database for a given library.
///
/// ## Acquisition
///
/// `try_acquire` opens the lock file and requests a non-blocking exclusive
/// advisory lock from the OS.  Only one process can hold the lock at a time.
///
/// ## Automatic release
///
/// The lock is tied to the open file handle, not to the file contents.  The OS
/// releases it automatically when the handle is dropped — including on crashes,
/// hard kills, or any other abnormal exit.  There are no stale lock files to
/// clean up manually.
///
/// ## Platform notes
///
/// Uses `flock(2)` on Unix (macOS, Linux) and `LockFileEx` on Windows via the
/// `fs2` crate.  Semantics — exclusive, non-blocking, released on process
/// death — are consistent across all three platforms.
pub struct LockFile {
    // The lock is held for as long as this handle is alive.
    _file: File,
}

impl LockFile {
    /// Returns the canonical path for the lock file inside `library_dir`.
    pub fn path_for(library_dir: &Path) -> PathBuf {
        library_dir.join("xaga.lock")
    }

    /// Attempts to acquire an exclusive advisory lock on `path`.
    ///
    /// Returns `Some(LockFile)` if the lock was granted, or `None` if another
    /// live process already holds it.  The lock is released automatically when
    /// the returned value is dropped.
    pub fn try_acquire(path: PathBuf) -> Option<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .ok()?;

        file.try_lock_exclusive().ok()?;

        Some(Self { _file: file })
    }
}
