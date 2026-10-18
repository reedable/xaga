# Reduce embedding backfill costs

## Summary

Backfill computes vectors one object at a time, retrieves the existing row, replaces it, and periodically compacts the growing table. Commit and compaction costs scale per row across the corpus. The current object-carrying path does not reread files during the embedding pass.

## Requirements

- Measure embedding compute separately from database read, write, and compaction time.
- Preserve checksum gating and reconcile-managed columns during vector updates.
- Bound memory and retain foreground-tier responsiveness.
- Keep interrupted backfills restartable from durable state.

## Remaining work

- Profile fresh and incremental backfills at representative scale.
- Compare vector-only updates, bounded batch replacement, and current per-row replacement.
- Evaluate compaction cadence together with the selected write shape.
- Implement the selected approach and verify restart, checksum, membership, and latency behavior.

## References

- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/db.rs`
- `docs/architecture/design_scale.md`
- [Bound fresh-reindex memory](defect__fresh-reindex-memory-ceiling.md)
