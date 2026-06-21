//! Query: read-side joins that draw on both persistence and indexing.
//!
//! These modules assemble UI-facing results from the embeddings index and the persisted files
//! (similarity lookups, filters, topic item lists). They are the read-side counterpart to the
//! write-side separation; nothing here is on the indexing write path.

pub mod filter;
pub mod item;
pub mod topics;
