# Delete Tags

## Summary

Let users delete a tag with defined treatment of existing Entry assignments and
safe completion of any library-wide cleanup.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format.
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

## Remaining work

- Connect the selection updates required above to
  [tag browsing](../../docs/architecture/list_view.md#tag-browsing), and coordinate
  with rename transitions.
- Coordinate deletion with in-progress rename/merge state so each operation's
  assignment and name-resolution guarantees remain intact.
- Connect delete controls and commands to stored deletion transitions; implement
  bounded file/index cleanup, checkpoint advancement, and recovery resumption.
- Verify assigned/unassigned deletion, interrupted/repeated execution, concurrent
  mutations, external edits, read-only refusal, and preservation of other data.
- Run affected checks and update durable domain and storage documentation.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Tag creation API](../../docs/architecture/tags.md#app-creation-api).
- [Entry assignment API](../../docs/architecture/tags.md#entry-assignment-api).
- [Rename Tags](enhancement__tags--rename.md)
- `crates/xaga-core/src/library_writer/mod.rs`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/design_scale.md`
