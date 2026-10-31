# Entry Tag Assignment

## Summary

Let users assign and remove zero or more tags on Entries/Articles, preserving
readable names in article files and stable IDs in LanceDB.

## Requirements

- Support selecting existing tags and removing assignments without deleting the
  tag definition or changing other article metadata.
- Own the complete Entry tag picker, including creation of a tag through the
  identity/creation story's API and immediate assignment in the same workflow.
- Place the picker in the article Inspector's Tags view. Present a searchable
  set of selectable MUI chips, with clearly distinguishable assigned and
  unassigned states, rather than a checkbox list resembling Topics.
- Clicking an unassigned chip assigns the tag immediately; clicking an assigned
  chip removes that assignment immediately while retaining the tag definition.
  Provide keyboard operation and accessible selected-state indication.
- When the entered name has no matching definition, offer Create “name”;
  selecting it creates the definition and assigns it to the current Entry.
- Do not require Save or confirmation when closing the Inspector; mutations
  are committed through the assignment actions.
- Store current readable tag names in article frontmatter and translate to IDs
  at the index boundary using library tag definitions.
- Serialize file mutations through the Entry writer lane, then carry typed tag
  intent to indexing and UI consumers. Support cache rebuild from files and
  definitions, concurrent article changes, and read-only refusal.
- Keep Feed, Topic, Site, Note, and annotation tagging outside this story.
- Increment the minor release version for storage-model changes introduced by
  this story and document any corresponding CLI upgrade.

## Remaining work

- Complete tag identity and creation first.
- Provide the article Inspector's Tags view before connecting the picker; the
  article Inspector story is a UI prerequisite.
- Implement preservation of names through core domain boundaries, assignment
  commands, name-to-ID indexing, events, and frontend state updates.
- Verify empty/multiple assignments, unrelated metadata preservation, concurrent
  changes, external edits/recovery, library isolation, and restart.
- Run affected Rust/frontend checks and production build; update durable docs.

## References

- [Organization model](../../docs/architecture/organization_model.md)

- [Article Inspector](../active/enhancement__inspector--article-panel-and-drawer.md) — UI prerequisite.
- [Tag Identity and Creation](enhancement__tags--identity-and-creation.md) — prerequisite.
- [Annotation Inspector](../ideas/annotation-inspector.md)
- `crates/xaga-core/src/persistence/entries.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `src/components/content/Toolbar.tsx`
- `src/platform/Platform.ts`
