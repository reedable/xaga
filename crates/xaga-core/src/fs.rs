//! Filesystem helpers shared across the crate.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fs2::FileExt;

/// The per-user xaga data root, e.g. `~/Library/Application Support/xaga` on
/// macOS. The single derivation of "platform data dir + app dir name": every
/// consumer — the embedding model dir, the CLI's default data dir, the app's
/// settings location — must obtain the root from here rather than deriving
/// its own, so the name cannot drift between call sites (Single Source of
/// Truth). `None` when the platform data dir is undeterminable; callers own
/// their fallback.
pub fn data_root() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("xaga"))
}

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

/// Replaces `dest` with the staged directory `staging`; [`write_atomic`] for
/// directories.
///
/// Crash-safe: files are fsynced before the rename. `dest` never holds
/// truncated files.
///
/// WARNING: replacing an existing `dest` is not atomic. A directory rename
/// cannot replace a non-empty target, so `dest` is removed first — readers
/// can see it absent. Creating a new `dest` is atomic.
///
/// On failure `staging` is left in place; the caller cleans it up.
pub(crate) fn replace_dir_atomic(staging: &Path, dest: &Path) -> std::io::Result<()> {
    sync_dir_contents(staging)?;

    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }

    fs::rename(staging, dest)?;

    // Same best-effort parent fsync as `write_atomic`: makes the rename itself
    // durable where the platform supports opening a directory for sync.
    if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(())
}

/// Async wrapper around [`replace_dir_atomic`], mirroring [`write_atomic_async`]:
/// the blocking fsync/rename work runs on tokio's blocking thread pool.
pub(crate) async fn replace_dir_atomic_async(
    staging: PathBuf,
    dest: PathBuf,
) -> std::io::Result<()> {
    match tokio::task::spawn_blocking(move || replace_dir_atomic(&staging, &dest)).await {
        Ok(res) => res,
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

/// Fsyncs the contents of every file under `dir`, recursively. The durability
/// half of [`replace_dir_atomic`]: without it, the rename can outlive a power
/// loss that the file contents do not.
fn sync_dir_contents(dir: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            sync_dir_contents(&path)?;
        } else {
            File::open(&path)?.sync_all()?;
        }
    }
    Ok(())
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

/// Whether a directory walk reliably observed the directory's contents.
///
/// Distinguishes a *trustworthy* view of the source ("the directory is absent or genuinely holds
/// these files") from an *incomplete* one ("the directory could not be read"). The distinction
/// matters to any caller that deletes based on absence: recovery treats a walk as authoritative for
/// pruning derived rows, so a permission/IO failure must never masquerade as "empty" — that would
/// delete the rebuildable mirror it should be rebuilding from (Single Source of Truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkOutcome {
    /// The directory was fully enumerated (it may legitimately hold zero matching files, or be
    /// absent). Every matching file was read. A trustworthy view — deletes derived from absence are
    /// safe.
    Enumerated,
    /// The directory could not be fully observed: `read_dir` failed (e.g. denied by the OS),
    /// iteration was truncated by an error, or a matching file existed but could not be read. The
    /// set of ids seen is incomplete; callers must not treat unseen ids as deleted.
    Unreadable,
}

/// A `.md` over this limit is not read. The cap bounds the app's runtime memory
/// use, and applies to hostile and legitimate files alike.
pub(crate) const MAX_MD_BYTES: u64 = 12 * 1024 * 1024;

/// Why a size-capped `.md` read failed.
#[derive(Debug)]
pub(crate) enum ReadMdError {
    /// The file is larger than [`MAX_MD_BYTES`]; it was not read.
    TooLarge {
        max_bytes: u64,
    },
    Io(std::io::Error),
}

impl From<std::io::Error> for ReadMdError {
    fn from(e: std::io::Error) -> Self {
        ReadMdError::Io(e)
    }
}

/// Reads a `.md`, rejecting one over [`MAX_MD_BYTES`]. Enforced on the read
/// (reads at most the cap), not via `metadata` stat.
pub(crate) async fn read_md_capped(path: &Path) -> Result<String, ReadMdError> {
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path).await?;
    let mut buf = Vec::new();

    file.take(MAX_MD_BYTES + 1).read_to_end(&mut buf).await?;

    if buf.len() as u64 > MAX_MD_BYTES {
        return Err(ReadMdError::TooLarge {
            max_bytes: MAX_MD_BYTES,
        });
    }

    String::from_utf8(buf)
        .map_err(|e| ReadMdError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
}

