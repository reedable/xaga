# Compliance: eliminate reqwest from feed fetching

## Summary

Reduce the attack surface from maintaining separate reqwest and WebKit interfaces
for external content retrieval. Replace reqwest feed fetching with article-loader
downloads to consolidate feed and article retrieval behind the loader’s security
boundary. This is security compliance work: removing a separate external interface
and its independent enforcement path.

## Requirements

- Convert feed responses into downloads before WebKit renders them.
- Atomically save each completed download, apply the existing format repair,
  parse it, merge the data into the local `.atom`, and atomically commit that
  file when complete.
- Reuse the article loader's network policy, isolation, and download lifecycle.
- Preserve feed add, refresh, and import behavior, including failure reporting.

## Remaining work

- Extend the existing Wry PDF response-policy override to support feed downloads.
  Scope the override to feed retrieval so ordinary article navigation retains
  its current behavior.
- Connect completed loader downloads to the existing feed repair, parsing, and
  merge path. Replace `fetch_xml` callers and remove the unused feed client factory.
- Resolve the disposition of feed-specific settings and behavior when moving to
  the loader: request headers, cookies, conditional requests, `feed_max_body_bytes`,
  `max_redirects`, `feed_timeout_secs`, and blocked-request error mapping. Reuse
  existing loader controls; update settings and durable documentation to match
  the resulting behavior.
- Verify XML, RSS, and Atom downloads preserve response content before parsing,
  including feeds requiring `fix_bare_ampersands`. Verify add, refresh, and import
  merge correctly, and failed downloads or parsing leave the local `.atom` unchanged.

## References

- [Wry response policy](../../../../wry/src/wkwebview/navigation.rs)
- [Article loader](../../../src-tauri/src/article_loader.rs)
- [Loader downloads](../../../src-tauri/src/loader_download.rs)
- [Feed persistence](../../../crates/xaga-core/src/persistence/feeds.rs)
- [Feed format repair and parsing](../../../crates/xaga-core/src/atom.rs)
- [Feed commands](../../../src-tauri/src/commands/feeds.rs)
- [Feed client factory](../../../src-tauri/src/fetch_client.rs)
- [Feed navigation probe](../../../src-tauri/tests/feed_navigation_webview.rs)
- [Remote content policy](../../../docs/architecture/security/fetching_remote_content.md)
- [Conditional-request policy](../../../docs/architecture/security/performance_tradeoff.md)
