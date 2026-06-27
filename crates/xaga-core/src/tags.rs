//! The app's tag vocabulary and the derivation of domain state booleans from it.
//!
//! Neutral leaf shared by `persistence` (the `.md`/`.atom` storage), `embeddings` (the LanceDB
//! `tags` column), and the command layer (state toggles). Depending on this module does not couple
//! those layers to each other — it is a shared leaf like `types`/`util`.

/// The `unread` state tag.
pub const UNREAD: &str = "unread";
/// The `bookmark` state tag.
pub const BOOKMARK: &str = "bookmark";
/// The `pinned` state tag.
pub(crate) const PINNED: &str = "pinned";

/// Whether the `unread` tag is present.
pub fn is_unread(tags: &[String]) -> bool {
    tags.iter().any(|t| t == UNREAD)
}

/// Whether the `bookmark` tag is present.
pub fn is_bookmarked(tags: &[String]) -> bool {
    tags.iter().any(|t| t == BOOKMARK)
}

/// Whether the `pinned` tag is present.
pub(crate) fn is_pinned(tags: &[String]) -> bool {
    tags.iter().any(|t| t == PINNED)
}
