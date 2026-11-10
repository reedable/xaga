# Upgrade libraries from Xaga

## Summary

Finish the in-app upgrade's startup, recovery, and operation-state presentation;
automate the failure cases below and prepare the user's desktop smoke test.
The library storage format does not change. Repository-wide test cleanup belongs
to the [separate story](compliance__locale-independent-ui-test-coverage.md).

## Requirements

- On read-only opening, show an explanation dialog before model initialization or
  download. **Continue read-only** dismisses it and proceeds with normal startup;
  ordinary rerenders must not reopen it. The **Read-only** button reopens the
  dialog and has no tooltip.
- Choose actions from the authoritative library status:

  | Cause | Dialog actions |
  | --- | --- |
  | Older, unversioned, or interrupted upgrade resumable by this app | **Upgrade**, **Continue read-only** |
  | Newer library or pending target newer than this app | **Upgrade Xaga**, **Continue read-only** |
  | Invalid metadata or unavailable write lock | **Continue read-only**; no library upgrade |

- Before **Upgrade**, explain the target version, older-app compatibility, complete
  library backup, and restoration. Only that explicit choice starts or resumes
  conversion; it must not initialize or download a model. **Upgrade Xaga** opens
  `https://github.com/reedable/xaga/releases` in the default browser.
- Keep ordinary writes disabled through conversion and restart. Isolate local
  failures, continue independent work, and report what needs retry. Stop dependent
  work when its prerequisites fail; do not terminate Xaga for a recoverable error.
  Complete the manifest only after required conversion and cache reconciliation
  succeed, retaining the original starting version while work remains pending.
- After conversion or restart failure, attempt to restore a valid read-only
  handle in place. If restoration fails, identify unavailable features and a
  recovery action; missing counts or memberships must not appear as empty data.
- Show confirmation, conversion progress, completion/restarting, and restart
  failure as distinct states. Remove pre-upgrade instructions after completion.
  Successful conversion triggers controlled restart into the same library;
  startup rechecks compatibility and write ownership. Restart failure reports
  completed conversion and offers restart retry.

## Remaining work

1. **Startup:** move the read-only choice ahead of the embedding readiness gate
   in `src/App.tsx` and model startup in `src-tauri/src/lib.rs`.
2. **Recovery:** restore `embeddings_read` after failed conversion or restart in
   `src-tauri/src/commands/library.rs`; propagate unavailable states to counts,
   result enrichment, and reverse Topic lookups when reopening fails.
3. **Presentation:** replace `LibraryUpgradeControl`'s tooltip/menu explanation
   with the dialog and operation states specified above.
4. **Verification:** add or adapt automated tests for this matrix using disposable
   libraries and controlled failures. Retain existing migration preservation,
   batching, scale, and app/CLI parity coverage. Check behavior and accessibility
   without exact-copy assertions. Prepare the desktop checklist after automation;
   the user will perform it and supply results.

| Trigger | Expected outcome | Required check |
| --- | --- | --- |
| No model installed; offline, delayed, or failed embedding startup | Dialog remains reachable; **Upgrade** makes no model calls; **Continue read-only** starts normal initialization without changing library files | Startup integration and browser tests for both choices |
| Newer library/target, invalid metadata, or contended lock | Actions match the cause; no unsafe conversion or metadata replacement | Core/app ownership tests and browser actions |
| Duplicate submissions, library switching during conversion, or manifest changed after inspection | No competing upgrade, stale mutation, or ordinary write | App command concurrency and manifest recheck tests |
| Malformed or unreadable file among valid files | Failed source preserved; independent files proceed; required work remains pending; same/newer-version retry preserves content, annotations, metadata, state, and memberships without duplication | Core mixed-file failure/retry fixtures |
| Interruption before/after source replacement, between store commit and legacy-marker removal, between source and cache updates, or before manifest completion | Complete old/new files, no lost committed membership, no premature completion; safe reapplication from the recorded starting version | Core fault-injection tests at these checkpoints |
| Cache open, reconciliation, or finalization failure | Preserve authoritative files, retain pending state for unfinished required work, and repair derived state on retry | Core cache failure/retry tests |
| Progress polling fails transiently | Conversion continues; UI reports lost progress and retries observation without claiming completion | Browser/platform failure fixture |
| Conversion fails, or conversion completes but settings save/successor spawn fails | App stays read-only and reports the correct outcome; read-handle restoration succeeds or explicitly exposes unavailable features; retry remains usable | App failure injection plus browser state/action tests, including counts and Topic lookups |
| Successful restart and reopening, including another writer acquiring the lock | Same selected library opens through normal compatibility and lock checks; no assumed write access | Automated process/startup integration test |
| Default-browser launch and complete desktop upgrade/restart | Releases page opens; selected library reopens with accurate mode and recovery controls | User desktop smoke test; supplementary to automated coverage |

## References

- [Converter and regression tests](../../crates/xaga-core/src/library/upgrade.rs)
- [Manifest contract](../../docs/architecture/library_version.md)
- [App upgrade commands](../../src-tauri/src/commands/library.rs)
- [Startup and read handles](../../src-tauri/src/lib.rs)
- [Application startup gate](../../src/App.tsx)
- [Upgrade presentation](../../src/components/sidebar/LibraryUpgradeControl.tsx)
- [CLI parity and locking test](../../crates/xaga-cli/tests/library_upgrade.rs)
- [Browser upgrade workflows](../../e2e/library-upgrade.spec.ts)
- [Upgrade guide](../../docs/library_upgrade.md)
