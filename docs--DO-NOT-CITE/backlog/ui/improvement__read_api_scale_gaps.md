# Read API scale gaps

## Summary

List-all and collection reads materialize complete result sets. Feed reads build one `id IN (...)` predicate for the entire membership, and database index coverage does not match the application sort order. `Virtuoso` limits rendered DOM rows but not backend reads or frontend retention.

## Requirements

- Bound query size, transferred rows, and retained list state at the design target.
- Preserve deterministic ordering across pages when sort keys tie or rows change.
- Preserve filtering, collection membership, selection, keyboard navigation, and live-event updates.
- Avoid constructing unbounded SQL predicates for large ID sets.
- Align indexes with the selected server-side filter and sort plan.

## Remaining work

- Measure all-items, large-feed, and large-Topic reads at target sizes.
- Compare offset windows, stable cursors, and chunked membership queries.
- Define page invalidation and reconciliation for live state and annotation events.
- Select the sort key, tie-breaker, and supporting indexes.
- Implement the read API and incremental `ItemList` loading with scale and navigation tests.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/query/items.rs`
- `crates/xaga-core/src/query/topics.rs`
- `src/components/list/ItemList.tsx`
- `docs/architecture/design_scale.md`
- [Read-model abstraction](../embeddings/refactor__db_as_cache_read_abstraction.md)
