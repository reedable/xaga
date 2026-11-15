# Refactor hotspots in codebase

## Summary

Separate existing responsibilities in application startup, EntityList, database
and writer internals, native payload decoding, and article commands. Preserve
behavior and existing consumer-facing APIs while making each responsibility
independently navigable and enforcing the native adapter's import boundary.

## Requirements

- Extract settings persistence and account locking into one internal module.
  Preserve the single shared account-lock registry, operation-scoped leases,
  locked read-modify-write, stale-view checks, and lock transfer during settings
  file replacement. Preserve settings serialization, defaults, and live updates.
- Extract native menu construction and menu-event handling together into one
  internal module, and theme application helpers into a separate internal
  module. Preserve menu IDs, enabled states, checkmarks, event routing, theme
  selection, system-theme behavior, and window initialization. Menu actions that
  change the theme must use the shared theme implementation.
- Extract startup initialization and shutdown/restart coordination into separate
  internal modules. Keep `run()` as the composition point and pass required
  shared state explicitly. Preserve proxy setup before outbound requests,
  settings initialization, library upgrade and read-only gates, service startup,
  and shutdown/restart ordering, including writer draining and lock release
  before a successor process starts.
- Extract `EntityList.tsx` data loading and event reconciliation together into
  a hook, and header controls and row presentation into separate components.
  Keep selection, history, virtualization, and keyboard/focus coordination owned
  by `EntityList`. The hook must preserve cancellation, request-version guards,
  subscription cleanup, and reconciliation of state events with in-flight reads.
  Preserve list membership and reading context under the ListView contract,
  filters, sorting, responsive controls, accessibility, and read-only behavior.
- Split `embeddings/db.rs` internally by Arrow schema/encoding and decoding,
  database lifecycle, queries, mutations, and similarity search. Retain
  `EmbeddingsDb` and its existing consumer-facing API; do not introduce a new
  cache abstraction in this story. Preserve schemas, projections, predicates,
  ranking, cache freshness, index maintenance, and compaction ordering.
- Split library-writer operation definitions/execution, submission methods, and
  queue scheduling into internal modules. Preserve one `LibraryWriter`, one
  lock owner, per-file lanes, priorities, the concurrency cap, responder outcomes,
  and shutdown draining/refusal rules. Keep write-token construction restricted
  to the writer; moving code must not permit callers to bypass serialization.
- Extract native payload schemas and decoding helpers from `TauriPlatform.ts`
  into an internal adapter directory. Keep native calls, subscriptions, and the
  `Platform` implementation in `TauriPlatform`. Preserve runtime validation,
  domain conversion, current error behavior, and the existing Platform API.
- Enforce schema/decoder visibility with a local Oxlint rule enabled as an error
  in `npm run lint`. Resolve imports to the internal directory and allow access
  only from the native adapter and its tests. Reject outside imports, including
  type-only and dynamic imports, and re-exports exposing these internals through
  another module. UI layers and their tests must use domain types and Platform
  contracts.
- Split `commands/articles.rs` by moving loader callbacks and resource capture
  into one command module and annotation operations into another. Keep article
  loading, saving, reading, and PDF access in `articles.rs`. Preserve command
  names, payloads, IPC permissions, loader identity checks, validation, cleanup,
  diagnostics, typed notifications, and save-then-enqueue ordering.
- Preserve existing behavior, layer ownership, write authority, and serialization.
  Keep symbol visibility limited to verified consumers.
- Review all affected documentation and update architecture, conventions,
  development, User Guide, and training documents as applicable before
  completion, including moved code references.

## Remaining work

- Move the settings persistence and account-lock implementation out of
  `src-tauri/src/lib.rs`, update its consumers, and retain regression coverage
  for settings writes and model-download leases sharing the account lock,
  including successful writes, contention, stale views, replacement failures,
  and lease release.
- Move menu construction and event handling out of `src-tauri/src/lib.rs`,
  retaining the existing specialized menu modules as the owners of their logic.
  Move theme helpers into their own module and update startup and settings
  consumers to use it. Verify menu actions, checkmark synchronization, and theme
  application to existing and newly created windows.
- Move startup initialization and shutdown/restart helpers to their respective
  modules and connect them through `run()`. Retain coverage for startup gates,
  read-only selection, service lifecycle, writer draining, and lock release;
  verify native quit and restart paths.
- Extract the list data hook and header/row components, updating
  `EntityList.tsx` to compose them. Retain component coverage for stale reads,
  event reconciliation, and subscription cleanup. Run browser workflows for
  filters, history, keyboard navigation, virtualized focus return, responsive
  collection selection, and preserved reading context.
- Move database implementation into internal modules grouped by the agreed
  responsibilities, updating internal imports and retaining existing public
  access paths. Run database regressions for row round-trips, projected reads,
  filtering/counts, state and membership mutations, similarity results, and
  read-handle visibility. Coordinate with the linked cache-abstraction story
  without implementing its proposed interface.
- Extract writer operations, submission methods, and scheduling while retaining
  existing consumer access paths. Run writer regressions for same-file
  serialization, different-file concurrency, shared tag/pin lanes, resource
  replacement, submit/shutdown races, and committed Interactive writes surviving
  shutdown.
