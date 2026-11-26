# Compliance: resource response classification and routing

## Summary

Investigate resource classification and routing on macOS using native webviews.
Record whether rendered resources and downloads reach their assigned validator
and consumer under the existing network policy. Remediation is follow-up work.

## Requirements

- Cover these cases across hidden/visible loader top frames and subframes, plus
  main/settings windows and their subframes:

  | Cases                  | Variants                                                                  |
  | ---------------------- | ------------------------------------------------------------------------- |
  | References and origins | Relative URL; cross-origin without CORS                                   |
  | Schemes                | HTTP(S), data, blob, file, unsupported scheme                             |
  | Redirects              | Permitted and policy-forbidden destinations                               |
  | Responses              | Attachment; HTML presented as an image; malformed image; missing resource |

- Exercise image, favicon, and PDF consumers and ordinary rendered responses.
  Record expected routing or refusal for each matrix cell; explain inapplicable
  cases rather than silently omitting them.
- Observe request attribution, validator/consumer selection, captured-content
  preservation, and staging/loader cleanup. Observe controlled destinations as
  well as the proxy when establishing network refusal.
- Use controlled fixtures and production configuration where possible; document
  fixture differences and do not generalize macOS results to other platforms.

## Remaining work

- Extend the native harness for the remaining matrix cells, including HTTPS
  loader requests, WebKit redirect refusals with destination observations,
  remaining child-frame response variants, and production main and Settings
  windows.
- Verify production capture, consumer selection, and cleanup in the remaining
  cases. The HTTP lifecycle and persisted-resource fixtures cover redirects
  through the PDF, favicon, and image consumers.
- Record expected and observed outcomes, inapplicable combinations, and
  reproducible commands for the full matrix.
- Record findings and limits in durable security documentation, create warranted
  follow-up stories, then delete this task.

## References

- [Loader lifecycle](../../src-tauri/src/article_loader.rs)
- [Resource acquisition](../../src-tauri/src/article_loader.js)
- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Validators](../../src-tauri/src/resource_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Existing native smoke cases](../../docs/development/resource_verification.md#native-image-rendering-smoke-cases)
- [Production loader lifecycle evidence](../../docs/architecture/security/fetching_remote_content.md#native-loader-lifecycle-verification)
- [Wry response policy](https://github.com/reedable/wry/blob/343436e2f856f941fb311cde9299fbc95a372901/src/wkwebview/response_policy.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
