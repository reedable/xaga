//! Filesystem helpers shared across the crate.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fs2::FileExt;

/// Atomically writes `bytes` to `path`: writes to a unique sibling temp file,
/// fsyncs it, then renames it over `path`. `rename(2)` is atomic, so a reader
/// never observes a partial file, and a crash mid-write leaves the previous
/// contents intact rather than a truncated file. Missing parent directories are
/// created. Both the temp file and the parent directory are fsynced so the
/// replacement survives power loss.
///
/// Concurrent writers to the same `path` each use a distinct temp file (pid +
/// counter), so they never corrupt a shared buffer; the last `rename` to
/// complete wins, and the result is always one writer's complete bytes.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = unique_tmp_path(path);

    // Scope the handle so the file is closed before the rename.
    {
        let mut f = File::create(&tmp_path)?;
        f.write_all(bytes)?;
        f.sync_all()?; // fsync the temp file's contents to disk
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path); // best-effort cleanup on a failed rename
        return Err(e);
    }

    // fsync the parent directory so the rename itself is durable. Best-effort:
    // opening a directory for sync is not supported on every platform (e.g.
    // Windows), where directory durability is handled differently.
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(())
}

/// Async wrapper around [`write_atomic`] for callers on the tokio runtime. The
/// blocking filesystem work runs on tokio's blocking thread pool via
/// `spawn_blocking`, so it never stalls an async executor thread. Takes owned
/// arguments because the spawned task must be `'static`.
pub(crate) async fn write_atomic_async(path: PathBuf, bytes: Vec<u8>) -> std::io::Result<()> {
    match tokio::task::spawn_blocking(move || write_atomic(&path, &bytes)).await {
        Ok(res) => res,
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

/// A unique sibling temp path for `path`, e.g. `settings.json` →
/// `settings.json.<pid>.<n>.tmp` in the same directory (same filesystem, so the
/// rename stays atomic). The pid + monotonic counter keep concurrent writers
/// from sharing one temp file.
fn unique_tmp_path(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();

    let mut name = path
        .file_name()
        .map(|f| f.to_os_string())
        .unwrap_or_default();

    name.push(format!(".{pid}.{n}.tmp"));

    match path.parent().filter(|p| !p.as_os_str().is_empty()) {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}

/// Visits every file in `dir` whose extension matches `ext`, reading each to a `String` and handing
/// `(path, contents)` to `visit`. Yields between files so a large scan never starves higher-priority
/// tasks. A missing directory, an unreadable directory entry, or a file that fails to read (e.g.
/// non-UTF-8) is skipped. Pure traversal — no knowledge of file format or contents.
pub(crate) async fn for_each_file<F>(dir: &Path, ext: &str, mut visit: F)
where
    F: FnMut(PathBuf, String),
{
    if !dir.exists() {
        return;
    }

    let mut dir_entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return,
    };

    while let Ok(Some(dir_entry)) = dir_entries.next_entry().await {
        let path = dir_entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }

        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(r) => r,
            Err(_) => continue,
        };

        visit(path, raw);

        tokio::task::yield_now().await;
    }
}

/// An exclusive advisory lock on a file, used to guarantee a single writer.
///
/// `try_acquire` opens the path and requests a non-blocking exclusive advisory
/// lock from the OS — `flock(2)` on Unix (macOS, Linux) and `LockFileEx` on
/// Windows, via the `fs2` crate. Only one process can hold it at a time.
///
/// The lock is tied to the open file handle, not the file contents, so the OS
/// releases it automatically when the handle is dropped — including on crashes,
/// hard kills, or any other abnormal exit. There are no stale lock files to clean
/// up manually.
pub struct FileLock {
    // The lock is held for as long as this handle is alive.
    _file: File,
}

impl FileLock {
    /// Attempts to acquire an exclusive advisory lock on `path`.
    ///
    /// Returns `Some(FileLock)` if the lock was granted, or `None` if another
    /// live process already holds it. The lock is released automatically when the
    /// returned value is dropped.
    pub(crate) fn try_acquire(path: PathBuf) -> Option<Self> {
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
