# Compliance: prevent loopback requests from bypassing the network proxy

## Problem and evidence

The macOS native `ui_network_webview` test demonstrates HTTP image requests to
`http://127.0.0.1:<port>` reaching a local test server directly from both UI test
webviews, without reaching their configured deny-all proxies. A proxy cannot
reject or report a request it never receives. The SSRF policy rejects loopback
when requests reach it; that address check does not establish routing through it.

The fixture uses a custom-protocol document, not the bundled application's CSP.
Exposure in the actual article loader and bundled main/settings UI remains to
be established. HTTPS loopback and other loopback representations remain
unverified. This observation does not establish a bypass for arbitrary Internet
requests or a successful attack against a local service.

Reproduce in a macOS GUI session from the Xaga repository:

```sh
XAGA_VERIFY_LOOPBACK_ISOLATION=1 cargo test -p xaga --test ui_network_webview
```

Inspect the local server's received requests and each proxy's observations;
a failed test alone is not evidence of a bypass. The ordinary test invocation
does not exercise the loopback case.

Outer Loop independently describes macOS WebKit bypassing proxies for localhost
URLs in its [July 2, 2026 release notes](https://outerloop.sh/release-notes/#0936-july-2-2026).
This supports investigating the platform behavior; it does not establish Xaga's
production exposure or supply a verified fix.

## Work

- Reproduce with the actual article-loader configuration, including subresource
  requests initiated by a controlled page. Test bundled main/settings UI with
  its actual CSP separately. Record which requests reach the local server,
  which reach the proxy, and which are stopped by another control.
- Cover HTTP and HTTPS, IPv4 and IPv6 loopback, alternate address representations,
  localhost names, and hostnames resolving to loopback. Exercise navigation,
  redirects, child frames, images, fetch, and WebSocket requests. Use controlled
  local endpoints; do not probe services belonging to the user.
- Choose and implement enforcement before a prohibited connection can occur.
  Evaluate tighter CSP and navigation restrictions for the trusted UI, and
  native WebKit content-blocking rules for the loader. These are candidates,
  not established solutions. URL matching must not replace DNS/address
  validation for requests that need it.
- Verify startup ordering and behavior when the proxy is unavailable. Preserve
  the required local app protocols and IPC, and explicitly scope any development
  server exception. Keep permitted article-loader retrieval functional.
- Report refusals through the existing error/diagnostic contract where the
  enforcing mechanism exposes them. Do not attribute a refusal to the proxy
  when another control blocked it.
- Update durable security documentation to describe the verified boundary and
  any remaining limitations, without claiming that proxy configuration alone
  guarantees complete routing.

## Acceptance

For the tested production configurations, prohibited loopback attempts cause no
connection to the controlled destination. Regression tests must establish this
at the destination as well as testing policy decisions. Local application
resources and IPC continue working, and legitimate loader retrieval remains
functional. Any uncovered request class or platform remains explicit.

## Relevant implementation

- `crates/xaga-core/src/network_proxy.rs`
- `src-tauri/src/ui_network.rs`
- `src-tauri/src/webview_policy.rs`
- `src-tauri/src/article_loader.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/tests/ui_network_webview.rs`
