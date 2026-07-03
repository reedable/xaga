# Enhance error handling (import failures and beyond)

Error handling across the app is incomplete. This item collects known gaps;
the import failure events below are the concrete starting point, but there are
likely others to enumerate.

## Feed / bookmark import failure events

Import/export are orchestrated natively and report completion by event
(`feeds:imported`, `feeds:exported`, `bookmarks:imported`,
`bookmarks:exported`) carrying `{ result?, error? }`
(`src-tauri/src/native/`). Gaps:

- **Feed import** — `FeedsProvider` surfaces `outcome.error` via `setError`,
  but the per-entry `failed` list inside a *successful* import (feeds that
  parsed but couldn't be added) is not shown to the user.
- **Bookmark import/export** — the native side emits `bookmarks:*` events, but
  no frontend consumer exists, so bookmark import/export failures are logged
  only. When a bookmarks UI is built, wire a listener that surfaces
  `outcome.error` (mirroring `FeedsProvider`).
- **Error hygiene at the source** — `parse_opml`'s error string embeds
  quick-xml's message (which can quote file fragments), and the import result
  echoes raw failed entry strings back. Prefer a generic error to the
  user/event and full detail to the log.

## Other error handling

Believed incomplete elsewhere; enumerate when this item is picked up.
