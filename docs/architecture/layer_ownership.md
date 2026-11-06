# Layer Ownership

**Core implements logic and file operations. Tauri command handlers coordinate writes and queued work. The CLI handles command-line presentation, migrations, and database management.**

This document records the standing layer rule.

## The layers

- **`crates/xaga-core` — logic and file API.** Core’s file operations read and write entities without choosing jobs for the embedding queue. The command handler decides which downstream work to queue.
- **`src-tauri` command handlers — coordination.** Each handler saves the entity through core, then queues a typed job with the saved values and appropriate priority (see `docs/principles/carry_intent_end_to_end.md`).
- **`crates/xaga-cli` — presentation, migration, database management.** The CLI does not own interactive CRUD. Its writes are bulk migrations reconciled by recovery; it also inspects and manages the database.

## What the rule decides

Place a capability according to its responsibility, regardless of which caller first needed it:

- Fetch/stage/verify logic belongs in core; progress printing and `--flag` surfaces belong in the CLI; write-then-enqueue sequencing belongs in the command layer.

## Sharing logic without moving presentation into core

Code that prints per-file progress or applies CLI-specific early exits mixes logic with presentation. Separate those responsibilities before sharing the logic through core. If sharing requires progress callbacks or operating modes, assess that added API explicitly.

## Library version ownership

Core owns `library.yaml`, its schema, parsing, serialization and migration-state
writes. Before 1.0, the CLI orchestrates library-format conversion through that
core API. At 1.0, responsibility for executing these migrations will move to core.
The app supplies its major/minor at runtime and selects read-only mode for a
library whose completed version does not match or cannot be established. See
[library version metadata](library_version.md).
