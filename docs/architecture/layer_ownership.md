# Layer Ownership

**Core implements logic and file operations. Tauri command handlers coordinate writes and queued work. The CLI handles command-line presentation and database management. Core owns shared library conversion.**

This document records the standing layer rule.

## The layers

- **`crates/xaga-core` — logic and file API.** Core’s file operations read and write entities without choosing jobs for the embedding queue. The command handler decides which downstream work to queue.
- **`src-tauri` command handlers — coordination.** Each handler saves the entity through core, then queues a typed job with the saved values and appropriate priority (see `docs/principles/carry_intent_end_to_end.md`).
- **`crates/xaga-cli` — presentation and database management.** The CLI does not own interactive CRUD. Its explicit upgrade command calls core’s shared converter; it also inspects and manages the database.

## What the rule decides

Place a capability according to its responsibility, regardless of which caller first needed it:

- Fetch/stage/verify logic belongs in core; progress printing and `--flag` surfaces belong in the CLI; write-then-enqueue sequencing belongs in the command layer.

## Sharing logic without moving presentation into core

Code that prints per-file progress or applies CLI-specific early exits mixes logic with presentation. Separate those responsibilities before sharing the logic through core. If sharing requires progress callbacks or operating modes, assess that added API explicitly.

## Library version ownership

Core owns `library.yaml`, its schema, parsing, serialization, migration-state
writes, content conversion and cache reconciliation. App and CLI supply their
runtime major/minor and hold the library lock while calling the shared converter.
Core reports typed progress and results; dialogs, browser launching and controlled
restart belong to the app, while terminal output belongs to the CLI. Conversion
is explicit and is never part of normal startup. See
[library version metadata](library_version.md).

## Internal implementation modules

- `src-tauri/src/lib.rs` composes the app and owns shared application state.
  `settings.rs` owns settings persistence and the shared account-lock registry;
  `menus.rs` owns menu construction and routing; `theme.rs` owns native theme
  helpers; `startup.rs` initializes resources; `shutdown.rs` coordinates service
  shutdown, writer draining, and termination.
- `commands/loader_capture.rs` owns loader callbacks, diagnostics, and image
  recovery. `commands/annotations.rs` coordinates annotation operations.
  `commands/articles.rs` retains article loading, saving, reading, and PDF access.
- `embeddings/db/` retains the `EmbeddingsDb` API. Private modules separate
  lifecycle, Arrow schema/encoding/decoding, queries, mutations, and similarity.
  This changes implementation organization, not cache ownership or write paths.
- `library_writer/operations.rs`, `submission.rs`, and `scheduler.rs` separate
  operation execution, submissions, and per-file dispatch. `mod.rs` retains the
  single writer, lock ownership, and private write-token constructor.
- `EntityList` owns selection, history, virtualization, and focus.
  `useEntityListData` owns loading and event reconciliation; `EntityListHeader`
  and `EntityListRow` render controls and entities.
- `TauriPlatform` owns native calls and subscriptions. `platform/native/payloads.ts`
  owns native schemas and domain conversion. The local native-adapter-boundary
  lint rule restricts imports to that directory, `TauriPlatform`, and its tests,
  and rejects re-exports from the adapter.
