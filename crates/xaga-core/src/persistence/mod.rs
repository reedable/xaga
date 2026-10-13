//! Persistence: the domain ↔ `.md`/`.atom`/`.html` boundary.
//!
//! These modules read and write entity files and map them to and from the domain model. They have
//! **no** dependency on the indexing layer (`embeddings`) — the separation the refactor enforces.

pub mod articles;
pub mod bookmarks;
pub mod entries;
pub mod feeds;
pub mod images;
pub mod opml;
pub mod topics;
pub(crate) mod walk;

pub mod pdfs;

pub(crate) mod documents;

pub mod summary_repair;
