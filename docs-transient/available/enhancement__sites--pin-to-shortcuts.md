# Pin Sites to Shortcuts

## Summary

Let a user pin a derived Site into Shortcuts, with the pinned normalized
hostnames stored as library-specific metadata.

## Requirements

- Store pinned Site identities in one library-level metadata file whose only
  authority is the set of pinned normalized hostnames.
- Derive the metadata path once in the core library-layout module and keep the
  file scoped to its library; pins must not cross library boundaries.
- Read a missing metadata file as an empty pin set.
- Validate and normalize every hostname at the persistence boundary; never use
  a hostname directly as a path component.
- Serialize every metadata mutation through the library writer and write the
  complete file atomically under the existing library write right.
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
- Update the durable source-of-truth and crash-consistency documentation for the
  new metadata file and its sole write path.

## Remaining work

- Complete `spike__define-tag-collection-bookmark-pin-shortcut-site.md` and apply
  its definitions and ownership decisions to this story.
- Define the library metadata schema and core read/write API for pinned Site
  hostnames.
- Add a library-writer operation and Tauri commands for pinning and unpinning a
  Site.
- Carry Site pin state through the platform boundary, add the Site Settings dialog,
  and integrate pinned Sites into Shortcuts.
- Test missing and malformed metadata, normalization, atomic replacement,
  concurrent pin changes, read-only refusal, library isolation, empty pinned
  Sites, counts, selection, and **All items** placement.
- Run Rust and frontend formatting, lint, tests, and the production build.

## References

- `docs-transient/active/spike__define-tag-collection-bookmark-pin-shortcut-site.md`
- `docs/architecture/sites.md`
- `crates/xaga-core/src/library.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src-tauri/src/commands/mod.rs`
- `src/components/sidebar/ShortcutList.tsx`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/single_writer_library_lock.md`
- `docs/principles/single_source_of_truth.md`
