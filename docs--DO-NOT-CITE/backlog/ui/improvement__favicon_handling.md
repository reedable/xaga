# Define favicon discovery, normalization, and reuse

## Summary

The loader and `entryWorker` select favicons separately and differ in base URL and conventional fallback behavior. Canvas acquisition produces square output while download validation preserves aspect ratio. Broader candidate selection and origin-level reuse are unscoped.

## Requirements

- Define one candidate-selection and URL-resolution contract for loader and fallback extraction.
- Keep all remote favicon acquisition behind the existing isolated downloader and validation controls.
- Define whether persisted favicons preserve aspect ratio or normalize to a square.
- Never render remote favicon URLs from the privileged main window.
- Define freshness, failure, and deletion semantics before sharing icons across entries.

## Remaining work

- Compare declared icons, conventional `/favicon.ico`, touch icons, manifest icons, `sizes="any"`, and multi-size candidates.
- Decide base-URL resolution and retry behavior when a preferred candidate fails.
- Decide normalization behavior for nonsquare and unsupported images.
- Evaluate per-entry storage against an origin cache, including freshness, negative caching, concurrency, read-only use, and garbage collection.
- Implement the selected scope and test discovery, acquisition, persistence, and local-only rendering.

## References

- `src-tauri/src/article_loader.js`
- `src/text/entryWorker.ts`
- `src-tauri/src/loader_download.rs`
- `crates/xaga-core/src/persistence/images.rs`
- `docs/architecture/security/fetching_remote_content.md`
