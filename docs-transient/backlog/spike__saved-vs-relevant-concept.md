# Define Saved and Relevant

## Summary

Saved and Relevant do not have consistent meanings. `Saved` currently filters to bookmarked entries or annotations with a parent reference; a manually curated topic entry is excluded unless it is also bookmarked. Topic results instead combine curated members with items above a similarity threshold, then `Relevance` orders that union by cosine similarity to the topic's own embedding. No topic-plus-members centroid exists.

## Requirements

- Define Saved independently of the current filter implementation.
- Define Relevant and Similar, including whether they name item state, query membership, or ordering.
- Define how topic membership, bookmarks, annotations, similarity thresholds, and relevance scores affect each concept.
- Keep global item lists and topic item lists consistent with the definitions.
- Use the settled Bookmark and Collection definitions.

## Remaining work

- Trace Saved and relevance behavior through query filtering, topic result construction, and UI controls.
- Identify cases where an item's classification changes only because it is viewed inside or outside a topic.
- Present definition and naming options with their query and UI consequences for decision.
- Record the chosen definitions in durable domain documentation and update the filters, topic queries, and UI vocabulary consistently.

## References

- `crates/xaga-core/src/types.rs`
- `crates/xaga-core/src/query/filter.rs`
- `crates/xaga-core/src/query/topics.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `src/types.ts`
- `src/components/list/ItemList.tsx`
