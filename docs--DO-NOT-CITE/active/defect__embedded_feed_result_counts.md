# Return complete feed metadata in semantic results

## Summary

Semantic feed hits are enriched with zero entry and unread counts and an unpinned state instead of the feed's current values. Their embedding rows also contain empty creation and update dates even though the domain `Feed` carries both.

## Requirements

- Return the same feed metadata and counts as the authoritative feed-list path.
- Preserve semantic result order and similarity scores.
- Avoid opening the embeddings database or materializing global state once per hit.
- Define behavior when a feed disappears between search and enrichment.
- Decide whether feed dates belong in the embeddings read model or are supplied only by authoritative enrichment.
- If dates are stored in embedding rows, update them through feed upserts and recovery without changing checksum or ranking semantics.
- Define repair behavior for existing rows with empty dates.

## Remaining work

- Reuse or extract the feed-summary logic for batched result enrichment.
- Load unread and bookmarked state once per search and intersect it with each matched feed's membership.
- Compare storing feed dates with supplying them during authoritative enrichment and select the read-model boundary.
- Populate counts, pinned state, and dates consistently with `list_feeds`.
- Test mixed feed and non-feed results, missing feeds, nonzero counts, and new, refreshed, and recovered feed dates.

## References

- `crates/xaga-core/src/query/items.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/persistence/feeds.rs`
- `src-tauri/src/commands/embeddings.rs`
- `src-tauri/src/commands/feeds.rs`
