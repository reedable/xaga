# Choose a destination folder when adding a shortcut

## Summary

Verify that adding and moving a shortcut through the destination picker updates
the live macOS Shortcuts menu.

## Requirements

- The native menu shows an added shortcut in the chosen group and reflects a
  later move without restarting Xaga.
- Activating the menu item opens its target through the existing shortcut route.

## Remaining work

- Run the native Shortcuts menu test in an active macOS desktop session.
- Check an Article Share add and a collection edit against the live menu using
  a test library. Verify the destination after each save and activate the moved
  shortcut from the menu.

## References

- [Pin storage and groups](../../docs/architecture/pins.md)
- [Native pin commands](../../src-tauri/src/commands/pins.rs)
- [Native menu test](../../src-tauri/tests/shortcuts_menu.rs)
