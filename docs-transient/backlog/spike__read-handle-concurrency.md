# Shared read handle under concurrent load

## Summary

Application reads share the embedding service's cloned LanceDB handle while the service writes through it. Behavior and throughput under concurrent reads, writes, compaction, and version advancement have not been measured at the design target.

## Requirements

- Verify thread safety and snapshot behavior from LanceDB guarantees and observed tests.
- Measure contention separately from query execution and result materialization.
- Preserve the rule that compaction advances the shared handle before pruned versions become unavailable.
- Do not add pooling or per-request connections without evidence that the shared handle is the bottleneck.

## Remaining work

- Build a workload covering simultaneous list, similarity, state, write, and compaction operations.
- Measure latency, throughput, failures, and visible version behavior at representative scale.
- Compare the shared handle with additional read handles only if contention is demonstrated.
- Record the selected concurrency and freshness contract in durable documentation.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `src-tauri/src/lib.rs`
- `docs/architecture/design_scale.md`
