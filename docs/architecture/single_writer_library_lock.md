# Single-Writer Library Lock — the Two Runtime Modes

**At most one participating process may write to a library. An exclusive advisory
file lock protects both entity files and LanceDB. The app opens read-write only
when its library version permits writing and it acquires the lock; otherwise it
opens read-only.**

## The mechanism

The exclusive `flock` on `<library>/xaga.lock` is acquired by `acquire_lock`
(`crates/xaga-core/src/library.rs`). `FileLock` ties it to an open handle, so the
OS releases it when the handle closes, including after a crash or forced exit.
A lock file remaining on disk does not require cleanup.

A completed library version matching the app is required before attempting the
lock. Missing, different, or incomplete metadata selects read-only mode;
unreadable metadata also reports its error. In read-write mode, `LibraryHandle`
contains both the library path and a `LibraryWriter` owning the lock. Read-only
mode has no writer. `is_library_read_only()` checks `writer.is_none()`
(`src-tauri/src/lib.rs`), rather than a separately stored mode flag. The Rust
accessor, Tauri command, `library:mode-changed` event payload, and frontend hook
use the same name (`src/hooks/useLibraryReadOnly.tsx`).

## Who takes it, and when

- **App startup:** resolves the library and checks its version before commands
  can run. Acquiring the lock selects read-write mode; failure selects read-only.
  `set_library` saves the new path and restarts. Before starting the successor,
  `drain_and_release_writer` finishes required writes and releases the old writer
  (`src-tauri/src/lib.rs`).
- **`LibraryWriter`:** `LibraryWriter::start(dir, lock)` owns the lock and handles
  runtime `.md` writes (`crates/xaga-core/src/library_writer/mod.rs`). Restart
  and shutdown drain it before dropping it.
- **`EmbeddingService`:** receives a shared `Arc` of the writer’s lock through
  `writer.lock()` and retains it for the service’s lifetime. The service runs
  only read-write and owns LanceDB writes, recovery, and the embedding queue.
  Both it and the writer must release their references before the lock is freed.
- **CLI writes:** use `require_write_lock`, failing immediately if another process
  holds the lock (`crates/xaga-cli/src/cmd/embeddings.rs`). Either the app or CLI
  may acquire the lock first.

## What read-only means

- The IPC check rejects library-writing commands with typed `E_READ_ONLY` errors.
  Add new write commands to `WRITE_COMMANDS` beside `is_invoke_allowed`
  (`src-tauri/src/commands/mod.rs`). This list supplies the early rejection and
  frontend control list. Independently, `require_writer` rejects access when no
  writer exists.
- `read_article` skips its incidental mark-read write. The article can be read
  without changing its unread status.
- The frontend reads the mode through `is_library_read_only` and
  `library:mode-changed`, shows a read-only badge, and disables write controls.
- File reads, search, and lists remain available. `EmbeddingsDb::open_readonly`
  opens an existing table without creating or migrating it. `open_writer` requires
  the library lock, which is retained across asynchronous mutations.

## In-process write serialization

The process lock excludes other writers. `LibraryWriter` separately coordinates
concurrent operations inside its process. Each file has a lane: a queue that runs
the complete read-modify-write operation serially. Different files can run in
parallel. Without this serialization, two Annotation edits could read the same
old Article and overwrite each other. Recovery cannot restore user content lost
from the source file.

- **Tiers:** `library_writer::Tier` distinguishes Interactive user actions
  (annotations, tag toggles, Topic edits, bookmark import) from Regular feed
  writes. Interactive is serviced first. It must also finish during shutdown,
  because its user-authored content may exist only in the queue. Regular backlog
  may be dropped; admit only content that can be recreated from its source.
- **Shutdown:** `drain_and_release_writer` drains Interactive backlog, rejects
  queued Regular work with `ShuttingDown`, and waits for in-flight writes before
  dropping the writer and releasing the lock. The `shutting_down` flag rejects
  late submissions immediately rather than leaving them in an unserviced queue.
- **Database work:** the writer serializes `.md` writes; `EmbeddingService`
  serializes LanceDB writes. Commands wait for the file write before queuing
  database sync. The files remain authoritative.
- **Feeds:** fetch outside the file lane. Create each new Entry in its own lane
  only if absent, preventing a second feed or user edit from being overwritten.
  Wait for all Entry writes before saving the feed’s `.atom` file in its lane.
  Refresh uses `require_exists` in that lane so a feed deleted during fetch is
  not recreated. That check and write serialize with `remove_feed`; `add_feed`
  writes unconditionally because it creates the feed.
- **Compile-time enforcement:** persistence write functions at `pub(crate)`
  require `WriteToken` or `&WriteToken`. Its constructor is private to
  `library_writer` and runs at `start` after lock acquisition. Other in-crate
  code cannot obtain a token to bypass the writer. Persistence unit tests can use
  the test-only `WriteToken::for_test()`.

## Known limits

- A command passing the initial read-only check still obtains its writer through
  `require_writer`. If shutdown starts before submission, the writer rejects it
  through `shutting_down`; it cannot write after releasing the lock.
- The lock coordinates participating Xaga processes only. External editors such
  as Obsidian do not acquire it. Their changes are reconciled by startup recovery,
  not coordinated live.

## Relation to the other rules

The process lock permits only one process to perform the two kinds of database
updates described in [Single Source of Truth](../principles/single_source_of_truth.md).
Within that process, file lanes prevent lost updates and the embedding service
serializes database writes. Recovery relies on those saved files.

`write_atomic_async` uses a temporary file and atomic rename
(`crates/xaga-core/src/fs.rs`). Readers see a complete old or new file rather
than partially written bytes; a crash before replacement leaves the old file.

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

### Explicit 0.9-to-0.10 CLI converter

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
central read-only check and require a writer. The offline converter holds the
library lock while committing central membership before stripping legacy markers;
its migration-specific file writes remain in the CLI.

### Library version manifest

Core owns `library.yaml` and its atomic read/write API. The app compares this
library's stored major/minor with the running app's major/minor before constructing
the main webview or starting the writer/service. It never initializes missing
version metadata on open. The existing read-only command check and controls apply
regardless of whether the cause is a version state or lock contention. An absent
cache is not created by read-only opening. Explicit pre-1.0 CLI migrations hold
the library lock and call the core manifest API before conversion and after
successful reconciliation. See [library version](library_version.md).
