# Fetching untrusted remote content

How Xaga secures the _network_ side of loading remote content — feeds, article pages, and PDFs. What happens to the bytes after they arrive (parsing, processing, the write into storage) is covered in the companion document, [handling_remote_content.md](handling_remote_content.md). Reading that content back and rendering it is a further boundary, covered in [handling_persisted_content.md](handling_persisted_content.md). Content the user supplies directly rather than fetches (OPML/bookmarks import) is covered in [handling_user_supplied_content.md](handling_user_supplied_content.md).

Code: `crates/xaga-core/src/network_proxy.rs` (the proxy), `src-tauri/src/article_loader.rs` (loader webview), `src-tauri/src/commands/articles.rs` (PDF persistence and presentation), `src-tauri/src/commands/feeds.rs` (feed fetch), `src-tauri/src/fetch_client.rs` (client factory), `src-tauri/src/lib.rs` (wiring).

## Intent

Three invariants govern every outbound request:

1. **Containment** — a fetch can only reach globally-routable addresses, no matter what URLs, redirects, or subresources an attacker supplies.
2. **Minimal, truthful disclosure** — a fetch reveals as little as possible about the app and the user, and everything it does reveal is true. Privacy is achieved by withholding information, never by misrepresenting it.
3. **Bounded resources** — no response can exhaust the app's memory or stall it indefinitely.

Where an invariant is at odds with UX, the resolution is a user-controllable trade-off whose default is the secure state (docs/principles/secure_by_default.md); see "User-controllable trade-offs".

## Threat model

The app fetches URLs it does not control: feed URLs entered by the user (or imported from OPML), article links found inside feeds, and every redirect and subresource those trigger. A hostile publisher can use any of these to aim requests at targets the app must never reach: `127.0.0.1`, LAN hosts (routers, printers, dev servers), link-local/cloud-metadata addresses.

A URL-string denylist cannot stop this:

- **DNS rebinding** — a hostname that looks public can resolve to a private address, or resolve publicly once for the check and privately again for the connection. The WKWebView does its own DNS resolution, so a check in the app never sees what the webview actually connects to.
- **Redirects** — the webview follows redirects inside its own network stack; the app never sees the intermediate URLs.
- **Subresources** — a public page can embed `<img src="http://192.168.1.1/…">` and the webview will fetch it.

## Main and settings network policy

`ui_network.rs` starts a separate scoped listener for each trusted UI window,
using the same proxy server and observer interface as the article loader. Its
policy rejects parsed HTTP and CONNECT requests before DNS or an upstream
connection. Each macOS UI window has its own nonpersistent WebKit data store,
so its proxy configuration does not replace a loader's configuration. Local
application protocols, data images and Blob presentation do not use this proxy.
Application settings and library persistence remain native operations.

Development permits only the configured loopback development server host and
port, pinned to a loopback address. Bundled builds have no proxy exception.
Unrequested new windows and native downloads are rejected. The existing native
WebRTC and speculative-DNS restrictions remain applied before construction.

**Enforcement gap:** the native `ui_network_webview` test demonstrates that
WebKit bypasses this proxy for HTTP loopback image requests: both trusted
windows reach the test server directly. Consequently, the proxy must not yet be
regarded as a complete network boundary. Image-reference persistence must not
assume complete isolation. Reproduce the failing check with:

```sh
XAGA_VERIFY_LOOPBACK_ISOLATION=1 cargo test -p xaga --test ui_network_webview
```

That proxy-bypass fixture does not apply the bundled application's CSP. The
bundled policy separately restricts `img-src` to `'self' data:`: network image
URLs outside the app origin are refused by WebKit before a request reaches the
proxy. SVG scripts, stylesheets, and fonts are governed by their corresponding
CSP directives, not by `img-src` alone.

`node scripts/verify-svg-webview.mjs` exercises native macOS WebKit fixtures
with the exact CSP read from `tauri.conf.json`, separate main/settings UI data
stores, and deny-all proxies. It compares them with a no-CSP control and records
requests at a controlled loopback server as well as at each proxy. Tested SVG
cases include inline scripts and event handlers, external scripts/styles/fonts,
`image` and `xlink:href`, `use`, `feImage`, foreign-object HTML images, and CSS
images. HTTP and HTTPS image targets include loopback and `.invalid` hosts.
The protected fixtures produce CSP violations and no destination or proxy
requests; the control executes scripts and reaches loopback directly. Data-URI
SVGs used as HTML/CSS images load without requesting their external resources,
including in the no-CSP control.

