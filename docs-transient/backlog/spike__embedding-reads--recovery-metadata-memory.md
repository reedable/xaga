# Measure recovery embedding metadata memory

## Summary

Recovery's `list_entries_meta` reads and collects the full embeddings table before
building its metadata map, materializing every non-null vector even though
recovery needs only identity, checksum, reference, vector presence, kind, and
membership. Determine whether a projected collection and the metadata map can
overlap acceptably at the design target or whether recovery needs a different
diff shape.

## Requirements

- Preserve recovery detection of missing, stale, pending-vector,
  annotation-drift, and orphaned rows.
- Preserve the rule that an incomplete filesystem walk cannot authorize deletion.
- Measure batch collection separately from metadata-map construction and
  retention.
- Verify projections with query plans or measured bytes, memory, and latency.
- If projected collection is insufficient at the design target, compare streaming
  and alternative diff shapes without selecting one.

## Remaining work

- Add an explicit metadata projection that derives vector presence without
  materializing vector values, if supported by the query API.
- Measure representative recovery duration and peak memory at the design target.
- Determine whether projected batches and the metadata map may overlap within the
  accepted bound.
- If not, report alternative diff shapes and their tradeoffs for decision.

## References

- `crates/xaga-core/src/embeddings/db.rs`
- `crates/xaga-core/src/recovery/embeddings.rs`
- `docs/architecture/design_scale.md`
- `docs/architecture/crash_consistency.md`
