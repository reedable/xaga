# Drain the library writer before main-window close exits or restarts

## Summary

The main window's `CloseRequested` path shuts down the embedding service and
then exits or restarts without draining the library writer. Correct this path
so pending Interactive writes finish and all library-lock holders are released
before exit or a successor process starts. This defect precedes the application
startup refactor.

## Requirements

- Main-window close must shut down any running embedding service, then drain and
  release any library writer before exiting or spawning a successor. The writer
  must still drain when no embedding service is present. Read-only close must
  remain possible without creating a writer or modifying the library.
- Preserve the existing shutdown policy: finish committed Interactive work,
  discard queued Regular work under the writer's existing rules, and retain the
  embedding service's Exit/Restart outcome. Closing must not bypass pending
  shutdown work; repeated close/quit requests must not cause early termination,
  duplicate shutdown, or multiple successor processes.
- Reuse the existing writer-drain and lock-release implementation. Preserve
  native close/hide behavior and existing Quit, signal, and library-switch
  shutdown contracts. Do not perform the broader startup/module refactor here.
- Review all affected documentation and update architecture, development, User
  Guide, and training documents as applicable, including code references. Keep
  the library-lock document authoritative for shutdown ordering.

## Remaining work

- Route main-window close through ordered service shutdown and writer draining,
  accounting for absent-service/read-only states and repeated shutdown requests.
- Add regressions that exercise the close-path coordination, rather than only
  calling `drain_and_release_writer` directly. Verify pending Interactive writes
  reach disk before termination, the writer's Regular-work policy is retained,
  and a successor can acquire the lock only after the service and writer release
  it. Cover Exit and Restart, service absent with a writer, read-only close, and
  repeated close/quit requests without real test-process termination.
- Run relevant application/core shutdown regressions and `npm run test:headless`.
  Verify main-window close, Quit, and restart natively with an isolated fixture
  library; check persisted edits and successor write mode. Report any native
  verification gap rather than inferring these outcomes from browser mocks.
- Update affected documentation. Complete when the close path obeys the shutdown
  contract and the regression and native checks establish persistence and
  lock-release ordering.

## References

- [Main-window close and shutdown helpers](../../src-tauri/src/lib.rs)
- [Library-switch restart](../../src-tauri/src/commands/library.rs)
- [Writer shutdown and regressions](../../crates/xaga-core/src/library_writer/mod.rs)
- [Embedding service shutdown](../../crates/xaga-core/src/embeddings/service.rs)
- [Library locking and shutdown](../../docs/architecture/single_writer_library_lock.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Priority](../../docs/principles/definition_of_priority.md)
- [Verification workflow](../../docs/development/verification.md)
- [Application startup refactor](refactor__hotspots--app-startup.md)
