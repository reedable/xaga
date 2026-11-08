# Include annotation assignments in tag lifecycle operations

## Summary

Extend tag rename, merge, and deletion to annotation assignments using the existing
bounded, resumable conversion mechanisms.

## Requirements

- Preserve established operation rules: rename retains identity; merge uses the
  destination name and existing survivor-selection rule; deletion removes tags
  without deleting articles or annotations.
- Include article and annotation assignments in indexed cleanup. Resolve annotation
  candidates to parent files, reread current assignments, and serialize file patches
  through the parent Entry's writer lane. Preserve unrelated file data and indexed
  fields, including vectors.
- Retain reservations, aliases, conflict checks, bounded batches, durable pending
  work, and retries. Resume unfinished work before discovery and replay pending
  file work after cache loss. Concurrent assignment, comment edits, recoloring,
  and deletion must not lose changes or strand cleanup under replaced IDs.
- Bound work at the documented design scale, including many matching annotations
  in one file. Preserve existing stale-index consequences and transition retirement;
  add no full-source verification scan or permanent tombstones.
- Use shared lifecycle events for annotation assignment controls. Confirmations
  must cover article and annotation assignments; preserve cancellation, read-only
  restrictions, and conflict feedback.
- Persisted transition/checkpoint changes require a minor-version increment while
  major is 0. Include them in the annotation tagging upgrade and support unfinished
  transitions from the previous format.

## Remaining work

- Depends on annotation tagging storage and indexing/recovery.
- Extend shared candidate reads, source patches, checkpoints, cache conversion,
  discovery, and recovery. Update lifecycle confirmation copy.
- Add backend regressions for mixed assignments, shared parent files, both merge
  participants on one target, concurrent writes/replacement, partial failure,
  shutdown, pending-batch replay, and cache rebuilds. Retain article coverage and
  measure batching at the documented design scale.
- Extend component/browser coverage for confirmation and cancellation. Run
  `npm run test:browser`, relevant component/backend checks, and workspace formatting
  checks; report native/persistence behavior not verified by browser mocks.
- Update tag lifecycle contracts, User Guide, and training.

## References

- [Storage prerequisite](enhancement__annotation-tagging--storage.md)
- [Indexing prerequisite](enhancement__annotation-tagging--indexing-and-recovery.md)
- [Tag transitions](../../docs/architecture/tags.md)
- [Tag operation rules](../../docs/architecture/organization_model.md)
- [Design scale](../../docs/architecture/design_scale.md)
- [Rename implementation](../../crates/xaga-core/src/tags/rename.rs)
- [Merge/deletion conversion](../../crates/xaga-core/src/tags/conversion.rs)
- [Tag actions](../../src/components/TagActions.tsx)
