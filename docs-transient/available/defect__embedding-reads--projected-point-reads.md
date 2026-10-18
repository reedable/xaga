# Project embedding point reads to required columns

## Summary

`get_entry` loads every column, including the vector, when some callers need only
a small subset. Add focused projected reads for those callers while retaining
full-row retrieval for mutations.

## Requirements

- Project only the columns required by read-only point callers.
- Keep full-row retrieval for mutations that must preserve all columns during
  replacement.
- Preserve missing-row and decoding behavior.
- Verify that projected calls do not read or materialize the vector column.

## Remaining work

- Classify each `get_entry` call as full-row mutation or partial read.
- Add focused database methods for recurring partial shapes.
- Replace partial-read call sites, beginning with topic membership lookups.
- Test projected decoding and compare representative point-read costs.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src-tauri/src/commands/topics.rs`
- `src-tauri/src/commands/articles.rs`

