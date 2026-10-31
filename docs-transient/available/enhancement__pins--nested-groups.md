# Nested Pin Groups

## Summary

Let users organize pinned navigation targets into nested groups in Shortcuts,
persisted in the shared library YAML pin store.

## Requirements

- Represent groups containing typed target references and other groups, with
  sibling order preserved in YAML and the shared in-memory model.
- Use group nodes alongside the existing target-reference nodes:

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

  Group IDs remain stable across renaming and moves. Target nodes retain the
  encoding defined by central pin storage; titles and counts remain resolved
  from targets.
- Support creating, renaming, moving, and removing groups and moving pins between
  groups without changing the referenced entities.
- Allow one pin per typed target across the entire tree. Moving a pin changes
  its containing group rather than creating another occurrence; toolbar unpin
  removes that target's single pin.
- Apply existing alphabetical/unread sorting within each group without changing
  membership. Manual ordering is outside this story.
- A group's unread-sort value is the sum of its descendants' shortcut unread
  counts, recursively. Do not deduplicate articles shared by pinned collections;
  article shortcuts retain their existing zero contribution. Use the existing
  title tie-breaker.
- Organize pins and groups using drag and drop into groups or the root, without
  permitting a group to be moved into itself or its descendants. Sorting remains
  governed by the selected mode rather than drop position.
- Add a plus button to the Shortcuts header and group headers to create a group
  in that container, following the existing Feed/Topic header affordance.
- Show an unpin/remove button on hover for each pinned target and group only in
  Shortcuts. Removing a group uses the child-promotion behavior above. Make the
  action available on keyboard focus as well.
- Removing a nonempty group promotes its children into its parent, preserving
  their pins and nested groups. Removing a root group promotes its children to
  the root sequence; it does not unpin the subtree's targets.
- Preserve existing flat pins when adopting the hierarchical format.
- Use shared pin persistence, serialized atomic writes, library isolation, and
  read-only gating. Preserve open navigation context during organization changes.
- Present nested groups in Shortcuts and provide access when the sidebar is hidden.
- Increment the minor release version for the storage-model change and provide
  the corresponding CLI upgrade and documentation.

## Remaining work

- Complete central pin storage first; Site support is not a prerequisite.
- Specify group renaming, expansion, keyboard move controls equivalent to drag
  and drop, and responsive navigation presentation.
- Extend the typed YAML schema, writer operations, platform API, and frontend
  projection; implement migration from flat pins with safe retries.
- Verify nested persistence, moves, duplicate handling, deletion, keyboard use,
  responsive access, read-only refusal, concurrent changes, and restart/recovery.
- Run affected Rust and frontend checks and update durable documentation.

## References

- [Centralize Pin Storage](refactor__pins--centralize-storage.md) — prerequisite.
- [macOS Shortcuts Menu](enhancement__pins--macos-menu.md)
- `src/components/sidebar/ShortcutList.tsx`
- `src/components/list/ItemList.tsx`
- `crates/xaga-core/src/library_writer/mod.rs`
- `docs/architecture/list_view.md`
- `docs/development/release_cycle.md`
