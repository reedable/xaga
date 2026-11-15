# Determine whether recovery should construct feed counts

## Summary

Each saved `.atom` file contains the feed's cumulative entry membership and is allowed to grow indefinitely. During recovery, Xaga only needs feed-level metadata to detect missing or stale feed embedding rows, but it obtains that metadata through the general-purpose `list_feeds` path.

Recovery passes empty unread and bookmarked ID sets to `list_feeds`. Nevertheless, its `atom_to_feed` conversion normalizes and hashes the URL of every cumulative entry, collects the resulting IDs, and scans them against both empty sets. The resulting unread count is always zero and the computed bookmarked count is unused. The suspected recovery cost therefore grows with the total historical entries across all saved feeds, not only with the number of feeds; whether that cost is material at the design target has not been measured.

Determine whether recovery should avoid this entry-level work or whether retaining the shared conversion is preferable. No direction is selected by this task.

## Requirements

- Establish the cost of the entry-ID conversion and count scans at the design-target feed and entry counts.
- Preserve the feed fields used by recovery: ID, title, description, pin state, and the metadata required to construct a feed embedding row.
- Preserve one read and parse of each `.atom` file during recovery.
- Compare the implementation and maintenance costs of:
  - a recovery-specific feed-listing or conversion path that omits entry-derived counts;
  - splitting or parameterizing the shared conversion so callers request only the data they need;
  - retaining the current shared conversion and accepting the extra work.
- Present the measured cost, boundary implications, and drift risk of each option for a separate decision; do not select or implement one in this task.

## Remaining work

- Establish representative design-target counts for feeds and entries per `.atom` file.
- Measure recovery feed conversion with the current empty unread and bookmarked sets.
- Identify the minimum feed representation required by recovery and `embed_feed`.
- Report the options and tradeoffs, including duplicated conversion logic, API complexity, and the cost of leaving the behavior unchanged.

## References

- [Feed recovery diff](../../../crates/xaga-core/src/recovery/embeddings.rs)
- [Feed persistence](../../../crates/xaga-core/src/persistence/feeds.rs)
- [Atom-to-feed conversion](../../../crates/xaga-core/src/atom.rs)
- [Feed embedding pipeline](../../../crates/xaga-core/src/embeddings/pipeline.rs)
- [Feed domain type](../../../crates/xaga-core/src/domain.rs)
