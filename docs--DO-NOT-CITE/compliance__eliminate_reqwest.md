# Compliance: eliminate all reqwest usage from Xaga

**Decision (2026-07-28, project owner): reqwest is to be removed from Xaga entirely.** All remote fetching moves to the article loader webview, which the codebase already designates as the fetch context — `article_loader.rs` states that "the loader is the only context permitted to fetch remote content."

This is not an open question. What remains open is sequencing and what replaces each property the reqwest paths currently carry.

Labelled `compliance__` per `docs/conventions/vocabulary/backlog_item_labels.md`: the label names the originator, and `compliance__` takes precedence where a gap is security- or policy-relevant. This item is a policy decision about the app's outbound network surface.

## Known reqwest call sites

Recorded from `docs/architecture/security/fetching_remote_content.md` and from reads taken earlier in the session that produced this document. **Not re-verified against the current tree** — confirm the list before scoping.

- **Feed XML** — `fetch_xml` (`crates/xaga-core/src/persistence/feeds.rs`), reached from `add_feed`, `refresh_all_feeds`, and `import_feeds` (`src-tauri/src/commands/feeds.rs`).
- **Inline-PDF probe** — `try_fetch_pdf` (`src-tauri/src/commands/articles.rs`).
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
- **`max_redirects`** — enforced on reqwest clients only. Enforcing it in the loader is itself unresolved; see `docs--DO-NOT-CITE/backlog/fetch/spike__article_loader_redirect_cap.md`, which records that the proxy cannot count https redirect hops and that Tauri's `on_navigation` behaviour under WKWebView is unverified.
- **`HTTP_BLOCKED_STATUS` (470) mapping** — `fetch_xml` maps the proxy's plaintext refusal to `FeedError::HttpBlocked`.
- **Per-request timeout** from `feed_timeout_secs`, separate from `loader_timeout_secs`.

Containment is unaffected: both reqwest and the loader dial through the network proxy, so DNS resolution, address validation, IP pinning, `http_allowed`, and `connect_nonstandard_ports_allowed` apply identically either way.

## Open items

1. Confirm the call-site list above against the current tree.
2. Feed XML through the loader: the loader returns `LoaderCapture.html`, produced by `document.documentElement.outerHTML` — a re-serialisation of WebKit's parsed DOM, not the response bytes. `fix_bare_ampersands` repairs malformed XML *before* parsing and operates on wire bytes; `docs--DO-NOT-CITE/backlog/fetch/improvement__feed_payload_checksum.md` names raw bytes as a checksum candidate. Whether raw bytes can be obtained from a webview load at all is unresolved, and decides whether this is feasible.
3. What WKWebView does when navigated to `application/xml`, `application/rss+xml`, or `application/atom+xml` — render, download, or error document. Unverified.
4. Cost of N sequential loader windows on a full feed refresh. Unmeasured.
5. PDF probe and model download: each needs its own decision, and the model download's URL is a compile-time constant with a server-controlled redirect chain.
6. Whether the properties listed above get loader-side equivalents or are accepted losses, item by item.

## Related

- `docs--DO-NOT-CITE/backlog/fetch/spike__article_loader_redirect_cap.md` — blocks the redirect-cap item.
