# Account lock: process contends with itself (pending)

The app's own settings save fails during its own model download with the
false message "another xaga process holds the account lock." Cause: the
account lock is acquired per call site — each acquire opens a fresh handle
on settings.json, and flock contends between handles even within one
process, so one process shows up as two competing strangers.

The owner's design is ownership semantics, not contention: *whoever has
the lock on settings.json has the right to account-wide xaga resources,
including writing settings.json* — the holder is the app instance; writes
happen under the held right, rotating the lock inode via the
tmp-lock-rename protocol with holder continuity (implemented as
`fs::write_locked`).

Code: `crates/xaga-core/src/fs.rs` (`FileLock`, `write_locked`,
`try_acquire_account_lock`), `src-tauri/src/lib.rs` (`update_settings`
funnel), `crates/xaga-core/src/embeddings/model.rs`
(`download_model_if_missing`).

## Vetted fix design (registry variant)

One canonical `FileLock` per process, in shared state; every in-process
user of the right shares it. Requirements established by vetting:

1. **Acquire must be atomic**: registry lookup → flock → register under one
   mutex hold. Non-atomic acquire lets two in-process threads both miss
   the registry and race the flock — recreating the false-contention bug
   in a window.
2. **Failed rotation must not forfeit the right**: `write_locked` consumes
   the held lock and drops it on error today; its error path must hand the
   original lock back (every failure is either before the rename — old
   inode still valid — or is the rename failing, same thing).
3. **Baseline read moves inside the writers turn**: otherwise two honest
   sequential in-process writers produce a spurious `StaleView` for the
   second.
4. Cross-process semantics unchanged: writers fail fast, downloaders skip;
   the "another xaga process" message becomes always literally true.

Known lesser flaws of the registry variant (recorded): path-aliasing
defeats the registry (latent — all callers derive the path from
`data_root()`); mutex poisoning on panic (inherited codebase style);
funnel-bypassing writers remain invisible (pre-existing, unchanged).

## Open decision

Registry variant (small diff, flaws above are mitigable) vs a dedicated
account-service actor owning the flock outright — single owner task, typed
requests, the flaw class vanishes by construction; `EmbeddingService` is
the in-repo idiom; larger change (channel plumbing, async funnel, actor
lifecycle, CLI story). Reusing `EmbeddingService` itself was analyzed —
its lifecycle is library-scoped (absent in lockless instances, restarted
on library switch) while the account right is machine-scoped — and the
owner has not ruled on it.
