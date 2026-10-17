# Verify embeddings writers hold the library lock

## Summary

The library lock is advisory and protects only participating writers. Verify that every application and CLI path that mutates or resets LanceDB holds the library's write authority for the operation's lifetime.

## Requirements

- Cover production application and CLI paths; test-only database instances are out of scope.
- Include create, reset, destroy, row mutation, compaction, index creation, and recovery.
- Hold the lock across asynchronous work, not only during admission.
- Refuse mutation in read-only processes.

## Remaining work

- Trace every production caller of mutating `EmbeddingsDb` methods to its lock acquisition.
- Identify paths that obtain a database handle before write authority is established or retain it after authority ends.
- Gate any bypass through the established application lease or CLI lock helper.
- Add tests for refused concurrent application and CLI writers.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-cli/src/cmd/embeddings.rs`
- `crates/xaga-core/src/fs.rs`
- `crates/xaga-core/src/library.rs`
- `docs/architecture/single_writer_library_lock.md`
