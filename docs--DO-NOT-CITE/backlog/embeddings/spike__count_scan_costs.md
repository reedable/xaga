# Measure embedding count costs

## Summary

Embedding status performs filtered counts per kind. Feed and Topic summaries materialize global unread and bookmark ID sets and intersect them with collection membership. Their costs at the design target are unknown.

## Requirements

- Measure before changing storage or count semantics.
- Preserve authoritative membership and state results.
- Include result cardinality, not only index lookup time, when evaluating materialized ID sets.
- Avoid denormalized counters without a defined repair and consistency model.

## Remaining work

- Benchmark embedding status and feed and Topic list counts at target row and membership sizes.
- Attribute time and allocation to filtered counts, ID-set materialization, and membership intersection.
- Compare current queries, per-collection database counts, and repairable denormalized counters if measurements require change.
- Present the measured bottleneck and options for decision.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-cli/src/cmd/embeddings.rs`
- `src-tauri/src/commands/feeds.rs`
- `src-tauri/src/commands/topics.rs`
- `docs/architecture/design_scale.md`