/// Visits every file in `dir` whose extension matches `ext`, reading each to a `String` and handing
/// `(path, contents)` to `visit`. Yields between files so a large scan never starves higher-priority
/// tasks. A missing directory, an unreadable entry, a file that fails to read (e.g. non-UTF-8), or a
/// `.md` over [`MAX_MD_BYTES`] is skipped. Pure traversal — no knowledge of file format or contents.
pub(crate) async fn for_each_file<F>(dir: &Path, ext: &str, mut visit: F) -> WalkOutcome
where
    F: FnMut(PathBuf, String),
{
    // A missing directory is a trustworthy empty; any *other* open error (permission denied, etc.)
    // leaves the contents unknown. `exists()` is deliberately not used here — it also returns false
    // on a permission error, which would collapse "denied" into "absent" and re-open the hole.
    let mut dir_entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return WalkOutcome::Enumerated,
        Err(_) => return WalkOutcome::Unreadable,
    };

    let mut outcome = WalkOutcome::Enumerated;

    loop {
        let dir_entry = match dir_entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            // Iteration failed partway: the remaining entries are unseen, so the view is incomplete.
            Err(_) => {
                outcome = WalkOutcome::Unreadable;
                break;
            }
        };

        let path = dir_entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }

        // `.md` reads go through the size-bounded reader; `.atom` reads are not
        // capped here. A file we could not read — including a `.md` over the cap
        // — leaves its id unaccounted for: keep walking, but flag the view
        // incomplete so absence-based deletes are withheld.
        let read = if ext == "md" {
            read_md_capped(&path).await.map_err(|_| ())
        } else {
            tokio::fs::read_to_string(&path).await.map_err(|_| ())
        };
        let raw = match read {
            Ok(r) => r,
            Err(()) => {
                outcome = WalkOutcome::Unreadable;
                continue;
            }
        };

        visit(path, raw);

        tokio::task::yield_now().await;
    }

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn read_md_capped_rejects_oversize_and_reads_small() {
        let tmp = tempfile::tempdir().unwrap();

        let big = tmp.path().join("big.md");
        tokio::fs::write(&big, vec![b'x'; (MAX_MD_BYTES + 1) as usize])
            .await
            .unwrap();
        assert!(matches!(
            read_md_capped(&big).await,
            Err(ReadMdError::TooLarge { .. })
        ));

        let small = tmp.path().join("small.md");
        tokio::fs::write(&small, b"hello").await.unwrap();
        assert_eq!(read_md_capped(&small).await.unwrap(), "hello");
    }

    #[tokio::test]
    async fn walk_skips_oversize_md_and_withholds_deletes() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("ok.md"), b"---\ntitle: T\n---\n\nx\n")
            .await
            .unwrap();
        tokio::fs::write(
            tmp.path().join("big.md"),
            vec![b'x'; (MAX_MD_BYTES + 1) as usize],
        )
        .await
        .unwrap();

        let mut seen = 0usize;
        let outcome = for_each_file(tmp.path(), "md", |_, _| seen += 1).await;
        // The oversize file is skipped (not visited) and the view is incomplete,
        // so absence-based deletes are withheld.
        assert_eq!(seen, 1);
        assert_eq!(outcome, WalkOutcome::Unreadable);
    }

    #[tokio::test]
    async fn walk_stays_enumerated_with_stray_non_md_files() {
        // Baseline for the entry-image feature: files that are not the requested
        // extension — stored images, and `write_atomic`'s `.tmp` leftovers — must
        // be filtered out by extension BEFORE any read, so they neither get
        // visited nor degrade the outcome. Contrast with the oversize-`.md` case
        // above, which is deliberately `Unreadable`. If this ever flips to
        // `Unreadable`, a stray file could suppress recovery's absence-deletes.
        let tmp = tempfile::tempdir().unwrap();
        tokio::fs::write(tmp.path().join("ok.md"), b"---\ntitle: T\n---\n\nx\n")
            .await
            .unwrap();
        // Non-UTF-8 bytes on purpose: these must never reach `read_to_string`.
        tokio::fs::write(tmp.path().join("photo.jpg"), [0xFF, 0xD8, 0xFF, 0x00])
            .await
            .unwrap();
        tokio::fs::write(tmp.path().join("chart.png"), [0x89, b'P', b'N', b'G'])
            .await
            .unwrap();
        tokio::fs::write(tmp.path().join("ok.md.1234.0.tmp"), b"partial")
            .await
            .unwrap();

        let mut seen = 0usize;
        let outcome = for_each_file(tmp.path(), "md", |_, _| seen += 1).await;
        assert_eq!(seen, 1);
        assert_eq!(outcome, WalkOutcome::Enumerated);
    }

    #[test]
    fn replace_dir_atomic_creates_dest_and_consumes_staging() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("model-download");
        fs::create_dir_all(staging.join("nested")).unwrap();
        fs::write(staging.join("model.onnx"), b"weights").unwrap();
        fs::write(staging.join("nested").join("extra.json"), b"{}").unwrap();
        let dest = tmp.path().join("model");

        replace_dir_atomic(&staging, &dest).unwrap();

        assert!(!staging.exists());
        assert_eq!(fs::read(dest.join("model.onnx")).unwrap(), b"weights");
        assert_eq!(fs::read(dest.join("nested/extra.json")).unwrap(), b"{}");
    }

    #[test]
    fn replace_dir_atomic_replaces_existing_dest() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("new.txt"), b"new").unwrap();
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("stale.txt"), b"stale").unwrap();

        replace_dir_atomic(&staging, &dest).unwrap();

        assert!(!dest.join("stale.txt").exists());
        assert_eq!(fs::read(dest.join("new.txt")).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[test]
    fn try_acquire_lands_on_the_current_inode_after_rename_over() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("lockfile");
        fs::write(&path, b"v1").unwrap();

        // Hold a lock on the original inode via a raw handle (not the
        // recheck-protected path), then rename a new inode over the path —
        // the situation a waiter can find itself in.
        let old_handle = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        old_handle.try_lock_exclusive().unwrap();
        let replacement = tmp.path().join("replacement");
        fs::write(&replacement, b"v2").unwrap();
        fs::rename(&replacement, &path).unwrap();

        // The old handle's lock is now stale: it guards an unlinked inode.
        assert!(!is_path_inode(&old_handle, &path));

        // try_acquire must succeed on the *new* inode despite the old lock
        // still being held — they are different inodes.
        let lock = FileLock::try_acquire(path.clone()).expect("acquire new inode");
        assert!(is_path_inode(&lock._file, &path));
    }

    #[cfg(unix)]
    #[test]
    fn write_locked_keeps_the_path_locked_by_the_holder() {
        use std::os::unix::fs::MetadataExt;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(&path, b"old").unwrap();

        let lock = FileLock::try_acquire(path.clone()).unwrap();
        let new_lock = write_locked(lock, &path, b"new")
            .map_err(|(e, _)| e)
            .unwrap();

        // Continuity: the returned lock's inode is the path's inode.
        assert!(is_path_inode(&new_lock._file, &path));
        assert_eq!(fs::read(&path).unwrap(), b"new");
        // The tmp is gone (renamed into place).
        assert!(!path.with_file_name("settings.json.tmp").exists());
        // And a contender cannot acquire while the holder lives.
        let contender = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            contender.metadata().unwrap().ino(),
            new_lock._file.metadata().unwrap().ino()
        );
        assert!(contender.try_lock_exclusive().is_err());
    }

    #[test]
    fn write_locked_reclaims_a_crashed_writers_tmp() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(&path, b"old").unwrap();
        // Junk left by a hypothetical crashed writer.
        fs::write(tmp.path().join("settings.json.tmp"), b"corrupt-garbage").unwrap();

        let lock = FileLock::try_acquire(path.clone()).unwrap();
        let _lock = write_locked(lock, &path, b"new")
            .map_err(|(e, _)| e)
            .unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn locked_read_modify_write_loses_no_updates() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("counter");
        fs::write(&path, b"0").unwrap();

        const THREADS: usize = 2;
        const ITERS: usize = 50;
        let mut handles = Vec::new();
        for _ in 0..THREADS {
            let path = path.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..ITERS {
                    // Funnel pattern: acquire → load inside the hold →
                    // mutate → write_locked → release.
                    let lock = loop {
                        match FileLock::try_acquire(path.clone()) {
                            Some(l) => break l,
                            None => std::thread::yield_now(),
                        }
                    };
                    let n: u64 = String::from_utf8(fs::read(&path).unwrap())
                        .unwrap()
                        .parse()
                        .unwrap();
                    let _lock = write_locked(lock, &path, (n + 1).to_string().as_bytes())
                        .map_err(|(e, _)| e)
                        .unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        let final_n: u64 = String::from_utf8(fs::read(&path).unwrap())
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(final_n, (THREADS * ITERS) as u64);
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
    ///
    /// A granted lock is verified against the path before being returned: flock
    /// binds to the inode behind the handle, so a contender can win a lock on an
    /// inode that was renamed away (see [`write_locked`]) between its open and
    /// its flock. On mismatch the stale lock is dropped and acquisition retries
    /// on whatever the path names now.
    pub fn try_acquire(path: PathBuf) -> Option<Self> {
        loop {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .ok()?;

            file.try_lock_exclusive().ok()?;

            if is_path_inode(&file, &path) {
                return Some(Self { _file: file });
            }
            // Stale: the path was renamed-over while we acquired. Drop (which
            // releases the lock) and retry on the new inode.
        }
    }
}

/// Whether `file`'s inode is the one `path` currently names. Unix-only
/// identity check; on other platforms this returns `true` (no stable inode
/// identity API) — recorded in the cross-platform spike.
#[cfg(unix)]
fn is_path_inode(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (file.metadata(), fs::metadata(path)) {
        (Ok(held), Ok(at_path)) => held.dev() == at_path.dev() && held.ino() == at_path.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn is_path_inode(_file: &File, _path: &Path) -> bool {
    true
}

/// The account-wide xaga resource lock. The lock file is `settings.json`
/// under [`data_root`]: holding its flock grants the right to write
/// settings.json, download the embedding model, and use any future
/// account-wide resource. Non-blocking: `None` means another live process
/// holds it (or the data root is undeterminable).
pub fn try_acquire_account_lock() -> Option<FileLock> {
    FileLock::try_acquire(data_root()?.join("settings.json"))
}

/// Writes `bytes` to `path` while holding `held` — the lock on `path`'s
/// current inode — and returns the lock on the *new* inode, so the caller's
/// exclusive hold survives the replacement without a gap.
///
/// Protocol (ordering is load-bearing):
/// 1. open `<path>.tmp` *without* truncating — a crashed writer's leftover
///    must not be destroyed before we hold its lock;
/// 2. flock the tmp — always uncontended under the protocol (only the main
///    holder touches the tmp), and required *before* the rename so the path
///    never resolves to an unlocked inode;
/// 3. truncate, write, fsync;
/// 4. rename tmp over `path` — the locked tmp inode becomes the path's inode;
/// 5. fsync the parent (best-effort, as in [`write_atomic`]);
/// 6. release the old inode's lock by dropping `held`.
/// On error the ORIGINAL `held` lock is returned in the `Err` variant, never dropped: every
/// failure is before or at the rename, so the old inode's lock is still valid and exclusion
/// must not be lost (a shared holder — e.g. a concurrent account-lock lease — may still need
/// it). The caller decides whether to retry or release.
pub fn write_locked(
    held: FileLock,
    path: &Path,
    bytes: &[u8],
) -> Result<FileLock, (std::io::Error, FileLock)> {
    let tmp_path = {
        let mut name = path
            .file_name()
            .map(|f| f.to_os_string())
            .unwrap_or_default();
        name.push(".tmp");
        match path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(parent) => parent.join(name),
            None => PathBuf::from(name),
        }
    };

    // Fallible steps in a closure so any early return hands `held` back instead of dropping it.
    let tmp = match (|| -> std::io::Result<File> {
        let mut tmp = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&tmp_path)?;
        tmp.try_lock_exclusive()
            .map_err(|e| std::io::Error::other(format!("tmp lock (protocol violation?): {e}")))?;
        tmp.set_len(0)?;
        tmp.write_all(bytes)?;
        tmp.sync_all()?;
        fs::rename(&tmp_path, path)?;
        Ok(tmp)
    })() {
        Ok(tmp) => tmp,
        Err(e) => return Err((e, held)),
    };

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    drop(held);
    Ok(FileLock { _file: tmp })
}
