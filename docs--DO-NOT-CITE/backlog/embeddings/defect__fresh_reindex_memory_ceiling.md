# Bound fresh-reindex memory

## Summary

Fresh recovery collects domain objects and enqueues content-carrying embedding jobs for the corpus. Its peak memory therefore grows with corpus size and is unbounded at the design target.

## Requirements

- Bound peak recovery and queued-payload memory independently of corpus size.
- Preserve the rule that embedding consumes the authoritative object produced by the recovery read unless a different consistency model is explicitly selected.
- Keep bulk database insertion and embedding backpressure compatible with priority-tier fairness and shutdown behavior.
- Do not replace memory pressure with unbounded file rereads or queued identifiers.

## Remaining work

- Measure peak memory and queue depth during fresh recovery at representative corpus sizes.
- Compare bounded channels, producer-consumer chunking, and staged recovery batches, including their consistency and throughput costs.
- Select a backpressure and ownership model.
- Implement bounded dispatch and verify memory, cancellation, restart, and Interactive-tier responsiveness.

## References

- `crates/xaga-core/src/recovery/dispatch.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `docs/architecture/design_scale.md`
- [Job payload modeling](spike__job_payload_modeling.md)
- [Embedding backfill costs](spike__embed_backfill_per_row_costs.md)
