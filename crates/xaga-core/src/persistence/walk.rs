//! Walking the library's `.md` and `.atom` files into domain objects. The filesystem traversal is
//! the floor's job (`fs::for_each_file`); this layer adds the file-format interpretation — stem-as-id
//! and frontmatter parsing for `.md`, and a caller-injected parse for `.atom`.

use std::path::{Path, PathBuf};

use crate::fs::WalkOutcome;
use crate::md::{Markdown, parse_document};

/// Walks `dir` for `.md` files, builds the `Markdown` IR for each (id from the file stem), maps it
/// into a domain object via `parse`, and hands the result to `visit`. The caller decides what to do
/// with each object — collect, fold, or filter — so a large scan need not be materialized. Malformed
/// documents are skipped. Traversal yields between files (see `fs::for_each_file`), so this never
/// starves higher-priority tasks.
///
/// Returns whether the directory was reliably observed ([`WalkOutcome`]); callers that prune based
/// on a stem being *absent* must honor it — a malformed `.md` is skipped (a real observation), but
/// an unreadable directory or file is not "empty."
pub(crate) async fn for_each_md<T, P, V>(dir: &Path, parse: &P, mut visit: V) -> WalkOutcome
where
    P: Fn(Markdown) -> T,
    V: FnMut(T),
{
    crate::fs::for_each_file(dir, "md", |path, raw| {
        if let Some((fm, body)) = parse_document(&raw) {
            visit(parse(Markdown {
                id: crate::util::item_id(&path),
                fm,
                body,
            }));
        }
    })
    .await
}

/// Walks `dir` for `.atom` files, parses each via the caller-supplied `parse`, and hands
/// `(path, parsed)` to `visit`; files that fail to read or whose `parse` returns `Err` are skipped.
/// Unlike [`for_each_md`], the path is handed through — the `.atom` → domain conversion needs it for
/// a later async `created_at` stat, which the sync visitor can't do, so a caller collects here and
/// runs that pass afterward. Extension matching also excludes `.atom.tmp`. `parse` is injected (not
/// hardcoded like `parse_document`) so this stays free of the atom parser, which is private to feeds.
pub(crate) async fn for_each_atom<T, E, P, V>(dir: &Path, parse: &P, mut visit: V) -> WalkOutcome
where
    P: Fn(&str) -> Result<T, E>,
    V: FnMut(PathBuf, T),
{
    crate::fs::for_each_file(dir, "atom", |path, raw| {
        if let Ok(parsed) = parse(&raw) {
            visit(path, parsed);
        }
    })
    .await
}
