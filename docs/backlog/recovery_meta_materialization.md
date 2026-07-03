# all_entries_meta materializes the whole table on every recovery (pending)

`all_entries_meta` (`crates/xaga-core/src/embeddings/db.rs`) loads
id/checksum/reference/has_vector/kind for every row into a `HashMap` on each recovery run. At
the design target (`docs/architecture/design_scale.md`) that is a multi-million-entry map per
recovery. No fix direction recorded; unmeasured.
