# macOS Shortcuts Menu

## Summary

Expose Shortcuts through the native macOS menu in addition to the sidebar,
using the same pin targets and navigation behavior.

## Requirements

- Consume the shared parsed pin model; do not introduce separate menu storage or
  parse YAML independently in menu presentation code.
- Add a top-level Shortcuts menu. Keep empty groups visible as submenus with a
  disabled Empty item.
- For the MVP, use stored sibling sequence order without additional menu sorting.
- Offer pinned targets as native menu items and nested groups as submenus when
  hierarchical pins are available.
- Selecting a menu target performs the same navigation as selecting its shortcut,
  including article scope and Site navigation for supported target types.
- Update menu contents for pin and group changes and target-title changes.
- Keep navigation available in read-only libraries; menu projection does not
  mutate stored pins.
- Preserve existing native menus and built-in All items navigation.

## Remaining work

- Complete central pin storage first. Flat menu support can precede nested groups;
  submenu integration depends on the nested-group schema. Site menu support follows
  Site pinning and must use the same resolver.
- Keep unavailable targets visible as cannot-found items, following the shared
  resolver's status; do not silently discard their references.
- Connect shared pin state to native menu construction and updates; route menu
  actions through the frontend's existing navigation boundary.
- Verify native selection, submenu navigation, live updates, library context,
  unavailable targets, and parity with sidebar navigation.
- Run affected Rust/frontend checks and validate the menu in the macOS app.

## References

- [Pin-store contract](../../docs/architecture/pins.md).
- [Nested Pin Groups](enhancement__pins--nested-groups.md)
- [Pin Sites to Shortcuts](enhancement__pins--pin-sites-to-shortcuts.md)
- `src-tauri/src/lib.rs`
- `src/hooks/useRouter.tsx`
- `src/components/sidebar/ShortcutList.tsx`
- `docs/architecture/list_view.md`
