# Single-Writer Library Lock — the Two Runtime Modes

**A library has at most one writing process at a time, enforced by an exclusive advisory
file lock whose scope is the WHOLE library: entity `.md` files and LanceDB alike. Every
xaga process runs in one of two modes, decided at startup by that lock: read-write (holds
the lock) or read-only (does not). Non-holders read but do not write — now true at library
scope, not just LanceDB scope.**

## The mechanism

- The lock is an exclusive advisory `flock` on `xaga.lock` at the library root
  (`acquire_lock`, `crates/xaga-core/src/library.rs`).
- The lock is tied to the open file handle, not the file's contents (`FileLock`,
  `crates/xaga-core/src/fs.rs`): the OS releases it automatically when the handle drops —
  including on crashes and hard kills. There are no stale lock files to clean up.
- **Mode is derived, never stored.** The app holds the acquired lock (or `None`) in the
  `LibraryHandle` — path and lock guarded as one unit — and
  `is_library_read_only()` reads it (`src-tauri/src/lib.rs`). There is no mode
  enum/bool anywhere that could drift from lock ownership; the lock IS the mode
  (single source of truth). The same name carries the concept at every layer: the Rust
  accessor, the Tauri command, the `library:mode-changed` event payload, and the
  frontend hook (`src/hooks/useLibraryReadOnly.tsx`).

## Who takes it, and when

- **The app:** at setup, immediately after resolving the library path and before any
  command can run. Winning = read-write; losing = read-only for the process's lifetime.
  Mode never changes in-process: a library switch (`set_library`) persists the new path
  and restarts the process, and the restart path releases the lock before spawning the
  successor (`release_library_lock`, `src-tauri/src/lib.rs`) so the new process can win
  it.
- **The `EmbeddingService`:** no longer acquires the lock; it receives it
  (`EmbeddingService::start(dir, lock)`) and holds its own `Arc` of it for its lifetime,
  preserving its invariant of never running unlocked. The service (LanceDB writes,
  recovery, embedding queue) exists only in a read-write instance.
- **The CLI:** write commands acquire it via `require_write_lock`
  (`crates/xaga-cli/src/cmd/embeddings.rs`), failing fast when another process holds it.
  The writer is whoever wins the flock — app or CLI, first-come (owner decision).

## What read-only means

- Every library-writing command is refused centrally at the IPC boundary with a typed
  `E_READ_ONLY` error — the `WRITE_COMMANDS` registry next to `is_invoke_allowed`
  (`src-tauri/src/commands/mod.rs`), the same single-choke-point pattern as the window
  allowlist. New write commands must be added to that registry (until the Phase C writer
  actor / Phase A capability token make bypass structurally impossible).
- Incidental writes hiding in read flows no-op silently: `read_article`'s mark-read is
  skipped, so reading stays seamless (owner decision) — the entry simply stays unread.
- The frontend learns the mode (`is_library_read_only` command + `library:mode-changed`
  event), shows a read-only badge, and disables write affordances.
- Reads are untouched: `.md` reads, LanceDB reads (`embeddings_read` when available),
  search, lists.

## Known limits (accepted, Phase B)

- A read-only instance stays read-only until relaunch or library switch — no lock-retry
  loop watches for the writer's exit.
- Check-to-use window: the gate answers "read-only at this instant"; an in-flight write
  can in principle overlap the shutdown path releasing the lock. Closed structurally by
  the Phase C writer actor (it owns the lock; no actor, no writes).
- The lock binds only xaga processes. External tools (Obsidian) are not participants by
  design: external edits are architecturally equivalent to a crash — reconciled from
  files by recovery at startup, not coordinated live (owner decision; see the concurrent
  library writes spike).

## Relation to the other rules

The lock is the process-level face of the same discipline as the two-producer rule
(`docs/principles/single_source_of_truth.md`): within the one writing process, LanceDB has
exactly two producers; across processes, the lock ensures there is only one writing process
for those producers to live in. Recovery's crash-healing (`crash_consistency.md`) assumes
writes were serialized through a single writer — the mode gate is what makes that
assumption hold for `.md` files. All library `.md` writes are temp+rename atomic
(`write_atomic_async`, `crates/xaga-core/src/fs.rs`), so a reader in any process never
observes a torn file and a crash mid-write leaves the old version intact.
