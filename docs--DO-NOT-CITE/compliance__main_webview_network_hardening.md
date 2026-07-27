# Compliance: harden the main webview against remote fetches

The main window has no `proxy_url` — only `article_loader.rs` sets one — so
anything it requests resolves DNS and connects directly, bypassing `is_blocked`
in `ssrf_proxy.rs`. The app CSP (`src-tauri/tauri.conf.json`,
`app.security.csp`) permits `img-src … https:` and `frame-src … https:`, and is
not enforced under `tauri dev`.

Content the ingestion rewrite cannot reach lands here too — e.g. a hand-edited
`.md` carrying a raw `<img src="https://…">`, which renders into the reader
iframe in this window. Unverified: whether that `blob:` iframe inherits the app
CSP, which decides whether lever 1 reaches it at all.

Three levers, in increasing order of strictness:

1. Tighten the app CSP so remote sources are not permitted.
2. Route the main webview through `ssrf_proxy`.
3. Ban outbound network from the main webview entirely.

Blocked on the favicon path (`compliance__favicon_fetch_bypasses_proxy.md`),
which is the one thing in the main window that currently fetches remote `https:`.
