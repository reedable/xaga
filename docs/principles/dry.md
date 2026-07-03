# DRY — Consolidate Parallel Implementations

**A piece of logic has one implementation. Parallel implementations of the same logic are
consolidated; where consolidation is deferred, the duplication is recorded as debt with fact and
location.**

This is standard engineering practice (Don't Repeat Yourself: every piece of knowledge has a
single, authoritative representation), adopted here explicitly. It is the code-side sibling of
[Single Source of Truth](single_source_of_truth.md): SoT governs where *data* is authoritative;
DRY governs where *logic* is.

## Why

- Parallel implementations drift independently. A fix or a hardening measure applied to one copy
  does not constrain the others — the duplicated embedding-model download is the recorded case:
  a verification mechanism implemented in one writer would not constrain what the app loads
  (`docs/backlog/refactor_embedding_model_download.md`).
- Duplication multiplies the audit surface: every security or correctness review must find and
  cover every copy.

## The recorded instances

An audit during the persistence↔embeddings decoupling recorded seven parallel-implementation
findings, kept for decisions rather than silently fixed. Among them:

- `sync_entry_row` / `sync_topic_row` — near-identical get → checksum compare → carry-forward →
  upsert (`crates/xaga-core/src/embeddings/pipeline.rs`);
- the same checksum/vector gate three times in `embed_entry_vector` / `embed_topic_vector` /
  `embed_feed` (same file);
- identical batch loops in `bulk_insert_articles` / `bulk_insert_topics`
  (`crates/xaga-core/src/recovery/dispatch.rs`);
- parallel row builders `from_article` / `from_topic` / `from_feed` with the same field set
  (`crates/xaga-core/src/embeddings/row.rs`);
- three near-identical `*_embed_text` helpers;
- Article construction in three-plus places, inconsistently
  (`crates/xaga-core/src/persistence/entries.rs`, `articles.rs`);
- state-tag string literals (`"unread"`/`"bookmark"`/`"pinned"`) scattered without a shared
  helper.

## Boundary

Consolidation is not free and is itself a design change: unifying two implementations with
genuinely different contracts moves the difference into parameters or modes (see the hypotheses
in `docs/backlog/refactor_embedding_model_download.md`). Whether a given duplication is
consolidated, parameterized, or kept-and-documented is a decision — recorded, with support,
decided by the user.
