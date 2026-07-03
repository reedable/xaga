# Rust-layer verb consistency audit (pending)

## Current state

The operation verbs are split by layer in identifiers but overloaded in prose:

- `sync_*` (`sync_entry_row`/`sync_annotations`, `crates/xaga-core/src/embeddings/pipeline.rs`)
  names `.md` → LanceDB-row reconciliation.
- `reconcile_*` (`crates/xaga-core/src/recovery/`) names filesystem-level reconciliation — and
  `reconcile_topic_membership` (pipeline.rs) names the reverse-index delta pass.
- "Reconcile" also appears in prose describing the `sync_*` DB operation and in the CLI
  `recover` doc ("Reconcile LanceDB with the filesystem",
  `crates/xaga-cli/src/cmd/embeddings.rs`) — one word across three senses.

A recorded naming asymmetry in the same domain: `embed_entry_vector` / `embed_topic_vector`
but `embed_feed` (no `_vector`); `sync_entry_row` / `sync_topic_row` but no `sync_feed_row`
(feeds are single-phase). Inconsistent vocabulary for the same role.

## The work

Audit the verbs (sync / reconcile / recover / embed / upsert) against the controlled
vocabulary (`docs/vocabulary/`): one verb per concept; align prose to identifiers; resolve the
feed naming asymmetry. Renames are decisions — surface, don't apply unilaterally.
