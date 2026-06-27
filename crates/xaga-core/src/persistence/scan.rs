//! Scanning the library's `.md` files into domain objects. The filesystem traversal is the floor's
//! job (`fs::for_each_file`); this layer adds the `.md` interpretation — stem-as-id, frontmatter
//! parsing, and the `Markdown` → domain conversion.

use std::path::Path;

use crate::md::{Markdown, parse_document};

/// Walks `dir` for `.md` files, builds the `Markdown` IR for each (id from the file stem), maps it
/// into a domain object via `parse`, and hands the result to `visit`. The caller decides what to do
/// with each object — collect, fold, or filter — so a large scan need not be materialized. Malformed
/// documents are skipped. Traversal yields between files (see `fs::for_each_file`), so this never
/// starves higher-priority tasks.
pub(crate) async fn for_each_md<T, P, V>(dir: &Path, parse: &P, mut visit: V)
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
    .await;
}
