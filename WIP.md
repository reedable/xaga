# WIP — LanceDB as the read source (Phase C cutover)

Resume-point for the in-progress roadmap. Full plan:
`~/.claude/plans/lancedb-embeddings-column-is-refactored-bentley.md`. Granular debt is in
[TECH_DEBT.md](TECH_DEBT.md); discrete tasks are in [TODO.md](TODO.md). This file tracks the
**cutover state**, the **agreed topic reverse-index design**, and remaining work.

## Goal

`.atom` = source of truth for feed **membership**; `.md` = source of truth for entry
**content + state**; LanceDB = the queryable **read layer** that mirrors them. Phase C repoints
the list commands to read from LanceDB and then retires `EntryStateCache` and the feed-search hack.

## Read-command repoints

| Command | Status | Notes |
| --- | --- | --- |
| `list_bookmarks` ([bookmarks.rs](src-tauri/src/commands/bookmarks.rs)) | ✅ user-verified | `bookmarked_entries()` + url sort |
| `list_saved` ([saved.rs](src-tauri/src/commands/saved.rs)) | ✅ repointed; re-verify on relaunch | bookmarks + annotations from LanceDB; see annotation fix below |
| `list_entries` per-feed ([feeds.rs](src-tauri/src/commands/feeds.rs)) | ✅ compiles; pending relaunch verify | `.atom` id-set → `entries_by_ids` → filter/sort; **first command off `EntryStateCache` for reads** |
| `list_all_articles` ([entries.rs](src-tauri/src/commands/entries.rs)) | ✅ compiles; pending relaunch verify | `all_entries()` + url sort |
| `list_entry_topics` ([topics.rs](src-tauri/src/commands/topics.rs)) | ✅ done + user-verified (entries + annotations) | reads LanceDB `topics` reverse index; maintained by the reconcile (T1–T6 + W1–W4) |
| sidebar feed/topic counts | ✅ repointed; verify on relaunch | cutover step 4 — Bitmap indexes + `unread_ids`/`bookmarked_ids`; see "Remaining cutover steps" + TECH_DEBT counts items |

Supporting work done:
- **AppState read handle** ([lib.rs](src-tauri/src/lib.rs)) — shared LanceDB read connection opened
  after the service; reads `reload()` before querying (handles are version-pinned). Reload-before-read
  strategy chosen; per-read cost logged in [TECH_DEBT.md](TECH_DEBT.md).
- **Annotation two-phase fix** ([pipeline.rs](crates/xaga-core/src/embeddings/pipeline.rs)) —
  `sync_annotations` now reconciles annotation **rows** in the embedder-free fast phase, so a new
  highlight is listable immediately (was gated behind the embed backlog). 15/15 core tests pass.
  Pending user relaunch verify (highlight → appears in Saved; color change; un-highlight).

## `list_entry_topics` — topic membership reverse index (cutover step 5)

**Status: ✅ COMPLETE (T1–T6 + W1–W4). `list_entry_topics` reads the LanceDB reverse index for
entries; the `annotation → topics` index is built; the dead `topics:` frontmatter is cleaned. Verified
by the user on relaunch (entries + annotations) and the data migration was run + confirmed. Manual
topic-filing can be re-enabled.**

