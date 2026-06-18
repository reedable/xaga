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
   ad-hoc biased `select!` keyed by message source; the exact message taxonomy is still
   to be defined.)
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
