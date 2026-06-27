//! Scanning the library's `.md` files into domain objects. The filesystem traversal is the floor's
//! job (`fs::for_each_file`); this layer adds the `.md` interpretation — stem-as-id, frontmatter
//! parsing, and the `Markdown` → domain conversion.

use std::path::Path;

use crate::md::{Markdown, parse_document};

/// Scans `dir` for `.md` files, builds the `Markdown` IR for each (id from the file stem), maps it
/// into a domain object via `parse`, and appends the result to `out`. Malformed documents are
/// skipped. Traversal yields between files (see `fs::for_each_file`), so this never starves
/// higher-priority tasks.
pub(crate) async fn for_each_md<T, F>(dir: &Path, out: &mut Vec<T>, parse: &F)
where
    F: Fn(Markdown) -> T,
{
    crate::fs::for_each_file(dir, "md", |path, raw| {
        let (fm, body) = match parse_document(&raw) {
            Some(x) => x,
            None => return,
        };

        out.push(parse(Markdown {
            id: crate::util::item_id(&path),
            fm,
            body,
        }));
    })
    .await;
}
