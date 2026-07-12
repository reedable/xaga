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
- **Import byte-cap rejection** — `import_feeds`/`import_bookmarks` reject a
  file exceeding `import_max_bytes` with a raw
  `format!("import file is {} bytes, exceeding the {} byte limit", ...)`
  string (`src-tauri/src/native/feeds.rs`, `native/bookmarks.rs`), same
  raw-string-to-event pattern as the gap above. Additionally, unlike a
  malformed-file error, this one is user-actionable: the cap is a setting, so
  the message should direct the user to Settings to raise it. No such
  direction exists today because the setting itself has no UI yet — see
  `docs-transient/backlog/fetch/improvement__user_settings.md`. This bullet is blocked
  on that item landing.

## Settings-write refusals (account lock, 2026-07-10)

Settings writes go through the `update_settings` funnel (`src-tauri/src/lib.rs`)
under the account-wide lock (the flock on settings.json) and can now be
refused. Both refusals reach the webview as plain error strings with no UI
treatment:

- **Lock contention** — "another xaga process holds the account lock":
  returned immediately (fail-fast, by decision) when e.g. the other instance
  is downloading the embedding model. Needs user-facing display; note the
  hold can be minutes long during a download.
- **Stale view** — "settings changed by another xaga process; view
  refreshed — retry": the process's in-memory mirrors are refreshed from the
  fresh content before the error returns, so a retry succeeds; the UI needs
  to surface the refusal and re-render the refreshed values.

## Other error handling

Believed incomplete elsewhere; enumerate when this item is picked up.