The same probe runs the production Markdown sanitizer, image preparation, and
reader HTML builder in an opaque-origin `allow-scripts` Blob iframe. A response
from the reader's hash-pinned script confirms the frame is running. These checks
establish the tested rendering boundaries, not complete proxy routing, all SVG
features, or end-to-end behavior of the packaged application. They do not cover
top-level SVG acquisition in the article loader or other operating systems.

`cargo test -p xaga --test loader_loopback_webview --offline` probes HTTP
publisher HTML and top-level SVG with the production loader native configuration
and `article_loader.js` (image-only mode for SVG). A fixture proxy serves the
controlled publisher origin and forwards other requests to the production SSRF
proxy. The publisher responses supply no CSP, as a remote publisher is permitted
to do; the trusted UI CSP is not injected. The fixture retains the windows and
acknowledges capture IPC so requests can be observed independently of capture
completion; it is not an end-to-end persistence test.

On macOS WebKit, both fixtures reached `127.0.0.1` and `localhost` directly via
HTTP fetch, no-CORS fetch, XHR, images, scripts, stylesheet imports, child frames,
beacons, and WebSocket handshake requests. The top-level SVG also requested its
declarative image. These requests bypassed the configured proxy. A direct control
request through the production proxy was refused before reaching the destination.
Fetch and XHR read the local fixture response because that fixture explicitly
allowed CORS; this does not establish arbitrary cross-origin response readability.
HTTPS fetch attempts also opened direct TLS connections to the controlled local
listener; the listener does not serve TLS, so successful HTTPS requests were not
established. WebSocket handshake arrival does not establish a successful upgrade.

This diagnostic exits successfully when its controls and observations complete,
not when containment is proven. It covers an HTTP publisher origin, IPv4 loopback
and the localhost name, not HTTPS publisher origins, IPv6, address aliases,
redirects, all request classes, or other platforms. Loader loopback containment
therefore remains an enforcement gap.

Refusals that reach the proxy use the existing error sink with “Remote content
was blocked.” and a Details action. Diagnostics identify the window and target
host/port, not the originating article or DOM element. Requests already blocked
by CSP do not reach the proxy and do not produce a proxy notification.

## Containment: the network proxy’s SSRF policy

The network proxy supports SSRF and deny-all policies. Its SSRF policy moves name resolution _out_ of the client. Every consumer (webview or reqwest) is pointed at a local HTTP proxy on `127.0.0.1:<random port>` (bound at startup, `lib.rs`), and for each request the proxy:

1. **Rejects local names before DNS**, including `.local` (case-insensitive, with or without a trailing dot) and unqualified names. Otherwise **resolves** the target host itself (`validate_target`, `network_proxy.rs`) — on the blocking pool, so DNS never stalls a runtime worker. DNS names are made absolute to prevent search-domain expansion into `.local`; IP literals skip DNS. This prevents direct local-name requests from triggering mDNS and macOS's Local Network permission prompt before the address filter runs.
2. **Rejects** the request if _any_ resolved address is disallowed — not filtered to the allowed ones. A host publishing both a public and a private record is unreachable entirely, closing the "one good record smuggles the request past the check" hole.
3. **Dials the exact validated IP** (pinning). There is no second lookup for a rebind to exploit.

Requests routed through the proxy are re-validated on every new connection, including redirects and subresources. This does not cover WebKit’s demonstrated loopback bypass described above.

Article loads use a scoped listener built from the same proxy configuration, so
network failures can be attributed to the load without adding outbound headers.
Its lifetime follows the loader window; other consumers retain the shared
startup listener. See [article loading diagnostics](../article_loading_diagnostics.md)
for collection and observation limits.

The proxy speaks two request forms (`parse_request_line`, `network_proxy.rs`): `CONNECT host:port` (HTTPS tunneling) and absolute-form `GET http://…` (plain HTTP).

### Address policy

`is_blocked` (`network_proxy.rs`) is the security-critical core: only globally-routable addresses pass.

