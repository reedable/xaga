# Define Tag, Collection, Bookmark, Pin, Shortcut, and Site

## Summary

Resolve the remaining Tag, Collection, Bookmark, Pin, Shortcut, and Site
definitions. Record domain definitions and
identify implementation and migration changes, including persistence for pinned
Sites.

## Requirements

- Define each concept independently of its persistence, cache, command, and UI
  representations.
- Define Tag identity, editing, and applicability across item types using the
  model in the linked idea; identify the required domain-model changes.
- Decide whether tags belong in the domain model and whether arbitrary persisted
  tags belong in the LanceDB mirror.
- Define Collection membership and identity and the relationship of Feed, Topic,
  and Site to Collection.
- Define Bookmark separately from Tags, including its applicability across
  entities and its relationship to bookmark state and import/export.
- Define Shortcut, including built-in navigation targets, pinned targets, typed
  target identity, ordering, and whether Shortcut has a durable domain
  representation or remains a UI projection.
- Define Site identity, existence, and lifecycle independently of its current
  LanceDB projection. State whether Site is an Item, Collection, navigation target,
  or another kind of value.
- Define whether persisted user intent can sustain a zero-member Site, how that
  differs from an unavailable Site read model, and what unpinning or hostname
  normalization changes do to that target.
- Keep the Rust and TypeScript domain models consistent with the definitions.

## Remaining work

- Trace each concept through the domain models, persistence formats, embedding
  rows, commands, navigation, and UI.
- Identify conflicting meanings and representation-specific assumptions,
  including the current Feed/Topic `isPinned` fields and the derived Site shape.
- Determine what information is lost when domain objects and LanceDB rows retain
  only state derived from persisted tags.
- Present options and modeling consequences for the unresolved definitions.
- Record the chosen definitions in durable domain documentation and identify the
  implementation and migration changes they require.

## References

- [Centralize Pin Storage](../available/refactor__pins--centralize-storage.md)
- [Tags and the Inspector](../ideas/tags-and-inspector.md)
- [Store Unread Outside Tags](../available/refactor__store-unread-outside-tags.md)
- [Article shortcut behavior](../../docs/architecture/list_view.md#article-shortcuts)
- `src/domain.ts`
- `src/types.ts`
- `src/Shortcuts.ts`
- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/site.rs`
- `crates/xaga-core/src/tags.rs`
- `crates/xaga-core/src/persistence/bookmarks.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `docs/architecture/sites.md`
- `docs/architecture/type_identity.md`
- `docs/principles/single_source_of_truth.md`
- `docs/conventions/vocabulary/`
