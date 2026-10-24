# Define Tag, Collection, Bookmark, Pin, Shortcut, and Site

## Summary

Tag, Collection, Bookmark, Pin, Shortcut, and Site do not have complete, mutually
consistent domain definitions. Persisted tags combine arbitrary labels with
reserved state markers; the domain exposes some markers as capabilities and
booleans; Shortcuts projects pinned entities alongside built-in navigation; and
Site is a derived hostname grouping that becomes durable in some sense if a pin
must survive after its Entries disappear. Define these concepts and their
relationships before selecting persistence for pinned Sites.

## Requirements

- Define each concept independently of its persistence, cache, command, and UI
  representations.
- Define Tag and the relationship between arbitrary tags and the reserved
  `unread`, `bookmark`, and `pinned` markers.
- Decide whether tags belong in the domain model and whether arbitrary persisted
  tags belong in the LanceDB mirror.
- Define Collection membership and identity and the relationship of Feed, Topic,
  and Site to Collection.
- Define Bookmark and its relationship to Entry, the `bookmark` tag, bookmark
  state, bookmark import/export, and the `Bookmarkable` capability.
- Define Pin as an action, state, or relationship; its relationship to the
  `pinned` marker and `Pinnable`; and whether it is intrinsic to a target or
  membership in Shortcuts.
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
- Compare Pin/Shortcut ownership models and their consequences for existing
  Feed/Topic pins, Site persistence, deletion, dangling targets, ordering,
  concurrency, migration, and external edits.
- Present definition options and their modeling consequences for decision.
- Record the chosen definitions in durable domain documentation and identify the
  implementation and migration changes they require.

## References

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
