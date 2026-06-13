# WIP — LanceDB as the read source (Phase C cutover)

Resume-point for the in-progress roadmap. Full plan:
`~/.claude/plans/lancedb-embeddings-column-is-refactored-bentley.md`. Granular debt is in
[TECH_DEBT.md](TECH_DEBT.md); discrete tasks are in [TODO.md](TODO.md). This file tracks the
**cutover state** and the **one open blocker**.

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
| `list_entry_topics` ([topics.rs](src-tauri/src/commands/topics.rs)) | ⛔ **BLOCKED** — see below | repoint deferred; still reads topic files |
| sidebar feed/topic counts | ⬜ not started | cutover step 4; see TECH_DEBT counts items |

Supporting work done:
- **AppState read handle** ([lib.rs](src-tauri/src/lib.rs)) — shared LanceDB read connection opened
  after the service; reads `reload()` before querying (handles are version-pinned). Reload-before-read
  strategy chosen; per-read cost logged in [TECH_DEBT.md](TECH_DEBT.md).
- **Annotation two-phase fix** ([pipeline.rs](crates/xaga-core/src/embeddings/pipeline.rs)) —
  `sync_annotations` now reconciles annotation **rows** in the embedder-free fast phase, so a new
  highlight is listable immediately (was gated behind the embed backlog). 15/15 core tests pass.
  Pending user relaunch verify (highlight → appears in Saved; color change; un-highlight).

## ⛔ Blocker — `list_entry_topics` repoint (cutover step 5)

`list_entry_topics(entry_id)` returns the **topic ids** a given entry belongs to. Old impl
([topics.rs:275](crates/xaga-core/src/topics.rs#L275)) reads `topic.items` from the topic `.md`
files. The intended repoint reads the entry's `topics` column from LanceDB.

**Why it's blocked:** the SoT migration is only half-done.
- `migrate-sot` moved membership into each entry's `topics:` frontmatter **once**, and the `topics`
  column mirrors that.
- But the ongoing membership write path — `add_topic_item` / `remove_topic_item`
  ([topics.rs:239](crates/xaga-core/src/topics.rs#L239), [:257](crates/xaga-core/src/topics.rs#L257))
  — still writes **only the topic file's `items` list** (via `save_topic`). It never updates the
  member entry's frontmatter `topics:`.

**Consequences:**
- The entry `topics` column is correct only *as of the migration*. Membership changed since (or in
  future) lives in `topic.items`, not the entry frontmatter → the column **drifts stale**.
- A recovery rebuilds the `topics` column from the (stale) entry frontmatter, **dropping**
  post-migration membership changes. So `topic.items` is the de-facto SoT — the opposite of the
  roadmap's intent (entry `.md` should own membership; LanceDB only mirrors).

**To unblock (the write-path SoT conversion — its own focused task):**
1. Make `add_topic_item` / `remove_topic_item` also write the member entry's frontmatter `topics:`
   (add/remove the topic id), so the entry `.md` becomes the real membership SoT.
2. Trigger a re-sync of that entry (file watcher → `sync_row` → `topics` column updates).
3. Decide what `topic.items` is for afterward (drop it as a membership store, or keep it as a
   denormalized cache rebuilt from entry frontmatter — a SoT decision).
4. One-time reconciliation for any drift between `topic.items` and entry frontmatter that accrued
   since the migration (a `migrate-sot`-style pass, or fold into the conversion).
5. **Then** repoint `list_entry_topics` to the `topics` column (via `get_entry` for now — pulls the
   full vector; projected read is a separate TECH_DEBT item).

Until then: `list_entry_topics` stays on the topic-file path (authoritative). Do **not** repoint it.

## Remaining cutover steps (after the blocker)

- Repoint sidebar feed/topic **counts** to LanceDB (cutover step 4) — see the count-scan items in
  [TECH_DEBT.md](TECH_DEBT.md).
- **Retire `EntryStateCache`** (cutover step 6) — only after every state read is repointed and
  verified; `get_unread`/`get_bookmarked` ([lib.rs](src-tauri/src/lib.rs)) and the `annotate_*`
  helpers ([filter.rs](crates/xaga-core/src/filter.rs)) still have other callers (counts, other
  feeds.rs commands), so the cache can't go until those move too.
- **Retire the feed-search hack** (cutover step 7).
- **End-to-end verification** at the design scale (cutover step 8) — see TECH_DEBT note; currently
  planned as functional only.

## Related open tasks (already filed)

In [TODO.md](TODO.md): `bulk_insert` annotation-row parity (from-reset listability); Rust-layer
symbol-naming review (`sync`/`reconcile` overload); embeddings write-lock audit; queue-priority
refactor; Virtuoso windowed LanceDB reads; pins cleanup on target removal.

In [TECH_DEBT.md](TECH_DEBT.md): reload-before-every-read cost; `all_entries`/`entries_by_ids`
unpaginated/unchunked; no ANN index; count-scan costs; `list_entry_topics` via `get_entry` reads
the vector.
