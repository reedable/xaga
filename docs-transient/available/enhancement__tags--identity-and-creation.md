# Tag Identity and Creation

## Summary

Introduce library-scoped tag definitions with stable identities and editable
human-readable names, and let users create tags for Entry organization.

## Requirements

- Article frontmatter stores readable tag names. The persistence/index boundary
  resolves names to stable tag IDs; LanceDB assignments store those IDs.
- Store library tag definitions in YAML as a root `tags` sequence, with `id`
  and display `name` fields for each definition.
- Generate tag IDs as UUID v4 strings. Keep the ID unchanged across renames;
  do not derive it from the tag's name.
- Renaming a tag must not require rewriting its LanceDB assignments. Do not copy
  display names into indexed assignments as a second identity.
- Compare names with Unicode-aware case-insensitive comparison after trimming
  surrounding whitespace. Preserve
  creation spelling as the display name; later assignments with different casing
  do not change it. Explicit renames, including case-only renames, can change it.
- Reject names empty after whitespace trimming. Punctuation-only names such as
  `---` are valid.
- Enforce one stable ID per normalized name with atomic lookup/creation, including
  concurrent creation and discovery. Repeated file labels map to that identity.
- During writable recovery, create definitions for unknown article tag names
  before indexing their assignments. Do not write definitions in read-only mode.
- For a previously unknown name, use the spelling first discovered as its display
  name; do not reorder discovery solely to choose casing. Later variants reuse
  the same definition without changing its display name.
- Preserve zero-or-more tags per Entry, existing organizational labels, and
  library isolation. Keep non-Entry tagging outside this release.
- Provide authoritative, durable tag definitions and serialized atomic mutations
  under the library write right. Preserve read-only behavior.
- Provide the app-facing creation operation and carry committed changes to
  consumers. The complete creation/assignment picker UI belongs to Entry
  assignment, not this story.
- Increment the minor release version for the new storage model and document
  the format and any required CLI upgrade.

## Remaining work

- Complete central pin storage and color-backed bookmarks before repurposing
  LanceDB tags for general-purpose identities.
- Implement UUID v4 generation and Unicode-aware name comparison.
- Implement core definitions, writer operations, platform creation API, and
  recovery discovery; connect the picker in the Entry-assignment story.
- Establish the name-resolution contract needed by resumable rename: old and new
  names can resolve to one identity while file rewriting is incomplete.
- Preserve arbitrary file tags through domain conversion; initialize definitions
  for existing labels and document reconstruction of indexed IDs from files and
  definitions.
- Verify creation, duplicates, external edits, restart, concurrent writes,
  read-only access, library isolation, and migration/recovery at design scale.
- Run affected Rust/frontend checks and update durable domain/storage docs.

## References

- [Organization model](../../docs/architecture/organization_model.md)

- [Centralize Pin Storage](refactor__pins--centralize-storage.md) — prerequisite.
- [Bookmark Articles with Colors](enhancement__bookmark-state--color-markings.md) — prerequisite.
- [Entry Tag Assignment](enhancement__tags--entry-assignment.md)
- [Rename Tags](enhancement__tags--rename.md)
- `crates/xaga-core/src/md.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src/domain.ts`
- `src/platform/Platform.ts`
- `docs/architecture/design_scale.md`
