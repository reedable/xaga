//! The core error umbrella raised to the command layer.
//!
//! `DomainError` aggregates the per-subsystem error enums — it does *not*
//! replace them. Each subsystem still owns its own enum next to its meaning
//! (`EntryError`, `FeedError`, …, honoring the anti-central-collection
//! convention in `consts.rs`); this type is the single value the command layer
//! (`src-tauri`) converts into an `AppError` for the webview. It is the error
//! sibling of `crate::events::DomainEvent`: one typed value per layer, each
//! variant owning a stable string `code()` the UI matches on.
//!
//! Two audiences per error:
//! - **`code()`** — a stable, colon/kebab identifier the UI switches on,
//!   mirrored by hand in `src/platform/errors.ts` (the code list is pinned by
//!   the test below; a rename is a deliberate edit in both places).
//! - **`user_message()`** — a generic, safe sentence with no file fragments,
//!   ids, or URLs. The full technical text stays in `Display`, which the
//!   command layer logs as the error's `detail`.
//!
//! Wrapper variants delegate downward: a `Writer(WriterError::Entry(..))`
//! surfaces the inner `entry:*` code, not a generic writer code — the UI sees
//! what actually failed.

use thiserror::Error;

use crate::embeddings::EmbeddingsError;
use crate::library_writer::WriterError;
use crate::persistence::bookmarks::BookmarksError;
use crate::persistence::entries::EntryError;
use crate::persistence::feeds::FeedError;
use crate::persistence::topics::TopicsError;
use crate::query::items::ItemError;
use crate::ssrf_proxy::ProxyError;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error(transparent)]
    Entry(#[from] EntryError),
    #[error(transparent)]
    Feed(#[from] FeedError),
    #[error(transparent)]
    Embeddings(#[from] EmbeddingsError),
    #[error(transparent)]
    Topics(#[from] TopicsError),
    #[error(transparent)]
    Bookmarks(#[from] BookmarksError),
    #[error(transparent)]
    Item(#[from] ItemError),
    #[error(transparent)]
    Writer(#[from] WriterError),
    #[error(transparent)]
    Proxy(#[from] ProxyError),
}

impl DomainError {
    /// The stable code the UI matches on. Delegates to the wrapped subsystem
    /// error so the code names the actual failure.
    pub fn code(&self) -> &'static str {
        match self {
            DomainError::Entry(e) => e.code(),
            DomainError::Feed(e) => e.code(),
            DomainError::Embeddings(e) => e.code(),
            DomainError::Topics(e) => e.code(),
            DomainError::Bookmarks(e) => e.code(),
            DomainError::Item(e) => e.code(),
            DomainError::Writer(e) => e.code(),
            DomainError::Proxy(e) => e.code(),
        }
    }

    /// A user-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            DomainError::Entry(e) => e.user_message(),
            DomainError::Feed(e) => e.user_message(),
            DomainError::Embeddings(e) => e.user_message(),
            DomainError::Topics(e) => e.user_message(),
            DomainError::Bookmarks(e) => e.user_message(),
            DomainError::Item(e) => e.user_message(),
            DomainError::Writer(e) => e.user_message(),
            DomainError::Proxy(e) => e.user_message(),
        }
    }
}

#[cfg(test)]
mod tests {
    /// Every distinct code owned across the subsystem error enums, mirrored in
    /// `src/platform/errors.ts` (`ERROR_CODE`). Delegating wrappers
    /// (`WriterError`, `ItemError`, `DomainError`) introduce no new codes — a
    /// wrapped error reuses the inner code by design. A new code is a
    /// deliberate edit here AND in `src/platform/errors.ts`.
    const OWNED_CODES: &[&str] = &[
        // EntryError
        "entry:io",
        "entry:invalid-url",
        "entry:invalid-id",
        "entry:not-found",
        "entry:color-change-requires-replace",
        // FeedError
        "feed:invalid-url",
        "feed:http",
        "feed:io",
        "feed:atom-parse",
        "feed:parse",
        "feed:not-found",
        "feed:missing-self-link",
        "feed:body-too-large",
        "feed:http-blocked",
        "feed:writer",
        // EmbeddingsError
        "embeddings:io",
        "embeddings:model-download",
        "embeddings:model",
        "embeddings:tokenize",
        "embeddings:lancedb",
        "embeddings:arrow",
        "embeddings:json",
        "embeddings:parse",
        // TopicsError
        "topic:io",
        "topic:not-found",
        // BookmarksError
        "bookmark:io",
        "bookmark:invalid-url",
        // ItemError (own)
        "item:io",
        // WriterError (own)
        "writer:shutting-down",
        "writer:closed",
        // ProxyError
        "proxy:bad-request",
        "proxy:dns",
        "proxy:blocked",
    ];

    /// Codes are well-formed (namespaced with a colon) and collision-free.
    #[test]
    fn owned_codes_are_wellformed_and_unique() {
        for code in OWNED_CODES {
            assert!(code.contains(':'), "code {code:?} is not namespaced");
        }
        let unique: std::collections::HashSet<&&str> = OWNED_CODES.iter().collect();
        assert_eq!(unique.len(), OWNED_CODES.len(), "duplicate error code");
    }

    /// Delegation is wired: a wrapped error surfaces the inner code, and the
    /// `#[from]` chain lands on the right umbrella variant.
    #[test]
    fn wrappers_delegate_to_inner_code() {
        use super::*;
        let entry = EntryError::NotFound;
        let via_writer: DomainError = WriterError::from(EntryError::NotFound).into();
        assert_eq!(via_writer.code(), entry.code());
        assert_eq!(via_writer.code(), "entry:not-found");
        let direct: DomainError = EntryError::InvalidId.into();
        assert_eq!(direct.code(), "entry:invalid-id");
        assert_eq!(direct.user_message(), "This item's identifier is invalid.");
    }
}
