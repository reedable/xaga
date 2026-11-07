# Sidebar and Menu Sorting Controls

## Summary

Expose the existing sidebar sort modes through visible UI controls and provide
the Shortcuts sorting control in the native macOS menu.

## Requirements

- Provide discoverable, keyboard-accessible sorting controls for Shortcuts,
  Topics, Feeds, and Sites, exposing alphabetical and unread-count modes and
  indicating the selected mode.
- Provide a sorting control in the macOS Shortcuts menu for the same Shortcuts
  preference used by the sidebar. Changes from either surface update both.
- Preserve each section's persisted preference and existing sorting semantics.
  Sorting must not rewrite pin membership or stored sequence order.
- Keep All items first in Shortcuts regardless of sort mode. Apply sorting within
  each nested group.
- Keep sorting controls usable in read-only libraries.

## Remaining work

- Specify sidebar control placement, appearance, and interaction, and the placement
  of sorting controls within the macOS Shortcuts menu.
- Expose the currently hidden sidebar controls and connect native menu controls
  to the shared Shortcuts sort state.
- Verify keyboard access, selected-state feedback, cross-surface updates,
  per-section persistence, read-only use, and ordering after unread-count changes.

## References

- [Nested pin groups](../../docs/architecture/pins.md#nested-groups-015)
- [Pin-store contract](../../docs/architecture/pins.md)
- `src/components/sidebar/SidebarSortControl.tsx`
- `src/components/sidebar/SidebarSortControl.module.css`
- `src/hooks/useSidebarSort.ts`
- `src/components/ShortcutsMenu.tsx`
- `src-tauri/src/shortcuts_menu.rs`
