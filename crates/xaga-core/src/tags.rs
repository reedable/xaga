//! The app's tag vocabulary and the derivation of domain state booleans from it.
//!
//! Neutral leaf shared by `persistence` (the `.md`/`.atom` storage), `embeddings` (the LanceDB
//! `tags` column), and the command layer (state toggles). Depending on this module does not couple
//! those layers to each other — it is a shared leaf like `types`/`util`.

/// The `bookmark` state tag.
pub const BOOKMARK: &str = "bookmark";

/// Whether the `bookmark` tag is present.
pub fn is_bookmarked(tags: &[String]) -> bool {
    tags.iter().any(|t| t == BOOKMARK)
}
