# Single-Writer Library Lock — the Two Runtime Modes

**A library has at most one writing process at a time, enforced by an exclusive advisory file lock whose scope is the WHOLE library: entity `.md` files and LanceDB alike. The app selects one of two modes from the library version and write lock: read-write (holds the lock) or read-only (does not). Non-holders read but do not write.**

## The mechanism

- The lock is an exclusive advisory `flock` on `xaga.lock` at the library root (`acquire_lock`, `crates/xaga-core/src/library.rs`).
- The lock is tied to the open file handle, not the file's contents (`FileLock`, `crates/xaga-core/src/fs.rs`): the OS releases it automatically when the handle drops — including on crashes and hard kills. There are no stale lock files to clean up.
- **Mode is derived, never stored.** A matching completed library version is a
  prerequisite for attempting the write lock at app startup. Missing, mismatched
  or incomplete metadata selects read-only mode; unreadable metadata selects
  read-only mode and reports its error. In read-write mode the app holds a `LibraryWriter` (which owns the flock) in the `LibraryHandle` — path and writer guarded as one unit; in read-only mode the writer is `None`. `is_library_read_only()` reads `writer.is_none()` (`src-tauri/src/lib.rs`). There is no mode enum/bool anywhere that could drift; the writer's presence IS the mode (single source of truth). The same name carries the concept at every layer: the Rust accessor, the Tauri command, the `library:mode-changed` event payload, and the frontend hook (`src/hooks/useLibraryReadOnly.tsx`).

## Who takes it, and when

- **The app:** at setup, immediately after resolving the library path and before any command can run. A completed matching library version permits taking the write lock; winning = read-write, otherwise read-only. A library switch (`set_library`) persists the new path and restarts the process, and the restart path drains and releases the writer before spawning the successor (`drain_and_release_writer`, `src-tauri/src/lib.rs`) so the new process can win the lock.
- **The `LibraryWriter`:** owns the flock (`LibraryWriter::start(dir, lock)`, `crates/xaga-core/src/library_writer/mod.rs`) and is the single component through which every `.md` write flows. Dropping it releases the lock; the restart/shutdown paths drain it first (`drain_and_release_writer`, `src-tauri/src/lib.rs`).
- **The `EmbeddingService`:** receives a shared `Arc` of the writer's lock (`EmbeddingService::start(dir, lock)`, from `writer.lock()`) and holds it for its lifetime, preserving its invariant of never running unlocked. The service (LanceDB writes, recovery, embedding queue) exists only in a read-write instance. The flock releases only once BOTH the writer and the service have dropped their `Arc`.
- **The CLI:** write commands acquire it via `require_write_lock` (`crates/xaga-cli/src/cmd/embeddings.rs`), failing fast when another process holds it. The writer is whoever wins the flock — app or CLI, first-come.

## What read-only means

- Every library-writing command is refused centrally at the IPC boundary with a typed `E_READ_ONLY` error — the `WRITE_COMMANDS` registry next to `is_invoke_allowed` (`src-tauri/src/commands/mod.rs`), the same single-choke-point pattern as the window allowlist. New write commands must be added to that registry (it drives the early, uniform read-only rejection and the frontend affordance list; the ultimate enforcement is that a read-only instance has no writer, so `require_writer` errors regardless).
- Incidental writes hiding in read flows no-op silently: `read_article`'s mark-read is skipped, so reading stays seamless — the entry simply stays unread.
- The frontend learns the mode (`is_library_read_only` command + `library:mode-changed` event), shows a read-only badge, and disables write affordances.
- For a read-only library, reads remain available: `.md` reads, search, and lists. LanceDB readers use
  `EmbeddingsDb::open_readonly`, which opens an existing table without creating or migrating it.
  LanceDB writers use `EmbeddingsDb::open_writer` only after acquiring the library lock; the
  writer retains that lock across asynchronous mutation work.

## In-process write serialization (the writer, axis 2)

The lock makes at most one process a writer; the `LibraryWriter` makes that process's own concurrent writes safe. Every `.md` write is submitted to the writer as a file-scoped request and dispatched on a **per-file lane**: same-file writes run strictly serially (the whole read-modify-write inside one lane, so two operations on one file can never interleave — the annotation-clobber class), while different files run in parallel. This is what closes the concurrent-write data loss the annotation era introduced: annotations made entity files the sole store of user-authored content, and before per-file serialization two async annotation writes on one article could each read-modify-write and destroy each other's change (unhealable, because recovery reconciles FROM files — a clobbered file is truth).

