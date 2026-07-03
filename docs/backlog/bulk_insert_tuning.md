# bulk_insert: sequential reads, unsurfaced batch size (pending)

`bulk_insert` (`crates/xaga-core/src/embeddings/service.rs`) reads files sequentially (awaits
each read) and uses a fixed batch size of 1024 — both chosen without being surfaced as
decisions. At the design target (`docs/architecture/design_scale.md`) a fresh recovery
bulk-inserts the whole corpus through this path. Whether sequential reads or the batch size is
a bottleneck has not been measured.
