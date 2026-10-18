# Consolidate parallel indexing and domain construction

## Summary

Indexing retains parallel per-kind implementations for row synchronization, vector embedding, fresh-recovery batching, row construction, and embedding-text selection. `Article` is also constructed independently in `md_to_article`, `article_from_fm`, and `set_entry_tags`.

## Requirements

- Consolidate only logic with the same contract and failure behavior.
- Keep kind-specific behavior explicit where consolidation would hide meaningful differences.
- Do not introduce mode flags or generic abstractions whose branches reproduce the current implementations internally.
- Preserve indexing priority, annotation reconciliation, checksum, vector carry-forward, and batch-flush behavior.

## Remaining work

- Compare `sync_entry_row` with `sync_topic_row` and identify the exact shared and kind-specific steps.
- Compare `embed_entry_vector`, `embed_topic_vector`, and `embed_feed`, including where checksum gating occurs.
- Compare the article/topic bulk-insert loops and the three `EmbeddingEntry` row builders.
- Determine whether the three embedding-text helpers contain shared logic or only share shape.
- Compare the `Article` construction paths and identify possible shared domain conversion boundaries.
- Present consolidation, parameterization, and documented-duplication options for each group before implementation.

## References

- `docs/principles/dry.md`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/recovery/dispatch.rs`
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/persistence/articles.rs`
- `crates/xaga-core/src/persistence/entries.rs`
