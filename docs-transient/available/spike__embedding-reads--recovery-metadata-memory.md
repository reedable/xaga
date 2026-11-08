# Measure recovery embedding metadata memory

## Summary

Recovery's `list_entries_meta` reads and collects the full embeddings table before
building its metadata map, materializing every non-null vector even though
recovery needs only identity, checksum, reference, vector presence, kind, and
membership. Compare the memory and duration of the current query with metadata
projection through the multi-million-entry design target.

## Requirements

- Preserve recovery detection of missing, stale, pending-vector,
  annotation-drift, and orphaned rows.
- Preserve the rule that an incomplete filesystem walk cannot authorize deletion.
- Measure collection time and retained batch bytes, map-construction time, peak
  memory while batches and map overlap, and memory after batches are dropped.
- Use equivalent caches with populated vectors and representative tags and
  memberships. Run variants in separate processes with fixture creation outside
  the measured process; distinguish buffer sizes from process memory.
- Report measurements and limits without assuming an accepted memory budget or
  selecting a remediation design.

## Remaining work

- Build a benchmark comparing the current query with an explicit metadata
  projection, deriving vector presence without loading vector values if the query
  API supports it. Verify equivalent metadata and record projection limitations.
- Run increasing corpus sizes through the design target and record reproducible
  commands, fixture characteristics, timings, and memory measurements.
- Record findings and limits in durable documentation. Create bounded follow-up
  stories for further investigation or remediation where warranted, then delete
  this spike when its investigation and outcome recording are complete.

## References

- [Metadata query](../../crates/xaga-core/src/embeddings/db.rs)
- [Recovery](../../crates/xaga-core/src/recovery/embeddings.rs)
- [Design scale and measurements](../../docs/architecture/design_scale.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
