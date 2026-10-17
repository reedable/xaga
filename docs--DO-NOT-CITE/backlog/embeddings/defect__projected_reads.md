# Unprojected reads pull the vector column

## Summary

`get_entry` loads every column, including the vector. Read paths such as `list_entry_topics` use it for a small subset of fields and pay for unrelated data.

## Requirements

- Project only the columns required by read-only callers.
- Keep full-row retrieval for mutations that must preserve all columns during replacement.
- Preserve missing-row and decoding behavior.
- Verify the optimization with query plans or measured bytes and latency.

## Remaining work

- Classify each `get_entry` call as full-row mutation or partial read.
- Add focused database methods for the recurring partial shapes.
- Replace partial-read call sites, beginning with topic membership lookups.
- Test projected decoding and compare representative read costs.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src-tauri/src/commands/topics.rs`
- `src-tauri/src/commands/articles.rs`
