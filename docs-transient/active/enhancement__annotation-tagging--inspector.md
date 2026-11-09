# Assign tags in Annotation Inspector

## Summary

Add Annotation Inspector's Tags view by sharing the Article tag picker.

## Requirements

- Retain searchable toggle chips, creation with immediate assignment, and no Save
  step. Removing an assignment retains its definition.
- Target the inspected parent Entry ID and Annotation ID, including popup targets
  that differ from route selection. Missing annotations must not fall back to
  article tags.
- Subscribe before reading assignments and retain committed changes received
  during that read. Discard stale target results; pending operations must not
  update another target's display. Closing inspection does not cancel a requested
  write. Failed writes must not appear saved.
- Follow replacement IDs after recoloring and the existing Inspector lifecycle
  after deletion. Preserve independent Article/Annotation tab selections, drawer
  and popup behavior, reading position, responsive presentation, and focus return.
- Reflect renamed definitions and merge/delete state using existing tag lifecycle
  behavior. Read-only inspection shows assignments and disables assignment and
  creation. Provide loading/error feedback, keyboard activation, accessible target
  naming, and chip pressed state. Follow the motion convention.

## Remaining work

- Depends on annotation tagging storage, indexing/recovery, and lifecycle support.
- Generalize `EntryTags` for Article and Annotation targets, connect the Annotation
  Tags tab/panel, and extend existing capture fixtures. Preserve Article behavior.
- Add component tests for popup targets differing from route selection, changes
  during initial reads, target switches during writes, closing during writes,
  failures, lifecycle events, and replacement/deletion.
- Add browser workflows for tag creation/toggling, read-only inspection, keyboard
  access, and drawer focus. Reuse capture reset/readiness. Run
  `npm run test:browser` and relevant frontend/backend checks; verify native commands
  and persistence separately and report remaining gaps.
- Update Inspector documentation, User Guide, and training.

## References

- [Storage prerequisite](enhancement__annotation-tagging--storage.md)
- [Indexing prerequisite](enhancement__annotation-tagging--indexing-and-recovery.md)
- [Lifecycle prerequisite](enhancement__annotation-tagging--lifecycle.md)
- [Inspector contract](../../docs/architecture/article_inspector.md)
- [Inspector](../../src/components/content/Inspector.tsx)
- [Existing picker](../../src/components/content/EntryTags.tsx)
- [Annotation popup](../../src/components/content/AnnotationTooltip.tsx)
- [UI testing](../../docs/conventions/ui_testing.md)
