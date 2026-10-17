# Track per-Topic centroid of saved items

## Summary

Topic similarity currently uses the Topic's own title and description embedding. A centroid over saved members could represent the collection's contents, but its semantics, storage, and update model are undefined.

## Requirements

- Keep a Topic's authored-text embedding distinct from any member-derived representation.
- Define behavior for empty Topics, members without vectors, membership changes, member re-embedding, and deleted members.
- Avoid rebuilding an entire large Topic for every individual membership change unless measurements justify it.
- Preserve recoverability from durable Topic and item data.

## Remaining work

- Decide which search or recommendation behavior requires a member centroid.
- Compare stored running centroids, query-time aggregation, and separate derived rows.
- Define weighting across entries and annotations and whether source-article relationships participate.
- Measure update and recovery costs at large Topic sizes.
- Present the representation and consistency options for decision.

## References

- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `docs--DO-NOT-CITE/backlog/domain/spike__saved_vs_relevant_concept.md`
- `docs/architecture/design_scale.md`
