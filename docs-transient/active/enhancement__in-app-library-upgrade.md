# Upgrade libraries from Xaga

## Summary

Verify the native desktop upgrade and default-browser launch using disposable
libraries. The user will perform this smoke test; automated failure coverage is
not replaced by it. Repository-wide UI test cleanup belongs to the
[separate story](compliance__locale-independent-ui-test-coverage.md).

## Requirements

- Verify the automatic read-only explanation, sidebar reopening, explicit upgrade,
  progress, restart into the same library, accurate mode and preserved data.
- Verify **Upgrade Xaga** opens the fixed releases page in the default browser.
- Record failures with the action, visible result, app version and OS. Resolve
  confirmed defects and rerun the affected automated and desktop checks before
  closing this story.

## Remaining work

The user runs the [desktop checklist](../../docs/development/library_upgrade_verification.md)
and supplies results. Resolve any reported failures; delete this story when the
checks pass.

## References

- [Upgrade guide](../../docs/library_upgrade.md)
- [Manifest and startup contract](../../docs/architecture/library_version.md)
- [Converter and failure tests](../../crates/xaga-core/src/library/upgrade.rs)
- [App recovery and restart tests](../../src-tauri/src/commands/library.rs)
- [Browser workflows](../../e2e/library-upgrade.spec.ts)
