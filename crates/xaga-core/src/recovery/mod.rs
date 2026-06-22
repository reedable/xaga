//! Recovery: the reconcile-side bridge over persistence + embeddings, mirroring `query` (the
//! read-side). Recovery operations own the *what* — what to reconcile — and delegate the *how*
//! (read/write) to persistence and embeddings primitives.
//!
//!   feed_entries — reconcile a feed's `.atom` membership into on-disk entries
//!   embeddings   — diff the filesystem against the index, producing `EmbeddingsReconResults`
//!   execute      — process the recon results in-process (CLI)
//!   dispatch     — process the recon results by enqueuing jobs onto the embedding service

pub mod dispatch;
pub mod embeddings;
pub mod execute;
pub mod feeds;

use crate::types::{Article, Feed, Topic};

/// The output of a recovery scan: what needs to happen to bring LanceDB back into sync with the
/// filesystem. Recovery is the filesystem-reading producer: it parses each file **once** here and
/// carries the domain object (`Article`/`Topic`) so the embedding pass never re-reads.
pub struct EmbeddingsReconResults {
    /// Topics missing from LanceDB or with changed checksums.
    /// Separated so callers can route them to a higher-priority channel.
    pub to_embed_topics: Vec<Topic>,
    /// Articles missing from LanceDB or with changed checksums.
    pub to_embed_articles: Vec<Article>,
    /// All current feed rows to (re)embed.
    pub to_embed_feeds: Vec<Feed>,
    /// IDs of LanceDB rows (items or feeds) whose source no longer exists on disk.
    pub to_delete: Vec<String>,
}
