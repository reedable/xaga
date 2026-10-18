# Compliance: resource loader lifecycle and concurrency

## Summary

Verify cancellation and concurrent article-resource loads, including resource
failure after article capture, and repair lifecycle leaks or cross-load
attribution failures.

## Requirements

- Cover cancellation before and after article capture.
- Cover concurrent loads with successful, failed, and stalled resources.
- Include favicon failure after article capture.
- Keep each response attributed to its originating load.
- Preserve captured article content while cleaning up loaders, scoped proxies,
  and staged files.

## Remaining work

- Add deterministic cancellation and concurrent-load cases to the native harness.
- Verify cleanup and attribution after each terminal event ordering.
- Fix lifecycle or attribution failures found by the tests.
- Update durable loader lifecycle and security documentation.

## References

- [Loader lifecycle](../../src-tauri/src/article_loader.rs)
- [Resource acquisition](../../src-tauri/src/article_loader.js)
- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
