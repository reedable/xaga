# Consolidate the compaction triggers

Enumerate compaction triggers in `crates/xaga-core/src/embeddings/service.rs`
and consolidate them under one policy. Review queue, interval, drain, and
per-tier triggers, including `BACKGROUND_COMPACT_INTERVAL` and the Interactive
interval. Measure backfill cost and user-visible pauses when selecting cadence.
