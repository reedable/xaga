# Layer Ownership

**xaga-core is pure logic. Orchestration lives in the command layer. The CLI is presentation, data migration, and database management.**

This document records the standing layer rule.

## The layers

- **`crates/xaga-core` — pure logic / file API.** Core reads and writes entity files and knows nothing about the queue or the embedding service. Deciding _what work to enqueue_ never lives in core.
- **`src-tauri` command layer — orchestration.** Each command writes the entity via the pure core API, then enqueues a typed, correctly-prioritized job itself — carrying intent end-to-end (see `docs/principles/carry_intent_end_to_end.md`).
- **`crates/xaga-cli` — presentation, migration, database management.** The CLI sheds interactive CRUD; its remaining writes are bulk migrations (reconciled by recovery), alongside database observation and management.

## What the rule decides

A capability's home is determined by what it is, not by who first needed it:

- Fetch/stage/verify logic belongs in core; progress printing and `--flag` surfaces belong in the CLI; write-then-enqueue sequencing belongs in the command layer.

## Boundary tension to weigh at design time

Pulling presentation-interleaved logic (per-file progress, mid-loop exits) up into core would move presentation concerns _into_ the logic crate — a violation in the other direction. Where a shared implementation needs progress callbacks or modes, that cost is weighed explicitly.
