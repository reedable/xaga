# Read-only instance never becomes the writer without a relaunch

A read-only instance (one that lost the library flock at
startup) stays read-only for its whole process lifetime: no lock-retry loop
watches for the writer's exit. When the writing instance quits, the read-only
instance does not pick up the lock — the user must relaunch it or switch
libraries to regain write access.

## Current behavior

- Mode is decided once, at startup, by winning or losing the flock
  (`acquire_lock`, `crates/xaga-core/src/library.rs`; setup in
  `src-tauri/src/lib.rs`). Mode never changes in-process; a library switch
  restarts the process (`docs/architecture/single_writer_library_lock.md`).
- Mode is derived from writer presence (`writer.is_none()`), never stored —
  any fix must preserve this (no mode flag that could drift).
- The frontend follows the mode via the `is_library_read_only` command and the
  `library:mode-changed` event; the event plumbing already exists
  (`src-tauri/src/events.rs`).

## Work

When the writer process exits, a read-only instance should be able to become
the writer without a manual relaunch.

## Design questions

- Acquisition mechanism: retry loop / OS-level lock wait / triggered attempt
  (e.g. on window focus) — each polls or blocks differently.
- Becoming the writer mid-process means constructing what read-write startup
  builds: `LibraryWriter`, `EmbeddingService`, recovery pass. Whether this is
  an in-process transition or an automatic relaunch (reusing the existing
  library-switch restart path) is undecided.
- Stale-view interaction: a read-only instance shows data as of its startup;
  on becoming the writer it must not write over state it has not read back.
  Related: `docs-transient/backlog/ui/improvement__read_only_mode_ux.md`
  (stale-view surfacing) — separate item, likely the same reload mechanism.
