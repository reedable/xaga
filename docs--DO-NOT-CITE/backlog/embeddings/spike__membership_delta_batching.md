# Membership delta-batching threshold

## Summary

Individual Topic membership edits enqueue one Interactive `AddMember` or `RemoveMember` job. Determine whether rapid edit bursts require batching; full Topic reconciliation already uses a separate chunked path.

## Requirements

- Measure before selecting a threshold.
- Preserve edit order and the final bidirectional Topic/member relationship.
- Bound latency so a single edit is not held indefinitely waiting for a batch.
- Keep this work separate from wholesale `reconcile_topic_membership` behavior.

## Remaining work

- Measure commit count, queue latency, and compaction during representative edit bursts.
- Define the burst source and maximum expected rate.
- Compare immediate deltas, time-bounded coalescing, and ID-bounded batches.
- If batching is warranted, define cancellation of opposing deltas and implement consistency tests.

## References

- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src-tauri/src/commands/topics.rs`
- `src-tauri/src/commands/articles.rs`