- Extract adapter schemas and decoders and update the native adapter imports.
  Retain adapter regressions for accepted/rejected payloads, domain kinds,
  event conversion, and errors. Add the import-boundary rule through the
  existing local lint plugin with positive and negative fixtures covering
  allowed adapter/test access and forbidden direct, type-only, dynamic, and
  re-export access; verify the rule runs in the existing release lint check.
- Update command registration and internal callers for the article-command
  extractions. Verify loader callbacks still pass existing identity and IPC
  checks, capture failures preserve content and clean up resources, and
  annotation operations retain saved content, indexing, and event outcomes.
- Run `npm run lint`, `npm run typecheck`, `npm run test:headless`, and
  `npm run test:browser`. Add or adjust meaningful regression coverage where
  existing tests do not establish the preservation requirements; retain test
  strength when moving tests with their implementation.
- Verify native menu/theme behavior, startup/read-only/upgrade gates, quit and
  restart, loader/resource capture, and annotation operations. Use the native
  verification procedures linked below and report any behavior they cannot
  establish. Browser mocks alone cannot verify IPC, disk persistence, native
  windows/menus, or process lock release.
- Review and update affected documentation and code links against the final
  module layout. Complete when all specified extractions, import-boundary
  enforcement, documentation updates, and required verification pass.

## References

- [Browser integration tests](../../docs/development/browser_tests.md)
- [Resource persistence verification](../../docs/development/resource_verification.md)
- [Library upgrade verification](../../docs/development/library_upgrade_verification.md)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Command surface](../../docs/architecture/security/command_surface.md)
- [Local lint plugin](../../lint-rules/index.js)
- [Frontend lint configuration](../../.oxlintrc.json)
- [Release checks](../../.github/workflows/release.yml)
- [Decision ownership](../../docs/principles/decision_ownership.md)
- [Least visibility](../../docs/principles/least_visibility_principle.md)
- [Single source of truth](../../docs/principles/single_source_of_truth.md)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Single-writer library lock](../../docs/architecture/single_writer_library_lock.md)
- [Single-writer account lock](../../docs/architecture/single_writer_account_lock.md)
- [ListView behavior](../../docs/architecture/list_view.md)
- [UI testing](../../docs/conventions/ui_testing.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [Documentation conventions](../../docs/conventions/documentation.md)
- [Verification and readiness review](../../docs/development/verification.md)
- [Read-model abstraction over the LanceDB cache](../backlog/refactor__db-as-cache-read-abstraction.md)

### Evidence and scope

The initial review examined 599 commits from March 27 through November 13, 2026,
through `3b58105` (Responsive article drawer and centralized layout widths).
Change frequency counts commits touching each path, using `git log --numstat
--no-renames`. Counts exclude uncommitted changes and do not combine renamed
paths. Merge resolution and formatting changes can contribute to the counts.

Line counts below describe the working tree at review time, including pending
changes, comments, and blank lines. Rust test modules are distinguished where
they materially inflate file size. Sizes are approximate evidence, not limits
to enforce.

### Frequently changed areas

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

### Candidate module boundaries

| Module                                       | Observed size                                                   | Assessment and potential boundaries                                                                                                                                                                                             |
| -------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/lib.rs`                       | 2,005 lines total; `run()` roughly 890 lines                    | Candidate: startup, shared state, settings persistence and account locking, menu/theme behavior, and shutdown coexist. Possible extractions: settings/account locking, menu/theme handling, and startup/shutdown orchestration. |
| `src/components/list/EntityList.tsx`         | 1,215 lines                                                     | Candidate: multiple entity sources, event reconciliation, navigation/selection, filtering, virtualization, and editing controls. Possible extractions: list-data coordination hooks and distinct header/row components.         |
| `crates/xaga-core/src/embeddings/db.rs`      | 3,633 lines total; roughly 2,220 before the main test module    | Mixes Arrow encoding/decoding, database lifecycle, metadata/tag queries, state mutations, topic membership, and similarity search. Separate these internally while retaining a database facade.                                 |
| `crates/xaga-core/src/library_writer/mod.rs` | 2,593 lines total; roughly 1,920 before the bottom test modules | Central ownership is justified, but operation definitions, operation execution, public submission methods, and queue scheduling can live in separate files. Preserve one write authority and per-file serialization.            |
| `src/platform/TauriPlatform.ts`              | 1,980 lines                                                     | Oversized but relatively cohesive adapter. Schemas and decoding helpers occupy much of the opening 420 lines. Possible extraction: schemas and decoding helpers; assess domain-specific adapters separately.                    |
| `src-tauri/src/commands/articles.rs`         | 1,340 lines total                                               | Remote-loader callbacks, HTML/favicon/image capture, PDF access, article operations, annotations, and image recovery exceed a single narrow responsibility. Loader callbacks and annotations are clear extraction candidates.   |

Moving embedded tests would improve navigation, but substantial production
modules would remain. Refactoring should reduce the responsibilities a reader
must understand together, rather than just reduce line counts.

`Platform.ts` (506 lines), `ArticleReader.tsx` (414), and `AppLayout.tsx` (450)
were not priorities based on size alone. Broad interfaces and cohesive
composition components can reasonably reach those sizes. Sidebar change
frequency also does not by itself justify another split; it already delegates
to several list components.
