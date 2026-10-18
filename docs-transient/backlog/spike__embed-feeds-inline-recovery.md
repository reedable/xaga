# Measure inline feed embedding during recovery

## Summary

Recovery reads and parses every saved feed, then embeds each missing or stale feed sequentially inside `dispatch_recovery`. Because the embedding service awaits `dispatch_recovery` from its control arm, it cannot process another queued job until recovery dispatch returns.

Determine how much of that blocking time comes from feed discovery and feed embedding at the design target. No fix direction is selected by this task.

## Requirements

- Measure feed discovery separately from embedding the feeds that require repair.
- Measure the per-feed database lookup, ONNX inference, and upsert together, and report how total time changes with feed count.
- Run the measurement at the design target, including the expected number and representative sizes of saved `.atom` files.
- Record whether inline feed work materially delays the embedding service from returning to its scheduling loop.
- If a change is warranted, present options and their tradeoffs for a separate decision; do not select or implement one in this task.

## Remaining work

- Establish the design-target feed count and representative `.atom` file sizes.
- Measure the time to read, parse, and compare all saved feeds with their database metadata.
- Measure the sequential `embed_feed` loop with controlled counts of missing or stale feed embeddings.
- Report the observed scheduling delay and identify which phase dominates it.

## References

- [Feed recovery diff](../../crates/xaga-core/src/recovery/embeddings.rs)
- [Inline recovery dispatch](../../crates/xaga-core/src/recovery/dispatch.rs)
- [Feed embedding pipeline](../../crates/xaga-core/src/embeddings/pipeline.rs)
- [Embedding service control loop](../../crates/xaga-core/src/embeddings/service.rs)
- [Feed persistence](../../crates/xaga-core/src/persistence/feeds.rs)
