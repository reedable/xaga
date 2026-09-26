# Fetching untrusted remote content

How Xaga secures the _network_ side of loading remote content — feeds, article pages, and PDFs. What happens to the bytes after they arrive (parsing, processing, the write into storage) is covered in the companion document, [handling_remote_content.md](handling_remote_content.md). Reading that content back and rendering it is a further boundary, covered in [handling_persisted_content.md](handling_persisted_content.md). Content the user supplies directly rather than fetches (OPML/bookmarks import) is covered in [handling_user_supplied_content.md](handling_user_supplied_content.md).

Code: `crates/xaga-core/src/ssrf_proxy.rs` (the proxy), `src-tauri/src/article_loader.rs` (loader webview), `src-tauri/src/commands/articles.rs` (PDF probe), `src-tauri/src/commands/feeds.rs` (feed fetch), `src-tauri/src/fetch_client.rs` (client factory), `src-tauri/src/lib.rs` (wiring).

## Intent

Three invariants govern every outbound request:

1. **Containment** — a fetch can only reach globally-routable addresses, no matter what URLs, redirects, or subresources an attacker supplies.
2. **Minimal, truthful disclosure** — a fetch reveals as little as possible about the app and the user, and everything it does reveal is true. Privacy is achieved by withholding information, never by misrepresenting it.
3. **Bounded resources** — no response can exhaust the app's memory or stall it indefinitely.

Where an invariant is at odds with UX, the resolution is a user-controllable trade-off whose default is the secure state (docs/principles/secure_by_default.md); see "User-controllable trade-offs".

## Threat model

The app fetches URLs it does not control: feed URLs entered by the user (or imported from OPML), article links found inside feeds, and every redirect and subresource those trigger. A hostile publisher can use any of these to aim requests at targets the app must never reach: `127.0.0.1`, LAN hosts (routers, printers, dev servers), link-local/cloud-metadata addresses.

A URL-string denylist cannot stop this (see the module docs at `ssrf_proxy.rs:1-18`):

- **DNS rebinding** — a hostname that looks public can resolve to a private address, or resolve publicly once for the check and privately again for the connection. The WKWebView does its own DNS resolution, so a check in the app never sees what the webview actually connects to.
- **Redirects** — the webview follows redirects inside its own network stack; the app never sees the intermediate URLs.
- **Subresources** — a public page can embed `<img src="http://192.168.1.1/…">` and the webview will fetch it.

## Containment: the SSRF proxy

The proxy moves name resolution _out_ of the client. Every consumer (webview or reqwest) is pointed at a local HTTP proxy on `127.0.0.1:<random port>` (bound at startup, `lib.rs`), and for each request the proxy:

1. **Rejects local names before DNS**, including `.local` (case-insensitive, with or without a trailing dot) and unqualified names. Otherwise **resolves** the target host itself (`validate_target`, `ssrf_proxy.rs`) — on the blocking pool, so DNS never stalls a runtime worker. DNS names are made absolute to prevent search-domain expansion into `.local`; IP literals skip DNS. This prevents direct local-name requests from triggering mDNS and macOS's Local Network permission prompt before the address filter runs.
2. **Rejects** the request if _any_ resolved address is disallowed — not filtered to the allowed ones. A host publishing both a public and a private record is unreachable entirely, closing the "one good record smuggles the request past the check" hole.
3. **Dials the exact validated IP** (pinning). There is no second lookup for a rebind to exploit.

Because the client never resolves or connects directly, every redirect hop and every subresource re-enters the proxy and is re-validated.

The proxy speaks two request forms (`parse_request_line`, `ssrf_proxy.rs:195-213`): `CONNECT host:port` (HTTPS tunneling) and absolute-form `GET http://…` (plain HTTP).

### Address policy

`is_blocked` (`ssrf_proxy.rs:57-105`) is the security-critical core: only globally-routable addresses pass.

