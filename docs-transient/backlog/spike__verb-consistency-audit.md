# Define indexing and recovery verbs

## Summary

Indexing and recovery use `sync`, `reconcile`, `repair`, `recover`, `embed`, and `upsert` without defined boundaries. `sync_entry_row`, `sync_topic_row`, and `sync_annotations` update derived rows; `reconcile_topic_membership` applies a full membership diff; `repair_item_embeddings` scans for drift; recovery orchestrates the repairs; and CLI prose describes recovery as reconciliation. `embed_entry_vector` and `embed_topic_vector` also differ from `embed_feed`, while the `UpsertEntry` and `UpsertTopic` jobs perform more than a database upsert.

## Requirements

- Assign one operation concept to each verb and use that meaning consistently across layers.
- Distinguish drift detection, repair orchestration, row mutation, membership diffing, vector computation, and database upsert.
- Align function names, job variants, comments, logs, CLI commands, and CLI help.
- Add undefined cross-cutting verbs to the controlled vocabulary or replace them with existing terms.
- Preserve distinct names where operations have materially different behavior.

## Remaining work

- Inventory `sync`, `reconcile`, `repair`, `recover`, `embed`, and `upsert` in indexing and recovery identifiers and prose.
- Define the operation performed by each current function and job variant before proposing names.
- Present coherent rename sets, including the feed embedding asymmetry and compound `UpsertEntry` and `UpsertTopic` jobs, for decision.
- Apply the chosen vocabulary atomically to code, tests, logs, CLI text, and durable documentation.

## References

- `docs/conventions/vocabulary/semantic_vocabulary.md`
- `docs/conventions/vocabulary/conventional_vocabulary.md`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/recovery/`
- `crates/xaga-cli/src/cmd/embeddings.rs`
