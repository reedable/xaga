# Compliance: prevent loopback requests from bypassing the network proxy

## Summary

Prevent prohibited loopback connections from webviews, including requests that
WebKit routes around the configured proxy. The proxy rejects loopback when it
receives the request; that policy alone does not establish network containment.
The loader-configuration probes demonstrate this bypass. Close the gap without
assuming the trusted UI's CSP protects the article loader.

## Requirements

- Enforce restrictions before a prohibited connection reaches a local service.
  Observe the destination as well as the proxy; do not attribute CSP refusals
  to the proxy or treat absent proxy observations as proof of no connection.
- Treat localhost reachability as an exposure even when response access fails.
  CORS failures and ambiguous timing are not containment controls.
- Keep remote-content acquisition in the isolated article loader. Preserve its
  legitimate remote retrieval, session isolation, and DNS/address validation.
- Preserve trusted UI app resources and IPC. Scope any development-server
  exception explicitly; do not allow arbitrary localhost access.
- Use controlled local endpoints for verification, not services belonging to
  the user. Keep unverified request classes and platforms explicit.

## Remaining work

- Cover HTTP and HTTPS, IPv4 and IPv6 loopback, alternate address representations,
  localhost names, and hostnames resolving to loopback. Exercise navigation,
  redirects, child frames, images, fetch, and WebSocket requests.
- Verify the full bundled main/settings UI separately. Extend beyond the SVG
  fixture to other request mechanisms, including fetch and navigation; preserve
  the intentional IPC exception.
- Choose and implement enforcement for uncovered paths. Evaluate native WebKit
  content-blocking rules for the loader and CSP/navigation restrictions for the
  trusted UI without assuming either candidate is a complete solution.
- Verify startup ordering, proxy-unavailable behavior, and refusal diagnostics.
  Regression checks must show no prohibited destination connection while local
  app resources, IPC, and permitted loader retrieval continue working.
- Update durable security documentation with the enforced boundary and limits.

## Reference

### Existing boundary and test context

- Main/settings use a deny-all proxy and the app CSP. `img-src 'self' data:`
  restricts images to the app origin and embedded data. `connect-src 'self' ipc:
  http://ipc.localhost` restricts fetch connections while retaining Tauri IPC.
  Executing a script does not exempt its requests from CSP. The UI app origin
  is distinct from ordinary HTTP localhost; it is not a filesystem/null origin.
- The article loader permits publisher scripts and remote networking through
  its isolated session and SSRF-filtering proxy. It does not inherit the
  trusted UI's restrictive app CSP. Native loader-configuration fixtures now
  demonstrate direct localhost access from HTTP publisher HTML and top-level SVG.
- `loader_loopback_webview` uses the production native loader configuration and
  loader script, with image-only mode for SVG. HTTP requests to `127.0.0.1` and
  `localhost` reach the controlled destination through fetch, no-CORS fetch, XHR,
  images, scripts, stylesheet imports, frames, beacons, and WebSocket handshakes.
  SVG declarative images also reach it. HTTPS fetch opens direct TLS connections;
  successful TLS/HTTP and WebSocket upgrades are not established. No such requests
  reach the fixture proxy; a direct production-proxy control is refused.
  CORS-enabled local responses are readable by fetch/XHR. The fixture retains
  windows and acknowledges capture IPC for observation; it does not exercise
  persistence or prove the exact request lifetime of a normal capture.
  HTTPS publisher origins, IPv6, aliases, redirects and other platforms remain
  unverified.
- `ui_network_webview` demonstrates HTTP image requests to `127.0.0.1` reaching
  a controlled server from both UI test webviews without reaching their proxies.
  Its custom-protocol page omits the app CSP. It establishes a routing bypass,
  not a demonstrated leak from the bundled application or the article loader.
- `svg_webview` uses the exact app CSP and main/settings native configuration.
  Tested SVG scripts, event handlers, and external resources are blocked before
  reaching either the proxy or destination. Its no-CSP control executes scripts
  and reaches loopback, including SVG image references surviving sanitization.
  This fixture includes the production reader pipeline but is not a full
  packaged-app test or a guarantee covering every request mechanism.

### Probing implications

- Publisher-script execution and remote requests in the isolated loader/downloader
  are intentional. The concern here is access to local services outside that
  intended network boundary, not script execution by itself.
- A cross-origin fetch/XHR response without appropriate CORS permission is not
  exposed to the calling script. A request can nevertheless reach the service;
  some requests require a preflight before the actual request is sent.
- A script generally receives a generic network failure rather than a reliable
  distinction between CORS rejection and connection refusal. A diagnostic shown
  in the browser console is not a diagnostic available to the calling script.
- Individual timings can be ambiguous, but repeated probes across likely-open
  and likely-closed ports provide a baseline. Differences between groups can
  reveal evidence of listening services. There is no fixed millisecond threshold
  or guaranteed direction of the difference. This statistical technique was
  discussed as a threat; the current fixture does not measure its accuracy.
- Timing is only one possible observable. Do not assume that lack of readable
  response bodies makes probing ineffective or exhausts potential attacks.
  Verification must establish that prohibited connections never reach the
  destination, including when the local service provides no CORS permission.

### Upstream context

- WebKit issue 186039, filed in 2018, raises local-service exploitation and
  fingerprinting risks and records competing requirements for legitimate
  browser-to-local-service communication:
  https://bugs.webkit.org/show_bug.cgi?id=186039
- WebKit tracks Local Network Access implementation in issue 250607, with
  permission-decision work in issue 324190. Upstream implementation work does
  not establish enforcement in the installed WKWebView tested here:
  https://bugs.webkit.org/show_bug.cgi?id=250607
  https://bugs.webkit.org/show_bug.cgi?id=324190
- These sources concern website access to local networks generally. They have
  not been established as reports of this exact configured-proxy bypass.

### Reproduction

Run in a macOS GUI session from the repository root:

```sh
XAGA_VERIFY_LOOPBACK_ISOLATION=1 cargo test -p xaga --test ui_network_webview
node scripts/verify-svg-webview.mjs
cargo test -p xaga --test loader_loopback_webview --offline
```

The first command exercises the known proxy-bypass failure; its ordinary test
invocation omits loopback. Inspect destination and proxy observations rather
than relying only on test exit status. The loader diagnostic succeeds when
observations complete, even when they demonstrate a bypass.

### Code and durable documentation

- `crates/xaga-core/src/network_proxy.rs`
- `src-tauri/src/ui_network.rs`, `src-tauri/src/webview_policy.rs`
- `src-tauri/src/article_loader.rs`, `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/tests/ui_network_webview.rs`, `src-tauri/tests/svg_webview.rs`
- `src-tauri/tests/fixtures/svg_probe.ts`, `scripts/verify-svg-webview.mjs`
- `docs/architecture/security/fetching_remote_content.md`
- `docs/architecture/security/handling_persisted_content.md`
- `docs/architecture/security/command_surface.md`
