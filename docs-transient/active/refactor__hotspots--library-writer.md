# Separate library writer operations, submission, and scheduling

## Summary

Separate operation definitions/execution, submission methods, and queue scheduling internally while retaining one `LibraryWriter` and its existing consumer API.

## Requirements

- Split library-writer operation definitions/execution, submission methods, and
  queue scheduling into internal modules. Preserve one `LibraryWriter`, one
  lock owner, per-file lanes, priorities, the concurrency cap, responder outcomes,
  and shutdown draining/refusal rules. Keep write-token construction restricted
  to the writer; moving code must not permit callers to bypass serialization.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Extract writer operations, submission methods, and scheduling while retaining
  existing consumer access paths. Run writer regressions for same-file
  serialization, different-file concurrency, shared tag/pin lanes, resource
  replacement, submit/shutdown races, and committed Interactive writes surviving
  shutdown.
- Run `npm run lint`, `npm run typecheck`, and `npm run test:headless`.
  Add or adjust meaningful regression coverage where existing tests do not
  establish the preservation requirements; retain test strength when moving
  tests with their implementation.
- Update affected documentation and code references against the final module
  layout. Complete when the specified extractions, documentation updates, and
  required verification pass.

## References

- [Library writer and regressions](../../crates/xaga-core/src/library_writer/mod.rs)
- [Resource lifecycle regressions](../../crates/xaga-core/src/library_writer/resource_lifecycle_tests.rs)
- [Library locking and scheduling](../../docs/architecture/single_writer_library_lock.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Priority](../../docs/principles/definition_of_priority.md)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification workflow](../../docs/development/verification.md)
- [Supporting hotspot evidence](../ideas/refactor-hotspots-evidence.md)
