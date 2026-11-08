# Read-only mode: complete the UI experience

## Summary

The backend rejects library mutations with `library:read-only`, but some write
controls remain enabled. Complete the documented disabled-control behavior.

## Requirements

- Disable remaining library-write controls in read-only mode, explaining why:
  article read-status actions, Mark all read, annotation comment/color/removal
  controls, Feed/Topic form mutations, native imports, and article/feed refresh.
- Preserve annotation and collection inspection, navigation, and non-mutating
  actions, including settings, export, and library switching.
- Retain backend enforcement and use the existing shared error presentation for
  residual refused writes. Do not display unsaved edits as successful changes.

## Remaining work

- Apply mode checks to the remaining controls and handlers, including native-menu
  enablement. Keep existing read-only controls intact.
- Test read-only startup, delayed mode delivery, disabled-control explanations,
  preserved inspection, and backend refusals; verify writable behavior still works.
- Run relevant component/backend checks and `npm run test:browser`. Verify native
  import-menu state separately and report remaining verification gaps.

## References

- [Read-only contract](../../docs/architecture/single_writer_library_lock.md)
- [Mode hook](../../src/hooks/useLibraryReadOnly.tsx)
- [Write-command policy](../../src-tauri/src/commands/mod.rs)
- [Article controls](../../src/components/content/Toolbar.tsx)
- [Annotation controls](../../src/components/content/AnnotationTooltip.tsx)
- [Item list](../../src/components/list/EntityList.tsx)
- [Dialogs](../../src/components/dialogs/)
- [Native menu](../../src-tauri/src/lib.rs)
