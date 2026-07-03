# Single-Writer Library Lock

**A library has at most one writing process at a time, enforced by an exclusive advisory file
lock. Everything else that opens the library reads.**

## The mechanism

- The lock is an exclusive advisory `flock` on `xaga.lock` at the library root
  (`acquire_lock`, `crates/xaga-core/src/library.rs`).
- The lock is tied to the open file handle, not the file's contents (`FileLock`,
  `crates/xaga-core/src/fs.rs`): the OS releases it automatically when the handle drops —
  including on crashes and hard kills. There are no stale lock files to clean up.

## Who takes it

- **The app:** `EmbeddingService::start` acquires the lock before touching LanceDB
  (`crates/xaga-core/src/embeddings/service.rs`). If another instance already holds it, start
  fails with `LockNotAcquired` and the caller runs without the service: embeddings remain
  readable, but this instance does not write (`src-tauri/src/lib.rs`, setup).
- **The CLI:** write commands acquire it via `require_write_lock`
  (`crates/xaga-cli/src/cmd/embeddings.rs`), which exits with a clear error when another
  process holds the lock.

## Scope and limits

- The lock is *advisory*: it constrains only participants that call `acquire_lock`. It
  coordinates xaga's own processes (app and CLI); it does not defend against third-party tools
  writing to the library or to LanceDB.
- Whether every write path in the workspace participates is an open audit question (recorded in
  the legacy TODO; that item's `lock.rs` / per-symbol citations predate the current code
  layout).

## Relation to the other rules

The lock is the process-level face of the same discipline as the two-producer rule
(`docs/principles/single_source_of_truth.md`): within the one writing process, LanceDB has
exactly two producers; across processes, the lock ensures there is only one writing process for
those producers to live in. Recovery's crash-healing (`crash_consistency.md`) assumes writes
were serialized through a single writer.
