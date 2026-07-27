# Consolidate the recorded parallel implementations (pending — decisions per item)

The seven duplication findings from the decoupling audit, to be judged against `docs/principles/dry.md` (which also lists them as the principle's recorded instances):

1. `sync_entry_row` vs `sync_topic_row` — near-identical get → checksum → carry-forward → upsert; differ only in row builder and annotation reconcile (`crates/xaga-core/src/embeddings/pipeline.rs`).
2. `embed_entry_vector` vs `embed_topic_vector` vs `embed_feed` — the same checksum/vector gate + compute + carry-forward + upsert, three times (same file).
3. `bulk_insert_articles` vs `bulk_insert_topics` — identical batch loop; differ only in row builder + annotation rows + job variant (`crates/xaga-core/src/recovery/dispatch.rs`).
4. `from_article` / `from_topic` / `from_feed` — parallel `EmbeddingEntry` construction, same field set, same tags-from-booleans block repeated (`crates/xaga-core/src/embeddings/row.rs`).
5. `article_embed_text` / `topic_embed_text` / `feed_embed_text` — three near-identical one-liners (same file).
6. Article construction in three-plus places, inconsistently: `entry_article` (`persistence/entries.rs`), `article_from_fm` / `article_from_document` (`persistence/articles.rs`), and `save_article` building `Article`/`Entry` inline instead of using the helper.
7. `is_unread`/`is_bookmarked` derivation and the `"unread"`/`"bookmark"`/`"pinned"` string literals scattered across `entries.rs`, `articles.rs`, `row.rs`, `pipeline.rs`, `db.rs` — no shared constant or helper. Partly pre-existing.

Per `dry.md`: whether each is consolidated, parameterized, or kept-and-documented is a decision. Items 1–5 interact with `docs-transient/backlog/embeddings/spike__job_payload_modeling.md` (the per-kind split is their direct cause); item 7 interacts with `docs-transient/backlog/domain/spike__domain_tags_model.md`.
