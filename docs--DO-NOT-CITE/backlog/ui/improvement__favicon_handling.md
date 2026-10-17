# Define favicon discovery, normalization, storage, and reuse

## Summary

The loader and `entryWorker` select favicons separately and differ in base URL and conventional fallback behavior. Canvas acquisition produces square output while download validation preserves aspect ratio. Acquisition is a separate loader phase, but persistence is coupled to article saving and per-entry image storage; the intended storage and reuse model is undefined.

## Requirements

- Define one candidate-selection and URL-resolution contract for loader and fallback extraction.
- Keep all remote favicon acquisition behind the existing isolated downloader and validation controls.
- Define whether persisted favicons preserve aspect ratio or normalize to a square.
- Never render remote favicon URLs from the privileged main window.
- Decide whether favicons remain per-entry resources or become shared site metadata.
- Define freshness, retry, failure, reference tracking or eviction, read-only behavior, and deletion semantics for the selected storage model.
- Keep favicon storage separate from body-image storage unless the selected model requires shared lifecycle behavior.

## Remaining work

- Compare declared icons, conventional `/favicon.ico`, touch icons, manifest icons, `sizes="any"`, and multi-size candidates.
- Decide base-URL resolution and retry behavior when a preferred candidate fails.
- Decide normalization behavior for nonsquare and unsupported images.
- Trace acquisition through article saving and identify duplicate storage and reload-coupled retries.
- Compare per-entry durable storage, shared durable site metadata, and a disposable origin cache.
- Implement the selected scope and test discovery, acquisition, persistence, and local-only rendering.
- Record the selected storage lifecycle and rationale in durable architecture documentation.

## References

- `src-tauri/src/article_loader.js`
- `src-tauri/src/commands/articles.rs`
- `src/text/entryWorker.ts`
- `src-tauri/src/loader_download.rs`
- `crates/xaga-core/src/persistence/articles.rs`
- `crates/xaga-core/src/persistence/images.rs`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/security/fetching_remote_content.md`
- `docs/architecture/security/handling_persisted_content.md`
