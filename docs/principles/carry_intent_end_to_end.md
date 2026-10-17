# Carry Intent End-to-End

**The layer that knows what the user did carries that intent — typed and prioritized — to every consumer that needs it. Intent is never serialized into a generic event and re-derived approximately downstream.**

## The failure mode this names

A command handler knows exactly what the user did: mark read, bookmark, annotate, curate into a topic. Flattening that into a generic `.md` write and re-deriving it from a bare "a file changed" event grinds the pig into spam and reconstructs an approximate pig: the downstream consumer cannot tell a content change from a metadata toggle, and the round-trip adds latency to the most user-facing actions.

## Application in this app

- **Filesystem events are not a message bus.** The Tauri command writes the entity via the pure core API, then itself enqueues a typed, correctly-prioritized job to the embedding service — the intent travels end-to-end instead of being re-derived from a file event (`src-tauri/src/commands/`, the `state.enqueue(Tier::…, Job::…)` sites).
- **Jobs carry what the producer knows.** `SyncState` carries the new state values in a `StatePatch` (no file read downstream); `AddMember`/`RemoveMember` carry the O(1) membership delta the command already knows, instead of forcing a full reconcile (the `Job` enum, `crates/xaga-core/src/embeddings/`).
- **Where no intent exists, nothing is carried.** Out-of-band changes — migrations, external edits (e.g. Obsidian) — have no command layer that knows what happened; they are a recovery concern, reconciled on recovery's next run. Eventual sync, not live sync.