IPv4 (`is_blocked_v4`): loopback `127/8`; private `10/8`, `172.16/12`, `192.168/16`; link-local `169.254/16` (includes `169.254.169.254` cloud metadata); unspecified `0.0.0.0`; broadcast; documentation ranges; CGNAT `100.64/10`; IETF protocol assignments `192.0.0/24`.

IPv6: `::1` and `::`; unique-local `fc00::/7` (includes `fd00:ec2::254` IMDS); link-local `fe80::/10`; and — the subtle part — every form that _carries_ an IPv4 destination is judged by the embedded v4 address, so a public-looking IPv6 wrapper cannot smuggle a private IPv4 past the filter:

- IPv4-mapped `::ffff:a.b.c.d`
- NAT64 `64:ff9b::/96`
- deprecated IPv4-compatible `::a.b.c.d`
- 6to4 `2002::/16` (v4 in the 2nd/3rd hextets)

The resolver is injectable (`HostResolver` trait) so tests can simulate DNS — including rebinding — deterministically and offline.

### Plaintext HTTP enforcement

The `http_allowed` setting (default `false`) is enforced in the proxy, not in the clients: when off, absolute-form `http://` requests are refused with status `HTTP_BLOCKED_STATUS` (470, defined in `persistence/feeds.rs`) and a plain-text explanation body. Because every consumer dials through the proxy, this one refusal point covers the feed client, the PDF client, the loader webview, its subresources, and every redirect hop — a `https → http` downgrade redirect from any client arrives as absolute-form http and is refused. The non-standard status lets fetch paths distinguish "blocked by the user's own setting" from a real server 4xx: `fetch_xml` maps it to `FeedError::HttpBlocked`, the PDF probe to its own error string, and the webview surfaces the explanation body. The proxy holds the setting handle and reads it per request; it starts before settings load, and the value is `false` (blocked) until seeded.

### CONNECT port policy

`CONNECT` establishes an opaque tunnel: once one is up, the proxy cannot see the bytes flowing through it, so it cannot tell TLS from plaintext (or any other protocol) inside the tunnel. The plaintext-HTTP refusal above only sees absolute-form `http://` requests — it does not fire for a `CONNECT host:80` that then carries plaintext HTTP through the tunnel.

The `connect_nonstandard_ports_allowed` setting (default `false`) closes this at the port level instead of the content level: `CONNECT` to any port other than 443 is refused with `403 Forbidden` before address validation or dialing. When `true`, `CONNECT` is permitted to any port (address validation still applies); feeds served on non-443 https ports need this enabled to connect. Same setting-handle pattern as `http_allowed`: read per request, `false` until settings are loaded.

The fetch clients do not set `.https_only()`: enforcing in the proxy covers every consumer at a single point; a client-side `.https_only()` would be a second, partial implementation of the same rule.

### Who dials through it

Every outbound consumer:

1. **The article loader webview** (`article_loader.rs:104-110`). Each loader window is built with `WebviewWindowBuilder::proxy_url` pointing at the proxy, so the webview never resolves DNS or connects directly. Requires the `macos-proxy` feature to take effect on macOS.
2. **Per-invocation fetch clients** (`fetch_client.rs`), reqwest clients whose `Proxy::all` points at the proxy. A client is built by the factory at each command invocation — feed commands build one per call; the PDF probe builds one per probe — so settings changes apply on the next fetch without a relaunch. The feed and PDF paths use separate client instances. TLS is rustls. Used for:
   - **Feed XML** — all feed commands (`add_feed`, `refresh_all_feeds`, `import_feeds`) fetch through it (`commands/feeds.rs`). Feed URLs are normalized to https, so they use the CONNECT path.
   - **The inline-PDF probe** (`try_fetch_pdf`, `commands/articles.rs`), which fetches candidate PDFs directly instead of via a webview.
