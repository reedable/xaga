# Delete Tags

## Summary

Let users delete a tag with defined treatment of existing Entry assignments and
safe completion of any library-wide cleanup.

## Requirements

- Allow deletion of assigned tags and remove their assignments in bounded,
  resumable batches; do not require users to clear assignments first.
- Expose Delete in the same sidebar/responsive tag actions menu as Rename.
  Confirm once that the tag and its article assignments will be removed while
  articles remain intact. Cancel leaves the tag unchanged.
- Mark a tag as deleting during cleanup. Prevent new assignments and rename or
  merge operations involving it until deletion completes.
- If selected in tag browsing, remove the tag from the selected filter set but
  preserve the current list. Refresh membership on the next explicit navigation
  or filter change; if no tags remain selected, show all Entries under the
  existing filters on that refresh.
- During deletion, externally encountered labels for the deleting tag join
  pending cleanup rather than recreating its definition. After completion,
  reintroduced labels can create a new identity.
- Removing an assignment through Inspector chips does not delete the definition.
- Preserve article content, unrelated labels, reading/color state, and pins.
- Keep definitions, readable file assignments, and indexed IDs coherent during
  deletion; do not silently reinterpret stale assignments as a newly created tag.
- Perform any file/index cleanup in bounded batches with durable progress and
  safe retries, rather than one mass transaction.
- Use existing library write rights, writer serialization, and read-only gating.
- Increment the minor release version for deletion-state storage changes and
  document migration and recovery behavior.

## Remaining work

- Complete identity/creation and Entry assignment first. Coordinate with tag
  browsing and rename transitions wherever those stories are implemented.
- Coordinate deletion with in-progress rename/merge state so each operation's
  assignment and name-resolution guarantees remain intact.
- Implement delete controls, definition lifecycle, bounded cleanup as required,
  index updates, checkpoints, and recovery.
- Verify assigned/unassigned deletion, interrupted/repeated execution, concurrent
  mutations, external edits, read-only refusal, and preservation of other data.
- Run affected checks and update durable domain and storage documentation.

## References

- [Organization model](../../docs/architecture/organization_model.md)

- [Tag Identity and Creation](enhancement__tags--identity-and-creation.md) — prerequisite.
- [Entry Tag Assignment](enhancement__tags--entry-assignment.md) — prerequisite.
- [Rename Tags](enhancement__tags--rename.md)
- `crates/xaga-core/src/library_writer/mod.rs`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/design_scale.md`
