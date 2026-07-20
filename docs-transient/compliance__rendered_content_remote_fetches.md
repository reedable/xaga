# Compliance: rendered content makes remote fetches (tracking / read receipts)

Raised 2026-07-16 from the summary-rendering sweep (ingestion → rendering; see
`docs/architecture/security/handling_persisted_content.md`).

**Verified.** The bundled CSP (`src-tauri/tauri.conf.json`, `app.security.csp`)
sets `img-src 'self' data: https:`. Any `https:` image in rendered content
loads from the reader iframe at read time: markdown `![](…)` images
(pre-existing) and, since the 2026-07-16 change, `<img>` in feed-summary HTML
(DOMPurify keeps benign image markup). A feed can learn when an entry is read
and from which IP.

- The project's threat model treats read-tracking as in-scope:
  `docs/architecture/security/performance_tradeoff.md` rejects conditional-GET
  validators specifically as a server-writable tracking channel. The image
  channel carries the same signal and is open by policy.
- `style-src 'unsafe-inline'` plus DOMPurify's default handling (keeps `style`
  attributes) means CSS `background-image: url(…)` is a second copy of the
  same channel.

**Unverified sub-item (ranked first to check):** whether reader-iframe
subresource fetches route through the SSRF proxy or go direct from the
webview. The invariants in
`docs/architecture/security/fetching_remote_content.md` (containment, minimal
disclosure, bounded resources) are enforced on app-initiated requests; if
iframe image loads bypass the proxy, they also bypass private-address
validation (a content-authored `https://192.168.1.1/…` probe class).
