# Nested Pin Groups

## Summary

Let users organize Shortcuts into nested groups, saved in the library pin store.

## Requirements

- Extend the ordered pin tree with groups containing target references or groups:

  ```yaml
  pins:
    - type: group
      id: group123
      title: Research
      children:
        - type: site
          hostname: example.com
        - type: group
          id: group456
          title: Background
          children:
            - type: entry
              id: abc123
  ```

  Group IDs are unique and stable across renames and moves. Preserve existing
  target encoding, resolved titles/counts, and one pin per typed target across
  the tree. Toolbar unpin removes that target wherever it occurs.
- Preserve the [pin-store contract](../../docs/architecture/pins.md), including
  validation, serialized atomic writes, library isolation, and read-only gating.
  Organization changes preserve referenced entities and open navigation context.
- Add groups through plus buttons on Shortcuts and group headers. Create and
  rename through a title dialog following Feed/Topic editing, with a
  keyboard-accessible group Settings action. Trim titles, reject empty titles,
  and allow duplicate titles.
- Move pins and groups into groups or the root using drag and drop or a
  keyboard-accessible **Move to…** destination picker. Refuse moves into the
  moved group or its descendants. Dropping into a collapsed group expands it.
- Show unpin/remove buttons on hover and keyboard focus in Shortcuts. Removing
  a group replaces it with its children, preserving their sequence and pins.
- Append newly pinned targets at root, new groups in their container, and moved
  nodes in their destination. Display uses existing alphabetical/unread sorting
  within every container; manual ordering is out of scope.
- Show and sort groups by their recursive descendant unread count. Count shared
  articles once per pinned collection; article shortcuts contribute zero. Use
  the existing title tie-breaker and show no separate group item count.
- Expand groups independently; new groups start expanded. Remember expansion
  locally per library and group ID. Filtering reveals matching descendants and
  their ancestor paths without changing saved expansion; a matching group title
  reveals its whole subtree.
- Show the hierarchy in the existing selectable navigation header when the
  sidebar is hidden. Group editing and pin organization use the sidebar.
- Build native macOS Shortcuts submenus from the shared hierarchy, using the
  sidebar's sorting and refreshing for hierarchy, target, and count changes.
  Empty groups contain a disabled **Empty** item. Keep **All items** first at
  root, existing navigation and unavailable-target actions, and read-only
  navigation. Menu code must not parse YAML or maintain separate storage.
- Increment the minor version while the app remains at major version 0. Provide
  an explicit CLI library upgrade that preserves flat pins and supports safe
  retries, with documentation. Older applications must open upgraded libraries
  read-only through existing version checks.

## Remaining work

- Extend the schema, writer operations, platform API, shared projections, and CLI
  upgrade. Establish the parser's existing nesting protection and any additional
  bound needed for safe recursive traversal.
- Implement the sidebar controls, filtering, selectable header, and native submenus.
- Verify the requirements, including concurrent writes, interrupted upgrades,
  library isolation, restart/recovery, and sidebar/native-menu parity.
- Run affected Rust and frontend checks and update durable documentation.

## References

- [Pin-store contract](../../docs/architecture/pins.md).
- `src/components/ShortcutsMenu.tsx`
- `src/hooks/useShortcutTargets.ts`
- `src-tauri/src/shortcuts_menu.rs`
- `src-tauri/tests/shortcuts_menu.rs`
- `src/components/sidebar/ShortcutList.tsx`
- `src/components/list/EntityList.tsx`
- `crates/xaga-core/src/library_writer/mod.rs`
- `docs/architecture/list_view.md`
- `docs/development/release_cycle.md`
