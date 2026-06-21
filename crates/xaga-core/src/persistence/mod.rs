//! Persistence: the domain ↔ `.md`/`.atom`/`.html` boundary.
//!
//! These modules read and write entity files and produce the neutral
//! [`Content`](crate::content::Content) payload. They have **no** dependency on the indexing
//! layer (`embeddings`) — the separation the refactor enforces.

pub mod articles;
pub mod bookmarks;
pub mod entries;
pub mod feeds;
pub mod frontmatter;
pub mod opml;
pub mod topics;
