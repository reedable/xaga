# Lockless instance: DB reads missing, set_library dead-ends (pending)

Two defects in the no-library-lock instance path. On the first, the owner
confirmed the implementation missed the intent ("the library lock was only
ever supposed to protect concurrent write to the database"). The second is
recorded on investigation, not yet ruled.

## 1. `embeddings_read` never set

The `LockNotAcquired` arm of service startup (`src-tauri/src/lib.rs`) only
logs "embeddings are read-only" — `embeddings_read` stays `None` for the
session, so the instance has no embeddings reads at all: `is_cache_ready`
stays false, unread/bookmarked sets come back empty, embeddings-backed
commands degrade. This contradicts the log line, the setup comment, and
`docs/architecture/single_writer_library_lock.md` ("non-holders read but
do not write").

Fix shape: open a standalone read-only `EmbeddingsDb` in that arm and
assign it (the CLI's read commands already open against a library whose
lock another process holds). Verify during implementation: the standalone
handle observes the other process's committed writes via the existing
reload-per-read pattern.

## 2. `set_library` does nothing without a service

`set_library` (`src-tauri/src/commands/library.rs`) persists the new
library path, returns success to the UI — and its restart lives inside
`if let Some(svc)`. In a lockless instance (`None`), no restart and no
reload happen: the instance keeps running against the old library while
settings.json and the UI claim the switch succeeded. The owner's read:
"sounds like a pure bug" pending further investigation; the investigation
found no historical rationale in git history. Not yet ruled.

Fix shape: the `None` arm calls `restart_process()` directly — there is no
service to drain. (Same dev-mode caveat as the existing restart path:
`restart_process` falls through to a clean exit under `tauri dev`.)
