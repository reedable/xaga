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

- [ ] **`embedding_status` runs 2 `count_rows` per kind** (up to 8 filtered counts);
  filtered `count_rows` without supporting indexes scans.
  [db.rs](crates/xaga-core/src/embeddings/db.rs),
  [cmd/embeddings.rs](crates/xaga-cli/src/cmd/embeddings.rs).
- [ ] **`all_entries_meta` materializes the whole table** (id/checksum/reference/
  has_vector/kind for every row) into a `HashMap` on every recovery.
  [db.rs](crates/xaga-core/src/embeddings/db.rs).
- [ ] **Embedded feed results return `entry_count`/`unread_count`/`bookmarked_count` = 0.**
  `enrich_embedded` builds `EmbeddedItem::Feed` without real stats; the server-side lookup
  (read the feed's `.atom` membership + intersect with the unread/bookmarked id-sets via
  `feed_summary`) was deferred. Decided design when picked up: thread `feeds_dir` + unread/
  bookmarked sets from the `query_by_text` command (same source as `list_feeds`), or open the
  db inside `enrich_embedded` only when a feed hit is present. Counts are O(U)+O(M) (see the
  per-feed count discussion). [item.rs](crates/xaga-core/src/item.rs) `enrich_embedded`.

## Feed ingestion

- [ ] **`embed_feeds` runs inline in `dispatch_recovery`, per-feed** (read+parse each
  `.atom`, per-feed `get_entry`+`upsert`+ONNX) — blocks the recovery loop, scaled by feed
  count. [pipeline.rs](crates/xaga-core/src/embeddings/pipeline.rs),
  [service.rs](crates/xaga-core/src/embeddings/service.rs).

## Phase C cutover — decided tradeoffs to revisit

- [ ] **Read handle reloads before every read.** Each repointed command calls
  `EmbeddingsDb::reload()` (`checkout_latest`) before querying, so reads are always current
  (handles are version-pinned). Chosen for simplicity/correctness over a per-read cost;
  `reload` re-reads the latest manifest (version + fragment list) on every read, even when
  nothing changed. Revisit (e.g. reload-on-`embedding:changed`) if read latency becomes a
  problem at the design scale. ([commands](src-tauri/src/commands/), `db.rs` `reload`.)

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

## Frontend type branding (`__kind`)

- [ ] **`toSavedItem` does not mint `__kind`, so the guards can't yet require brand *and*
  shape.** The intended model: an `Entry` or `Feed` is identified by **both** the shape
  *and* the brand matching — neither alone is sufficient. The shape check is the shared
  `isWebResource` (`Item & url`), the structural foundation common to Entry and Feed; it is
  never removed. `__kind` layers on top to resolve the role (feed vs entry) that the shared
  shape cannot. Target guards:
  - `isEntry(item)  = isWebResource(item) && item.__kind === "entry"`
  - `isFeed(item)   = isWebResource(item) && item.__kind === "feed"`
  (`isWebResource` is not yet extracted as a helper — extract it and have both guards call it.)

  Blocker: `Entry`/`Feed` are branded and minted at the wire boundary
  (`toEntry`/`toFeed`/`toAnyEmbeddedItem`), but `SavedItem` (the DTO for `listAllItems` /
  `listTopicItems` — the All-items and Topic lists) is **not** branded. So requiring the brand
  would drop every saved row. Until `toSavedItem` mints `__kind:"entry"` on its bookmark/entry
  rows (annotation rows stay detected by `isAnnotation`'s shape), the guards run an interim
  **brand-first, shape-fallback** (a present brand decides; absent a brand, shape alone). Once
  `toSavedItem` mints, switch the guards to the brand-**and**-shape form above.
  [TauriPlatform.ts](src/platform/TauriPlatform.ts) `toSavedItem`,
  [types.ts](src/types.ts) `isEntry`/`isFeed`/`isWebResource`.
