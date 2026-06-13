# Tech Debt

Places introduced or accepted during the LanceDB read-source work where a scale
assumption or a "good enough" judgement was baked in. Recorded as facts and
locations — severity and ordering are not assigned here.

## Embed-vector backfill (the slow per-row pass)

- [ ] **Per-row upsert in `embed_vector`.** Each row does a `get_entry` + `upsert`
  (delete-then-insert) — the same per-row-commit pattern removed from the sync pass.
  Indexed by the time it runs, but still N per-row commits over the whole backlog.
  [pipeline.rs](crates/xaga-core/src/embeddings/pipeline.rs) `embed_vector`,
  [service.rs](crates/xaga-core/src/embeddings/service.rs) embed-vector arm.
- [ ] **`BACKGROUND_COMPACT_INTERVAL = 100`** reused unchanged in the embed-vector arm
  — roughly one compaction per 100 rows across the backfill, each on a growing table.
  [service.rs](crates/xaga-core/src/embeddings/service.rs).
- [ ] **`embed_vector` re-reads the `.md`** that `sync_row` already read — a second
  per-row file read across the two passes. [pipeline.rs](crates/xaga-core/src/embeddings/pipeline.rs).

## Bulk insert (added for fresh recovery)

- [ ] **`bulk_insert` reads files sequentially** (awaits each read) and uses a fixed
  batch size of 1024 — both chosen without surfacing them.
  [service.rs](crates/xaga-core/src/embeddings/service.rs) `bulk_insert`.

## Phase C read API

- [ ] **`all_entries` has no limit/pagination** — "list all" materializes every entry
  row. [db.rs](crates/xaga-core/src/embeddings/db.rs).
- [ ] **`entries_by_ids` builds one `id IN (…)` over all ids, no chunking** — large feed
  id-sets produce a large IN list. [db.rs](crates/xaga-core/src/embeddings/db.rs).
- [ ] **`ensure_indexes` indexes only `id` and `published_at`.** The list sort order is
  `updated_at → published_at → created_at` (`filter::sort_items`); `created_at`/
  `updated_at` are unindexed, so ordered pagination by the actual sort key isn't fully
  covered. [db.rs](crates/xaga-core/src/embeddings/db.rs).

## Search and counts

- [ ] **No vector/ANN index.** `find_related`, `find_by_vector`, `find_related_to_many`,
  `sample_similarities`, `full_scan_scored` all `bypass_vector_index()` — every semantic
  query is an O(N) cosine scan over all rows. Scalar indexes were added; a vector index
  was not. [db.rs](crates/xaga-core/src/embeddings/db.rs).
- [ ] **`embedding_status` runs 2 `count_rows` per kind** (up to 8 filtered counts);
  filtered `count_rows` without supporting indexes scans.
  [db.rs](crates/xaga-core/src/embeddings/db.rs),
  [cmd/embeddings.rs](crates/xaga-cli/src/cmd/embeddings.rs).
- [ ] **`all_entries_meta` materializes the whole table** (id/checksum/reference/
  has_vector/kind for every row) into a `HashMap` on every recovery.
  [db.rs](crates/xaga-core/src/embeddings/db.rs).

## Feed ingestion

- [ ] **`embed_feeds` runs inline in `dispatch_recovery`, per-feed** (read+parse each
  `.atom`, per-feed `get_entry`+`upsert`+ONNX) — blocks the recovery loop, scaled by feed
  count. [pipeline.rs](crates/xaga-core/src/embeddings/pipeline.rs),
  [service.rs](crates/xaga-core/src/embeddings/service.rs).

## Migrations

- [ ] **`migrate-sot` and `migrate-pins` process files sequentially** (read → render →
  write, one at a time) across the whole library.
  [cmd/migrate_sot.rs](crates/xaga-cli/src/cmd/migrate_sot.rs),
  [cmd/migrate_pins.rs](crates/xaga-cli/src/cmd/migrate_pins.rs).

## Phase C cutover — assumptions to resolve when building it

Forward-looking; the read-side cutover isn't built yet. Excluded here because they're
not assumptions: per-feed listing by id-set and `EntryStateCache` retirement are ratified
roadmap decisions; whole-library pagination and full-scan (no-ANN) search are already
listed above.

- [ ] **AppState read handle (cutover step 1).** A single shared read connection
  alongside the service's writer, under concurrent read/write, with one shared handle (no
  pooling, no per-request connection). Behaviour under concurrent load at the design scale
  is unexamined. ([lib.rs](src-tauri/src/lib.rs) `AppState`.)
- [ ] **Per-feed / per-topic counts (cutover step 4).** Deriving unread/bookmark counts
  from LanceDB on each `list_feeds`/`list_topics` call — count throughput scaled by feed/
  topic count, and the count path scans without supporting indexes (same root as the
  `embedding_status` count item above).
- [ ] **`list_entry_topics` via `get_entry` (cutover step 5).** `get_entry` loads every
  column including the 384-float vector; reading an entry's `topics` through it is an
  unprojected read. ([db.rs](crates/xaga-core/src/embeddings/db.rs).)
- [ ] **Cutover verification (cutover step 8).** Planned as functional verification on the
  running app, with no load/scale verification at the design target.
