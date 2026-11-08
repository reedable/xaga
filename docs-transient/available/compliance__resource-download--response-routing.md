# Compliance: resource response classification and routing

## Summary

Investigate resource classification and routing on macOS using native webviews.
Record whether rendered resources and downloads reach their assigned validator
and consumer under the existing network policy. Remediation is follow-up work.

## Requirements

- Cover these cases across hidden/visible loader top frames and subframes, plus
  main/settings windows and their subframes:

  | Cases | Variants |
  | --- | --- |
  | References and origins | Relative URL; cross-origin without CORS |
  | Schemes | HTTP(S), data, blob, file, unsupported scheme |
  | Redirects | Permitted and policy-forbidden destinations |
  | Responses | Attachment; HTML presented as an image; malformed image; missing resource |

- Exercise image, favicon, and PDF consumers and ordinary rendered responses.
  Record expected routing or refusal for each matrix cell; explain inapplicable
  cases rather than silently omitting them.
- Observe request attribution, validator/consumer selection, captured-content
  preservation, and staging/loader cleanup. Observe controlled destinations as
  well as the proxy when establishing network refusal.
- Use controlled fixtures and production configuration where possible; document
  fixture differences and do not generalize macOS results to other platforms.

## Remaining work

- Extend the native harness and run the matrix, retaining reproducible commands
  and evidence for passes, failures, and unverified behavior.
- Record findings and limits in durable security documentation, create warranted
  follow-up stories, then delete this task.

## References

- [Loader lifecycle](../../src-tauri/src/article_loader.rs)
- [Resource acquisition](../../src-tauri/src/article_loader.js)
- [Download consumers](../../src-tauri/src/loader_download.rs)
- [Validators](../../src-tauri/src/resource_download.rs)
- [Native tests](../../src-tauri/tests/resource_download_webview.rs)
- [Wry response policy](https://github.com/reedable/wry/blob/566f7b984147d58e05e69c8dcf8cf6336cc465bb/src/wkwebview/response_policy.rs)
- [Remote content policy](../../docs/architecture/security/fetching_remote_content.md)
