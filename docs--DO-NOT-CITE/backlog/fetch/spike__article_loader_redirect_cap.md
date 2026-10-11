# Redirect cap for the article loader

The maximum-redirects setting is enforced on reqwest fetch paths (feeds and the PDF probe), but not on the article loader. Determine how to enforce the cap there.

Implementation constraints to verify:

- The loader is a WKWebView routed through the network proxy (`src-tauri/src/article_loader.rs`); it follows redirects inside its own network stack.
- The proxy cannot count redirects for https traffic: each hop arrives as an independent `CONNECT`, indistinguishable from a subresource fetch (`crates/xaga-core/src/network_proxy.rs`).
- Tauri exposes `WebviewWindowBuilder::on_navigation(Fn(&Url) -> bool)`, which could reject or count top-level navigations. Not verified: whether WKWebView on macOS invokes it once per server-redirect hop. Any verification must use the bundled build (`tauri build`), not `tauri dev`.
- `on_navigation` covers top-level navigations only. Subresource redirect chains are enforceable at no point visible to the app: the webview follows them internally, and the proxy sees only opaque tunnels (each hop is scheme/address-validated individually, but not counted as a chain).
