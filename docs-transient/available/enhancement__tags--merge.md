# Merge Tags

## Summary

Let users explicitly merge one tag into another when renaming to an existing
name, preserving Entry assignments while consolidating tag identity.

## Requirements

- Use the [Tag storage](../../docs/architecture/tags.md)
  contract and existing 0.14 format.
- Offer merge confirmation on a rename collision; never merge implicitly.
  Cancellation leaves both tags unchanged. Until this feature is implemented,
  rename collisions remain a hard stop.
- Use the destination display name regardless of population. Retain the ID with
  more indexed Entry assignments; on equal populations retain the destination ID.
  Persist this choice in transition state so retries do not select a new survivor.
  Preserve articles carrying either or both tags without duplicate assignments.
- Rewrite only losing-ID assignments in LanceDB. Update readable article labels
  that differ from the destination name, even when those articles already use
  the surviving ID in the index. ID survival does not eliminate file renames.
  Unlike rename, merge changes indexed identity and therefore requires bounded
  index updates as well as file writes.
- Persist progress and resume safely after interruption; resolve transitional
  names coherently until all assignment conversion and index work is complete.
- Reserve both names during merging and resolve them to the surviving ID,
  including during recovery. Externally encountered non-destination labels join
  pending file rewriting rather than creating another definition. New assignments
  use the destination name and surviving ID.
- Wait for existing transitions on either participating tag to finish before
  accepting the merge. During merging, block rename, deletion, and another merge
  involving either tag; unrelated tag operations may continue.
- Retire the losing definition and transitional name mapping only after file and
  index conversion complete. Afterward, the non-destination name may create a
  new tag identity when encountered again.
- Preserve content, unrelated tags, read/color state, pins, and browsing context.
  Use the library write right, serialized registry changes, and atomic file
  read–modify–writes inside Entry writer lanes. Preserve read-only behavior.

## Remaining work

- Complete tag identity/creation, Entry assignment, and resumable rename first.
- Connect collision confirmation and commands to stored merge transitions;
  implement bounded file/index conversion, recovery resumption, and finalization.
- Verify source-only, destination-only, and dual-tag assignments, cancellation,
  concurrent writes, external edits, interruption/retry, cache rebuild, and
  read-only behavior. Measure batch behavior at design scale.
- Run affected Rust/frontend checks and update durable domain/storage docs.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Tag Identity and Creation](enhancement__tags--identity-and-creation.md)
- [Entry Tag Assignment](enhancement__tags--entry-assignment.md)
- [Rename Tags](enhancement__tags--rename.md) — prerequisite.
- [Delete Tags](enhancement__tags--delete.md)
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/recovery/`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/design_scale.md`
