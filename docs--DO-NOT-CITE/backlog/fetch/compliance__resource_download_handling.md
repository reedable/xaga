# Compliance: resource retrieval and download safety

Scope: shared security controls for remote resource retrieval and processing,
including resources WebKit renders and those it routes into the download pipeline.

## Resource limits before and during decoding

The image-document branch in `src-tauri/src/article_loader.js` re-encodes an image
through a canvas capped at 64 pixels. That output limit does not bound the
network response size or the source image's decode allocation. Review and
address resource limits for this branch, including byte and source-dimension
limits. Determine what can be enforced before WebKit decodes the image.

## Remaining verification

Extend controlled-endpoint and native-webview coverage for cases not yet
established by the download unit tests and native redirect smoke test:

- Relative resource references, unusual schemes, and author-controlled URLs
  must pass the common security controls regardless of resource type.
- Cross-origin retrieval without CORS and redirects to forbidden proxy destinations.
- Native handling of attachment responses, HTML error responses, malformed or
  oversized images, missing resources, and stalled transfers.
- Cancellation and concurrent loads: no orphaned loader, lingering proxy, or
  partial file; preserve already captured article content.
- Local reuse, missing files, refresh, and read-only storage.
- Native verification of Downloads permission behavior with permission either
  undecided or denied.
- Local-only main-window rendering for new, legacy, and edited favicon entries.

Document verified controls and limitations in durable documentation. Address
failures found by these tests.

## Consumer routing verification

Verify that resource candidates enter only their assigned validator and consumer.
In particular, favicon candidates must not enter PDF persistence, inline rendering,
or default-app opening. Review hidden and visible loaders, other windows, and
subframes with forced PDF download routing. Handler presence alone
cannot scope that routing because Wry supplies a default download handler.
