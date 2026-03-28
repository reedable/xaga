pub mod articles;
pub mod bookmarks;
pub mod entries;
pub mod embeddings;
pub mod feeds;
pub mod library;
pub mod settings;
pub mod topics;

pub(crate) fn parse_item_filter(
    item_filter: Option<String>,
) -> Result<xaga_core::types::ItemFilter, String> {
    item_filter
        .as_deref()
        .map(|s| s.parse::<xaga_core::types::ItemFilter>())
        .transpose()
        .map_err(|e| e)?
        .map_or(Ok(Default::default()), Ok)
}
