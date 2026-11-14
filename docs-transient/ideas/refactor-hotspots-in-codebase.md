# Refactor hotspots in codebase

Findings recorded November 13, 2026. This idea identifies candidates for focused
refactoring; frequent changes alone do not establish poor design or defects.

## Evidence and scope

The initial review examined 599 commits from March 27 through November 13, 2026,
through `3b58105` (Responsive article drawer and centralized layout widths).
Change frequency counts commits touching each path, using `git log --numstat
--no-renames`. Counts exclude uncommitted changes and do not combine renamed
paths. Merge resolution and formatting changes can contribute to the counts.

Line counts below describe the working tree at review time, including pending
changes, comments, and blank lines. Rust test modules are distinguished where
they materially inflate file size. Sizes are approximate evidence, not limits
to enforce.

## Frequently changed areas

| File                                                 | Commit touches | Main reasons for changes                                                                                                              |
| ---------------------------------------------------- | -------------: | ------------------------------------------------------------------------------------------------------------------------------------- |
| `src/platform/TauriPlatform.ts`                      |             93 | Frontend implementation of backend capabilities: tags, pins, library upgrades, fetch settings, and read-only behavior.                |
| `src-tauri/src/lib.rs`                               |             89 | Startup, application state, native integration, and command registration.                                                             |
| `src/platform/Platform.ts`                           |             79 | Frontend contract evolves alongside backend capabilities.                                                                             |
| `src-tauri/src/commands/articles.rs`                 |             59 | Article loading and persistence, annotations, images, bookmarks, unread state, and topic membership.                                  |
| `crates/xaga-core/src/embeddings/db.rs`              |             50 | Entity metadata, tags, pins, unread state, query performance, and LanceDB compatibility as well as vector search.                     |
| `src/components/list/ItemList.tsx` (historical path) |             49 | Tags, bookmarks, shortcuts, site browsing, and layout; renamed to `EntityList.tsx` on November 4.                                     |
| `src/components/content/ArticleReader.tsx`           |             48 | Reader behavior, image handling, inspectors, reload feedback, and error handling.                                                     |
| `src/components/sidebar/Sidebar.tsx`                 |             47 | Tags, nested pin groups, sorting/filtering, scrolling, and available-height handling.                                                 |
| `crates/xaga-core/src/embeddings/pipeline.rs`        |             42 | Indexing changes accompanying domain-model and metadata changes.                                                                      |
| `crates/xaga-core/src/embeddings/service.rs`         |             41 | Embedding-service coordination and lifecycle changes.                                                                                 |
| `src/AppLayout.tsx`                                  |             33 | Resizable columns, drawers, library upgrade UI, and responsive reading layouts.                                                       |
| `crates/xaga-core/src/library_writer/mod.rs`         |             28 | Centralized writes, locking, crash consistency, resource lifecycle, and tag/pin mutations; 23 touches in the preceding 90-day window. |

Release mechanics dominate the unfiltered ranking: `package.json` has 145
touches, `Cargo.lock` 133, `src-tauri/Cargo.toml` 129,
`src-tauri/tauri.conf.json` 121, `crates/xaga-core/Cargo.toml` 118, and
`crates/xaga-cli/Cargo.toml` 114. Recent manifest history contains repeated
release-candidate version bumps. Treat these separately from implementation
hotspots.

The platform boundary has broad change exposure; storage changes carry data
consistency implications; navigation and reading views reflect continuing
product iteration.

## Modules that warrant splitting

| Module                                       | Observed size                                                   | Assessment and potential boundaries                                                                                                                                                                                            |
| -------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src-tauri/src/lib.rs`                       | 2,005 lines total; `run()` roughly 890 lines                    | Strongest concern: startup, shared state, settings persistence and account locking, menu/theme behavior, and shutdown coexist. Extract settings/account locking, menu/theme handling, and startup/shutdown orchestration.      |
| `src/components/list/EntityList.tsx`         | 1,215 lines                                                     | Strong frontend candidate: multiple entity sources, event reconciliation, navigation/selection, filtering, virtualization, and editing controls. Extract list-data coordination into hooks and distinct header/row components. |
| `crates/xaga-core/src/embeddings/db.rs`      | 3,633 lines total; roughly 2,220 before the main test module    | Mixes Arrow encoding/decoding, database lifecycle, metadata/tag queries, state mutations, topic membership, and similarity search. Separate these internally while retaining a database facade.                                |
| `crates/xaga-core/src/library_writer/mod.rs` | 2,593 lines total; roughly 1,920 before the bottom test modules | Central ownership is justified, but operation definitions, operation execution, public submission methods, and queue scheduling can live in separate files. Preserve one write authority and per-file serialization.           |
| `src/platform/TauriPlatform.ts`              | 1,980 lines                                                     | Oversized but relatively cohesive adapter. Schemas and decoding helpers occupy much of the opening 420 lines. Extract those first; assess domain-specific adapters afterward.                                                  |
| `src-tauri/src/commands/articles.rs`         | 1,340 lines total                                               | Remote-loader callbacks, HTML/favicon/image capture, PDF access, article operations, annotations, and image recovery exceed a single narrow responsibility. Loader callbacks and annotations are clear extraction candidates.  |

Moving embedded tests would improve navigation, but substantial production
modules would remain. Refactoring should reduce the responsibilities a reader
must understand together, rather than just reduce line counts.

`Platform.ts` (506 lines), `ArticleReader.tsx` (414), and `AppLayout.tsx` (450)
were not priorities based on size alone. Broad interfaces and cohesive
composition components can reasonably reach those sizes. Sidebar change
frequency also does not by itself justify another split; it already delegates
to several list components.

## Suggested approach

1. Start with `lib.rs` and `EntityList.tsx`, where orchestration and implementation
   details mix and many different features cause edits.
2. Separate database and writer internals along existing responsibilities,
   preserving their current ownership boundaries. Keep durable files
   authoritative and LanceDB a derived read model.
3. Extract platform schemas/decoders and separate loader callbacks and
   annotation commands when working in those areas.
4. Make focused, behavior-preserving changes with relevant existing regression
   checks. Review event ordering, selection/navigation behavior, startup and
   shutdown sequencing, write serialization, and read-only behavior where
   affected.

These are review findings and proposed boundaries, not a completed dependency
analysis or evidence that any particular split will reduce defects. Confirm
coupling and consumer needs before committing to new abstractions.

## Related work

- [Read-model abstraction over the LanceDB cache](../backlog/refactor__db-as-cache-read-abstraction.md)
  overlaps the database boundary discussion. Coordinate with that work rather
  than introducing a second abstraction independently.
