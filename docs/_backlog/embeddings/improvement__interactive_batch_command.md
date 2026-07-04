# Interactive-tier rapid updates: compaction fix (done) and batch command (pending)

## Symptom

Read state appeared "not saved": articles stayed unread in the UI after being read.
Alongside it, the embedding pipeline was frozen and the whole app degraded — list/read
operations that normally return in milliseconds were taking seconds to tens of seconds.

## What we found

The write path for read state was actually correct: `read_article` / `mark_entry_read`
rewrite the `.md` frontmatter and enqueue an Interactive `SyncState` job, `apply_state`
updates the LanceDB row and fires an `unread:changed` event, and the item list patches the
row in place. None of that was broken.

The real problem was a degradation spiral in the embedding service's work loop:

- The loop is a `biased` `tokio::select!` over four tiers, in order:
  Control → Interactive → Regular → Maintenance. Biased means each iteration polls
  Interactive before Regular/Maintenance, so a ready Interactive job always preempts them.
- `db.compact()` (which prunes accumulated LanceDB dataset versions/fragments) is only
  called from the **Regular** and **Maintenance** arms. The **Interactive** arm never
  compacted — it only set a `compacted_after_drain = false` flag for a Maintenance-arm
  "post-drain" compaction to honor later.
- Under a sustained stream of Interactive work, the loop never falls through to
  Regular/Maintenance (there is always a ready Interactive job when it re-polls), so
  compaction never runs. The "post-drain" compaction is unreachable for the same reason:
  it lives inside the Maintenance arm, which is only selected when Interactive is empty.

With compaction never running, Lance versions accumulated unbounded. Every commit reads an
ever-larger manifest, so commit latency grew without limit — and reads degraded too, since
each `EmbeddingsDb::open` pays for the bloated manifest.

### Evidence (from a live session)

- `embedded` count frozen at a fixed value for ~40 minutes — zero embeds completing
  (Regular/Maintenance never reached), while `pending` climbed as new rows were created.
- Commit interval (Lance "Auto cleanup triggered" log lines, one per commit) grew from ~8s
  to 4–10 minutes over ~37 minutes: `8, 10, 14, 15, 18, 17, 20, 21, 23, 26, 34, 38, 52, 82,
  125, 239, 329, 628, 254, 289` seconds.
- Interactive backlog drain rate fell from ~14/min to ~2/min as bloat compounded.
- Lance auto-cleanup reported `old_versions: 0` on every commit — nothing was being pruned.
- Frontend logs showed `list_feeds` taking 21.7s and `list_all_items` / `list_entries`
  aborting after ~2s — the read-side manifestation of the same bloat.

### Rapid-update paths that feed the Interactive tier

All of these enqueue per-item work onto Interactive and share the same failure mode:

- `refresh_all_feeds` — one `UpsertEntry` per new entry (~1 commit each).
- OPML `import_feeds` — one `UpsertEntry` per new entry.
- `add_feed` — one `UpsertEntry` per new entry.
- "Mark all read" — the frontend fires one `mark_entry_read` invocation per item
  (`Promise.all(ids.map(markEntryRead))`); each enqueues a `SyncState`, and `apply_state`'s
  `upsert` is delete-then-add (**~2 commits per item**) plus an `.md` rewrite and an IPC call.
- Bulk bookmark / annotation changes follow the same per-item `SyncState` shape.

`refresh_all` and "mark all read" are the same problem through the same machinery; "mark all
read" is somewhat heavier per item (~2 commits vs ~1).

Note: putting these on Interactive is intentional and correct — the user is waiting to *see*
the entries / state change, and visibility (the row write) is already decoupled from
embedding (the vector, deferred to Regular via `embed_follow`). The defect was never the
tier; it was that per-item commits on that tier could accumulate unbounded because
compaction was unreachable from it.

## Two options considered

### Option 1 — batch the work into fewer commits (PENDING)

Collapse a rapid-update burst into a single (or chunked) commit instead of N per-item
commits:

- Bulk **ingest**: a batched Interactive job that does one `db.add_batch` for all newly
  created entries (they are genuinely new, so a pure add is safe), then fans out N
  `EmbedEntry` follow-ups to Regular. `db.add_batch` already exists; a new `Job` variant
  (e.g. `UpsertEntries { articles }`) does not.
- Bulk **state update** (mark-all-read etc.): a batched command that does one file-tag pass
  and a single batched row update setting `is_unread=false` on N rows in one commit. This is
  a different shape from the ingest batch, which is why designing them together — after
  enumerating every rapid path — is preferable to bolting one on at a time.

