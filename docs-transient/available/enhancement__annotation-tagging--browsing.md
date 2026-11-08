# Include tagged annotations in Tags browsing

## Summary

Extend Tags browsing to articles and annotations. Collection-scoped tag filtering
and its navigation redesign are outside this story.

## Requirements

- Each result must directly carry every selected tag. Never combine assignments
  across entities. Preserve existing alias resolution and unavailable/deleting tag
  handling.
- No selected tags behaves the same as All items under the active filters.
  With tags selected, retain Saved/All and Unread suppression. Saved includes
  matching annotations and color-marked articles. Unread counts count matching
  unread Entries once; annotations do not contribute.
- Retain mixed-entity ordering, text filtering, and keyboard selection. Selecting
  an annotation opens its parent article at the passage and retains selected tags.
- Assignment, reading/color changes, and index notifications preserve current list
  membership. Explicit navigation, tag/filter changes, and refresh reload it.
  Discard superseded requests.
- Preserve tag selection in history, sidebar/header consistency, and read-only
  browsing. Replace article-only Tags labels and empty-state copy where needed.
- Recoloring follows the replacement Annotation ID. Deleting the selected
  annotation selects its parent without clearing browsing context. Use the shared
  route handling covered by the existing annotation selection defect.

## Remaining work

- Depends on annotation tagging storage, indexing/recovery, and lifecycle support.
- Extend core/native tag list APIs and platform mappings to return mixed results;
  update list events, navigation controls, and existing capture fixtures. Coordinate
  replacement/deletion handling with the annotation selection defect.
- Add backend tests for mixed intersections, assignments split across entities,
  empty-selection parity, filters/counts, aliases, and unavailable/deleted tags.
  Verify query bounds and index use at the documented design scale.
- Add component/browser tests for passage and keyboard navigation, history,
  responsive access, membership stability, replacement/deletion, and stale results.
  Reuse capture fixtures, reset, and readiness. Run `npm run test:browser`, relevant
  Rust/frontend checks, and workspace formatting checks; report native/persistence
  behavior not verified by browser mocks.
- Update tag query, ListView, organization, User Guide, and training documentation.

## References

- [Storage prerequisite](enhancement__annotation-tagging--storage.md)
- [Indexing prerequisite](enhancement__annotation-tagging--indexing-and-recovery.md)
- [Lifecycle prerequisite](enhancement__annotation-tagging--lifecycle.md)
- [Annotation selection defect](defect__annotation-list--replacement-and-membership.md)
- [ListView contract](../../docs/architecture/list_view.md)
- [Tag query contract](../../docs/architecture/tags.md)
- [Design scale](../../docs/architecture/design_scale.md)
- [Index queries](../../crates/xaga-core/src/embeddings/db.rs)
- [Item list](../../src/components/list/EntityList.tsx)
- [Routing](../../src/hooks/useRouter.tsx)
