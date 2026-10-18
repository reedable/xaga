# Keep favicon downloads isolated from PDF consumers

## Summary

Verify that a favicon response labeled or shaped as a PDF remains assigned to the
favicon image validator and cannot reach PDF persistence or presentation.

## Requirements

- Select the favicon consumer from the native request purpose, not response MIME
  type, disposition, URL, or filename.
- Validate favicon bytes as an image and apply the existing favicon preparation.
- A PDF response must not reach PDF persistence, inline PDF rendering, or
  default-app opening through a favicon request.
- Rejection must preserve captured article content and complete without favicon
  data.

## Remaining work

- Add a native download case in which a favicon navigation returns a PDF response.
- Assert that the image validator rejects it, no PDF consumer runs, and the
  captured article is returned without favicon data.
- Fix consumer selection or completion only if the case exposes a failure.
- Record the verified request-purpose boundary in the durable remote-content
  documentation.

## References

- `src-tauri/src/loader_download.rs`
- `src-tauri/src/article_loader.rs`
- `src-tauri/src/resource_download.rs`
- `src-tauri/tests/resource_download_webview.rs`
- `docs/architecture/security/fetching_remote_content.md`

