# Count costs: scans and full-set materialization

Read costs to investigate:

- **`embedding_status` runs 2 `count_rows` per kind** (up to 8 filtered counts); filtered `count_rows` without supporting indexes scans (`crates/xaga-core/src/embeddings/db.rs`, `crates/xaga-cli/src/cmd/embeddings.rs`).
- **Per-feed / per-topic counts** derive unread/bookmark counts from LanceDB on each `list_feeds`/`list_topics` call — count throughput scaled by feed/topic count.
- **Residual after the Bitmap indexes:** the per-refresh counts materialize the full unread id-set; the Bitmap makes _finding_ unread rows index-fast, but the set size is inherent. Recorded revisit options if the sidebar lags at the design scale: denormalized counts, or per-collection `count_rows`.

Severity and ordering unassigned; no measurement at the design target exists.
