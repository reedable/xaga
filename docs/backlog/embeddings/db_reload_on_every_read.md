# DB reload on every read (pending)

## Current state

LanceDB table handles are version-pinned: a handle opened at table version N keeps serving
version N while newer versions are committed. The app holds two connections — the embedding
service owns the writer; `AppState.embeddings_read` (`src-tauri/src/lib.rs`) is a separate read
handle opened once at startup. Without intervention, every read command would serve a snapshot
frozen at launch.

The current strategy: every read command calls `db.reload()` (LanceDB `checkout_latest`) before
querying — `get_unread_ids`/`get_bookmarked_ids` (`src-tauri/src/lib.rs`), `list_entries`
(`src-tauri/src/commands/feeds.rs`), and the other LanceDB-backed commands. Reads are always
current; there is no invalidation machinery.

## Cost

`reload` re-reads the table manifest (version + fragment list) on every read, even when nothing
has changed since the last read. The per-read cost at the design scale (see
`docs/architecture/design_scale.md`) is not measured.

## Observed symptom

Marking an entry read propagates slowly to the UI, and the sidebar unread count more slowly
still. The chain: `.md` write → queue → LanceDB commit (async, via the service queue) → count
re-query (`reload` + full unread-set materialization). Whether the reload step is the dominant
cost in that chain has not been measured.

## Option: reload driven from the service.rs loop

The service's `run_loop` (`crates/xaga-core/src/embeddings/service.rs`) is the sole writer, so
it knows exactly when a commit has occurred. Instead of every command reloading
unconditionally, the loop would signal after committing (or perform the reload on the shared
read handle), and commands would read without a per-read reload.

- Reads would stop paying the manifest re-read when nothing changed.
- Reads between a `.md` write and the service's commit would serve the pre-commit state — the
  same window that exists today (the reload cannot see an uncommitted write); whether any
  consumer depends on the difference has not been investigated.
- The signal path (channel, event, or direct reload of the shared handle) and its interaction
  with the existing UI-notification channels would need design.

## Option: reload tied to the existing `embedding:changed` event

The event already exists: the service exposes `priority_drained` / `background_drained`
channels, and two forwarder tasks in the setup closure emit `"embedding:changed"` to the
frontend whenever either queue drains (`src-tauri/src/lib.rs`; the frontend subscribes in
`src/platform/TauriPlatform.ts`). Under this option, that same signal — at the Rust layer,
the drained channels themselves, or a subscription alongside the forwarders — would mark the
read handle stale, and commands would call `reload()` only when the flag is set instead of
unconditionally.

- Reads between drain events would skip the manifest re-read.
- The event fires on queue *drain*, not per commit: a long-running drain phase commits rows
  before the drained signal fires, so reads during it would serve the pre-drain version —
  today's per-read reload would see those intermediate commits. Whether any consumer depends
  on the difference has not been investigated.
- The staleness flag shared between the event task and the read commands would need a home
  and a synchronization choice.

Whether to act, which option, and the measurement to base it on, are open.
