# Define the feed-date read-model boundary

## Summary

The domain `Feed` carries creation and update dates, but feed embedding rows store
both as empty strings. Determine whether semantic feed results should receive
dates from authoritative feed enrichment or from the embeddings read model.

## Requirements

- Compare authoritative enrichment with storing dates in embedding rows.
- Account for search-read cost, duplicated state, update paths, recovery, and
  existing rows with empty dates.
- Preserve embedding checksum and ranking semantics.
- Present the options and tradeoffs without selecting one.

## Remaining work

- Trace creation and update date changes through add, refresh, rename, recovery,
  and semantic search.
- Compare the two read-model boundaries.
- Define repair requirements for existing rows if embedding storage is selected.
- Record the selected boundary in a follow-up implementation task.

## References

- `crates/xaga-core/src/query/items.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/recovery/embeddings.rs`
- `crates/xaga-core/src/persistence/feeds.rs`
