# Batch rapid Interactive-tier updates

## Summary

Feed refresh, feed import, bookmark import, and bulk state changes enqueue per-entry Interactive jobs. Each state job replaces one LanceDB row, increasing commit and compaction work during large user-initiated operations.

## Requirements

- Preserve Interactive precedence, writer serialization, per-entry domain events, and backend-authoritative UI state.
- Keep vector computation off the Interactive tier.
- Bound batch size and latency so individual Interactive work remains responsive.
- Preserve crash recovery and file/database agreement.
- Base batching thresholds on measurements at the design target.

## Remaining work

- Measure commit count, queue-drain time, event latency, and compaction pauses for large ingestion and state-change bursts.
- Decide which producers and job types warrant batching and select size or time thresholds.
- Design batched row updates without bypassing the library writer for file changes.
- Implement the selected batches and retain fair scheduling between batches.
- Verify an individual state change during a burst and verify recovery after interruption.

## References

- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src-tauri/src/commands/feeds.rs`
- `src-tauri/src/native/feeds.rs`
- `src-tauri/src/native/bookmarks.rs`
- `docs/architecture/design_scale.md`
- `docs/architecture/single_writer_library_lock.md`
