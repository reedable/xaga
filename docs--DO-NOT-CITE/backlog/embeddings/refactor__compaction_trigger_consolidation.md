# Consolidate the compaction triggers

## Summary

The service triggers compaction through separate Interactive intervals, Regular and Maintenance intervals, post-Interactive-drain handling, explicit recovery jobs, and database reset flows. Their shared policy and counter-reset behavior are distributed through the run loop.

## Requirements

- Preserve bounded version and fragment growth during sustained work on every tier.
- Avoid redundant compactions and user-visible pauses.
- Keep recovery's final compaction and index creation ordered correctly.
- Make cadence and reset rules explicit and independently tunable where priorities require it.

## Remaining work

- Enumerate every trigger and the state each trigger resets.
- Measure compaction duration and fragment growth for Interactive bursts and recovery backfills.
- Define a single policy abstraction or justify the triggers that remain tier-specific.
- Consolidate implementation and add tests for interval, drain, and explicit-compaction behavior.

## References

- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/recovery/dispatch.rs`
