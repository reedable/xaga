# Pin Sites to Shortcuts

## Summary

Let a user pin a derived Site into Shortcuts using the shared library pin store.
Implement after central pin storage is available.

## Requirements

- Store typed Site targets identified by normalized hostname in the shared pin
  store; do not create a separate Site pin file or persist Site membership.
- Validate and normalize every hostname at the persistence boundary; never use
  a hostname directly as a path component.
- Use the shared pin mutation API and its serialized atomic persistence.
- Refuse pin changes in read-only library mode through the existing command
  gating and error path.
- Show pinned Sites in Shortcuts with their current total and unread counts and
  route selection to the Site's article list.
- Give the selected Site the same collection-header Settings affordance used for
  Feeds and Topics. Its dialog shows the normalized hostname as read-only identity
  and uses the existing **Add to Shortcuts** checkbox pattern to pin or unpin it.
- Disable the Site pin control in read-only mode and restore its persisted state
  whenever the dialog opens.
- Keep **All items** fixed at the top of Shortcuts; include pinned Sites in the
  ordering applied to the remaining pinned collections.
- Retain a pin when its Site temporarily has no derived Entries, so transient
  cache or out-of-band state does not silently erase user intent; represent the
  empty pinned Site with zero counts.
- Increment the minor release version when introducing the persisted Site target
  representation; document its compatibility and update the pin-store contract.

## Remaining work

- Complete the central-pin-storage story first. Extend its typed target schema,
  resolution, and commands for normalized Site hostnames.
- Carry Site pin state through the platform boundary, add the Site Settings dialog,
  and integrate pinned Sites into Shortcuts.
- Test missing and malformed metadata, normalization, atomic replacement,
  concurrent pin changes, read-only refusal, library isolation, empty pinned
  Sites, counts, selection, and **All items** placement.
- Run Rust and frontend formatting, lint, tests, and the production build.

## References

- [Centralize Pin Storage](refactor__pins--centralize-storage.md) — prerequisite.
- [Organization model](../../docs/architecture/organization_model.md)
- `docs/architecture/sites.md`
- `crates/xaga-core/src/library.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src-tauri/src/commands/mod.rs`
- `src/components/sidebar/ShortcutList.tsx`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/single_writer_library_lock.md`
- `docs/principles/single_source_of_truth.md`
