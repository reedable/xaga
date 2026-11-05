use chrono::{DateTime, Utc};

use crate::types::{EmbeddedEntry, EntityFilter, Entry, ManagedEntity};

/// The read/bookmark/reference state the list filters need. Implemented by the URL-bearing,
/// stateful entity types (`Entry` and its embedded form) — these are query-layer accessors, not
/// part of the domain `ManagedEntity` contract.
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

/// Filter `entities` according to `entity_filter` — predicate only, no sorting.
pub async fn filter_entities<T: Filterable>(
    mut entities: Vec<T>,
    entity_filter: &EntityFilter,
) -> std::io::Result<Vec<T>> {
    match entity_filter {
        EntityFilter::All => {}

        EntityFilter::Unread => {
            entities.retain(|i| i.url().is_some() && i.is_unread());
        }

        EntityFilter::Bookmarked => {
            entities.retain(|i| i.color().is_some());
        }

        EntityFilter::Saved => {
            entities.retain(|i| i.color().is_some() || !i.reference().is_empty());
        }
    }

    Ok(entities)
}

/// Sort `entities` by most-recent timestamp, descending.
pub fn sort_entities<T: ManagedEntity>(entities: &mut Vec<T>) {
    entities.sort_by(|a, b| timestamp(b).cmp(&timestamp(a)));
}

fn timestamp<T: ManagedEntity>(entity: &T) -> DateTime<Utc> {
    parse_ts(entity.updated_at())
        .or_else(|| parse_ts(entity.published_at()))
        .or_else(|| parse_ts(Some(entity.created_at())))
        .unwrap_or(DateTime::UNIX_EPOCH)
}

/// Parse an RFC-3339 string, returning `None` for unparseable or epoch values.
fn parse_ts(s: Option<&str>) -> Option<DateTime<Utc>> {
    let dt = DateTime::parse_from_rfc3339(s?).ok()?.to_utc();
    if dt.timestamp() <= 0 { None } else { Some(dt) }
}
