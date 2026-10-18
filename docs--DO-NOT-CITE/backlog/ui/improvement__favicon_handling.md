# Complete favicon discovery and cleanup

## Summary

The loader selects one `rel="icon"` URL and falls back to `/favicon.ico`. It does not consider touch or manifest icons and cannot try another candidate when acquisition fails. Shared favicon files also have no cleanup path.

## Requirements

- Define a deterministic candidate order covering declared icons, touch icons, manifest icons, and `/favicon.ico`.
- Resolve relative candidates against the document base URL.
- Try the next candidate when a candidate cannot be fetched or validated.
- Prefer an appropriate display size without stretching nonsquare images.
- Keep acquisition in the isolated loader and render only persisted local bytes in the main window.
- Preserve an article's existing favicon when every candidate fails.
- Remove shared favicon files only when no article references them.

## Remaining work

- Replace the current exact `32x32`, then `16x16` preference with ordered candidate discovery covering the required icon sources and size forms.
- Add touch-icon and web-manifest discovery.
- Pass multiple candidates through the loader so acquisition can continue after a fetch or validation failure.
- Define and implement cleanup of unreferenced files under `favicons/`.
- Add coverage for candidate ordering, relative URLs, fallback after failure, a changed favicon URL, read-only display, and late loader callbacks during favicon navigation.
- Verify the discovery and fallback behavior in the native webviews on macOS, Windows, and Linux.

## References

- `src-tauri/src/article_loader.js`
- `src-tauri/src/commands/articles.rs`
- `src-tauri/src/loader_download.rs`
- `crates/xaga-core/src/persistence/articles.rs`
- `crates/xaga-core/src/persistence/images.rs`
- `docs/architecture/security/fetching_remote_content.md`
- `docs/architecture/security/handling_persisted_content.md`