IPv4 (`is_blocked_v4`): loopback `127/8`; private `10/8`, `172.16/12`, `192.168/16`; link-local `169.254/16` (includes `169.254.169.254` cloud metadata); unspecified `0.0.0.0`; broadcast; documentation ranges; CGNAT `100.64/10`; IETF protocol assignments `192.0.0/24`.

IPv6: `::1` and `::`; unique-local `fc00::/7` (includes `fd00:ec2::254` IMDS); link-local `fe80::/10`; and — the subtle part — every form that _carries_ an IPv4 destination is judged by the embedded v4 address, so a public-looking IPv6 wrapper cannot smuggle a private IPv4 past the filter:

- IPv4-mapped `::ffff:a.b.c.d`
- NAT64 `64:ff9b::/96`
- deprecated IPv4-compatible `::a.b.c.d`
- 6to4 `2002::/16` (v4 in the 2nd/3rd hextets)

The resolver is injectable (`HostResolver` trait) so tests can simulate DNS — including rebinding — deterministically and offline.

### Plaintext HTTP enforcement

The `http_allowed` setting (default `false`) is enforced in the proxy, not in the clients: when off, absolute-form `http://` requests are refused with status `HTTP_BLOCKED_STATUS` (470, defined in `persistence/feeds.rs`) and a plain-text explanation body. Because every consumer dials through the proxy, this one refusal point covers the feed client, the loader webview, its subresources, and every redirect hop — a `https → http` downgrade redirect from any client arrives as absolute-form http and is refused. The non-standard status lets fetch paths distinguish "blocked by the user's own setting" from a real server 4xx: `fetch_xml` maps it to `FeedError::HttpBlocked`, and the webview surfaces the explanation body. The proxy holds the setting handle and reads it per request; it starts before settings load, and the value is `false` (blocked) until seeded.

### CONNECT port policy

`CONNECT` establishes an opaque tunnel: once one is up, the proxy cannot see the bytes flowing through it, so it cannot tell TLS from plaintext (or any other protocol) inside the tunnel. The plaintext-HTTP refusal above only sees absolute-form `http://` requests — it does not fire for a `CONNECT host:80` that then carries plaintext HTTP through the tunnel.

The `connect_nonstandard_ports_allowed` setting (default `false`) closes this at the port level instead of the content level: `CONNECT` to any port other than 443 is refused with `403 Forbidden` before address validation or dialing. When `true`, `CONNECT` is permitted to any port (address validation still applies); feeds served on non-443 https ports need this enabled to connect. Same setting-handle pattern as `http_allowed`: read per request, `false` until settings are loaded.

The fetch clients do not set `.https_only()`: enforcing in the proxy covers every consumer at a single point; a client-side `.https_only()` would be a second, partial implementation of the same rule.

### Who dials through it

Every outbound consumer:

1. **The article loader webview** (`article_loader.rs:104-110`). Each loader window is built with `WebviewWindowBuilder::proxy_url` pointing at the proxy. The proxy validates requests it receives; the WebKit loopback bypass described above remains outside that enforcement. Requires the `macos-proxy` feature to take effect on macOS.
2. **Per-invocation fetch clients** (`fetch_client.rs`), reqwest clients whose `Proxy::all` points at the proxy. A client is built by the factory at each command invocation — feed commands build one per call — so settings changes apply on the next fetch without a relaunch. TLS is rustls. Used for:
   - **Feed XML** — all feed commands (`add_feed`, `refresh_all_feeds`, `import_feeds`) fetch through it (`commands/feeds.rs`). Feed URLs are normalized to https, so they use the CONNECT path.
