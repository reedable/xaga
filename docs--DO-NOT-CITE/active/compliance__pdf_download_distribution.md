# Compliance: PDF download distribution

## Summary

Make the patched PDF download dependency reproducible and establish platform support.

## Requirements

Builds must resolve the reviewed Wry patch without a sibling checkout. Only claim
PDF download support on platforms where response-to-download routing is verified.

## Remaining work

- Replace the Wry path override with the reviewed fork's exact Git revision,
  retaining the 0.57.0 patch base. Verify dependency resolution from a clean checkout.
- Evaluate Windows/Linux response-to-download support and document supported
  behavior and limitations.

## References

- [Workspace dependency override](../../Cargo.toml)
- [Wry response policy](../../../wry/src/wkwebview/response_policy.rs)
- [Native download tests](../../src-tauri/tests/resource_download_webview.rs)
- [Loader redirect-cap task](../backlog/network/spike__article_loader_redirect_cap.md)
- [Resource download safety](compliance__resource_download_handling.md)
