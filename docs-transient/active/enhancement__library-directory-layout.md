# Library directory layout

## Summary

Separate runtime and rebuildable files from persistent library data, keeping
primary content visible at the library root and supporting data in a visible
`metadata/` directory.

## Requirements

- Use this layout for new and upgraded libraries:

  ```text
  Library/
  ├── .xaga/
  │   ├── xaga.lock
  │   └── embeddings.lance/
  ├── metadata/
  │   ├── library.yaml
  │   ├── favicons/
  │   ├── feeds/
  │   ├── pins.yaml
  │   └── tags.yaml
  ├── entries/
  ├── topics/
  ├── pdfs/
  └── images/
  ```

- Keep the manifest and lock as separate files. Keep all persistent library
  data outside `.xaga/`; with every process using the library closed, deleting
  `.xaga/` must allow Xaga to recreate its lock and rebuild its database without
  losing persistent data. The visible contents must suffice for restoration.
- Keep `metadata/` visible so file browsing and backups expose subscriptions,
  organization, resources, and format metadata. Keep Topic Markdown visible to
  Obsidian and other Markdown tools. Do not change entry filenames, content
  directory organization, or the existing extensionless body-image reference
  convention in this task.
- Centralize paths and stored resource-reference construction and parsing in
  the core library layout API. Update shared favicon references to
  `../metadata/favicons/<hash>` and preserve their resolution and traversal
  protections. Preserve existing entry/image relative references.
- Increment the application's minor version while it remains at major version
  0, following the library-version contract. Upgrade existing libraries through
  the explicit shared core upgrade used by the app and CLI; opening a library
  must not silently migrate it.
- Require successful layout conversion before opening an old-layout library
  for browsing or ordinary operations, including read-only browsing. At app
  startup and after selecting such a library, offer **Upgrade**, **Choose another
  Library**, and **Quit**; do not offer **Continue read-only**. Keep the existing
  backup explanation and explicit user confirmation before conversion. Declining
  conversion must leave persistent library data unchanged.
- Keep the startup gate active after an interrupted or failed layout upgrade.
  Offer retry, choosing another library, and quitting without exposing a
  partially relocated library to ordinary reads or writes. Metadata errors or
  lock contention must explain the failure and permit retry or leaving the
  library without bypassing the gate. Successful conversion uses the existing
  restart and compatibility checks before opening the library. Read-only access
  due to lock contention in a completed, compatible new-layout library remains
  available under the existing contract.
- Preserve source content, annotations, custom frontmatter, membership and
  ordering, tag identities and transitions, pins, and saved resource bytes.
  Manifest relocation must retain completed and pending migration state.
  Interrupted upgrades must remain identifiable and resumable without guessing
  the source format or overwriting conflicting files.
- Preserve single-writer exclusion during transition between the old root lock
  and `.xaga/xaga.lock`, including contention with an old-layout process.
  Establish the exclusion protocol before relocating files; do not delete or
  replace an actively held lock in a way that permits competing writers.

## Remaining work

- Trace layout dependencies in persistence, recovery, indexing, loader and
  rendering paths, app startup and library switching, CLI commands, fixtures,
  and development tools. Identify migration handling for legacy paths, partial
  moves, duplicate manifests, and lock contention; resolve any required design
  decisions before implementation.
- Implement the layout API and caller changes, new-library initialization,
  explicit migration, metadata inspection for upgrade eligibility and progress,
  and the mandatory layout-upgrade gate. Legacy-path access is needed for
  inspection and conversion, not ordinary old-layout browsing.
  Ensure empty-library detection accounts for the new internal directories.
- Verify new-library creation, old-library conversion, interrupted-upgrade
  resumption, conflict handling, and app/CLI writer exclusion. Verify article
  and PDF reads, image and favicon resolution, feed refresh, Topic membership,
  tags, and pins after conversion. Verify restoration from visible contents
  without `.xaga/` rebuilds derived data and preserves user state.
- Update browser workflow coverage for the mandatory gate, confirmation,
  cancellation, library selection, failure/retry, and restart. Verify no
  old-layout or partially relocated library can reach ordinary browsing through
  a read-only choice. Preserve coverage for compatible-library read-only access.
  Run `npm run test:browser` following the UI testing convention and relevant backend and
  native checks for persistence, migration, and process locking; run
  `cargo fmt --all --check` after Rust changes.
- Update durable layout, version, locking, resource, and backup documentation,
  including instructions to close all library users before deleting `.xaga/`
  and to preserve every visible library directory when backing up. Document the
  mandatory layout conversion and its exception to the existing option to
  continue browsing an older library read-only.

## References

- [Library layout API](../../crates/xaga-core/src/library.rs)
- [Library manifest](../../crates/xaga-core/src/library/manifest.rs)
- [Library upgrades](../../crates/xaga-core/src/library/upgrade.rs)
- [Library version contract](../../docs/architecture/library_version.md)
- [Library upgrade verification](../../docs/development/library_upgrade_verification.md)
- [Single-writer library lock](../../docs/architecture/single_writer_library_lock.md)
- [Single source of truth](../../docs/principles/single_source_of_truth.md)
- [Crash consistency](../../docs/architecture/crash_consistency.md)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
