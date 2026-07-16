# Theme-mismatch flash at launch (pending — cause diagnosed)

## Symptom

With an explicit theme that differs from the OS (e.g. Dark set while the OS is Light), launch briefly shows dark → white → dark (or the reverse for explicit Light on a dark OS).

## Diagnosed cause (recorded)

Not the transparent-webview approach: the native window paints the correct color first (Rust resolves the persisted theme in `setup()` and calls `set_background_color`). The middle frame is the frontend rendering with the wrong `themeMode`: the persisted theme loads asynchronously (`getTheme()`) _after_ first paint, so the initial render falls back to `"auto"` + the OS preference, which disagrees with the explicit setting (`src/providers/AppearanceProvider.tsx` — `themeSetting` init, `themeMode` derivation, the async `getTheme()` effect).

## Recorded fix direction

Make the resolved theme available synchronously to the first render (e.g. a Rust-injected value or a `localStorage` mirror) — not anything about `drawsBackground`.
