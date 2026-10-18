# Define Tag, Collection, and Bookmark

## Summary

Tag, Collection, and Bookmark do not have complete domain definitions. Persisted tags combine reserved state markers with arbitrary labels, while the domain exposes only derived state booleans and LanceDB rows reconstruct only the reserved tags. Collection is a TypeScript abstraction but an implicit Rust shape. Bookmark names an action, tag-backed state, capability, and imported record without defining whether it is a domain object, state, or relationship.

## Requirements

- Define each concept independently of its persistence, cache, command, and UI representations.
- Define the relationship between arbitrary tags and the reserved `unread`, `bookmark`, and `pinned` state markers.
- Decide whether tags belong in the domain model and whether arbitrary persisted tags belong in the LanceDB mirror.
- Define Collection membership, identity, and the relationship of Feed and Topic to Collection.
- Define Bookmark and its relationship to Entry, the `bookmark` tag, bookmark state, and bookmark import/export.
- Keep the Rust and TypeScript domain models consistent with the definitions.

## Remaining work

- Trace each concept through the domain models, persistence formats, embedding rows, commands, and UI.
- Identify conflicting meanings and representation-specific assumptions.
- Determine what information is lost when domain objects and LanceDB rows retain only state derived from persisted tags.
- Present definition options and their modeling consequences for decision.
- Record the chosen definitions in durable domain documentation and identify the implementation changes they require.

## References

- `src/domain.ts`
- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/tags.rs`
- `crates/xaga-core/src/persistence/bookmarks.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `docs/conventions/vocabulary/`
