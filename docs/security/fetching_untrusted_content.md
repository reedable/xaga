# Fetching untrusted remote content: the SSRF proxy

How Xaga secures the *network* side of loading remote content — feeds, article
pages, and PDFs. The companion document,
[handling_untrusted_content.md](handling_untrusted_content.md), covers what
happens to the bytes after they arrive (parsing, processing, persistence,
display).

Code: `src-tauri/src/ssrf_proxy.rs` (the proxy),
`src-tauri/src/article_loader.rs` (loader webview),
`src-tauri/src/commands/articles.rs` (PDF probe),
`src-tauri/src/commands/feeds.rs` (feed fetch),
`src-tauri/src/fetch_client.rs` (client factory), `src-tauri/src/lib.rs`
(wiring).

## Threat model

The app fetches URLs it does not control: feed URLs entered by the user (or
imported from OPML), article links found inside feeds, and every redirect and
subresource those trigger. A hostile publisher can use any of these to aim
requests at targets the app must never reach: `127.0.0.1`, LAN hosts
(routers, printers, dev servers), link-local/cloud-metadata addresses.

A URL-string denylist cannot stop this (see the module docs at
`ssrf_proxy.rs:1-18`):

- **DNS rebinding** — a hostname that looks public can resolve to a private
  address, or resolve publicly once for the check and privately again for the
  connection. The WKWebView does its own DNS resolution, so a check in the app
  never sees what the webview actually connects to.
- **Redirects** — the webview follows redirects inside its own network stack;
  the app never sees the intermediate URLs.
- **Subresources** — a public page can embed `<img src="http://192.168.1.1/…">`
  and the webview will fetch it.

## Design: resolve, validate, pin

The proxy moves name resolution *out* of the client. Every consumer (webview
or reqwest) is pointed at a local HTTP proxy on `127.0.0.1:<random port>`
(bound at startup, `lib.rs`), and for each request the proxy:

1. **Resolves** the target host itself (`validate_target`,
   `ssrf_proxy.rs:132-147`) — on the blocking pool, so DNS never stalls a
   runtime worker.
2. **Rejects** the request if *any* resolved address is disallowed — not
   filtered to the allowed ones. A host publishing both a public and a private
   record is unreachable entirely, closing the "one good record smuggles the
   request past the check" hole.
3. **Dials the exact validated IP** (pinning). There is no second lookup for a
   rebind to exploit.

Because the client never resolves or connects directly, every redirect hop and
every subresource re-enters the proxy and is re-validated.

The proxy speaks two request forms (`parse_request_line`,
`ssrf_proxy.rs:195-213`): `CONNECT host:port` (HTTPS tunneling) and
absolute-form `GET http://…` (plain HTTP). Request heads are capped at
`MAX_HEAD_BYTES` (16 KiB) to bound memory and reject header floods.

## Plaintext HTTP policy

The `http_allowed` setting (default `false`) is enforced in the proxy, not in
the clients: when off, absolute-form `http://` requests are refused with
status `HTTP_BLOCKED_STATUS` (470, defined in `persistence/feeds.rs`) and a
plain-text explanation body. Because every consumer dials through the proxy,
this one refusal point covers the feed client, the PDF client, the loader
webview, its subresources, and every redirect hop — a `https → http`
downgrade redirect from any client arrives as absolute-form http and is
refused. The non-standard status lets fetch paths distinguish "blocked by the
user's own setting" from a real server 4xx: `fetch_xml` maps it to
`FeedError::HttpBlocked`, the PDF probe to its own error string, and the
webview surfaces the explanation body. The proxy holds the setting handle and
reads it per request; it starts before settings load, and the value is
`false` (blocked) until seeded.

Not closed by this: a `CONNECT` to port 80 would tunnel plaintext the proxy
cannot see. Today's consumers do not do this (reqwest uses CONNECT only for
https; the webview uses absolute-form for http), so enforcement rests on
client convention, not on the proxy.

## Address policy

`is_blocked` (`ssrf_proxy.rs:57-105`) is the security-critical core: only
globally-routable addresses pass.

IPv4 (`is_blocked_v4`): loopback `127/8`; private `10/8`, `172.16/12`,
`192.168/16`; link-local `169.254/16` (includes `169.254.169.254` cloud
metadata); unspecified `0.0.0.0`; broadcast; documentation ranges; CGNAT
`100.64/10`; IETF protocol assignments `192.0.0/24`.

IPv6: `::1` and `::`; unique-local `fc00::/7` (includes `fd00:ec2::254` IMDS);
link-local `fe80::/10`; and — the subtle part — every form that *carries* an
IPv4 destination is judged by the embedded v4 address, so a public-looking
IPv6 wrapper cannot smuggle a private IPv4 past the filter:

- IPv4-mapped `::ffff:a.b.c.d`
- NAT64 `64:ff9b::/96`
- deprecated IPv4-compatible `::a.b.c.d`
- 6to4 `2002::/16` (v4 in the 2nd/3rd hextets)

The resolver is injectable (`HostResolver` trait) so tests can simulate DNS —
including rebinding — deterministically and offline.

## Who fetches through it

Everything that touches attacker-influenced URLs:

1. **The article loader webview** (`article_loader.rs:104-110`). Each loader
   window is built with `WebviewWindowBuilder::proxy_url` pointing at the
   proxy, so the webview never resolves DNS or connects directly. Requires the
   `macos-proxy` feature to take effect on macOS.
