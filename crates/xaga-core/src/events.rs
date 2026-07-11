//! Domain events raised to the UI.
//!
//! Each variant pairs an event with its typed payload, and the enum owns the
//! event name the UI receives (`name()`). The app layer (src-tauri) forwards
//! these through its single emit funnel; the UI listens by the same names
//! (`src/platform/events.ts`, hand-maintained — see
//! docs-transient/backlog/build/spike__ts_type_codegen.md for the codegen
//! candidate that would derive it from here).
//!
//! App-layer events (settings, menu, loader, import/export) are deliberately
//! not here: their payload types and semantics belong to src-tauri (see
//! `AppEvent` there and docs/architecture/layer_ownership.md).

use crate::types::{Entry, Feed, ListItem};

pub enum DomainEvent {
    /// An entry's unread state changed.
    UnreadChanged(Entry),
    /// An entry's bookmark state (or color) changed.
    BookmarkChanged(Entry),
    /// An annotation was created or updated.
    AnnotationChanged(ListItem),
    /// An annotation was removed.
    AnnotationRemoved { id: String },
    /// The embedding service drained a queue tier; embeddings-backed reads
    /// may return new results.
    EmbeddingChanged,
    /// The embedding service is shutting down (library switch, quit).
    EmbeddingShutdown,
    /// A feed finished refreshing.
    FeedRefreshed(Feed),
    /// A refresh-all pass finished.
    FeedsRefreshed,
}

impl DomainEvent {
    /// The event name as the UI receives it — mirrored in
    /// `src/platform/events.ts` (`DOMAIN_EVENT`).
    pub fn name(&self) -> &'static str {
        match self {
            DomainEvent::UnreadChanged(_) => "unread:changed",
            DomainEvent::BookmarkChanged(_) => "bookmark:changed",
            DomainEvent::AnnotationChanged(_) => "annotation:changed",
            DomainEvent::AnnotationRemoved { .. } => "annotation:removed",
            DomainEvent::EmbeddingChanged => "embedding:changed",
            DomainEvent::EmbeddingShutdown => "embedding:shutdown",
            DomainEvent::FeedRefreshed(_) => "feed:refreshed",
            DomainEvent::FeedsRefreshed => "feeds:refreshed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry {
            id: String::new(),
            url: String::new(),
            title: String::new(),
            created_at: String::new(),
            published_at: None,
            updated_at: None,
            description: None,
            is_unread: false,
            is_bookmarked: false,
            color: None,
        }
    }

    fn all_variants() -> Vec<DomainEvent> {
        vec![
            DomainEvent::UnreadChanged(entry()),
            DomainEvent::BookmarkChanged(entry()),
            DomainEvent::AnnotationChanged(ListItem {
                id: String::new(),
                title: String::new(),
                created_at: String::new(),
                url: String::new(),
                is_unread: false,
                is_bookmarked: false,
                reference: None,
                color: None,
            }),
            DomainEvent::AnnotationRemoved { id: String::new() },
            DomainEvent::EmbeddingChanged,
            DomainEvent::EmbeddingShutdown,
            DomainEvent::FeedRefreshed(Feed {
                id: String::new(),
                url: String::new(),
                title: String::new(),
                created_at: String::new(),
                updated_at: None,
                description: None,
                is_pinned: false,
                entry_count: 0,
                unread_count: 0,
                members: Vec::new(),
            }),
            DomainEvent::FeedsRefreshed,
        ]
    }

    /// Names match the expected list exactly (a rename is a deliberate edit
    /// here AND in src/platform/events.ts) and are unique.
    #[test]
    fn names_are_the_expected_set_and_unique() {
        let names: Vec<&str> = all_variants().iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            [
                "unread:changed",
                "bookmark:changed",
                "annotation:changed",
                "annotation:removed",
                "embedding:changed",
                "embedding:shutdown",
                "feed:refreshed",
                "feeds:refreshed",
            ]
        );
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len());
    }
}
