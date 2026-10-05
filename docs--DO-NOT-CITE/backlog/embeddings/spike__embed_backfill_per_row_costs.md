# Embed-vector backfill per-row costs

Scale assumptions baked into the slow per-row backfill pass, to investigate:

- **Per-row upsert in the embed pass.** Each row does a `get_entry` + `upsert` (delete-then-insert) — N per-row commits over the whole backlog (`crates/xaga-core/src/embeddings/pipeline.rs`, the embed-vector functions; `service.rs`, the embed arm).
- **Compaction interval reused unchanged.** `BACKGROUND_COMPACT_INTERVAL = 100` applies in the embed arm — roughly one compaction per 100 rows across the backfill, each on a growing table (`crates/xaga-core/src/embeddings/service.rs`).
- **Double file read across the two passes.** The embed pass re-reads the `.md` that the sync pass already read (`pipeline.rs`).

At the design target (`docs/architecture/design_scale.md`), the backlog is the whole corpus. Measure the impact and choose a fix.