- **Tiers.** Requests carry `Interactive` or `Regular` (`library_writer::Tier`). User actions (annotations, tag toggles, topic edits, bookmark import) are Interactive; feed writes are Regular. The dispatcher services Interactive ahead of Regular so a bulk feed refresh cannot starve a user write. The tier also carries a **shutdown contract**: Interactive = user-authored, exists only in the queue until written, MUST drain at shutdown; Regular = re-derivable from source, droppable. **Admission rule: only enqueue re-derivable content as Regular** — the shutdown drop is keyed to the tier.
- **Shutdown** (`drain_and_release_writer`): the Interactive backlog drains to disk, the Regular backlog is dropped (refused with `ShuttingDown`), in-flight writes complete; only then is the writer dropped and the flock released. A `shutting_down` flag makes late submits fast-fail rather than strand in a channel the dispatcher will no longer drain.
- **Boundary with `EmbeddingService`:** two serialized subsystems, split by what they own — the writer owns every `.md` write; the service owns every LanceDB write. A command submits the file write, then enqueues the LanceDB sync job. Files stay the source of truth; LanceDB is the derived mirror written downstream.
- **Feeds:** the network fetch stays OUTSIDE the lane. Each new entry `.md` is committed on ITS OWN entry lane (create-if-absent, atomic against another feed carrying the same entry or a user write to it — a single entry can appear in multiple feeds), and only once every entry has landed is the `.atom` written on the feed lane; awaiting the entry writes before the atom preserves the entries-before-atom ordering invariant. A refresh's `.atom` write is existence-gated (`require_exists`): if the user removed the feed during the fetch, the write is skipped rather than resurrecting it — the check and write are atomic on the feed lane, serialized against `remove_feed`. `add_feed` writes unconditionally (it creates the feed).
- **Classification rule for a new write feature:** does it need the writer? Then it is a write. This is enforced at compile time by the **`WriteToken`** capability: every `pub(crate)` persistence write takes a `&WriteToken` / `WriteToken`, and the token's constructor is private to `library_writer` (minted once at `start`, having taken the lock). So a persistence write can be called ONLY from within the writer — a future in-crate caller that tried to write directly, bypassing the per-file lanes, would not compile (it cannot obtain a token). `WriteToken::for_test()` (test-only) lets persistence unit tests call writes directly.

## Known limits

- **Check-to-use:** the read-only gate answers "at this instant," but a write command that passed it cannot execute against a released lock. It gets the writer handle via `require_writer`; if shutdown has begun, the writer's `shutting_down` flag fast-fails the submit rather than writing.
- The lock binds only xaga processes. External tools (e.g. Obsidian) are not participants: external edits are architecturally equivalent to a crash — reconciled from files by recovery at startup, not coordinated live.

## Relation to the other rules

The lock is the process-level face of the same discipline as the two-producer rule (`docs/principles/single_source_of_truth.md`): within the one writing process, LanceDB has exactly two producers; across processes, the lock ensures there is only one writing process for those producers to live in. Recovery's crash-healing (`crash_consistency.md`) assumes writes were serialized through a single writer — the mode gate is what makes that assumption hold for `.md` files. All library `.md` writes are temp+rename atomic (`write_atomic_async`, `crates/xaga-core/src/fs.rs`), so a reader in any process never observes a torn file and a crash mid-write leaves the old version intact.

### URL-associated PDFs

`LibraryWriter::save_pdf` serializes PDF and Entry-format companion writes in
the normalized URL's entry lane, alongside Markdown operations. Validation
precedes mutation; a missing companion is atomically created before the PDF.
An existing canonical Entry remains untouched and suppresses creation of a
duplicate companion. Successful HTML Markdown saves remove the associated PDF
before its dormant companion in that
lane; PDF saves leave canonical Markdown and annotations intact. A newly
downloaded PDF requires the library writer, while validated local reads do not
require write ownership.

### Disposable 0.9-to-0.10 CLI converter

`xaga-cli library upgrade` is an explicit exception to the runtime writer lane
requirement. It holds the exclusive library lock for the entire conversion and
cache reconciliation, runs file conversions sequentially, and commits each file
with fsync and atomic replacement. All migration-specific rules and direct cache
updates live in the CLI; no legacy conversion API is built into core. See the
[upgrade contract](../library_upgrade.md) and [crash ordering](crash_consistency.md#article-reading-state-and-explicit-library-upgrades).

### Central pins

The library writer also owns `pins.yaml`. Every pin mutation, regardless of target
type, uses the same Interactive lane and atomically replaces the complete store.
It drains at shutdown. Pinning no longer writes entry tags, Topic tags, or Feed
categories. Generic `set_pin` and the existing entity pin commands all pass the
central read-only gate and require a writer. The offline converter holds the
library lock while committing central membership before stripping legacy markers;
its migration-specific file writes remain in the CLI.

### Library version manifest

Core owns `library.yaml` and its atomic read/write API. The app compares this
library's stored major/minor with the running app's major/minor before constructing
the main webview or starting the writer/service. It never initializes missing
version metadata on open. The existing read-only command gate and controls apply
regardless of whether the cause is a version state or lock contention. An absent
cache is not created by read-only opening. Explicit pre-1.0 CLI migrations hold
the library lock and call the core manifest API before conversion and after
successful reconciliation. See [library version](library_version.md).
