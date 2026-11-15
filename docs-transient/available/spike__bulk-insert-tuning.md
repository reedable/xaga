# Tune fresh-recovery bulk insertion

## Summary

Fresh recovery walks source files and inserts rows in fixed batches of 1,024. The effects of read concurrency and batch size on throughput, memory, and Interactive responsiveness have not been measured at the design target.

## Requirements

- Keep memory and outstanding I/O bounded.
- Preserve deterministic recovery outcomes and the source-read reliability checks that gate deletion.
- Do not starve Interactive work while recovery reads or commits batches.
- Select concurrency and batch size from measurements rather than a fixed assumption.

## Remaining work

- Measure source-read, row-construction, batch-commit, and embedding time separately.
- Benchmark representative batch sizes and bounded read concurrency on realistic libraries.
- Decide whether settings should be constants, adaptive values, or configuration.
- Implement the selected tuning and verify memory, throughput, and foreground latency.

## References

- `crates/xaga-core/src/recovery/dispatch.rs`
- `crates/xaga-core/src/recovery/embeddings.rs`
- `crates/xaga-core/src/embeddings/db.rs`
- `docs/architecture/design_scale.md`
