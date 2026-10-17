# Measure full-table metadata materialization during recovery

## Summary

Every recovery begins by calling `list_entries_meta` for the entire embeddings table. Recovery uses the result to detect missing or stale rows, pending vectors, annotation drift, and rows whose source files no longer exist.

`list_entries_meta` executes an unprojected table query and collects every returned Arrow batch before building a second representation: a `HashMap` containing each row's ID, checksum, reference, vector-presence flag, kind, and membership IDs. The collected batches therefore include columns recovery does not consume, including each non-null 384-element embedding vector. A vector contains 1,536 bytes of raw `f32` values, or approximately 1.54 GB per million non-null rows before Arrow, batch, map, string, and membership overhead. The batches remain alive while the map is constructed, so the two full-table representations overlap.

At the multi-million-row design target, determine the recovery duration and peak-memory cost of this materialization and whether a different read or diff shape is required. No fix direction is selected by this task.

## Requirements

- Measure `list_entries_meta` duration and process peak memory at representative design-target row counts and row-kind distributions.
- Separate the cost of reading and collecting Arrow batches from the cost of constructing and retaining the metadata map.
- Account for vector data, strings, and `items` membership lists in the measured row distribution.
- Preserve recovery's ability to detect:
  - absent, checksum-mismatched, or vector-pending rows;
  - annotation content and membership drift;
  - embedding rows whose source files no longer exist.
- Preserve the rule that an incomplete filesystem walk cannot authorize deletion of mirror rows.
- Compare the implementation and operational tradeoffs of:
  - projecting only the columns recovery consumes while retaining the full metadata map;
  - streaming projected database batches to reduce overlapping materialization;
  - changing the recovery diff shape so it does not retain metadata for the entire table at once;
  - retaining the current implementation.
- Present measurements and tradeoffs for a separate decision; do not select or implement an option in this task.

## Remaining work

- Define representative design-target counts for entries, annotations, topics, feeds, non-null vectors, and membership-list sizes.
- Instrument the full-table query, batch collection, map construction, and retained-map phases separately.
- Measure elapsed time and peak resident memory for each phase.
- Determine which metadata must be simultaneously available for source comparison and stale-row deletion.
- Report the options and tradeoffs, including memory bounds, database query cost, implementation complexity, and recovery correctness.

## References

- [Design Scale](../../../docs/architecture/design_scale.md)
- [Recovery diff](../../../crates/xaga-core/src/recovery/embeddings.rs)
- [Embeddings database](../../../crates/xaga-core/src/embeddings/db.rs)
- [Crash Consistency](../../../docs/architecture/crash_consistency.md)
