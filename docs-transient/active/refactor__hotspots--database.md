# Separate database implementation responsibilities

## Summary

Separate Arrow schema/encoding and decoding, database lifecycle, queries, mutations, and similarity search internally while retaining the `EmbeddingsDb` API.

## Requirements

- Split `embeddings/db.rs` internally by Arrow schema/encoding and decoding,
  database lifecycle, queries, mutations, and similarity search. Retain
  `EmbeddingsDb` and its existing consumer-facing API; do not introduce a new
  cache abstraction in this story. Preserve schemas, projections, predicates,
  ranking, cache freshness, index maintenance, and compaction ordering.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Move database implementation into internal modules grouped by the agreed
  responsibilities, updating internal imports and retaining existing public
  access paths. Run database regressions for row round-trips, projected reads,
  filtering/counts, state and membership mutations, similarity results, and
  read-handle visibility. Coordinate with the linked cache-abstraction story
  without implementing its proposed interface.
- Run `npm run lint`, `npm run typecheck`, and `npm run test:headless`.
  Add or adjust meaningful regression coverage where existing tests do not
  establish the preservation requirements; retain test strength when moving
  tests with their implementation.
- Update affected documentation and code references against the final module
  layout. Complete when the specified extractions, documentation updates, and
  required verification pass.

## References

- [Database implementation and regressions](../../crates/xaga-core/src/embeddings/db.rs)
- [Embedding exports](../../crates/xaga-core/src/embeddings/mod.rs)
- [Source of truth](../../docs/principles/single_source_of_truth.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Cache abstraction proposal](../backlog/refactor__db-as-cache-read-abstraction.md)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification workflow](../../docs/development/verification.md)
- [Supporting hotspot evidence](../ideas/refactor-hotspots-evidence.md)
