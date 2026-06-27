//! Filesystem helpers shared across the crate.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::md::{Markdown, parse_document};

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
pub async fn write_atomic_async(path: PathBuf, bytes: Vec<u8>) -> std::io::Result<()> {
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

/// Scans a directory for .md files, and appends the parsed .md file into `out` vector along with
/// its path. Caller is responsible for providing the `Markdown` intermediate representation into
/// domain objects via `parse` argument.  Yields between reads to avoid starving
/// high-priority tasks.
pub(crate) async fn scan_md_dir<T, F>(dir: &Path, out: &mut Vec<T>, parse: &F)
where
    F: Fn(Markdown) -> T,
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

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            // We only care about .md files
            continue;
        }

        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(r) => r,
            Err(_) => continue,
        };

        let id = crate::util::item_id(&path);

        let (frontmatter, body) = match parse_document(&raw) {
            Some(x) => x,
            None => continue,
        };

        let md = Markdown {
            id,
            fm: frontmatter,
            body,
        };

        out.push(parse(md));

        tokio::task::yield_now().await;
    }
}
