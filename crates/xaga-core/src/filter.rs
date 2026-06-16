use chrono::{DateTime, Utc};

use crate::types::{Item, ItemFilter};

/// Filter `items` according to `item_filter` — predicate only, no sorting.
///
/// Works on any `T` that implements `Item`. Items without a `source` are
/// treated as authored (already read, not bookmarkable).
pub async fn filter_items<T: Item>(
    mut items: Vec<T>,
    item_filter: &ItemFilter,
) -> std::io::Result<Vec<T>> {
    match item_filter {
        ItemFilter::All => {}

        ItemFilter::Unread => {
            items.retain(|i| i.url().is_some() && i.is_unread());
        }

        ItemFilter::Bookmarked => {
            items.retain(|i| i.is_bookmarked());
        }

        ItemFilter::Saved => {
            items.retain(|i| i.is_bookmarked() || !i.reference().is_empty());
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
