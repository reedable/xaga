# macOS Shortcuts Menu

## Summary

Expose Shortcuts through the native macOS menu in addition to the sidebar,
using the same pin targets and navigation behavior.

## Requirements

- Consume the shared parsed pin model; do not introduce separate menu storage or
  parse YAML independently in menu presentation code.
- Add a top-level Shortcuts menu immediately to the right of History. Keep empty groups visible as submenus with a
  disabled Empty item.
- Respect the user's selected Shortcuts sort mode, using the same ordering as
  the sidebar. Sorting is presentation state and must not rewrite stored pins.
- Always place All items first, including when there are no pins; follow it with
  a separator when pins exist.
- Offer pinned targets as native menu items and nested groups as submenus when
  hierarchical pins are available.
- Selecting a menu target performs the same navigation as selecting its shortcut,
  including article scope and Site navigation for supported target types.
- Update menu contents for pin and group changes, target-title changes, selected
  sort-mode changes, and unread-count changes that affect ordering.
- Selecting an unavailable target opens the existing Shortcut unavailable dialog
  with Retry and Remove shortcut actions, including existing read-only gating.
- Keep navigation available in read-only libraries; menu projection does not
  mutate stored pins.
- Preserve existing native menus and built-in All items navigation.

## Remaining work

- Flat menu support can precede nested groups; submenu integration depends on
  the nested-group schema. Use the shared resolver for all target types, including Sites.
- Keep unavailable targets visible as cannot-found items, following the shared
  resolver's status; do not silently discard their references.
- Connect shared pin state to native menu construction and updates; route menu
  actions through the frontend's existing navigation boundary.
- Verify native selection, submenu navigation, live updates, library context,
  unavailable-target actions, empty-menu behavior, sorting, and parity with sidebar navigation.
- Run affected Rust/frontend checks and validate the menu in the macOS app.

## References

- [Pin-store contract](../../docs/architecture/pins.md).
- [Nested Pin Groups](../available/enhancement__pins--nested-groups.md)
- [Sidebar and menu sorting controls](../backlog/enhancement__sidebar-sorting-controls.md)
- [Site pinning](../../docs/architecture/sites.md#shortcuts)
- `src-tauri/src/lib.rs`
- `src/hooks/useRouter.tsx`
- `src/components/sidebar/ShortcutList.tsx`
- `docs/architecture/list_view.md`
