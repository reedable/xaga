# Separate application settings, menus, themes, and lifecycle

## Summary

Separate settings/account locking, menu handling, theme application, startup initialization, and shutdown/restart coordination from `src-tauri/src/lib.rs`, retaining `run()` as the composition point.

## Requirements

- Depends on [correct main-window-close shutdown](defect__window-close-writer-drain.md).
  Complete that defect first; this story preserves the corrected shutdown
  behavior rather than implementing the defect repair as part of extraction.
- Extract settings persistence and account locking into one internal module.
  Preserve the single shared account-lock registry, operation-scoped leases,
  locked read-modify-write, stale-view checks, and lock transfer during settings
  file replacement. Preserve settings serialization, defaults, and live updates.
- Extract native menu construction and menu-event handling together into one
  internal module, and theme application helpers into a separate internal
  module. Preserve menu IDs, enabled states, checkmarks, event routing, theme
  selection, system-theme behavior, and window initialization. Menu actions that
  change the theme must use the shared theme implementation.
- Extract startup initialization and shutdown/restart coordination into separate
  internal modules. Keep `run()` as the composition point and pass required
  shared state explicitly. Preserve proxy setup before outbound requests,
  settings initialization, library upgrade and read-only gates, service startup,
  and shutdown/restart ordering, including writer draining and lock release
  before a successor process starts.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Move the settings persistence and account-lock implementation out of
  `src-tauri/src/lib.rs`, update its consumers, and retain regression coverage
  for settings writes and model-download leases sharing the account lock,
  including successful writes, contention, stale views, replacement failures,
  and lease release.
- Move menu construction and event handling out of `src-tauri/src/lib.rs`,
  retaining the existing specialized menu modules as the owners of their logic.
  Move theme helpers into their own module and update startup and settings
  consumers to use it. Verify menu actions, checkmark synchronization, and theme
  application to existing and newly created windows.
- Move startup initialization and shutdown/restart helpers to their respective
  modules and connect them through `run()`. Retain coverage for startup gates,
  read-only selection, service lifecycle, writer draining, and lock release;
  verify native main-window close, Quit, and restart paths.
- Run `npm run lint`, `npm run typecheck`, and `npm run test:headless`.
  Add or adjust meaningful regression coverage where existing tests do not
  establish the preservation requirements; retain test strength when moving
  tests with their implementation.
- Verify native menu/theme behavior, startup/read-only/upgrade gates, quit and restart, including lock release before starting a successor. Browser mocks do not verify native menus, windows, or process locks.
- Update affected documentation and code references against the final module
  layout. Complete when the specified extractions, documentation updates, and
  required verification pass.

## References

- [Prerequisite shutdown defect](defect__window-close-writer-drain.md)
- [Application entry point](../../src-tauri/src/lib.rs)
- [Settings commands](../../src-tauri/src/commands/settings.rs)
- [Account locking](../../docs/architecture/single_writer_account_lock.md)
- [Library locking and shutdown](../../docs/architecture/single_writer_library_lock.md)
- [Library upgrade verification](../../docs/development/library_upgrade_verification.md)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification workflow](../../docs/development/verification.md)
- [Supporting hotspot evidence](../ideas/refactor-hotspots-evidence.md)
