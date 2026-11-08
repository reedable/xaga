# Index and recover annotation tag assignments

## Summary

Index saved annotation tags and rebuild assignments from library files without
re-embedding unchanged content.

## Requirements

- Resolve each annotation's readable labels through the shared registry into UUIDs
  on its own LanceDB row. Files own assignments; the registry owns identities.
- Commands enqueue committed typed assignment changes at Interactive priority.
  Failed source writes must not change the index. Patch only tags, preserving
  content, vectors, color, Topic membership, and parent reference.
- Synchronization, deferred embedding, replacement, and delayed article snapshots
  must preserve newer saved assignments. Stale assignment jobs must not recreate
  deleted or replaced annotations. Use existing synchronization/recovery paths
  for missing rows.
- Recovery discovers labels through the registry writer and honors active tag
  transitions. Compare resolved assignments independently of content checksums;
  repair tag-only differences without re-embedding. Cover incremental recovery,
  fresh-cache insertion, and external file edits; retain read-only restrictions.
- Reuse registry resolution and assignment ordering protections. Keep readable
  labels distinct from indexed UUIDs and retain the existing index producers.

## Remaining work

- Depends on annotation tagging storage and its format upgrade.
- Extend annotation row builders, assignment jobs, synchronization, and deferred
  embedding handling. Extend recovery discovery, assignment comparison, and bulk
  insertion paths.
- Add regressions for assignment/removal before row creation, stale snapshots,
  vector completion, deletion/replacement races, external edits, unknown labels,
  active transitions, read-only recovery, and cache rebuilds. Verify tag-only
  repair leaves vectors unchanged. Run relevant Rust checks and workspace
  formatting checks.
- Update tag indexing, recovery, and crash-consistency contracts.

## References

- [Storage prerequisite](enhancement__annotation-tagging--storage.md)
- [Tag storage and indexing](../../docs/architecture/tags.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Annotation indexing](../../crates/xaga-core/src/embeddings/pipeline.rs)
- [Index service](../../crates/xaga-core/src/embeddings/service.rs)
- [Recovery diff](../../crates/xaga-core/src/recovery/embeddings.rs)
- [Recovery dispatch](../../crates/xaga-core/src/recovery/dispatch.rs)
