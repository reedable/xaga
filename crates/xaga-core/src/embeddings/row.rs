//! The `Content → EmbeddingEntry` mapping — the indexing layer's inbound boundary.
//!
//! Lives here (not in persistence) so persistence has no dependency on `embeddings::db`. The
//! indexing consumer builds a LanceDB row from the neutral [`Content`](crate::content::Content)
//! payload it received on the queue, never from a file.

use crate::content::Content;

use super::db::EmbeddingEntry;

impl EmbeddingEntry {
    /// Builds an entry/topic row from `content` and an optional vector. `topics` (the entry→topic
    /// reverse index) and `items` (the topic-membership base) are reconcile-managed, not sourced
    /// from content — written empty here and carried forward by `sync_row`/`embed_vector`.
    pub fn from_content(content: &Content, vector: Option<Vec<f32>>) -> EmbeddingEntry {
        EmbeddingEntry {
            id: content.id.clone(),
            kind: content.kind,
            source: content.source.clone(),
            title: content.title.clone(),
            created_at: content.created_at.clone(),
            published_at: content.published_at.clone().unwrap_or_default(),
            updated_at: content.updated_at.clone().unwrap_or_default(),
            checksum: content.content_checksum(),
            reference: String::new(),
            color: String::new(),
            summary: content.summary.clone().unwrap_or_default(),
            tags: content.tags.clone(),
            topics: vec![],
            items: vec![],
            is_unread: content.is_unread(),
            is_bookmarked: content.is_bookmarked(),
            is_pinned: content.is_pinned(),
            vector,
        }
    }
}
