# Batch rapid Interactive-tier updates

## Summary

Feed refresh, feed import, bookmark import, bulk state changes, and rapid Topic membership edits enqueue per-item Interactive jobs. Each state or membership job can replace rows independently, increasing commit and compaction work during bursts.

## Requirements

- Preserve Interactive precedence, writer serialization, per-entry domain events, and backend-authoritative UI state.
- Keep vector computation off the Interactive tier.
- Bound batch size and latency so individual Interactive work remains responsive.
- Preserve crash recovery and file/database agreement.
- Base batching thresholds on measurements at the design target.
- Preserve membership edit order and the final bidirectional Topic/member relationship.
- Keep individual-delta batching separate from wholesale `reconcile_topic_membership` behavior.

## Remaining work

- Measure commit count, queue-drain time, event latency, and compaction pauses for large ingestion, state-change, and individual membership-delta bursts.
- Decide which producers and job types warrant batching and select size or time thresholds.
- For membership deltas, define the burst source and compare immediate writes, time-bounded coalescing, and ID-bounded batches, including cancellation of opposing deltas.
- Design batched row updates without bypassing the library writer for file changes.
- Implement the selected batches and retain fair scheduling between batches.
- Verify individual state and membership changes during a burst, final relationship consistency, and recovery after interruption.

## References

- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src-tauri/src/commands/feeds.rs`
- `src-tauri/src/commands/topics.rs`
- `src-tauri/src/commands/articles.rs`
- `src-tauri/src/native/feeds.rs`
- `src-tauri/src/native/bookmarks.rs`
- `docs/architecture/design_scale.md`
- `docs/architecture/single_writer_library_lock.md`
