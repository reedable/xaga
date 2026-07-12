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
- **Mode is derived, never stored.** In read-write mode the app holds a `LibraryWriter`
  (which owns the flock) in the `LibraryHandle` — path and writer guarded as one unit;
  in read-only mode the writer is `None`. `is_library_read_only()` reads
  `writer.is_none()` (`src-tauri/src/lib.rs`). There is no mode enum/bool anywhere that
  could drift; the writer's presence IS the mode (single source of truth). The same name
  carries the concept at every layer: the Rust accessor, the Tauri command, the
  `library:mode-changed` event payload, and the frontend hook
  (`src/hooks/useLibraryReadOnly.tsx`).

## Who takes it, and when

- **The app:** at setup, immediately after resolving the library path and before any
  command can run. Winning = read-write; losing = read-only for the process's lifetime.
  Mode never changes in-process: a library switch (`set_library`) persists the new path
  and restarts the process, and the restart path drains and releases the writer before
  spawning the successor (`drain_and_release_writer`, `src-tauri/src/lib.rs`) so the new
  process can win the lock.
- **The `LibraryWriter`:** owns the flock (`LibraryWriter::start(dir, lock)`,
  `crates/xaga-core/src/library_writer/mod.rs`) and is the single component through which
  every `.md` write flows. Dropping it releases the lock; the restart/shutdown paths drain
  it first (`drain_and_release_writer`, `src-tauri/src/lib.rs`).
- **The `EmbeddingService`:** no longer acquires the lock; it receives a shared `Arc` of
  the writer's lock (`EmbeddingService::start(dir, lock)`, from `writer.lock()`) and holds
  it for its lifetime, preserving its invariant of never running unlocked. The service
  (LanceDB writes, recovery, embedding queue) exists only in a read-write instance. The
  flock releases only once BOTH the writer and the service have dropped their `Arc`.
- **The CLI:** write commands acquire it via `require_write_lock`
  (`crates/xaga-cli/src/cmd/embeddings.rs`), failing fast when another process holds it.
  The writer is whoever wins the flock — app or CLI, first-come (owner decision).

## What read-only means

- Every library-writing command is refused centrally at the IPC boundary with a typed
  `E_READ_ONLY` error — the `WRITE_COMMANDS` registry next to `is_invoke_allowed`
  (`src-tauri/src/commands/mod.rs`), the same single-choke-point pattern as the window
  allowlist. New write commands must be added to that registry (it drives the early,
  uniform read-only rejection and the frontend affordance list; the ultimate enforcement is
  that a read-only instance has no writer, so `require_writer` errors regardless).
- Incidental writes hiding in read flows no-op silently: `read_article`'s mark-read is
  skipped, so reading stays seamless (owner decision) — the entry simply stays unread.
- The frontend learns the mode (`is_library_read_only` command + `library:mode-changed`
  event), shows a read-only badge, and disables write affordances.
- Reads are untouched: `.md` reads, LanceDB reads (`embeddings_read` when available),
  search, lists.

## In-process write serialization (the writer, axis 2)

The lock makes at most one process a writer; the `LibraryWriter` makes that process's own
concurrent writes safe. Every `.md` write is submitted to the writer as a file-scoped
request and dispatched on a **per-file lane**: same-file writes run strictly serially (the
whole read-modify-write inside one lane, so two operations on one file can never
interleave — the annotation-clobber class), while different files run in parallel. This is
what closes the concurrent-write data loss the annotation era introduced: annotations made
entity files the sole store of user-authored content, and before per-file serialization two
async annotation writes on one article could each read-modify-write and destroy each
other's change (unhealable, because recovery reconciles FROM files — a clobbered file is
truth).

- **Tiers.** Requests carry `Interactive` or `Regular` (`library_writer::Tier`). User
  actions (annotations, tag toggles, topic edits, bookmark import) are Interactive; feed
  writes are Regular. The dispatcher services Interactive ahead of Regular so a bulk feed
  refresh cannot starve a user write. The tier also carries a **shutdown contract**:
  Interactive = user-authored, exists only in the queue until written, MUST drain at
  shutdown; Regular = re-derivable from source, droppable. **Admission rule: only enqueue
  re-derivable content as Regular** — the shutdown drop is keyed to the tier.
- **Shutdown** (`drain_and_release_writer`): the Interactive backlog drains to disk, the
  Regular backlog is dropped (refused with `ShuttingDown`), in-flight writes complete; only
  then is the writer dropped and the flock released. A `shutting_down` flag makes late
  submits fast-fail rather than strand in a channel the dispatcher will no longer drain.
- **Boundary with `EmbeddingService`:** two serialized subsystems, split by what they own —
  the writer owns every `.md` write; the service owns every LanceDB write. A command
  submits the file write, then enqueues the LanceDB sync job. Files stay the source of
  truth; LanceDB is the derived mirror written downstream.
- **Feeds:** the network fetch stays OUTSIDE the lane. Each new entry `.md` is committed on
  ITS OWN entry lane (create-if-absent, atomic against another feed carrying the same entry
  or a user write to it — a single entry can appear in multiple feeds), and only once every
  entry has landed is the `.atom` written on the feed lane; awaiting the entry writes before
  the atom preserves the entries-before-atom ordering invariant. A refresh's `.atom` write is
  existence-gated (`require_exists`): if the user removed the feed during the fetch, the
  write is skipped rather than resurrecting it — the check and write are atomic on the feed
  lane, serialized against `remove_feed`. `add_feed` writes unconditionally (it creates the
  feed).
- **Classification rule for a new write feature:** does it need the writer? Then it is a
  write. This is enforced at compile time by the **`WriteToken`** capability: every
  `pub(crate)` persistence write takes a `&WriteToken` / `WriteToken`, and the token's
  constructor is private to `library_writer` (minted once at `start`, having taken the
  lock). So a persistence write can be called ONLY from within the writer — a future
  in-crate caller that tried to write directly, bypassing the per-file lanes, would not
  compile (it cannot obtain a token). `WriteToken::for_test()` (test-only) lets
  persistence unit tests call writes directly.

## Known limits (accepted)

- A read-only instance stays read-only until relaunch or library switch — no lock-retry
  loop watches for the writer's exit.
- Check-to-use window (now closed): the read-only gate answers "at this instant," but a
  write command that passed it cannot execute against a released lock — it gets the writer
  handle via `require_writer`, and if shutdown has begun the writer's `shutting_down` flag
  fast-fails the submit rather than writing.
- The lock binds only xaga processes. External tools (Obsidian) are not participants by
  design: external edits are architecturally equivalent to a crash — reconciled from
  files by recovery at startup, not coordinated live (owner decision). Live last-wins
  inside the ms-scale read-modify-write window is accepted; no external-editor detection is
  built.

## Relation to the other rules

The lock is the process-level face of the same discipline as the two-producer rule
(`docs/principles/single_source_of_truth.md`): within the one writing process, LanceDB has
exactly two producers; across processes, the lock ensures there is only one writing process
for those producers to live in. Recovery's crash-healing (`crash_consistency.md`) assumes
writes were serialized through a single writer — the mode gate is what makes that
assumption hold for `.md` files. All library `.md` writes are temp+rename atomic
(`write_atomic_async`, `crates/xaga-core/src/fs.rs`), so a reader in any process never
observes a torn file and a crash mid-write leaves the old version intact.
