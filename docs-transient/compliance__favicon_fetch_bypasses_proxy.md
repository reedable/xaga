# Compliance: favicon fetch bypasses the SSRF proxy

Raised 2026-07-16 from the summary-rendering sweep (ingestion → rendering; see
`docs/architecture/security/handling_persisted_content.md`).

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
