# Stale code comments referencing removed names (pending)

Doc comments referencing names that no longer exist, recorded during the decoupling audit
(line numbers as of the audit; re-locate before fixing):

- `crates/xaga-core/src/embeddings/service.rs` — three comments referencing
  `EmbedVector`/`UpsertContent` (the pre-split job names; the enum now has
  `EmbedEntry`/`UpsertEntry` etc.).
- `crates/xaga-core/src/embeddings/pipeline.rs` — a comment referencing `sync_row`.
- `crates/xaga-core/src/persistence/topics.rs` — a comment referencing `sync_row` and the
  retired "watcher".

Per the ratified-design rule (`docs/principles/decision_ownership.md`, checklist 10), comments
describing a changed design are the same class of divergence as stale ratifying docs.
