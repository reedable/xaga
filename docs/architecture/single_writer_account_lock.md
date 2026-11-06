# Single-Writer Account Lock

**An exclusive `flock` on `settings.json` coordinates resources shared by the
same machine/user account, including settings writes and model downloads. No
separate account lock file is used.**

## Scope

Account resources live under `xaga_core::fs::data_root()` independently of any
library. They include `settings.json` and the model directory. The
[library lock](single_writer_library_lock.md) protects one library’s files and
LanceDB; neither lock replaces the other.

Settings writes replace the whole file after reading it. Using `settings.json`
as the lock file coordinates those writes and other account operations through
the same lock.

## Acquisition

- `FileLock::try_acquire` takes an exclusive advisory `flock` on `settings.json`
  (`crates/xaga-core/src/fs.rs`). It creates a missing file without truncating it.
- The OS releases the handle’s lock on exit, including after a crash.
- After acquisition, compare the handle’s `dev` and `ino` with the file currently
  at the path. Atomic replacement may have changed that file while acquisition
  was pending. On mismatch, release the old handle and retry the current path.
  This identity check is Unix-specific; the locking protocol is POSIX-verified.

## In-process sharing

`AccountLock` shares a single reference-counted handle inside the process
(`src-tauri/src/lib.rs`). Reacquiring a separate `FileLock` for each operation
previously made the process contend with itself: a model download held one
handle while a settings write attempted another.

- Acquire the lock when the first account operation starts; release it when the
  last finishes. Holding it for the whole process lifetime would prevent an
  instance using another library from calling `set_library`, which writes the
  path in `settings.json`.
- Settings writes hold the registry mutex across `write_locked`, serializing
  complete read-modify-write operations.
- Model download holds an `AccountLease` across the asynchronous fetch. The lease
  retains the shared reference count, not one particular file handle. A settings
  write can therefore replace the file and transfer the lock without ending the
  download’s exclusion of other processes.
- If `write_locked` fails to replace the file, it returns the still-valid original
  lock. The failure must not release a lock still required by another lease.

## Writing settings.json: the rotation protocol

Every app settings write calls `update_settings` (`src-tauri/src/lib.rs`). Build
its payload from settings read while holding the lock plus the requested change,
not from a snapshot read before acquisition. `fs::write_locked` then:

1. Opens `settings.json.tmp` without truncation, preserving a previous writer’s
   temporary file until its lock is acquired.
2. Locks the temporary file before rename, so the destination path never names
   an unlocked replacement.
3. Truncates, writes, and fsyncs the temporary file.
4. Renames it over `settings.json`, keeping the new inode locked.
5. Releases the previous inode’s lock.

After a crash, the next writer can lock and truncate a leftover temporary file.
Process exit releases its flocks; no stale-lock cleanup is needed.

## Stale-view check

Each process records the file’s modification time when reading its settings
view, initially at startup and again after every write outcome. A mismatch on a
later write means the file changed since that view was loaded. Under the lock,
return fresh settings as `SettingsOutcome::StaleView` instead of overwriting
them. The caller refreshes and the user retries against the current values.

This prevents a whole-file update from overwriting unrelated newer fields. It
also detects changes by external programs that did not acquire the lock.

## Contention behavior

- Settings writers return an explicit error immediately if another process holds
  the lock.
- A model download’s caller holds the lock across check, download, and commit
  (`download_model`, `crates/xaga-core/src/embeddings/model.rs`). The app uses
  an account lease; the CLI takes the flock directly. A caller that encounters
  contention skips downloading and rechecks the files produced by the holder.
- Readers do not lock. Atomic rename ensures a complete old or new file is read.

## Who takes it

- Every app settings write through `update_settings`.
- Model download from app startup and `xaga-cli embeddings download-model`.
- Future operations on account resources, using this same lock.

## Limits

The lock is advisory: external tools can write without acquiring it. The
stale-view check detects those changes before a later settings update; it does
not prevent the external write itself.

## Relation to the other rules

The single settings write function follows
[Single Source of Truth](../principles/single_source_of_truth.md). The replacement
order preserves the file and lock across interruption, like the atomic file
replacement described in [Crash Consistency](crash_consistency.md).

## Frontend refusals

Settings writes return `settings:busy` on contention and `settings:stale-view`
when the backend refreshes its settings after detecting a stale write.
`SettingsProvider.saveSettings` reports these through the shared error display
and reloads general, appearance, and privacy settings. Local values change only
after a successful write. Refused writes reload backend values before retry.
