//! Domain events raised to the UI.
//!
//! Each variant pairs an event with its typed payload, and the enum owns the
//! event name the UI receives (`name()`). The app layer (src-tauri) forwards
//! these through its single emit funnel; the UI listens by the same names
//! (`src/platform/events.ts`, hand-maintained). Keeping the two in step is
//! manual: the `name()` tests below pin this list, and the TS side pins its
//! own, so a name added on one side without the other fails a test rather
//! than silently dropping events.
//!
//! App-layer events (settings, menu, loader, import/export) are deliberately
//! not here: their payload types and semantics belong to src-tauri (see
//! `AppEvent` there and docs/architecture/layer_ownership.md).

use crate::types::{Article, Entry, Feed, ListItem};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationReplacement {
    pub old_id: String,
    pub item: ListItem,
}

pub enum DomainEvent {
    /// An entry's unread state changed.
    UnreadChanged(Entry),
    /// An entry's bookmark state (or color) changed.
    BookmarkChanged(Entry),
    /// An article pin changed in its authoritative file.
    EntryPinChanged(Entry),
    /// An annotation was created or updated.
    AnnotationChanged(ListItem),
    /// An annotation's identity changed while preserving its place in collections.
    AnnotationReplaced(AnnotationReplacement),
    /// An annotation was removed.
    AnnotationRemoved { id: String },
    /// An article save changed or refreshed its locally persisted favicon.
    FaviconChanged(Article),
    /// The embedding service drained a queue tier; embeddings-backed reads
    /// may return new results.
    EmbeddingChanged,
    /// The embedding service is shutting down (library switch, quit).
    EmbeddingShutdown,
    /// A feed finished refreshing.
    FeedRefreshed(Feed),
    /// A refresh-all pass finished.
    FeedsRefreshed,
    /// The library's runtime mode, announced at startup. `true` = this instance did not
    /// win the library lock: write commands are refused (`E_READ_ONLY`) and the UI
    /// disables write affordances. Mode is fixed for the process's lifetime — a library
    /// switch restarts the process.
    LibraryModeChanged { is_library_read_only: bool },
}

impl DomainEvent {
    /// The event name as the UI receives it — mirrored in
    /// `src/platform/events.ts` (`DOMAIN_EVENT`).
    pub fn name(&self) -> &'static str {
        match self {
            DomainEvent::UnreadChanged(_) => "unread:changed",
            DomainEvent::BookmarkChanged(_) => "bookmark:changed",
            DomainEvent::EntryPinChanged(_) => "entry-pin:changed",
            DomainEvent::AnnotationChanged(_) => "annotation:changed",
            DomainEvent::AnnotationReplaced(_) => "annotation:replaced",
            DomainEvent::AnnotationRemoved { .. } => "annotation:removed",
            DomainEvent::FaviconChanged(_) => "favicon:changed",
            DomainEvent::EmbeddingChanged => "embedding:changed",
            DomainEvent::EmbeddingShutdown => "embedding:shutdown",
            DomainEvent::FeedRefreshed(_) => "feed:refreshed",
            DomainEvent::FeedsRefreshed => "feeds:refreshed",
            DomainEvent::LibraryModeChanged { .. } => "library:mode-changed",
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
            is_pinned: false,
            color: None,
        }
    }

    fn all_variants() -> Vec<DomainEvent> {
        vec![
            DomainEvent::UnreadChanged(entry()),
            DomainEvent::BookmarkChanged(entry()),
            DomainEvent::EntryPinChanged(entry()),
            DomainEvent::AnnotationChanged(ListItem {
                id: String::new(),
                title: String::new(),
                created_at: String::new(),
                url: String::new(),
                is_unread: false,
                is_bookmarked: false,
                is_pinned: false,
                reference: None,
                color: None,
            }),
            DomainEvent::AnnotationReplaced(AnnotationReplacement {
                old_id: String::new(),
                item: ListItem {
                    id: String::new(),
                    title: String::new(),
                    created_at: String::new(),
                    url: String::new(),
                    is_unread: false,
                    is_bookmarked: false,
                    is_pinned: false,
                    reference: None,
                    color: None,
                },
            }),
            DomainEvent::AnnotationRemoved { id: String::new() },
            DomainEvent::FaviconChanged(crate::types::Article {
                meta: entry(),
                content: String::new(),
                annotations: Vec::new(),
                favicon: None,
            }),
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
                "entry-pin:changed",
                "annotation:changed",
                "annotation:replaced",
                "annotation:removed",
                "favicon:changed",
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
