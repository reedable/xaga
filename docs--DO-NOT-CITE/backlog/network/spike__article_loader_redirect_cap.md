# Spike: article-loader redirect cap

## Summary

Determine whether and where Xaga can enforce `max_redirects` on requests handled
by the WebKit article loader. The loader currently has a total deadline and
per-destination proxy validation, but no redirect-count limit.

## Requirements

The investigation must use the production loader configuration and controlled
redirect endpoints. It must distinguish the initial navigation, server redirects,
user or script navigation, and subresource requests. Proxy observations alone do
not establish redirect count, especially for HTTPS tunnels and reused connections.

## Remaining work

- Test HTTP and HTTPS main-frame redirect chains, both same-origin and
  cross-origin, and record whether `on_navigation` fires once for every server
  redirect before the request proceeds.
- Determine which redirect classes are visible through current Tauri/Wry hooks,
  including response-to-download navigation, and whether a Wry extension would
  expose any missing main-frame events.
- Establish whether subresource redirect chains can be observed and refused. If
  they cannot, separate that limitation from enforceable main-frame behavior.
- Present the enforceable scopes and their tradeoffs for a user decision. The
  result must be sufficient to replace this spike with a concrete implementation
  task or to retain deadline-only handling with a documented limit.

## References

- [Article loader](../../../src-tauri/src/article_loader.rs)
- [Wry navigation policy](https://github.com/reedable/wry/blob/566f7b984147d58e05e69c8dcf8cf6336cc465bb/src/wkwebview/navigation.rs)
- [Network proxy](../../../crates/xaga-core/src/network_proxy.rs)
- [Remote-content policy](../../../docs/architecture/security/fetching_remote_content.md)
- [Fetch-policy settings UI](improvement__user_settings.md)
- [Feed retrieval migration](compliance__eliminate_reqwest.md)
