# Feed row dates emptied (pending — unapproved change awaiting decision)

`from_feed` (`crates/xaga-core/src/embeddings/row.rs`) sets `created_at`/`updated_at` to `""` to mimic older behavior, even though the domain `Feed` carries real values. Made during the decoupling work without approval. Decision: carry the real dates or ratify the empty strings.
