# Compliance: eliminate all reqwest usage from Xaga

Remove reqwest from Xaga. Move remote fetching to the article loader, preserving
the required security and privacy controls.

## Known reqwest call sites

- **Feed XML** — `fetch_xml` (`crates/xaga-core/src/persistence/feeds.rs`), reached from `add_feed`, `refresh_all_feeds`, and `import_feeds` (`src-tauri/src/commands/feeds.rs`).
- **Embedding model download** — `download_model` (`crates/xaga-core/src/embeddings/model.rs`), plus the CLI's `download-model`, which starts its own proxy instance.
- **Client factory** — `src-tauri/src/fetch_client.rs`, and `network_proxy::proxied_client`.

## What the reqwest paths carry today

Each of these is a property of the reqwest clients specifically. The loader webview does not have them, so each needs a decision — replacement, loader-side equivalent, or accepted loss.

- **No `User-Agent`.** The loader sends WebKit's platform default.
- **No `Accept-Encoding`** unless `gzip_allowed`. The loader negotiates whatever WebKit negotiates.
- **No `Referer` on redirect hops** unless `referer_allowed`.
- **No cookie jar.** The loader is incognito by default, with per-domain persistent stores for `allowed_domains`.
- **No conditional-GET validators.** `ETag`/`If-Modified-Since` are deliberately not sent (see `docs/architecture/security/performance_tradeoff.md`).
- **`feed_max_body_bytes`** — chunked reads aborting with `FeedError::BodyTooLarge`. No webview equivalent exists.
- **`max_redirects`** — enforced on reqwest clients only. The proxy cannot count HTTPS redirect hops; establish loader-side enforcement.
- **`HTTP_BLOCKED_STATUS` (470) mapping** — `fetch_xml` maps the proxy's plaintext refusal to `FeedError::HttpBlocked`.
- **Per-request timeout** from `feed_timeout_secs`, separate from `loader_timeout_secs`.

Verify routing and enforcement for each replacement. WebKit proxy configuration
alone does not establish that every request reaches the proxy.

## Open items

1. Audit production reqwest call sites and their callers before replacement.
2. Feed XML through the loader: the loader returns `LoaderCapture.html`, produced by `document.documentElement.outerHTML` — a re-serialisation of WebKit's parsed DOM, not the response bytes. `fix_bare_ampersands` repairs malformed XML *before* parsing and operates on wire bytes; payload checksums may also require raw bytes. Whether raw bytes can be obtained from a webview load at all is unresolved, and decides whether this is feasible.
3. What WKWebView does when navigated to `application/xml`, `application/rss+xml`, or `application/atom+xml` — render, download, or error document. Unverified.
4. Cost of N sequential loader windows on a full feed refresh. Unmeasured.
5. Choose how to acquire embedding models; their download URL is a compile-time constant with a server-controlled redirect chain.
6. Whether the properties listed above get loader-side equivalents or are accepted losses, item by item.
