// Core layers, kept deliberately separate:
//   domain      — the pure model
//   content     — the neutral parsed payload that crosses the queue (depended on by both sides)
//   persistence — domain <-> .md/.atom/.html (produces `content`; no dependency on `embeddings`)
//   embeddings  — domain <-> LanceDB (consumes `content`; no .md parsing)
//   query       — read-side joins over both
//   recovery    — the filesystem-reading producer that reconciles files into the index
pub mod content;
pub mod domain;
pub mod embeddings;
pub mod fs;
pub mod persistence;
pub mod query;
pub mod recovery;
pub mod types;
pub mod util;

// Re-exports preserving the historical flat module paths (`xaga_core::entries`, `::feeds`,
// `::item`, …) so external callers and existing `crate::` references are unaffected by the
// persistence/ and query/ regrouping.
pub use persistence::{articles, bookmarks, entries, feeds, opml, topics};
pub use query::{filter, item};
