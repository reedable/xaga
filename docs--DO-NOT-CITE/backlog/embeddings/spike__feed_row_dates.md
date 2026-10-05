# Feed row dates

`from_feed` (`crates/xaga-core/src/embeddings/row.rs`) sets
`created_at`/`updated_at` to `""`, even though the domain `Feed` carries values.
Decide whether to carry the real dates or retain empty strings.
