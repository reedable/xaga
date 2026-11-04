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
- Use indexed Entry assignments for either participating ID as file-cleanup
  candidates, plus persisted pending work. Do not require a full-source scan or
  verification pass.
- Persist progress and resume safely after interruption; resolve transitional
  names coherently until indexed conversion and pending work complete. Recovery
  resumes unfinished merge batches before discovering article labels, retaining
  both name reservations and the persisted survivor choice during interruption.
- Reserve both names during merging and resolve them to the surviving ID,
  including during recovery. Externally encountered non-destination labels join
  pending file rewriting rather than creating another definition. New assignments
  use the destination name and surviving ID.
- Wait for existing transitions on either participating tag to finish before
  accepting the merge. During merging, block rename, deletion, and another merge
  involving either tag; unrelated tag operations may continue.
- Retire the losing definition and transitional name mapping after indexed file
  and index conversion and persisted pending work complete. A file missed by the
  index may retain its non-destination label; subsequent recovery or discovery
  can create a new tag identity from it. Accept this stale-index consequence, as
  with rename and deletion; do not retain a tombstone or require proof that no
  non-destination source labels remain.
- Preserve content, unrelated tags, read/color state, pins, and browsing context.
  Use the library write right, serialized registry changes, and atomic file
  read–modify–writes inside Entry writer lanes. Preserve read-only behavior.

## Remaining work

- Connect collision confirmation and commands to stored merge transitions;
  implement bounded file/index conversion, recovery resumption, and finalization.
- Verify source-only, destination-only, and dual-tag assignments, cancellation,
  concurrent writes, external edits, interruption/retry, cache rebuild, and
  read-only behavior. Cover interrupted merge resumption while reservations are
  active and new-identity discovery from missed labels after retirement,
  including missing/stale caches. Measure batch behavior at design scale.
- Run affected Rust/frontend checks and update durable domain/storage docs.

## References

- [Tag storage](../../docs/architecture/tags.md).
- [Organization model](../../docs/architecture/organization_model.md)

- [Tag creation API](../../docs/architecture/tags.md#app-creation-api)
- [Entry assignment API](../../docs/architecture/tags.md#entry-assignment-api)
- [Runtime rename](../../docs/architecture/tags.md#runtime-rename).
- [Runtime deletion](../../docs/architecture/tags.md#runtime-deletion)
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/recovery/`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/design_scale.md`
