# Return authoritative feed counts and pinned state in semantic results

## Summary

Semantic feed hits are enriched with zero entry and unread counts and an unpinned
state instead of the feed's current values.

## Requirements

- Return the same feed metadata and counts as the authoritative feed-list path.
- Preserve semantic result order and similarity scores.
- Avoid opening the embeddings database or materializing global state once per hit.
- Omit a feed result when its authoritative `.atom` file no longer exists during
  enrichment; preserve the relative order and similarity scores of remaining
  results.

## Remaining work

- Reuse or extract the feed-summary logic for batched result enrichment.
- Load unread and bookmarked state once per search and intersect it with each matched feed's membership.
- Populate counts and pinned state consistently with `list_feeds`.
- Test mixed feed and non-feed results, missing feeds, and nonzero counts.

## References

- `crates/xaga-core/src/query/items.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/persistence/feeds.rs`
- `src-tauri/src/commands/embeddings.rs`
- `src-tauri/src/commands/feeds.rs`