3. **The embedding model download** (`download_model`, `crates/xaga-core/src/embeddings/model.rs`). Its URL is a compile-time constant, but its redirect chain is server-controlled (`resolve/main` redirects to HF's CDN), so it dials through the proxy like everything else: the app passes a client built by `ssrf_proxy::proxied_client` (`lib.rs`); the CLI's `download-model` command starts its own proxy instance with both settings at their secure defaults.

## What a fetch discloses

On the reqwest fetch paths, a request carries nothing about the app or the user beyond what the connection itself requires. By default a request sends:

- **No `User-Agent`.** The request names neither the app nor its version — the header whose value would otherwise advertise exploitable versions to every polled host. Omission is spec-compliant (RFC 9110 makes the header a SHOULD, not a MUST) and matches the default shape of the client class the requests belong to.
- **No `Accept-Encoding`.** Nothing is negotiated and responses are never decompressed; the bytes counted against `feed_max_body_bytes` are exactly the bytes on the wire. A request that asks for nothing discloses nothing about the build. (Opt-in: `gzip_allowed`, below.)
- **No `Referer` on redirect hops**, so redirect targets don't learn the originating feed URL. (Opt-in: `referer_allowed`.)
- **No cookies** — the clients have no cookie jar, so there is no cross-fetch state a host could use to correlate requests.
- **No conditional-GET validators.** `ETag`/`If-Modified-Since` are not sent: validators are a server-writable tracking channel — see [performance_tradeoff.md](performance_tradeoff.md).

Scope: this covers the reqwest paths (feed XML, PDF probe). The article loader webview sends WebKit's platform-default UA — the genuine identity of the engine making that fetch, consistent with the "everything carried is true" intent.

### Encoding negotiation

Compression is a user-controllable trade-off on top of the negotiate-nothing default: the `gzip_allowed` setting (boolean, default `false`) makes the factory build both clients — feed XML and PDF probe — with gzip negotiation, in lockstep, so the two paths always present the same header shape to hosts; negotiating on only one path would itself be a distinguishing signal. The consequence of enabling it: every fetched host learns that this client negotiates gzip — a truthful statement of capability, but one more property narrowing which software the client could be. In exchange, feed transfers shrink to a fraction of their uncompressed size (typically well over half saved; feed XML compresses heavily).

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
- **Scheme guard** (`article_loader.rs:87-96`): only `http`/`https` URLs are ever loaded. Without this, a crafted feed link (`file://`, `data:`, custom schemes) would be loaded and its DOM shipped back by the init script — local-file and intranet disclosure. The PDF probe mirrors the same guard (`commands/articles.rs:151-158`).
- **Ephemeral state by default**: loader windows are incognito — no cookies written to disk. Only user-allow-listed domains (the `allowed_domains` setting) get a persistent, per-domain `WKWebsiteDataStore` whose identifier is derived by hashing the domain (`domain_store_id`), so sites cannot correlate visits through shared storage.
- **Injection-safe bootstrap**: the values prepended to the init script are emitted via `serde_json::to_string`, so special characters in the URL cannot break out of the script context (`article_loader.rs:69-78`).
- **Hidden means hidden**: a hidden loader window re-hides itself on any focus event the remote page manages to trigger (`article_loader.rs:154-168`).
- **Kill after capture**: once the page's HTML is delivered, the webview is closed immediately so untrusted scripts cannot keep running or spam IPC (`receive_html`, `commands/articles.rs:98-120`).
- **Per-request timeout**: the whole load is bounded by the configurable `loader_timeout_secs`.
- **PDF spoofing dead-end**: a response claiming `application/pdf` whose body lacks the `%PDF-` signature is rejected outright (`E_SPOOFED_PDF`) and never rendered in a webview (`PdfProbe::Invalid`).

## Limits

Places where the invariants are not fully upheld:

- Proxy behavior is a property of the bundled app; `tauri dev` runtime checks are not representative for webview-level policies (see the CSP note in [handling_persisted_content.md](handling_persisted_content.md)).

The macOS native policy has a GUI smoke test: `cargo test -p xaga --test webview_policy`. It checks that standard and prefixed peer-connection constructors are absent at document-start, in a subframe, and after another navigation, using only in-memory HTML and ephemeral storage. Proxy unit tests verify that local names never reach the resolver. These controls do not establish the cause of any previously observed permission prompt.
