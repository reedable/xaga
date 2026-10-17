# Read-model abstraction over the LanceDB cache

## Summary

Commands and queries call `EmbeddingsDb` directly even though durable files are authoritative and LanceDB is a derived read model. Decide whether a cache-oriented interface would clarify ownership or only add indirection.

## Requirements

- Preserve the distinction between authoritative persistence and derived reads.
- Define an interface around consumer needs rather than mirroring every LanceDB method.
- Do not introduce an unused alternate implementation solely to justify a trait.
- Account for freshness, unavailable-cache, and rebuilding states.

## Remaining work

- Inventory direct database consumers and group their required read shapes.
- Compare concrete application services, a trait boundary, and retaining direct access with clearer naming.
- Identify how projected and paginated reads would affect the proposed surface.
- Present the options and migration cost for decision.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/query/items.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands`
- `docs/principles/single_source_of_truth.md`
