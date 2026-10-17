# Single-Writer Account Lock

**The flock on `settings.json` is the account-wide xaga resource lock: whoever holds it has the right to account-wide resources — writing settings.json itself, downloading the embedding model, and any future account-scoped resource. The holder is the app instance; there is no separate lock file.**

## Scope

The account is machine/user scope: everything under the data root (`xaga_core::fs::data_root()`) that exists independently of any library, including settings.json and the model directory. This is a different domain from the [library lock](single_writer_library_lock.md), which is per-library and protects concurrent writes to the whole library (`.md` files and LanceDB); the two locks answer different questions and neither substitutes for the other.

settings.json doubles as the lock file by design: it is itself the account resource most in need of write coordination (its writes are whole-file read-modify-write), so locking it protects it while also serializing every other account-wide operation — one lock, no invented lock file.

## Acquisition

- Exclusive advisory `flock` on `settings.json` under the data root (`FileLock::try_acquire`, `crates/xaga-core/src/fs.rs`); the file is created if absent, never truncated by acquisition.
- The lock is handle-tied: the OS releases it on any exit, including crashes. No stale lock files.
- **Acquisition verifies the inode.** Because writes replace settings.json by rename (below), a contender can win a flock on an inode that was renamed away while it waited. After every successful flock, the acquirer compares the handle's `dev`+`ino` against the path; on mismatch it releases and retries on whatever the path names now. Unix-only identity check (POSIX-verified protocol overall).

## In-process sharing (one holder, reference-counted)

Within a process the flock is held by a single shared, reference-counted handle (`AccountLock`, `src-tauri/src/lib.rs`), **not** re-acquired per call site. Acquiring a fresh `FileLock` per operation was a bug: the model download holds the lock across its whole fetch, and `flock(2)` binds to the open file description, so a settings write during the download opened a _second_ handle and the process contended with itself ("another xaga process holds the account lock" — but it was the same one).

- **Operation-scoped, not process-lifetime.** The flock is acquired when the first in-process account operation begins and released when the last finishes. It is _not_ held for the process's life, because the library path lives in `settings.json` and `set_library` is an account write — a process-lifetime hold would stop any _other_ instance (on a different library) from ever selecting its library.
- **Settings writes are synchronous, single-streamed.** Each holds the registry mutex across `write_locked`, so two whole-file read-modify-writes can never interleave (the same clean invariant `write_locked`'s tmp flock assumes).
- **The model download holds a lease** (`AccountLease`) that carries only the refcount across its async fetch — it never writes settings, so a settings write can rotate the inode underneath it; the download's exclusion transfers to the new handle because the lease references the count, not a specific handle.
- **`write_locked` hands the lock back on error** (`fs.rs`): a failed rotation returns the original still-valid lock rather than dropping it, so a settings-write failure never releases the flock a concurrent lease still needs.

## Writing settings.json: the rotation protocol

Every settings.json write in the app goes through one funnel (`update_settings`, `src-tauri/src/lib.rs`); the write payload is built only from state loaded _inside_ the hold plus the caller's delta — never from data read before the lock. The write itself (`fs::write_locked`):

1. open `settings.json.tmp` _without_ truncating (a crashed writer's leftover must not be destroyed before its lock is held);
2. flock the tmp — before the rename, so the path never resolves to an unlocked inode;
3. truncate, write, fsync;
4. rename the tmp over `settings.json` — the locked tmp inode becomes the path's inode;
5. release the old inode's lock. The holder's exclusive hold survives the replacement without a gap.

Crash story: a stale tmp is reclaimed by the next writer's truncation; flocks die with the process; no cleanup code exists or is needed.

## Stale-view check

A write can be refused for a reason other than contention: the baseline. Each process records settings.json's mtime at the read that formed its view (startup load; refreshed on every write outcome). A write whose baseline no longer matches the file means another process changed settings since this one last looked; the funnel returns the fresh content as a normal outcome (`SettingsOutcome::StaleView`, read under the lock) rather than silently overwriting — the caller refreshes its view and the user retries against current truth. This closes the lost-update race (whole-file writes carrying stale values of every field but the one being changed) and detects writers that bypassed the lock entirely.

## Contention behavior

- **Settings writers** fail fast with an explicit error when another process holds the lock; nothing blocks.
- **The model download** (`download_model`, `crates/xaga-core/src/embeddings/model.rs`) runs while its **caller** holds the right for the whole check + download + commit — the app via its in-process account lease, the CLI via a direct flock — and the caller _skips_ when another process holds it; the raced loser re-checks and finds the winner's files.
- **Readers never lock.** Rename atomicity guarantees a reader sees an old-complete or new-complete file, never a torn one.

## Who takes it

- The settings write funnel (`src-tauri/src/lib.rs`) — every settings.json write in the app.
- The model download, from both the app's startup task and `xaga-cli embeddings download-model`.
- Any future account-scoped resource acquires the same right rather than minting its own lock.

## Limits

- The lock is _advisory_: it binds only participants that acquire it. An external program editing settings.json directly is not excluded — the stale-view check is the tripwire that keeps such writes from being silently overwritten, not a fence that prevents them.

## Relation to the other rules

The funnel is the enumerated-producer discipline of `docs/principles/single_source_of_truth.md` applied to settings.json: one write path, whose payload provenance is stated at the write site. The rotation protocol is the directory-file sibling of `fs::write_atomic`'s crash story (`crash_consistency.md`): ordering decides what survives, and every crash point leaves either the old complete file or the new one.

## Frontend refusals

Settings writes return `settings:busy` for lock contention and `settings:stale-view` when the backend refreshes its mirrors after a stale write. Preference setters use `SettingsProvider.saveSettings` to report refusals through the shared error display and trigger reloads in general, appearance, and privacy settings. Local values change after a successful write; refused writes reload the backend values before the user retries.