`list_entry_topics(entry_id)` returns the **topic ids** an entry belongs to. Old impl
([topics.rs:275](crates/xaga-core/src/topics.rs#L275)) reads `topic.items` from the topic `.md`.

### Why the naive repoint was rejected

The earlier plan moved membership into each member's `topics:` frontmatter and read the mirrored
column. But the membership write path — `add_topic_item` / `remove_topic_item`
([topics.rs:239](crates/xaga-core/src/topics.rs#L239), [:257](crates/xaga-core/src/topics.rs#L257))
— writes **only the topic file's `items`** (via `save_topic`), never the member's frontmatter. So
the column drifts, and a recovery rebuilt from stale frontmatter would *drop* membership changes.
Making the member frontmatter the SoT also loses what `topic.items` carries: **insertion order**,
**type** (`Entry` vs `Annotation`), and the annotation **`reference`**.

### Decided design — topic `.md` stays SoT; the column is a derived reverse index

Two directions, treated differently (the key insight):
- **Forward — topic → its members** (the topic list view): read the topic `.md` directly. This is
  exactly the `.atom` pattern (collection file yields the member id-set; LanceDB supplies member
  data). The topic file **doubles as source-of-read for this direction only**. Already how
  `list_topic_items` works. **No change.**
- **Reverse — entry/annotation → its topics** (`list_entry_topics`): served by a **derived reverse
  index** in LanceDB. The *only* thing that needs the column.

1. **SoT = topic `.md`** — owns membership (entries **and** annotations), ordered, typed, carrying
   the annotation `reference`. Unchanged. The membership write path (`add_topic_item` /
   `remove_topic_item`) stays as-is, writing only the topic file.
2. **Topic LanceDB row stores its member-id list** — *not* for queries; it is the **prior-membership
   base** for cheap delta computation (point-read by topic id on the BTree `id` index, ~O(log N)).
   Rebuilt on recovery. (Alternative considered: in-memory map seeded at startup — chose the row so
   it survives restart.)
3. **Each entry *and annotation* row carries a `topics` column** = the derived reverse index. Serves
   `list_entry_topics` only; never used for forward listing. (Members include annotation rows, so
   the column and the fan-out treat members uniformly as LanceDB rows by id — not entries only.)
4. **Live maintenance — synchronous inline fan-out.** A topic `.md` change → watcher →
   `Embed(topic_id)` on the queue → in **one `run_loop` turn**, synchronously and **off the queue**:
   - diff **new** items (from the topic `.md`) against **old** (the topic row's stored member list);
   - for **added** members, add `topic_id` to their `topics` column; for **removed**, remove it
     (touch only the delta — usually 1);
   - **also refresh the topic row's stored member list to the new set** (so the next diff has a
     correct base).
   Doing all of this inline keeps it **atomic w.r.t. graceful shutdown** — the biased `select!` only
   re-checks between turns, so shutdown can't catch a half-applied delta. Chunk only the rare bulk
   cases (topic delete, bulk import) to avoid blocking the loop.
5. **Recovery backstop — unconditional reverse-index rebuild.** Recovery reads the topic `.md`
   (like it already reads `.atom`) and rebuilds the topic-row member lists + member `topics` columns.
   **This must be decoupled from the checksum/embedding gate**: a membership-only change never
   touches the entry's `.md`, so `build_recovery_plan`'s gate would skip it. So the rebuild runs every
   recovery regardless of per-entry embedding state. Cheap (column writes, no ONNX). Backstops a
   *dropped topic event* (shutdown before it ran) and a *hard crash mid-fan-out* (the fan-out is
   several separate commits, not one transaction — synchronicity covers graceful shutdown, not a kill).
6. **Drop the entry/annotation frontmatter `topics:` field** — the drift source. The `migrate-sot`
   artifact in entry files becomes dead.

### Implementation tasks (work order: T1 → T2 → T4 → T3 → T5 → T6)

- **T1 — Column ownership (load-bearing). ✅ DONE** (compiles, 16/16 tests). The entry/annotation
  sync sites (`sync_row`, `embed_vector`, `sync_annotations`, `embed_annotations`) now **carry the
  existing `topics` column forward** on every upsert instead of sourcing it from `fm`, and `topics`
  was dropped from `sync_row`'s change-trigger. Routine edits can no longer wipe the index.
- **T2 — Topic row `items` column. ✅ DONE** (compiles, 16/16 tests incl. `items_column_round_trips`).
  New nullable `items` `List<String>` column ([db.rs](crates/xaga-core/src/embeddings/db.rs)) holding
  a topic row's member ids (the prior-membership base). Populated from the topic `.md` `items:` via
  `to_embedding_entry`. **Rollout: `open()` adds the column to a pre-existing table via
  `add_columns(AllNulls)` — verified metadata-only (no rewrite, no re-embed); no reset needed.**
- **T3 — Live fan-out. ✅ DONE** (compiles, 17/17). The priority-queue topic `Embed` handler
  ([service.rs](crates/xaga-core/src/embeddings/service.rs)) calls `reconcile_topic_membership`
  inline after `sync_row` when the changed file is under `topics/` — synchronous, off the queue;
  covers entries + annotations (the reconcile is member-type-agnostic; it's fed `topic.items`).
- **T4 — Unconditional recovery rebuild. ✅ DONE** (compiles, 17/17 incl. the reconcile test). From
  topic files (point 5), decoupled from the checksum
  gate. **Covers entries and annotations** (annotation rows exist post-`bulk_insert`; W1/W2 widened
  this from the original entries-only scope). Shares the reconcile core with T3: per
  topic, diff file members vs the topic-row `items` base → add/remove `topic_id` on the delta members'
  `topics` → set `items` = file members; bounded by drift (first run O(total memberships),
  column-writes only). **Refines T2:** `items` is reconcile-managed — `to_embedding_entry` writes it
  empty, `sync_row`/`embed_vector` carry it forward like `topics`, and the reconcile is its sole
  writer, so the column *lags* the file and the diff is meaningful.
- **T5 — Repoint `list_entry_topics`. ✅ DONE.** Reads the entry's `topics` column via the AppState
  read handle (`reload` + `get_entry`) — pulls the full vector (TECH_DEBT; projected read later).
- **T6 — Stop sourcing `topics` from frontmatter. ✅ CORE DONE.** `to_embedding_entry` writes `topics`
  empty (reconcile-managed). **Full removal of the `topics:` field from render/parse is DEFERRED** to
  the annotation follow-up — it's entangled with annotation `topics` (annotations stay on the old
  path for now). The dead field in entry `.md` is inert (ignored).

### Open verification items (before/while building)

- **Shutdown semantics:** does `svc.shutdown()` **drain** the queue or **abort** it? Determines
  whether a queued `Embed(topic_id)` can be lost (→ relies on the recovery backstop) vs. always runs.
- **Optional hardening:** batch the member-row updates into a **single `merge_insert` commit** per
  topic change, shrinking the hard-crash window toward zero.
- **`LabelList` index:** only relevant if we ever reverse-scan the `topics` column; the point-read base
  avoids it. Verify `lancedb` 0.27 support only if that path is ever needed.

### `annotation → topics` reverse-index — ✅ W1 + W2 DONE (compiles, 18 tests)

- **W1 ✅** — `bulk_insert` now creates annotation rows (null-vector) alongside entries, so all
  members exist at recovery time.
- **W2 ✅** — the recovery + live reconcile callers now pass *all* members (entries + annotations);
  the reconcile core was already member-type-agnostic. Covered by `reconcile_handles_annotation_members`.

**W3 ✅ DONE** (compiles, 17 tests) — removed `topics` from `render_entry` + `Frontmatter`/
`FrontmatterRaw` + `Annotation`, deleted the now-obsolete `migrate` and `migrate-sot` CLI commands,
updated all callers. Parsing safely ignores the now-unknown `topics:` key in existing `.md`.

**W4 ✅ DONE** — user ran `migrate-strip-topics` (app-closed) and confirmed; the throwaway command
has been deleted (workspace compiles). The dead entry-level and per-annotation `topics:` frontmatter
is gone.

**✅ DONE** — the scheduled follow-up (originally scoped out of the entries-only T4) is complete.
`bulk_insert` now creates each `.md`'s annotation rows (null-vector) in the bulk phase
([service.rs](crates/xaga-core/src/embeddings/service.rs) `bulk_insert`), so all member rows
(entries + annotations) exist before the recovery reconcile — which iterates `topic.items` (entries
**and** annotations) into `reconcile_topic_membership` (the reconcile core is member-type-agnostic;
test `reconcile_handles_annotation_members`). So the `annotation → topics` reverse index is built for
both kinds, and the bulk-insert annotation-row parity (also the from-reset listability fix) is done.

**Stale references to clean up elsewhere:** [TODO.md](TODO.md) still lists "`bulk_insert`
annotation-row parity + coupled annotation→topics" as open; the `embed_annotations` doc-comment
([pipeline.rs:199](crates/xaga-core/src/embeddings/pipeline.rs#L199)) still says "`bulk_insert`
inserts entry rows … but not annotation rows"; and the T4 note above still reads "Scoped to ENTRY
members." All three predate this completion.

## Remaining cutover steps (after the blocker)

- ✅ **Sidebar feed/topic counts** repointed to LanceDB (cutover step 4). Added **Bitmap** indexes on
  `is_unread`/`is_bookmarked`/`is_pinned` ([db.rs](crates/xaga-core/src/embeddings/db.rs)
  `ensure_indexes`, built via recovery's `Compact`), `unread_ids()`/`bookmarked_ids()` (index-driven,
  `id`-only projection), and `get_unread`/`get_bookmarked` ([lib.rs](src-tauri/src/lib.rs)) now read
  LanceDB, not the cache. Compiles + 17 tests. **Verify on relaunch** (bitmaps build once; counts
  should match the old values). Residual scale note: the per-feed/topic counts materialize the full
  unread id-set per refresh — the Bitmap makes *finding* unread rows index-fast, but the set size is
  inherent; revisit if the sidebar lags at the design scale (denormalized counts / per-collection
  `count_rows`).
- ✅ **Retired `EntryStateCache`** (cutover step 6). Deleted the `entry_cache` module +
  `ServiceMsg::RecoverCache` + `trigger_cache_recovery`; simplified `EmbeddingService::shutdown` (no
  cache/persist arg); removed the `AppState` field + all wiring; file watchers no longer touch it;
  `is_cache_seeded` → **`is_cache_ready`** (repointed to the read handle; frontend `isCacheReady`,
  `useCacheReady` kept). Workspace compiles + 17 tests; **rebuild the frontend** (TS rename) and verify
  on relaunch. Notes: the `.entry_state_cache.json` snapshot is now an orphaned file (harmless);
  startup no longer does the cache scan/seed (state is read on-demand from LanceDB). Possible dead-code
  follow-up: `annotate_*` / `filter::unread_ids`/`bookmarked_ids` ([filter.rs](crates/xaga-core/src/filter.rs))
  if now unused.
- ✅ **Feed-search hack** (cutover step 7) — **DONE** (compiles + tsc clean; not yet relaunch-verified).
  Decided **hybrid (C)**: feeds searchable both semantically and by name.
  - ✅ **Part 1** — feeds embed **title + `atom:subtitle`** for real semantic content (`FeedMeta.subtitle`,
    `feed_embedding_meta` reads `feed.subtitle()`, `embed_feeds` embeds title+subtitle, folds subtitle into
    `feed_checksum`, stores it in `summary`). Re-embeds all feeds on next recovery. **Verify on relaunch.**
  - ✅ **Part 2 — `Feed`/`Entry` branding + hybrid tab.**
    - **Backend:** promoted `Kind` to [types.rs](crates/xaga-core/src/types.rs) (serde lowercase, `as_str`,
      `FromStr`, `From<&CollectionItem>`), retired `RowKind`, `predicate()` kept as `impl Kind` in db.rs;
      `EmbeddingEntry.kind: Kind` (Arrow-converted at edges); `EmbeddedEntry` gained `kind: Kind` threaded
      through all four query paths via `BatchSummaryColumns::kind_at`; new `EmbeddedFeed` + `EmbeddedItem::Feed`;
      `enrich_embedded` routes feed hits to `EmbeddedItem::Feed`.
    - **Frontend:** `WebResource`, branded `Entry`/`Feed` (`readonly __kind`), minted in `toEntry`/`toFeed`/
      `toAnyEmbeddedItem` (`EmbeddedFeedSchema`, `kind` on `EmbeddedEntrySchema`). `isEntry`/`isFeed` are
      **interim brand-first / shape-fallback** (shape is the foundation, never removed; `__kind` only resolves
      the feed-vs-entry role). Hybrid feeds tab in [SearchResultList.tsx](src/components/search/SearchResultList.tsx):
      name-matches first, then `results.filter(isFeed)` semantic hits appended, deduped by id.
  - **Deferred (in [TECH_DEBT.md](TECH_DEBT.md)):** embedded feed counts return **0** (server-side stats lookup
    in `enrich_embedded` deferred); `toSavedItem` does **not** mint `__kind` yet → once it does, extract
    `isWebResource` and switch guards to the brand-**and**-shape form.
  - Design rule from the long discussion: **shape is SoT for *intrinsic* types** (no discriminator); **`__kind` is a *derived distillation*, legitimate only for *extrinsic* role distinctions** (mint from SoT, never trust over shape). For the *intrinsic* kinds (Annotation, Topic) cross-validate `__kind` against shape at the boundary. Don't brand composite/view types.
- **End-to-end verification** at the design scale (cutover step 8) — see TECH_DEBT note; currently
  planned as functional only.

## Related open tasks (already filed)

In [TODO.md](TODO.md): `bulk_insert` annotation-row parity (from-reset listability); Rust-layer
symbol-naming review (`sync`/`reconcile` overload); embeddings write-lock audit; queue-priority
refactor; Virtuoso windowed LanceDB reads; pins cleanup on target removal.

In [TECH_DEBT.md](TECH_DEBT.md): reload-before-every-read cost; `all_entries`/`entries_by_ids`
unpaginated/unchunked; no ANN index; count-scan costs; `list_entry_topics` via `get_entry` reads
the vector.