3. **The embedding model download** (`download_model`, `crates/xaga-core/src/embeddings/model.rs`). Its URL is a compile-time constant, but its redirect chain is server-controlled (`resolve/main` redirects to HF's CDN), so it dials through the proxy like everything else: the app passes a client built by `network_proxy::proxied_client` (`lib.rs`); the CLI's `download-model` command starts its own proxy instance with both settings at their secure defaults. Download verifies every streamed file against the pinned manifest before installation. Each later load re-verifies the already-read installed bytes before fastembed or ONNX Runtime receives them; mismatch fails locally without deleting files or initiating network recovery.

## What a fetch discloses

On the reqwest fetch paths, a request carries nothing about the app or the user beyond what the connection itself requires. By default a request sends:

- **No `User-Agent`.** The request names neither the app nor its version — the header whose value would otherwise advertise exploitable versions to every polled host. Omission is spec-compliant (RFC 9110 makes the header a SHOULD, not a MUST) and matches the default shape of the client class the requests belong to.
- **No `Accept-Encoding`.** Nothing is negotiated and responses are never decompressed; the bytes counted against `feed_max_body_bytes` are exactly the bytes on the wire. A request that asks for nothing discloses nothing about the build. (Opt-in: `gzip_allowed`, below.)
- **No `Referer` on redirect hops**, so redirect targets don't learn the originating feed URL. (Opt-in: `referer_allowed`.)
- **No cookies** — the clients have no cookie jar, so there is no cross-fetch state a host could use to correlate requests.
- **No conditional-GET validators.** `ETag`/`If-Modified-Since` are not sent: validators are a server-writable tracking channel — see [performance_tradeoff.md](performance_tradeoff.md).

Scope: this covers the reqwest paths (feed XML). The article loader webview sends WebKit's platform-default UA — the genuine identity of the engine making that fetch, consistent with the "everything carried is true" intent.

### Encoding negotiation

Compression is a user-controllable trade-off on top of the negotiate-nothing default: the `gzip_allowed` setting (boolean, default `false`) makes the factory build feed clients with gzip negotiation. The consequence of enabling it: every fetched host learns that this client negotiates gzip — a truthful statement of capability, but one more property narrowing which software the client could be. In exchange, feed transfers shrink to a fraction of their uncompressed size (typically well over half saved; feed XML compresses heavily).

The body cap keeps its meaning under compression: reqwest inflates before `chunk()` yields bytes, so `feed_max_body_bytes` counts _decompressed_ bytes and a small compressed response cannot smuggle an oversized body past the cap. This decompression-bomb bound is pinned by test (`persistence::feeds::tests::gzip_cap_counts_decompressed_bytes`), and peak per-fetch memory stays bounded by the cap exactly as in the uncompressed case.

## Resource bounds

- **Request heads** into the proxy are capped at `MAX_HEAD_BYTES` (16 KiB) to bound memory and reject header floods.
- **Per-request timeout** — every feed request carries a total timeout (`fetch_xml`, `persistence/feeds.rs`) covering connection negotiation, TLS, request send, header wait, and body read; the value comes from the `feed_timeout_secs` setting (default 10 seconds, `DEFAULT_FEED_TIMEOUT_SECS`), mirroring `loader_timeout_secs`. The refresh loop is sequential — one feed at a time — so with N feeds the worst-case refresh wall time is N × timeout.
- **Body byte cap** — feed response bodies are read in chunks and the fetch aborts with `FeedError::BodyTooLarge` once the accumulated bytes would exceed the cap (`fetch_xml`, `persistence/feeds.rs`), bounding the memory a single response can occupy. The value comes from the `feed_max_body_bytes` setting (default 10 MiB, `DEFAULT_FEED_MAX_BODY_BYTES`). The declared `Content-Length` is not pre-checked — an over-limit response downloads up to the cap before aborting. `fix_bare_ampersands` allocates a second copy of the body, so peak per-fetch memory is ~2× the cap. A feed whose response exceeds the cap fails to add/refresh; compatibility of 10 MiB with large legitimate feeds is unmeasured. reqwest is built without the `charset` feature, so the chunked read's lossy-UTF-8 decode matches the `.text()` it replaced. Under gzip the cap counts decompressed bytes (see "Encoding negotiation").
- **Redirect cap** — each reqwest client caps redirects at the `max_redirects` setting (default 3, hard ceiling 10, clamped at settings load; 0 = the initial request only, verified by test `limited_zero_refuses_first_redirect`).

## User-controllable trade-offs

Fetch behavior that trades an invariant against UX is exposed as a setting whose default is the secure state, per docs/principles/secure_by_default.md; enabling one is the user accepting the stated consequence.

- **`http_allowed`** (default `false`) — allow plaintext `http://` fetches. Consequence: traffic to such feeds is readable and modifiable on path.
- **`max_redirects`** (default 3, range 0–10; 10 is always in force, the user cannot exceed it) — redirect hops honored on the reqwest paths. Consequence of raising it: each permitted hop is another attacker-influenced URL the app will follow. The default itself is a recorded exception — see below.
- **`referer_allowed`** (default `false`) — send `Referer` on redirect hops. Consequence: redirect targets learn the originating feed URL.
- **`gzip_allowed`** (default `false`) — negotiate gzip on the reqwest paths. Consequence: hosts learn the client negotiates gzip (see "Encoding negotiation").
- **`allowed_domains`** (default empty) — domains granted persistent webview storage in the loader (see below). Consequence: those sites can recognize a returning reader.
- **`connect_nonstandard_ports_allowed`** (default `false`) — allow `CONNECT` to target a port other than 443. Consequence: `CONNECT` to any port is tunneled opaquely, so a plaintext (or otherwise non-TLS) service on a non-443 port is reachable through the tunnel with no visibility into what the tunneled bytes are.

### Recorded exception to Secure by Default

Every setting above defaults to its secure state, with one exception: **the `max_redirects` default is 3, not 0.** The most secure default is 0 — if a URL does not load in one shot, no redirect is followed. The default of 3 serves UX, not security, and violates docs/principles/secure_by_default.md. Any change to this default, or to the principle, must revisit this record.

## Fetch-adjacent hardening in the loader

The proxy controls _where_ requests can go; the loader adds controls on _what_ gets loaded and what the loaded page can do:

- **WebRTC disabled on macOS** (`webview_policy.rs`): every Xaga-created WebView receives a native `WKWebViewConfiguration` with peer connections disabled before construction and the first navigation. This applies to subframes and later navigations, including persistent allow-listed loader sessions. JavaScript cannot undo the engine preference. The WebKit `_setPeerConnectionEnabled:` SPI is availability-checked and read back; if enforcement fails, the WebView is not created. The main window is built explicitly in setup for the same reason. This SPI is private API, as is the existing macOS proxy integration. Other platforms do not yet have an equivalent native WebRTC policy.
- **Remote popups denied**: loader content cannot create unmanaged WebViews through `window.open` or new-window links.
- **Speculative DNS disabled on macOS**: the same native configuration disables and verifies `_dnsPrefetchingEnabled` before construction. WebKit's speculative lookups can otherwise bypass the HTTP proxy. Normal proxied requests still use the proxy's pre-resolution hostname policy.
- **Scheme guard** (`article_loader.rs:87-96`): only `http`/`https` URLs are ever loaded. Without this, a crafted feed link (`file://`, `data:`, custom schemes) would be loaded and its DOM shipped back by the init script — local-file and intranet disclosure.
- **Ephemeral state by default**: loader windows are incognito — no cookies written to disk. Only user-allow-listed domains (the `allowed_domains` setting) get a persistent, per-domain `WKWebsiteDataStore` whose identifier is derived by hashing the domain (`domain_store_id`), so sites cannot correlate visits through shared storage.
- **Injection-safe bootstrap**: the values prepended to the init script are emitted via `serde_json::to_string`, so special characters in the URL cannot break out of the script context (`article_loader.rs:69-78`).
- **Hidden means hidden**: a hidden loader window re-hides itself on any focus event the remote page manages to trigger (`article_loader.rs:154-168`).
- **Kill after capture (hidden loaders)**: once the page's HTML is delivered, the hidden webview is closed immediately so untrusted scripts cannot keep running or spam IPC (`receive_html`, `commands/articles.rs:98-120`).
- **Per-request timeout**: the whole load is bounded by the configurable `loader_timeout_secs`.
- **PDF format validation**: the article download consumer requires a `%PDF-` prefix before persistence or presentation. Other formats produce `article:unsupported-download`; rejection does not imply a deceptive server.

## Loader fingerprint exposure

Ephemeral storage does not make the loader indistinguishable from Safari or
remove stable device/browser characteristics. The native `fingerprint_webview`
probe compares the same local page in ordinary Safari and two fresh loader data
stores, using the production native policy and loader initialization script.
Run `cargo test -p xaga --test fingerprint_webview --offline` in a macOS GUI
session. It opens a Safari tab and saves measurements locally to
`/tmp/xaga-fingerprint-comparison.json`; it contacts no public endpoint.

The measured loader user agent omits Safari's `Version/... Safari/...` suffix
in both JavaScript and request headers. WebRTC is unavailable in the loader but
constructible in Safari. The hidden loader reports a different viewport and zero
outer-window dimensions. Page scripts can see the loader's named globals,
including `__xagaCaptureImage` and `__LOADER_ID__`, which provide a direct Xaga
implementation signature beyond generic embedded-WebKit detection.

On the tested machine, canvas and offline-audio hashes, sampled font widths,
WebGL details, screen information, language, timezone, and hardware concurrency
matched Safari. All recorded fields were identical across the two fresh loader
stores. These results show stable observable signals, not a measured probability
of identifying a person or matching them across devices.

Scope: one machine, ordinary Safari with its existing preferences, and a
loopback HTTP origin (a secure context). The fixture retains loader windows by
acknowledging capture IPC and does not configure the network proxy. It measures
page-visible characteristics and the received user agent, not remote-origin
privacy behavior, TLS fingerprints, private Safari browsing, or population
uniqueness. Raw measurements contain machine characteristics and remain in /tmp.

## Visible loader: offer to update on close

A user-opened article loader remains open after its initial capture. When the
user closes it, capture the current page again and deliver `loader:closed` before
closing the window. Completing the initial capture removes the pending request;
that absence must not suppress the close capture or the update prompt.

For the current article, when native attribution confirms the same page and HTML
extraction produces Markdown, the reader must offer **Update article?** with
**Keep old** and **Update**. Closing the loader does not authorize a save: only
**Update** saves the extracted content to the locally stored `.md`. **Keep old**
or dismissing the dialog leaves the saved article unchanged. A different page
or a PDF does not trigger this HTML article update prompt.

Active native downloads and programmatic download cleanup close without an HTML
capture. The second close, after the close capture has been delivered, must pass
through without starting another capture.

Run `sh scripts/test-article-update-on-close.sh` for the regression checks of the
native close decision, JavaScript recapture, and reader confirmation/save flow.

## Limits

Places where the invariants are not fully upheld:

- Proxy behavior is a property of the bundled app; `tauri dev` runtime checks are not representative for webview-level policies (see the CSP note in [handling_persisted_content.md](handling_persisted_content.md)).

The macOS native policy has a GUI smoke test: `cargo test -p xaga --test webview_policy`. It checks that standard and prefixed peer-connection constructors are absent at document-start, in a subframe, and after another navigation, using only in-memory HTML and ephemeral storage. Proxy unit tests verify that local names never reach the resolver. These controls do not establish the cause of an operating-system permission prompt.

## Native resource downloads

`resource_download.rs` owns app-generated temporary staging paths and the
explicit `ByteValidator` boundary. It performs no network requests and no
library writes. A consumer supplies a validator and a byte limit. Successful
transport alone cannot release content: the staged regular file is read with a
cap and the validator must return accepted output. Server filenames and callback
completion paths do not choose which file is read. Each request retains its own
assigned path, including on macOS where completion reports no path.

`loader_download.rs` is the article-loader adapter. It accepts at most one
resource per loader. The native request ID and pending capture phase choose
the consumer: PDF before article HTML capture, favicon during favicon capture,
or content image for an explicitly image-purpose request. Content images can
start directly from an image URL without a preceding article capture. Downloads
without a pending consumer are rejected. Renderer callbacks cannot replace a
capture while its native download is active. The callback may report a redirected URL, as
verified on macOS. It is not proof of the initiating frame. During the favicon
phase the first accepted candidate must pass the image validator regardless of
its URL; it cannot become arbitrary persisted content. The original requested
favicon URL remains storage identity. Completion is matched to the accepted
callback URL, ignoring fragments. Other backends need native verification.

Favicon and recovery responses use the same `ImageValidator`, for both native
downloads and image-document callbacks. Image documents supply original bytes
through the bounded same-origin fetch; favicon acquisition does not use a canvas
capture. Renderer MIME claims do not determine acceptance. Both consumers have
the same 8 MiB input limit and format checks. After validation, favicon processing
thumbnails PNG/JPEG/GIF/ICO/WebP to at most 64-by-64 with preserved aspect ratio
and PNG encoding. SVG and AVIF retain their validated original bytes because the
Rust thumbnail decoder does not support them. Recovery always retains original
bytes. The resulting data URI rejoins `LoaderCapture`. The library writer stores
it at the extensionless shared path `favicons/<normalized-url-hash>` and records
only that local reference in the article. A successful later remote load
atomically replaces the same identity; acquisition failure preserves the cached
resource. PDF validation remains separate.

Standalone SVG image documents are recognized by their image content type;
capture does not require an HTML `img` element or canvas readback. This supports
SVG favicons as well as summary/body image recovery through the shared path.
Navigation can execute SVG scripts and load subresources in the isolated loader,
just as publisher HTML can. Script execution there is intentional; byte validation
is not an execution barrier. The confirmed loopback proxy-bypass limitation
described above applies to this acquisition context too.

Disk growth is checked every 50 ms during transfer. Crossing the consumer limit (8 MiB for icons, 64 MiB for PDFs) completes the
capture with failure and lets the normal loader lifetime close the hidden window.
This sampled guard is not a hard disk quota: writes can overshoot between checks.
The existing overall loader deadline remains in force. No per-download cancel or
progress API is exposed by the current Tauri callback. Temporary directories are
removed on normal completion, validation rejection, timeout, and window cleanup;
process crashes may leave OS-temporary remnants. Automatic crash-recovery cleanup
is not implemented. Decoding runs off the native UI thread; late results after a
load has ended are discarded rather than persisted.

The PDF consumer uses the shared validator contract to enforce a `%PDF-` prefix
and a 64 MiB limit, then commits accepted bytes through the library writer.
Xaga enables the local Wry PDF response-routing feature by default on macOS.
Local reuse, presentation, and the patch policy are described in
`docs/architecture/wry_pdf_routing.md`. The PDF reqwest probe is removed.
WebKit navigation does not use the reqwest redirect-count setting; destination
checks apply to requests that reach the proxy, subject to the loopback bypass
described above, and the loader deadline bounds waiting.


## Direct content-image recovery

The shared loader accepts an explicit native image purpose in addition to the
article purpose. Image recovery retains the loader's HTTP(S) scheme check,
proxy/session configuration, WebRTC/DNS policy, timeout and teardown. It does
not navigate a visible article window away from the user's page.

For a displayed image document, the loader reads original bytes with a bounded
same-origin fetch of the final document URL (`credentials: omit`, `force-cache`).
This can issue an additional request when the response is not reusable from
cache. It avoids cross-origin article-page readback. Native downloads use the same temporary staging boundary with
an image consumer. Both paths pass through `ImageValidator`; renderer-supplied
MIME is not trusted as proof of format. The `receive_image` callback requires the
invoking loader's matching request ID and image purpose, and rechecks ownership
after validation so cancellation or a native download can win safely.

Content-image validation currently decodes PNG, JPEG, GIF, WebP and ICO using the
existing Rust codecs, with a 32 MiB decoder allocation budget, and returns the
original bytes rather than a re-encoded thumbnail. Dimensions and animation are
therefore retained. This is not a claim that every animation frame was decoded
by validation. SVG is checked with a non-rendering XML parser for a complete
SVG root in the SVG namespace; validation does not execute scripts, resolve
external entities, or fetch resources. It does not sanitize the original SVG.
AVIF is checked using `avif-parse` for container structure and AV1 primary/alpha
metadata, with a primary-image limit of 8 megapixels; this is not pixel decoding.
The parser accepts still AVIF, not animated `avis` containers. Existing stored
files still follow the local reader path. Acquisition continues to navigate in
the isolated loader; these byte checks do not disable publisher scripts during
that preceding navigation.

Acquisition and readback are capped at 8 MiB per resource; a recovery invocation
uses the existing 24 MiB decoded/32 MiB base64 transport budget and a shared
loader-timeout budget. Duplicate URLs are requested once per invocation, and
already-persisted resources for the same URL are reused locally. Native download
size monitoring remains sampled, not a hard disk quota. Favicon-specific
thumbnailing and PDF validation remain separate consumers. Image recovery
updates only image references through the entry writer; it does not save an
article response or replace a PDF.
