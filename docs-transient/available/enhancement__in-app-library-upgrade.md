# Upgrade libraries from Xaga

## Summary

Let users upgrade an older library from the app instead of running `xaga-cli`.
For newer libraries, direct users to an app release compatible with their library.
Share migration implementation in core between the app and CLI.

## Requirements

- Compare the selected library's authoritative major/minor version with the
  running app version, using the existing manifest API. Keep mismatched libraries
  read-only until compatibility and write ownership are established.
- For a newer library, show the **Read-only** badge with a dropdown caret and an
  **Upgrade Xaga** menu item. Open the fixed Xaga GitHub Releases page in the
  default browser: `https://github.com/reedable/xaga/releases`.
- For an older library, offer **Upgrade Library** from the badge. Open a dialog
  explaining the format change, that it cannot be undone in the app, that older
  app versions may no longer use the library safely, and that the user should
  back up the complete library first. Restoring that backup is the way to return
  to the previous format. Provide Cancel and **Upgrade Library** actions; start
  only after explicit confirmation.
- Expose the read-only cause, stored version, pending target, and app version as
  typed data. Lock contention alone must not suggest an app or library upgrade;
  malformed metadata must not be overwritten or guessed from file contents.
- Offer **Upgrade Library** for unversioned libraries using existing CLI
  conversion semantics and the same confirmation/backup warning. For an
  interrupted upgrade targeting the running app's version, offer **Resume Library
  Upgrade** and explain that changes are already partially applied. For a
  different pending target, explain which matching app is required and keep
  read-only without changing the target. Explain malformed metadata and lock
  contention without offering an unsafe upgrade.
- Move existing library conversion and cache reconciliation from CLI into core;
  retain the CLI command as a caller of the same implementation. Keep dialogs,
  browser launching, and CLI output in their respective application layers.
- Acquire the exclusive library lock and recheck the selected library and
  manifest before mutation. Prevent concurrent upgrades and ordinary library
  writes during conversion. Existing reader processes are not excluded by this
  lock; explain that other library users should be closed before upgrading.
- Preserve existing conversion fidelity, bounded batching, atomic writes,
  pending-target checks, and retry behavior. Complete the manifest only after
  conversion and cache reconciliation succeed. Failures leave the library
  read-only with actionable failure/retry feedback; no model download is needed.
- Show upgrade progress and completion/failure without blocking the UI. Do not
  imply cancellation can roll back committed changes.
- After successful completion, restart Xaga through a controlled shutdown and
  reopen the same library using normal startup compatibility and lock checks.
  Report restart failure; successful migration alone must not imply write access
  before startup establishes it.
- This moves the upgrade mechanism without changing the storage format. Any
  additional storage-model change requires a minor-version increment at major 0.

## Remaining work

- Extract the shared core API with typed results/progress, migrate CLI callers
  and tests, and connect app status, commands, badge menu, dialog, and restart.
- Automate older/equal/newer comparisons, numeric version ordering, unversioned
  and invalid metadata, pending targets, lock contention, duplicate submissions,
  failure/interruption/retry, preservation, and app/CLI parity. Retain existing
  migration regression and scale coverage.
- Add browser workflow tests for badge actions, confirmation/cancel, progress,
  errors/retry, and success. Verify native browser opening, locking, and the
  selected reopening path separately. Run Rust/frontend checks, workspace
  formatting, and `npm run test:browser`; report verification gaps.
- Update layer ownership, library-version, locking/crash contracts, upgrade guide,
  User Guide, and training to describe explicit in-app upgrades and shared core
  ownership, replacing the current CLI-only/pre-1.0 restriction.

## References

- [CLI converter and regression tests](../../crates/xaga-cli/src/cmd/library_upgrade.rs)
- [Core manifest](../../crates/xaga-core/src/library/manifest.rs)
- [App library commands](../../src-tauri/src/commands/library.rs)
- [Sidebar badge](../../src/components/sidebar/Sidebar.tsx)
- [Library version contract](../../docs/architecture/library_version.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Library locking](../../docs/architecture/single_writer_library_lock.md)
- [Upgrade guide](../../docs/library_upgrade.md)
