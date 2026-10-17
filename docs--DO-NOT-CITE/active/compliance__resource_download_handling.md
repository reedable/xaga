# Compliance: resource retrieval and download safety

## Summary

Verify remaining security and lifecycle cases for rendered resources and native
downloads in the article loader.

## Requirements

Resources must reach their assigned validator and consumer under the existing
network policy. Failures must preserve captured article content and clean up
loaders, scoped proxies, and staged files. Persisted-resource handling must respect
read-only storage and local-only main-window rendering.

## Remaining work

- Test oversized image-document navigation to determine whether WebKit resource
  management and the loader deadline suffice before Rust validation; document
  the decision and implement any required limit.
- Extend native coverage to relative references, unusual schemes, cross-origin
  resources without CORS, forbidden redirect destinations, attachment responses,
  HTML returned as an image, malformed images, missing resources, and stalls.
- Test cancellation and concurrent loads, including favicon failure after article
  capture.
- Verify routing in hidden and visible loaders, other windows, and subframes.
  Favicon responses labeled as PDF must not reach PDF persistence, inline PDF
  rendering, or default-app opening.
- Check private staging with macOS Downloads permission undecided and denied.
- Verify resource reuse, missing-file recovery, refresh, and read-only operation,
  including new, legacy, and edited favicon entries.
- Fix failures from these cases and update durable security documentation.

## References

- [Loader lifecycle](../../src-tauri/src/article_loader.rs)
- [Resource acquisition](../../src-tauri/src/article_loader.js)
- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Validators](../../src-tauri/src/resource_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Wry response policy](https://github.com/reedable/wry/blob/566f7b984147d58e05e69c8dcf8cf6336cc465bb/src/wkwebview/response_policy.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
- [Persisted-content policy](../../docs/architecture/security/handling_persisted_content.md)
