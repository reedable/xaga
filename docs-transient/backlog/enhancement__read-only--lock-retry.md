# Read-only instance never becomes the writer without a relaunch

## Summary

A process that loses the library lock at startup remains read-only for its lifetime. When the writer exits, the read-only process does not attempt to acquire the lock and must be relaunched or switch libraries before it can write.

## Requirements

- Allow a read-only instance to acquire write ownership after the current writer releases the lock.
- Continue deriving mode from `LibraryWriter` presence; do not add independent mode state.
- Do not enable writes until the instance has refreshed file and LanceDB state and started all read-write services.
- Emit `library:mode-changed` only after the transition is complete.
- Preserve single-writer exclusion throughout acquisition and transition.

## Remaining work

- Compare periodic retry, blocking lock acquisition, focus-triggered acquisition, and automatic relaunch.
- Define how a successful transition constructs `LibraryWriter`, `EmbeddingService`, recovery, and the shared read handle without exposing partial state.
- Define how stale file and LanceDB views are discarded or reloaded before writes are admitted.
- Present in-process transition and automatic-relaunch designs with failure and shutdown behavior for decision.
- Implement and verify contention, writer exit, failed acquisition, successful promotion, and shutdown during promotion.

## References

- `docs/architecture/single_writer_library_lock.md`
- `crates/xaga-core/src/library.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/events.rs`
- `src/hooks/useLibraryReadOnly.tsx`
