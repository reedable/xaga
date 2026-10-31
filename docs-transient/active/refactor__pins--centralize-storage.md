# Centralize Pin Storage

## Summary

Move Feed, Topic, and article pin ownership from entity files to one
library-scoped store of typed target references. Migrate existing pins and
connect existing controls and Shortcuts to the new authority in the same story.

## Requirements

- Store pin membership centrally; reference targets by type and stable identity
  rather than embedding entity content. Keep pins isolated to their library.
- Use YAML for the pin store. Parse it into a shared typed model consumed by
  presentation layers. Allow evolution to nested groups without implementing
  grouping or native-menu presentation in this story.
- Use a root `pins` sequence of typed target references:

  ```yaml
  pins:
    - type: entry
      id: abc123
    - type: feed
      id: feed456
    - type: topic
      id: topic789
    - type: site
      hostname: example.com
  ```

  Target identity is type plus ID, or type plus normalized hostname for Sites.
  Resolve titles and counts from targets rather than duplicating them in storage.
  The Site reference shape is reserved for the dependent Site-pinning story.
- Pinning an already-pinned typed target in the flat store is a no-op. Retain
  unresolved target references rather than silently removing pins.
- Keep unresolved pins visible in Shortcuts with an indication that the target
  cannot be found and a manual unpin action, subject to read-only gating.
  Identify them using the stored target type and identity when no title can be
  resolved. If the user restores the target, resolve the retained reference again
  without requiring them to recreate the pin.
- Preserve YAML sequence order; existing alphabetical/unread sorting remains a
  presentation concern. Future groups use `type: group`, a stable `id`, a `title`,
  and a `children` sequence containing the same target nodes or nested groups.
- Derive the store path in the core library-layout module. A missing store is
  an empty pin set for normal access.
- Serialize all pin mutations through one shared library-writer lane and replace
  the complete file atomically under the existing library write right.
- Preserve existing Feed, Topic, and article pin controls, shortcut sorting,
  article count suppression, responsive navigation, and current-list stability.
- Derive any displayed pin state from central membership. Entity files and
  LanceDB must not remain independent pin authorities; recovery must not restore
  legacy pins from entity state after conversion.
- Provide an explicit xaga-cli upgrade for legacy Feed categories and
  Topic/article tags. Commit central membership before removing legacy markers;
  retries must preserve pins already transferred and leave unrelated data intact.
- Preserve read-only behavior and carry committed pin changes to the UI without
  waiting for embedding work.
- Increment the minor release version for this storage-model change and
  document compatibility and the CLI upgrade procedure.
- Keep Site controls and Site shortcut presentation in the dependent story.

## Remaining work

- Implement core persistence, writer operations, commands, and platform access.
- Replace per-entity pin discovery and event handling with shared membership
  resolution; update domain projections and cache/recovery paths accordingly.
- Extend CLI conversion and reconcile legacy cached pin state.
- Update source-of-truth, crash-consistency, and upgrade documentation.
- Verify restart, migration retry/interruption, concurrent pin mutations,
  read-only refusal, library isolation, unavailable targets, existing controls,
  sorting, navigation, live updates, and recovery without a cache.
- Run workspace formatting, affected Rust tests, frontend checks, and production
  build.

## References

- [Pin Sites to Shortcuts](enhancement__pins--pin-sites-to-shortcuts.md) — follows
  this story and uses its shared store.
- [Nested Pin Groups](enhancement__pins--nested-groups.md)
- [macOS Shortcuts Menu](enhancement__pins--macos-menu.md)
- `docs/principles/single_source_of_truth.md`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/list_view.md`
- `docs/architecture/single_writer_library_lock.md`
- `docs/library_upgrade.md`
- `crates/xaga-core/src/library.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-cli/src/cmd/library_upgrade.rs`
- `src/platform/Platform.ts`
- `src/providers/FeedsProvider.tsx`
- `src/providers/TopicsProvider.tsx`
- `src/hooks/usePinnedEntries.ts`
- `src/components/sidebar/Sidebar.tsx`
- `src/components/sidebar/ShortcutList.tsx`
