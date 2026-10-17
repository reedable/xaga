# Project embedding reads to required columns

## Summary

`get_entry` loads every column, including the vector, when some callers need only a small subset. Recovery's `list_entries_meta` also reads and collects the full table before building its metadata map, materializing every non-null vector even though recovery needs only identity, checksum, reference, vector presence, kind, and membership.

## Requirements

- Project only the columns required by read-only callers.
- Keep full-row retrieval for mutations that must preserve all columns during replacement.
- Preserve missing-row and decoding behavior.
- Preserve recovery detection of missing, stale, pending-vector, annotation-drift, and orphaned rows.
- Preserve the rule that an incomplete filesystem walk cannot authorize deletion.
- Measure recovery batch collection separately from metadata-map construction and retention.
- Verify projected reads with query plans or measured bytes, memory, and latency.

## Remaining work

- Classify each `get_entry` call as full-row mutation or partial read.
- Add focused database methods for the recurring partial shapes.
- Replace partial-read call sites, beginning with topic membership lookups.
- Add an explicit projection to `list_entries_meta` and measure representative recovery duration and peak memory.
- Determine whether projected batches and the metadata map may still overlap acceptably at the design target; compare streaming or a different diff shape if not.
- Test projected decoding and compare representative point-read and full-recovery costs.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/recovery/embeddings.rs`
- `src-tauri/src/commands/topics.rs`
- `src-tauri/src/commands/articles.rs`
- `docs/architecture/design_scale.md`
- `docs/architecture/crash_consistency.md`
