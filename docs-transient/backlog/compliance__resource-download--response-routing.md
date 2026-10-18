# Compliance: resource response classification and routing

## Summary

Verify that rendered resources and native downloads reach only their assigned
validator and consumer under the existing network policy.

## Requirements

- Cover relative references, unusual schemes, cross-origin resources without
  CORS, forbidden redirect destinations, attachment responses, HTML returned as
  an image, malformed images, and missing resources.
- Verify routing in hidden and visible loaders, other windows, and subframes.
- Failures must preserve captured article content and clean up staged files and
  loader-owned resources.
- Update durable security documentation with verified routing behavior.

## Remaining work

- Extend the native harness with the required response and origin cases.
- Exercise each case through every applicable window and frame context.
- Fix classification, attribution, or routing failures found by the matrix.
- Record the resulting routing guarantees and limits.

## References

- [Loader lifecycle](../../src-tauri/src/article_loader.rs)
- [Resource acquisition](../../src-tauri/src/article_loader.js)
- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Validators](../../src-tauri/src/resource_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Wry response policy](https://github.com/reedable/wry/blob/566f7b984147d58e05e69c8dcf8cf6336cc465bb/src/wkwebview/response_policy.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
