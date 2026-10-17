# Performance/privacy trade-off: resource teardown is a privacy tool

Xaga tears down its network-facing resources instead of pooling or reusing them: the reqwest fetch clients are built per command invocation (`src-tauri/src/fetch_client.rs`) and discarded afterward, and each article load gets a fresh loader webview that is killed as soon as its HTML is captured (`src-tauri/src/article_loader.rs`). Rebuilding these looks like an inefficiency to be eliminated. It is not only that: the teardown is itself a privacy mechanism, and pooling or reusing these resources would reopen tracking channels that the teardown currently closes.

The underlying principle, shared with the absence of conditional GET (`ETag`/`If-Modified-Since`): **the fetch path keeps no client-side state that a server can write to and read back.** Any such state — a session ticket, a cached validator, a cookie, a warm connection — lets a provider link one request to another and profile the reader. The provider tracking the consumer is the more immediate privacy risk; a third-party observer is secondary.

## Fetch clients (reqwest): what teardown discards

- **TLS session tickets / session cache.** A resumed TLS session tells the server "same client as before" across requests, surviving IP changes. Each factory-built client starts with an empty session cache, so no ticket survives from one refresh to the next; a server cannot link two refresh passes through TLS resumption.
- **Connection pool.** A kept-alive connection reused across fetches is trivially the same client — no inference needed. Per-invocation clients bound connection lifetime to a single command (one feed refresh pass).
- **No cookie jar is configured** on these clients at all, so cookies are refused even within a single invocation.

If the clients were pooled or made long-lived "for performance": session tickets and warm connections would persist across refreshes, giving every feed host a reliable cross-refresh identifier — precisely what refusing `ETag`/`If-Modified-Since` validators avoids. (A long-lived client would also serve stale settings; the per-invocation build is what makes settings apply on the next fetch.)

## Loader webviews: what teardown discards

- **Ephemeral data stores.** Loader windows are incognito by default; only user-allow-listed domains get a persistent store, and those are segregated per domain (`domain_store_id`), so sites cannot correlate visits through shared storage.
- **Kill after capture.** The webview is closed as soon as the page's HTML is delivered, discarding cookies, HTTP cache, script state, and WebKit's own TLS session state along with it.
- **The HTTP cache is a tracking surface, not just a performance feature.** Cached responses carry validators (`ETag`, `Last-Modified`) that the webview would echo back on revalidation — the same server-writable read-back channel as conditional GET, just operated by WebKit instead of by us.

If loader webviews were reused across article loads: cookies, cache validators, and script-observable state would accumulate across sites and across time within the reused instance, letting both the visited sites and any embedded third parties correlate reads that are currently isolated per-load.

## The price, stated plainly

The costs of teardown are: client construction per command, cold connections per refresh pass, full body re-downloads on every refresh (no conditional GET), and webview construction per article. The network cost buys the no-server-writable-state property.

Any future change that pools, caches, or extends the lifetime of these resources should be treated as reopening the channels above and reviewed as a privacy change, not a performance change.
