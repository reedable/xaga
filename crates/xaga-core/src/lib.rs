// Core layers, kept deliberately separate:
//   domain      — the pure model, carried end-to-end as the queue payload
//   persistence — domain <-> .md/.atom/.html (produces domain objects; no dependency on `embeddings`)
//   embeddings  — domain <-> LanceDB (consumes domain objects; no .md parsing)
//   query       — read-side joins over both
//   recovery    — the filesystem-reading producer that reconciles files into the index
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
