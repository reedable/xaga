# Spike: article filtering integration feasibility

## Summary

Determine which integration mechanisms can block unwanted requests and content
in Xaga's remote article loaders. No engine or mechanism has been selected.

## Requirements

- Compare uBlock Origin itself, native WebKit content-blocking rules, translation
  of a documented filter-syntax subset, an embedded compatible engine, and proxy
  filtering. Separate Safari extension support from embedded WKWebView support.
- Distinguish domain identification, request matching, cosmetic filtering, and
  script-based rules. Report unsupported syntax and semantic differences rather
  than claiming uBlock Origin compatibility from list parsing alone.
- Use current primary upstream documentation and pinned source revisions. Record
  tested OS, Tauri/Wry versions, configuration, and reproducible evidence.
- Preserve existing isolation, truthful disclosure, timeouts, and per-load
  teardown. Account for opaque HTTPS tunnels and the documented WebKit proxy
  bypass; proxy observations alone cannot establish filtering coverage.

## Remaining work

- Trace the production loader, native policy configuration, proxy, capture,
  favicon/image recovery, and download paths to identify integration points.
- Compare the mechanisms above, including adblock-rust and SafariConverterLib
  from the source idea, for integration effort, dependencies, coverage, limits,
  and maintenance requirements. Do not rank candidates solely by compatibility.
- Build bounded, disposable probes for plausible mechanisms using controlled
  HTTP/HTTPS destinations, main documents, subframes, scripts, images, redirects,
  cosmetic rules, exceptions, and unsupported rules. Verify destination arrivals
  independently of filter/proxy reports and verify installation before navigation.
- Measure startup/compilation time, load latency, and memory with a recorded list
  revision; distinguish measured results from expectations. Record cancellation,
  concurrent loaders, hidden/visible loaders, persistent-domain sessions, and
  rule-installation failures for mechanisms taken forward.
- Present viable alternatives and limits for a user decision, incorporating the
  supply-policy and compatibility spikes before selecting an implementation.
  Completion requires reproducible findings and explicit unresolved decisions;
  production integration belongs to the dependent implementation story.

## References

- [Source idea](../ideas/ublock-origin-integration.md)
- [Supply policy](spike__ublock-origin--supply-policy.md)
- [Compatibility and user controls](spike__ublock-origin--compatibility-controls.md)
- [Loader implementation](enhancement__ublock-origin--loader-filtering.md)
- [Remote-content network policy](../../docs/architecture/security/fetching_remote_content.md)
- [Privacy and resource teardown](../../docs/architecture/security/performance_tradeoff.md)
- [Article loader](../../src-tauri/src/article_loader.rs)
- [Native webview policy](../../src-tauri/src/webview_policy.rs)
- [Network proxy](../../crates/xaga-core/src/network_proxy.rs)
