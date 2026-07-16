# Embedded feed search results return zero counts (pending — decided design attached)

## Current state

Semantic feed hits carry `entry_count`/`unread_count`/`bookmarked_count` = 0: `enrich_embedded` builds `EmbeddedItem::Feed` without real stats; the server-side lookup was deferred when hybrid feed search was built.

## The decided design (recorded when deferred)

When picked up: read the feed's `.atom` membership and intersect with the unread/bookmarked id-sets (as `feed_summary` does) — either by threading `feeds_dir` + the id-sets from the `search_by_text` command (same source as `list_feeds`), or by opening the db inside `enrich_embedded` only when a feed hit is present. Counts are O(U)+O(M).

Location cited at recording time: `enrich_embedded` (was `item.rs`; re-locate in the current tree before starting).
