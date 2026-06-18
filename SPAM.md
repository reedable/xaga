# Retire "pig-spam-pig" file watcher

## Objective

The current pipeline is linear and lossy: a command handler knows exactly what the
user did (mark read, bookmark, annotate, curate into a topic), but that intent is
serialized into a generic `.md` write and then _re-derived_ by a file watcher from a
bare "a file changed" event — pig ground into spam, then reconstructed back into an
approximate pig. The watcher fit when the app only read feeds and indexed entries, but
as user-driven state accumulated it can no longer tell a content change from a metadata
toggle, and the round-trip adds latency to the most user-facing actions. The objective
is to retire the watcher and move orchestration to the layer that still holds the
intent: the Tauri command writes the file and then enqueues a typed, correctly
prioritized message to the embedding service directly. Out-of-band changes (migrations,
external/Obsidian edits) are reconciled by recovery — the existing lost-job finder —
accepting eventual "sync at some point" rather than live sync.

## Key Architectural Decisions

1. **The file watcher will be retired.** The live filesystem watchers that convert
   `.md` events into embedding work are removed; filesystem events are no longer a
   message bus.
2. **`xaga-core` stays a pure file API.** Core reads and writes entity files and knows
   nothing about the queue or the embedding service. Orchestration — deciding what work
   to enqueue — lives entirely in the caller, never in core. The watcher model's one
   virtue, a pure core, is preserved even as orchestration moves up.
3. **The Tauri command layer becomes the orchestration layer.** Each command writes the
   entity via the pure `xaga-core` API, then enqueues a typed, correctly-prioritized
   message to the embedding service itself, carrying intent end-to-end instead of letting
   it be re-derived from a file event.
4. **xaga-cli becomes the data-migration and database-management tool.** It sheds
   interactive feed/entry CRUD; its remaining writes are bulk migrations (reconciled by
   recovery), alongside database observation and management.
5. **Interop with external systems (e.g. Obsidian) is a recovery concern.** Out-of-band
   edits are not reflected live; recovery — the existing lost-job finder — picks them up
   on its next run. Eventual sync, not live sync.
6. **Formalized queue priorities and a standardized message structure.** This is where
   much of the design and maintainability win comes from. Priority is assigned by the
   _nature/urgency of the work_, not by what kind of operation it is, in explicit tiers:
   system-level (shutdown/reset) → UI-level (live, user-driven edits) → regular →
   background/maintenance (recovery bulk, embedding backfill, compaction). Messages are
   standardized to carry the action and a reference to the entity; priority is implied by
   the queue the message lands on rather than re-derived downstream. (Replaces today's
   ad-hoc biased `select!` keyed by message source; the message taxonomy is defined below
   under _Ratified Taxonomy — Step 1_.)
7. **LanceDB state (the cache) has exactly two producers: the command layer and
   recovery.** LanceDB is a cache of state derived from the filesystem source of truth —
   vectors _and_ state such as unread/bookmark. In the running app only two producers
   feed it: the command layer (live, from user intent) and recovery (reconcile /
   lost-job finder). A third producer of cache writes is, by definition, drift — the
   clearest on-course check.

## High-Level Tasks (Rough Sequence)

1. Define the message taxonomy and priority tiers.
2. Refactor the embedding service to the formalized tiers and typed messages.
3. Wire the command layer to write-then-enqueue typed messages across all mutation surfaces.
4. Confirm recovery is the complete backstop for out-of-band changes.
5. Retire the file watcher.
6. Refocus xaga-cli (drop interactive CRUD; keep migration and database management).

## Ratified Taxonomy — Step 1

Settled. These are the decisions; do not re-open without a concrete reason.

### Priority tiers (queues), drained strictly highest-first: T0 > T1 > T2 > T3

| Tier | Name | Holds |
|----|----|----|
| **T0** | Control | shutdown, reset, start-recovery |
| **T1** | Interactive | cheap, must-reflect-now updates from a single user action — state toggles, content-row upsert (so the item appears), annotation sync, single-member membership delta, single delete. **No ONNX.** |
| **T2** | Regular | ONNX vector (re)compute for an _individual, live/user-originated_ item |
| **T3** | Maintenance | bulk/backfill embeds (recovery, large imports), batch deletes, compaction, index maintenance |

