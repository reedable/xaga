# Redirect cap for the article loader

Status (2026-07-02): no decision made. The maximum-redirects setting (see
`docs-transient/backlog/fetch/improvement__user_settings.md`) is enforced on the reqwest fetch paths (feeds,
PDF probe) only. The article loader is not exempt from the cap — it is unaddressed.

Facts known at time of writing:

- The loader is a WKWebView routed through the SSRF proxy
  (`src-tauri/src/article_loader.rs`); it follows redirects inside its own network
  stack.
- The proxy cannot count redirects for https traffic: each hop arrives as an
  independent `CONNECT`, indistinguishable from a subresource fetch
  (`crates/xaga-core/src/ssrf_proxy.rs`).
- Tauri 2.10.3 (in use) exposes `WebviewWindowBuilder::on_navigation(Fn(&Url) -> bool)`,
  which could reject or count top-level navigations. Not verified: whether WKWebView on
  macOS invokes it once per server-redirect hop. Any verification must use the bundled
  build (`tauri build`), not `tauri dev`.
- `on_navigation` covers top-level navigations only. Subresource redirect chains are
  enforceable at no point visible to the app: the webview follows them internally, and
  the proxy sees only opaque tunnels (each hop is scheme/address-validated individually,
  but not counted as a chain).
