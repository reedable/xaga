use chrono::{DateTime, Utc};

use crate::types::{EmbeddedEntry, Entry, Item, ItemFilter};

/// The read/bookmark/reference state the list filters need. Implemented by the URL-bearing,
/// stateful item types (`Entry` and its embedded form) — these are query-layer accessors, not
/// part of the domain `Item` contract.
pub trait Filterable {
    fn is_unread(&self) -> bool;
    fn color(&self) -> Option<&str>;
    fn url(&self) -> Option<&str>;
    fn reference(&self) -> &str;
}

impl Filterable for Entry {
    fn is_unread(&self) -> bool {
        self.is_unread
    }
    fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }
    fn url(&self) -> Option<&str> {
        Some(&self.url)
    }
    fn reference(&self) -> &str {
        ""
    }
}

impl Filterable for EmbeddedEntry {
    fn is_unread(&self) -> bool {
        self.entry.is_unread
    }
    fn color(&self) -> Option<&str> {
        (self.kind == crate::types::Kind::Entry)
            .then_some(self.entry.color.as_deref())
            .flatten()
    }
    fn url(&self) -> Option<&str> {
        Some(&self.entry.url)
    }
    fn reference(&self) -> &str {
        &self.reference
    }
}

/// Filter `items` according to `item_filter` — predicate only, no sorting.
pub async fn filter_items<T: Filterable>(
    mut items: Vec<T>,
    item_filter: &ItemFilter,
) -> std::io::Result<Vec<T>> {
    match item_filter {
        ItemFilter::All => {}

        ItemFilter::Unread => {
            items.retain(|i| i.url().is_some() && i.is_unread());
        }

        ItemFilter::Bookmarked => {
            items.retain(|i| i.color().is_some());
        }

        ItemFilter::Saved => {
            items.retain(|i| i.color().is_some() || !i.reference().is_empty());
        }
    }

    Ok(items)
}

/// Sort `items` by most-recent timestamp, descending.
pub fn sort_items<T: Item>(items: &mut Vec<T>) {
    items.sort_by(|a, b| timestamp(b).cmp(&timestamp(a)));
}

fn timestamp<T: Item>(item: &T) -> DateTime<Utc> {
    parse_ts(item.updated_at())
        .or_else(|| parse_ts(item.published_at()))
        .or_else(|| parse_ts(Some(item.created_at())))
        .unwrap_or(DateTime::UNIX_EPOCH)
}

/// Parse an RFC-3339 string, returning `None` for unparseable or epoch values.
fn parse_ts(s: Option<&str>) -> Option<DateTime<Utc>> {
    let dt = DateTime::parse_from_rfc3339(s?).ok()?.to_utc();
    if dt.timestamp() <= 0 { None } else { Some(dt) }
}
