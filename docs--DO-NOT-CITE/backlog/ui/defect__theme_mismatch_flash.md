# Theme-mismatch flash at launch

## Summary

The native window starts with the persisted theme, but `AppearanceProvider` initializes to `auto` and loads the saved choice asynchronously. When the explicit theme differs from the OS, the first frontend paint temporarily uses the wrong theme.

## Requirements

- Make the first frontend paint use the same resolved theme as the native window.
- Keep the persisted account setting authoritative across processes.
- Avoid rendering application content until the initial theme is known if no synchronous handoff is selected.
- Preserve live system-theme changes while the setting is `auto`.

## Remaining work

- Compare a native-injected bootstrap value, a validated local mirror, and gating the first render on `getTheme()`.
- Define reconciliation when a local bootstrap value differs from account settings.
- Implement the selected startup handoff.
- Test explicit light and dark themes against both OS themes and test automatic-theme changes.

## References

- `src/providers/AppearanceProvider.tsx`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/settings.rs`
