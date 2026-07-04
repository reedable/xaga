# embed_feeds runs inline in recovery dispatch, per-feed (pending)

`embed_feeds` runs inline in `dispatch_recovery`, per feed: read+parse each `.atom`, then a
per-feed `get_entry`+`upsert`+ONNX (`crates/xaga-core/src/embeddings/pipeline.rs`,
`crates/xaga-core/src/recovery/dispatch.rs`). This blocks the recovery loop, scaled by feed
count. No fix direction recorded; unmeasured at the design target.