Pros:
- Attacks the root cause (commit count): 1 commit instead of N, so the version-bloat storm
  does not happen in the first place.
- Interactive drains in one step, so Regular embeds and the drain-compaction resume promptly.
- Entries/state still appear immediately (a single fast commit).

Cons:
- New `Job` variant(s) + wiring; more change than Option 2.
- The batch path must still fire the per-entry `unread`/list events so the UI updates.
- Does not, by itself, bound bloat from *other* sustained Interactive sources — periodic
  compaction is still wanted regardless (see below).

### Option 2 — periodic compaction reachable from the Interactive arm (IMPLEMENTED)

Make the Interactive arm compact on its own cadence, so bloat is bounded even when the loop
never yields to Regular/Maintenance.

Implementation (`crates/xaga-core/src/embeddings/service.rs`, `run_loop`):
- Added `const INTERACTIVE_COMPACT_INTERVAL: usize = 100;`, deliberately separate from
  `BACKGROUND_COMPACT_INTERVAL` (also 100 today). The names are kept distinct because this
  cadence is paid on the user-facing path and may need independent tuning; "background" is
  semantically misleading for the interactive path. They may be converged later — with a
  better name — if the value proves solid after real use.
- Added a dedicated `since_interactive_compact` counter (not the shared `since_compact`,
  which belongs to Regular/Maintenance).
- After each Interactive job: increment the counter; every 100 jobs run `db.compact()`,
  reset the counter, set `compacted_after_drain = true` (so the Maintenance drain-compaction
  does not redundantly re-fire), and log `interactive tier: 100-item interval — compacting`.

Why this is worth keeping regardless of Option 1: `add_batch` only collapses commits from
bulk *ingestion*. Every other Interactive write (mark-all-read `SyncState`, bookmark,
annotations, `set_feed_pinned`, `save_article`) still commits per item, and Lance accrues
versions on every commit no matter how writes are batched. Option 1 reduces the *rate* of
bloat from one source; Option 2 *bounds* bloat from all sources. They solve different halves.

Known gap: Option 2 bounds bloat but does not reduce commit count — a large refresh
/ mark-all-read still issues N per-item commits. They now stay *cheap* (versions pruned every
100 jobs) instead of decaying. And compaction now runs on the user-facing path, so every
100th interactive action takes a visible pause; `INTERACTIVE_COMPACT_INTERVAL` is the knob for
that cost-vs-bloat trade.

## Verification of Option 2

After relaunching (which clears the in-memory Interactive backlog and lets recovery re-embed
on the Maintenance tier) and waiting for the embedding backlog to reach 0 pending:

1. Picked a feed with 400+ unread items and did "Mark all read".
2. Switched to the "All items" list — it loaded fast (previously this read was seconds-slow).
3. Opened the dev console, selected one still-unread item; observed the burst of
   `read`-state-change events streaming from the Rust layer (the 400+ items being applied).
4. The freshly selected item was marked read in the UI within ~5–10s, confirming both the
   `.md` write and the LanceDB update completed.

This is behavioral confirmation (fast list load, mark-all-read completing, single-item
round-trip ~5–10s) — it did not decay the way the pre-fix session did. Assessed as
"pretty good."

Measured confirmation followed: a later burst captured the new path firing directly in the
logs —

    interactive tier: 100-item interval — compacting
    EmbeddingsDb::compact: compacting fragments
    Compaction task ...: Begin compacting 10161 rows across 101 fragments

The interval fired on schedule (~100 interactive jobs), and the compaction merged 101
fragments — i.e. the per-item commits had produced roughly one fragment each and were being
collapsed rather than accumulating unbounded. This is the bloat bound working as designed.

## Related design note: backend-authoritative UI (intentional)

The UI currently waits for the backend signal before updating presentation state, on purpose:
the goal is to surface backend performance/correctness issues (like the one above) rather than
mask them. Optimistic presentation-state updates are planned, but only after the backend
vertical stack is well-performing and problem-free. This is why the ~5–10s round-trip above is
visible in the UI at all, and why keeping backend commit/read latency low matters now.

## When to revisit Option 1

If, in real use over the next few days, N cheap per-item commits from a big refresh or
mark-all-read is still too heavy (commit-interval logs no longer flat, or the every-100th
compaction pause is too noticeable), enumerate all rapid-update paths and design the batch
commands (ingest + state-update) together.

## Observation (2026-07-02)

While testing feed fetch (a single click of refresh), the burst-queue behavior was
observed again. It is not as bad as before Option 2: the pre-fix state paralyzed all
db reads/writes, and that no longer happens. The interactive batch insert (Option 1)
remains beneficial. Moved from docs/_backlog to docs/_sprint on this basis.
