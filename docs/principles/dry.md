# DRY — Consolidate Parallel Implementations

**A piece of logic has one implementation. Parallel implementations of the same logic are consolidated; where consolidation is deferred, the duplication is recorded as debt with fact and location.**

This is standard engineering practice (Don't Repeat Yourself: every piece of knowledge has a single, authoritative representation). [Single Source of Truth](single_source_of_truth.md) specifies which copy of data is authoritative. DRY applies the same requirement to duplicated logic.

## Why

- Parallel implementations drift independently. A fix or a hardening measure applied to one copy does not constrain the others.
- Every security or correctness review must find and check all copies of duplicated logic.

## Current instances

Current parallel implementations awaiting decisions include:

- `sync_entry_row` / `sync_topic_row` — near-identical read → compare checksum → preserve existing fields → upsert (`crates/xaga-core/src/embeddings/pipeline.rs`);
- parallel checksum/vector work in `embed_entry_vector` / `embed_topic_vector` / `embed_feed`, with the caller checking whether the feed needs embedding (same file);
- identical batch loops in `bulk_insert_articles` / `bulk_insert_topics` (`crates/xaga-core/src/recovery/dispatch.rs`);
- parallel row builders `from_article` / `from_topic` / `from_feed` with the same field set (`crates/xaga-core/src/embeddings/row.rs`);
- three near-identical `*_embed_text` helpers;
- Article construction in three-plus places, inconsistently (`crates/xaga-core/src/md.rs`, `persistence/entries.rs`, `persistence/articles.rs`).

## When consolidation changes behavior

Consolidating implementations with different requirements may need parameters or modes to preserve those differences. Explain the alternatives and supporting evidence to the user before deciding whether to combine them, parameterize them, or retain and document the duplication.
