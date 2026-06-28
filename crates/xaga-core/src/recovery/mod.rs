//! Recovery: the reconcile-side bridge over persistence + embeddings, mirroring `query` (the
//! read-side). Recovery operations own the *what* — what to reconcile — and delegate the *how*
//! (read/write) to persistence and embeddings primitives.
//!
//!   embeddings   — walk the filesystem, stream items needing embedding to a `RepairSink`, return the residue
//!   inline       — CLI sink: collect, then embed in-process
//!   dispatch     — service sink: enqueue jobs onto the embedding tiers (or bulk-insert on fresh)

pub mod dispatch;
pub mod embeddings;
pub mod inline;

use crate::types::{Article, Feed, Topic};

/// Receives each item that needs embedding as `repair_item_embeddings` walks the library, so the
/// caller decides how to apply the work without the scan materializing the whole corpus. The
/// service path enqueues onto the embedding tiers; the CLI path collects for in-process embedding.
/// Sync by necessity — the walk's visitor cannot await (see `fs::for_each_file`).
pub(crate) trait RepairSink {
    fn topic(&mut self, topic: Topic);
    fn article(&mut self, article: Article);
}

/// The post-walk residue of a repair scan: the parts that can only be known once every file has been
/// seen, so they cannot stream item-by-item to the [`RepairSink`]. Item embeds stream during the
/// walk; these are returned.
pub(crate) struct RepairResidue {
    /// All current feed rows to (re)embed. Feeds have no `.md`, so they're listed, not walked.
    pub to_embed_feeds: Vec<Feed>,
    /// IDs of LanceDB rows (items or feeds) whose source no longer exists on disk.
    pub to_delete: Vec<String>,
}
