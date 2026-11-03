# Delete Tags

## Summary

Let users delete a tag with defined treatment of existing Entry assignments and
bounded, resumable cleanup of indexed assignments.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format, with indexed cleanup replacing that
  document's current full-source deletion cleanup requirement.
- Allow deletion of assigned tags; do not require users to clear assignments
  first. Use indexed Entry IDs as file-cleanup candidates, plus persisted pending
  work. Do not require a full-source scan or verification pass.
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
  pending cleanup rather than recreating its definition. Interrupted deletion
  retains its transition and name reservation; recovery resumes unfinished work
  before discovering article labels.
- Complete deletion after indexed file/index batches and persisted pending work
  finish, then retire the definition and name reservation. A file missed by the
  index may retain its label; subsequent recovery or discovery can create a new
  tag identity from that label. Accept this stale-index consequence, as with
  rename; do not retain a tombstone or require proof that no source labels remain.
- Removing an assignment through Inspector chips does not delete the definition.
- Preserve article content, unrelated labels, reading/color state, and pins.
- Keep definitions, readable file assignments, and indexed IDs coherent during
  deletion; do not reinterpret labels reserved by an active deletion as a new tag.
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
  Cover recovery resumption while the transition is active and new-identity
  discovery from missed labels after retirement, including missing/stale caches.
- Run affected checks and update durable domain and storage documentation to
  replace the full-source deletion guarantee with the indexed completion and
  post-retirement discovery rules above.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Tag creation API](../../docs/architecture/tags.md#app-creation-api).
- [Entry assignment API](../../docs/architecture/tags.md#entry-assignment-api).
- [Runtime rename](../../docs/architecture/tags.md#runtime-rename)
- `crates/xaga-core/src/library_writer/mod.rs`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/design_scale.md`
