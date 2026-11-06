# Carry Intent End-to-End

**A command that knows what the user did passes that information to downstream consumers as typed data, with an appropriate priority. Consumers should not have to infer the action from a generic file-change event.**

## Why the action must be preserved

A command handler knows whether the user marked an article read, bookmarked it,
added an annotation, or changed Topic membership. A generic “a file changed” event
loses that distinction. A consumer would have to reread the file and infer the
change, adding work and delaying the visible result.

## Application in this app

- **Commands queue work after saving.** The Tauri command writes through core,
  then queues the typed embedding-service job itself (`state.enqueue(Tier::…,
  Job::…)` in `src-tauri/src/commands/`). Filesystem events do not replace this
  communication.
- **Jobs include information the command already has.** `SyncState` carries the
  saved values in a `StatePatch`, avoiding another file read. `AddMember` and
  `RemoveMember` carry the individual membership change instead of requiring a
  full comparison (`Job` in `crates/xaga-core/src/embeddings/`).
- **External changes use recovery.** Migrations and external editors such as
  Obsidian have no command handler reporting their changes. Recovery reconciles
  the saved files on its next run; these changes are not synchronized live.
