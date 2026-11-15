# Release the library lock before starting a successor

## Summary

The shutdown coordinator drains the writer, but a command retaining a cloned
`LibraryWriter` also retains its `Arc<FileLock>`. Shutdown can return before a
successor can acquire the library lock.

## Remaining work

- Decide how shutdown releases ownership when commands retain writer handles.
  Waiting for all handles preserves current ownership but can wait on unrelated
  asynchronous work. Centralizing lock ownership separates submission handles
  from the lock lifetime, but changes the writer/service ownership contract.
- Implement the selected approach while preserving service-before-writer shutdown,
  Interactive draining, Regular refusal, read-only operation, and single shutdown.
- Make the retained-command regression in `src-tauri/src/shutdown.rs` pass; it
  currently fails because a successor cannot acquire the lock after shutdown.
- Verify native close, Quit, and restart with retained command handles, update
  the authoritative lock contract, and pass the completion gate.

## References

- [Shutdown coordinator](../../src-tauri/src/shutdown.rs)
- [Writer ownership](../../crates/xaga-core/src/library_writer/mod.rs)
- [Library lock contract](../../docs/architecture/single_writer_library_lock.md)
- [Decision ownership](../../docs/principles/decision_ownership.md)