2. **Per-invocation fetch clients** (`fetch_client.rs`), reqwest clients whose
   `Proxy::all` points at the proxy. A client is built by the factory at each
   command invocation — feed commands build one per call; the PDF probe builds
   one per probe — so settings changes apply on the next fetch without a
   relaunch. The feed and PDF paths use separate client instances. TLS is
   rustls; no cookie jar is configured, so there is no cross-fetch tracking
   state. Each client caps redirects at the `max_redirects` setting
   (default 3, hard ceiling 10, clamped at settings load; 0 = the initial
   request only, verified by test `limited_zero_refuses_first_redirect`).
   The clients do not set `.https_only()` — plaintext enforcement lives in
   the proxy (below). Used for:
   - **Feed XML** — all feed commands (`add_feed`, `refresh_all_feeds`,
     `import_feeds`) fetch through it (`commands/feeds.rs`). Feed URLs are
     normalized to https, so they use the CONNECT path. Every feed request
     carries a per-request total timeout (`fetch_xml`,
     `persistence/feeds.rs`) covering connection negotiation, TLS, request
     send, header wait, and body read; the value comes from the
     `feed_timeout_secs` setting (default 10 seconds,
     `DEFAULT_FEED_TIMEOUT_SECS`), mirroring `loader_timeout_secs`. The
     refresh loop is sequential — one feed at a time — so with N feeds the
     worst-case refresh wall time is N × timeout.
   - **Feed response bodies are byte-capped** (`fetch_xml`,
     `persistence/feeds.rs`): the body is read in chunks and the fetch
     aborts with `FeedError::BodyTooLarge` once the accumulated bytes would
     exceed the cap, bounding the memory a single response can occupy. The
     value comes from the `feed_max_body_bytes` setting (default 10 MiB,
     `DEFAULT_FEED_MAX_BODY_BYTES`), plumbed like `feed_timeout_secs` (no
     setter command; read from settings.json at startup). The declared
     `Content-Length` is not pre-checked — an over-limit response downloads
     up to the cap before aborting. `fix_bare_ampersands` allocates a second
     copy of the body, so peak per-fetch memory is ~2× the cap. A feed whose
     response exceeds the cap fails to add/refresh; compatibility of 10 MiB
     with large legitimate feeds is unmeasured. reqwest is built without the
     `charset` feature, so the chunked read's lossy-UTF-8 decode matches the
     `.text()` it replaced.
   - **The inline-PDF probe** (`try_fetch_pdf`, `commands/articles.rs`),
     which fetches candidate PDFs directly instead of via a webview.

## Fetch-adjacent hardening in the loader

The proxy controls *where* requests can go; the loader adds controls on *what*
gets loaded and what the loaded page can do:

- **Scheme guard** (`article_loader.rs:87-96`): only `http`/`https` URLs are
  ever loaded. Without this, a crafted feed link (`file://`, `data:`, custom
  schemes) would be loaded and its DOM shipped back by the init script —
  local-file and intranet disclosure. The PDF probe mirrors the same guard
  (`commands/articles.rs:151-158`).
- **Ephemeral state by default**: loader windows are incognito — no cookies
  written to disk. Only user-allow-listed domains (the `allowed_domains`
  setting) get a persistent, per-domain `WKWebsiteDataStore` whose identifier
  is derived by hashing the domain (`domain_store_id`), so sites cannot
  correlate visits through shared storage.
- **Injection-safe bootstrap**: the values prepended to the init script are
  emitted via `serde_json::to_string`, so special characters in the URL cannot
  break out of the script context (`article_loader.rs:69-78`).
- **Hidden means hidden**: a hidden loader window re-hides itself on any
  focus event the remote page manages to trigger (`article_loader.rs:154-168`).
- **Kill after capture**: once the page's HTML is delivered, the webview is
  closed immediately so untrusted scripts cannot keep running or spam IPC
  (`receive_html`, `commands/articles.rs:98-120`).
- **Per-request timeout**: the whole load is bounded by the configurable
  `loader_timeout_secs`.
- **PDF spoofing dead-end**: a response claiming `application/pdf` whose body
  lacks the `%PDF-` signature is rejected outright (`E_SPOOFED_PDF`) and never
  rendered in a webview (`PdfProbe::Invalid`).

## Known gaps

- Feeds or articles that resolve to private/loopback addresses
  (intranet/localhost dev feeds) are blocked by the proxy's address policy;
  there is no mechanism to reach them.
- Redirect cap default: the maximum-redirects setting ranges 0–10 (10 is
  always in force; the user cannot exceed it). The most secure default is 0 —
  if a URL does not load in one shot, no redirect is followed. The default is
  3, chosen for UX, not security. This violates
  docs/principles/secure_by_default.md; the project owner granted this
  exception on 2 July 2026.
- The redirect cap applies to the reqwest fetch paths only; the article
  loader webview follows redirects internally, uncapped — no decision made
  yet (`docs/backlog/article_loader_redirect_cap.md`).
- Fetch privacy: no `User-Agent` sent on the reqwest paths (the loader
  webview sends WebKit's default UA) — tracked with options in
  `docs/sprint/secure_feed_fetch.md` (finding 4). The `Referer` part is
  fixed: the `referer_allowed` setting (default false) withholds the header
  on redirect hops unless the user enables it. Conditional GET is rejected
  by decision, not missing: validators are a server-writable tracking
  channel — see [performance_tradeoff.md](performance_tradeoff.md).
- `import_feeds`/`export_feeds` accept an arbitrary `file_path` from the
  webview — tracked with an option in `docs/sprint/secure_feed_fetch.md`
  (finding 5).
- The embedding model download over the unproxied `http_client` fetches
  unpinned, unverified content from a moving third-party ref
  (`resolve/main`) and feeds it to a native parser (onnxruntime) — risks and
  hardening options in `docs/backlog/secure_embedding_model_download.md`.
- Proxy behavior is a property of the bundled app; `tauri dev` runtime checks
  are not representative for webview-level policies (see the CSP note in
  [handling_untrusted_content.md](handling_untrusted_content.md)).
