# Compliance: bound resources before native validation

## Summary

Determine whether WebKit resource management and the loader deadline sufficiently
bound oversized image-document navigation and stalled resource responses before
Rust receives bytes for validation. Implement a pre-validation limit only if the
existing bounds are insufficient.

## Requirements

- Test oversized image-document navigation and stalled resource responses through
  the native loader path.
- Measure the bytes, duration, and lifecycle behavior observable before Rust
  validation.
- Preserve captured article content when a resource fails or exceeds a bound.
- Clean up the loader, scoped proxy, and staged files on every failure path.
- Document the observed boundary and the selected policy in the durable remote-
  content documentation.

## Remaining work

- Add native cases for oversized image documents and stalled resource responses.
- Determine whether WebKit resource management plus the loader deadline provides
  an adequate bound.
- If it does not, present enforcement options and their tradeoffs for decision.
- Implement and verify the selected limit, if one is required.

## References

- [Loader lifecycle](../../src-tauri/src/article_loader.rs)
- [Resource acquisition](../../src-tauri/src/article_loader.js)
- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Validators](../../src-tauri/src/resource_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
