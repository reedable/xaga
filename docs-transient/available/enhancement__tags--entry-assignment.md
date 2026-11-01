# Entry Tag Assignment

## Summary

Let users assign and remove zero or more tags on Entries/Articles, preserving
readable names in article files and stable IDs in LanceDB.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format.
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

## Remaining work

- Complete tag identity and creation first.
- Add the Entry tag picker to the shared Inspector and expose its Tags tab when
  assignment is available.
- Implement assignment commands, typed index/UI updates, and frontend state.
- Verify empty/multiple assignments, unrelated metadata preservation, concurrent
  changes, external edits/recovery, library isolation, and restart.
- Run affected Rust/frontend checks and production build; update durable docs.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Inspector](../../docs/architecture/article_inspector.md)
- [Tag Identity and Creation](../active/enhancement__tags--identity-and-creation.md) — prerequisite.
- [Annotation Inspector](../ideas/annotation-inspector.md)
- `crates/xaga-core/src/persistence/entries.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `src/components/content/Inspector.tsx`
- `src/platform/Platform.ts`