**Tier follows the work's _cost_, not its _trigger_.** A user-triggered job is only T1 if
it is cheap and must reflect now; an expensive job (large membership fan-out, bulk delete,
backfill) goes to T2/T3 even when a user started it, so it never blocks the interactive tier.

### Message structure

One `Job` enum (the **action**, orthogonal to priority) carrying an **entity reference**
that includes `kind` (entry/topic/feed/annotation), enqueued onto one of four typed
tier-channels. **The tier _is_ the priority.** The same `Job` may ride different tiers by
different producers (e.g. `EmbedVector` on T2 from a live save, T3 from recovery). No
per-tier enums — that would re-couple action to tier.

### Job vocabulary

- `SyncState { id, kind, <state values> }` — **carries the new state value(s)**
  (is_unread / is_bookmarked / is_pinned); consumer applies them and emits the UI change
  event. No file read, no embed (write-through). Recovery stays the file-authoritative
  reconciler for drift.
- `UpsertContent { id, kind }` — content created/edited; consumer reads the file, syncs the
  row, and (if checksum changed) enqueues an `EmbedVector` follow-up.
- `EmbedVector { id, kind }` — ONNX vector (re)compute, checksum-gated. T2 (live) or T3 (bulk).
- `SyncAnnotations { id }` — reconcile an entry's annotation rows (embedder-free).
- `AddMember { topic_id, member_id }` / `RemoveMember { topic_id, member_id }` — precise O(1)
  reverse-index delta the command already knows. Cheap → T1.
- `ReconcileMembership { topic_id }` — full diff + chunked bulk fan-out over a topic's whole
  member list. Expensive → T3 (recovery/drift) or T2 (one wholesale topic edit). Never T1.
- `Delete { id, kind }` / `DeleteBatch { ids }` — remove cached row(s).
- `Compact` / `EnsureIndexes` — maintenance (T3).
- Control: `Shutdown` / `Reset` / `StartRecovery` (T0).

### Producer → Job → tier

| Producer | Job | Tier |
|----|----|----|
| `mark_entry_read/unread`, `add/remove_bookmark`, `set_*_pinned` | `SyncState` | T1 |
| `save_article`, `create_article`, `save_topic`, `create_topic` | `UpsertContent` (→ `EmbedVector`) | T1 (→ T2) |
| `add_annotation`, `remove_annotation` | `SyncAnnotations` | T1 |
| `add_topic_item`, `remove_topic_item` (and `remove_annotation` topic cleanup) | `AddMember`/`RemoveMember` | T1 |
| `save_topic` wholesale item-list edit; recovery membership rebuild | `ReconcileMembership` | T2 (one topic) / T3 (bulk) |
| `delete_topic` | `Delete` | T1 |
| `refresh_feed`/`add_feed` new entries; `import_feeds`/`import_bookmarks` | `UpsertContent`/`SyncState` ×N | T3 |
| `set_feed_pinned` / `rename_feed` (kind=feed) | `SyncState` / `UpsertContent` | T1 / T2 |
| `remove_feed` (entries stay) | `Delete` (kind=feed) | T1 |
| Recovery (`dispatch_recovery`) | `DeleteBatch`, `UpsertContent`/`EmbedVector` ×N, `Compact` | T3 |
| App lifecycle | `Shutdown` / `Reset` / `StartRecovery` | T0 |

Feeds reuse the same jobs with `kind=feed` — no feed-specific jobs.

### Deferred to Task #2 (implementation, not taxonomy)

- Compaction-trigger consolidation (today's three-way queue/interval/drain paths).
- Membership delta-batching threshold (start per-delta; add a batch cutoff only if a burst
  proves it necessary).
- Concrete Rust wiring of the `Job` enum + tier channels.

### Out of scope / unchanged

- Outbound UI-notification channels (`unread_changed`, `bookmarked_changed`,
  `annotation_changed`, `priority_drained`, `background_drained`) are orthogonal to the
  priority tiers and stay as-is.
- Embedding stays checksum-gated end-to-end (no plan to change).
