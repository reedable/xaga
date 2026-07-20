# Compliance: remaining weaknesses, ingestion → rendering (remote and fs)

Raised 2026-07-16 from the summary-rendering fix (feed summaries are stored as
feed-supplied HTML; the render pipeline dropped raw HTML, so summaries never
displayed; fixed by parsing embedded HTML via `rehype-raw` and sanitizing the
serialized output with DOMPurify — see
`docs/architecture/security/handling_persisted_content.md`, "Rendering").
The review that followed swept ingestion → rendering across both origins
(remote fetch and `.md` read-back from the filesystem, which
`handling_persisted_content.md` declares untrusted regardless of provenance).

Each item is marked **verified** (code/config read on 2026-07-16) or
**unverified** (open question; not traced to the bottom). No dispositions have
been made.

## 1. Render-time fetches by content (tracking / read receipts)

**Verified.** The bundled CSP (`src-tauri/tauri.conf.json`, `app.security.csp`)
sets `img-src 'self' data: https:`. Any `https:` image in rendered content
loads from the reader iframe at read time: markdown `![](…)` images (
pre-existing) and, since the 2026-07-16 change, `<img>` in feed-summary HTML
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

## 2. Favicon path

**Verified 2026-07-16.** No Rust code fetches favicons; the fetch is made by
the main app window's webview engine, twice:

- a probe via `new Image()` — `getFavicon` (`src/components/content/Toolbar.tsx`)
  sets `img.src` in the main window's DOM and awaits `onload`;
- the render, `<img src={favicon}>` in the toolbar's React tree.

The URL is content-influenced in both sources: `{article origin}/favicon.ico`
derived from the article URL, or the persisted `entry.favicon` extracted from
the page's `<link rel~=icon>` by `extractFavicon` (`src/text/entryWorker.ts`)
— the latter fully author-controlled.

Consequences:

- **Bypasses every invariant in `fetching_remote_content.md`** — no SSRF
  proxy, no resolve/validate/pin, no per-invocation client, no bounded
  resources. A page can persist an internal-host favicon URL
  (e.g. `https://192.168.1.1/favicon.ico`) and the app probes it at read time
  from inside the network.
- **The only constraint is the app CSP** (`img-src 'self' data: https:`):
  `http:` blocked in the bundled app, `data:` and any `https:` host allowed;
  no constraint under `tauri dev` (CSP not enforced there — see
  `handling_persisted_content.md`, "Testing caveat").
- **The fetch originates in the privileged window** (the context holding IPC
  access), not the sandboxed reader iframe.
- Adjacent defect (correctness, not security): `getFavicon` attaches no
  `onerror`, so a failed load leaves the promise unsettled and the awaiting
  effect hangs (abandoned on the next effect run, never resolved).
