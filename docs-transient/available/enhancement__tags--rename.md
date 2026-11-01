# Rename Tags

## Summary

Rename a tag while keeping article files readable, preserving tag identity and
avoiding mass rewrites of LanceDB assignments.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format.
- Keep the stable tag ID unchanged and update its displayed name.
- Expose Rename in a tag-row actions menu in sidebar tag navigation. Show its
  actions button on hover and keyboard focus, with Tab access and Enter/Space
  activation. Right-click may open the same menu as an additional route.
- Expose the same actions button and menu in responsive tag navigation when the
  sidebar is hidden. Inspector chips remain assignment/removal controls.
- Open a dialog prefilled with the current name. Enter or Rename submits;
  Escape or Cancel leaves it unchanged. Show name collisions inline. Restore
  focus to the invoking control on dismissal.
- While a tag's file transition runs, show Renaming… and disable another Rename
  for that tag without blocking unrelated tags.
- A name owned by another tag is a hard stop until the separate merge story is
  implemented. Case-only renames of the same tag remain allowed.
- Sequence renames of the same tag: finish its file transition before accepting
  another rename. Renames of different tags may proceed concurrently with
  serialized registry changes and bounded work.
- Rewrite affected article frontmatter names in bounded batches rather than
  one library-wide transaction. Preserve content, unrelated tags, and metadata.
- During rewriting, resolve both old and new names to the same ID so mixed file
  states remain queryable and recovery does not create a second tag identity.
- Persist progress and resume safely after interruption. Retire the old-name
  transition only when rewriting has completed; reruns must not corrupt state.
- On recovery, load persisted rename transitions before discovering article
  tags. Resolve old and new names to the existing tag ID. Files discovered with
  the old name join pending rewrite work rather than creating another definition.
  Resume bounded rewrites and retire the old-name mapping only after completion.
- Protect concurrent assignments and article writes through existing writer
  serialization and library write rights. Preserve read-only behavior.
- Run each file's entire read–modify–write inside its existing Entry writer lane
  and replace the file atomically. Concurrent tag operations must not overwrite
  one another's changes.
- Once a rename is complete and its old-name mapping is retired, an externally
  introduced old name is a newly encountered label, not an alias of this tag.
- Do not rewrite unchanged LanceDB tag assignments solely for a label rename.

## Remaining work

- Complete identity/creation and Entry assignment first; coordinate displayed
  name updates with tag browsing when that story is implemented.
- Implement the shared rename dialog, sidebar/responsive actions menus, and
  per-tag transition indication.
- Connect commands to stored rename transitions; implement bounded dispatch,
  checkpoint advancement, recovery resumption, and finalization.
- Define and measure batch/resource limits at design scale.
- Verify interrupted/repeated execution, mixed names, external edits, concurrency,
  collisions, restart, and unchanged indexed IDs.
- Run affected checks and update durable format and operational documentation.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Tag Identity and Creation](../active/enhancement__tags--identity-and-creation.md) — prerequisite.
- [Entry Tag Assignment](enhancement__tags--entry-assignment.md) — prerequisite.
- [Merge Tags](enhancement__tags--merge.md) — adds explicit collision merging
  after rename; until then collisions remain a hard stop.
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/recovery/`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/design_scale.md`
