# Feed row dates

## Summary

`EmbeddingEntry::from_feed` stores empty `created_at` and `updated_at` values although the domain `Feed` carries both. Semantic feed results are consequently unable to recover those dates from the row without rereading the feed.

## Requirements

- Define whether feed dates are part of the embeddings read model.
- If stored, derive them from the authoritative `Feed` and update them through normal feed upserts and recovery.
- Preserve search ranking and checksum behavior unless dates are intentionally made embeddable.
- Define migration or recovery behavior for existing rows with empty dates.

## Remaining work

- Identify consumers of feed dates in semantic results and other database reads.
- Compare storing the dates with enriching feed hits from authoritative persistence.
- Decide the read-model boundary and update behavior.
- Implement and test the chosen behavior for new, refreshed, and recovered feeds.

## References

- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/query/items.rs`
- `crates/xaga-core/src/persistence/feeds.rs`
