# Compliance: consolidate model downloads into the article loader

## Summary

Reduce the attack surface from separate reqwest and WebKit external retrieval
interfaces. Move embedding model downloads to the article loader's downloader,
with model validation and visible failures through the shared download lifecycle.
Cover the app and CLI download entry points so a separate reqwest path does not
remain after migration.

## Requirements

- Acquire all model files through the loader's downloader and existing network
  policy, isolation, staging, timeout, and cleanup mechanisms.
- Supply a model validator using the pinned revision and per-file SHA-256 manifest.
  Preserve per-file size limits, account-wide locking, and atomic installation of
  the complete validated file set. Failed attempts must not install partial or
  unverified models or replace an existing usable installation.
- Surface download, timeout, validation, installation, and embedding-service startup
  failures to the user. Distinguish downloading, validating, installing, ready,
  failed, and unavailable due to lock contention or read-only operation; do not
  use `is_cache_ready` alone to represent these states.
- Provide an in-session retry entry point that rechecks model presence and acquires
  the account right before starting. Coordinate user initiation with the separate
  first-run consent task.

## Remaining work

- Add a model consumer and completion path to `LoaderDownloads`, which currently
  dispatches only image, icon, and PDF consumers. Extend the Wry response-policy
  override for model requests, including renderable JSON companion files; select
  validation from the native request purpose and manifest, not the response name.
- Adapt the injectable `ByteValidator` staging boundary for model files. Its current
  `finish` reads the whole file into memory and deletes staging after validation;
  support file-based checksum validation and transfer of accepted staged files to
  the model installer without losing the existing bounded-memory behavior.
- Replace the app's model client and connect completion to atomic model-set
  installation and embedding-service startup. Choose a model-appropriate deadline
  using the loader timeout mechanism; the article deadline is not a model-download
  budget. Release staging and the account lease on failure or cancellation.
- Connect loader diagnostics and completion errors to model status through
  `DomainEvent` → the Tauri event funnel → platform events and UI. Existing loader
  diagnostics do not automatically provide model readiness, installation errors,
  service-start errors, or a retry control.
- Add download progress reporting: the current callback interface exposes request
  and completion but no transferred-byte progress. Expose bytes received and total
  when known through the native/Wry boundary and model status events; show
  indeterminate progress when the total is unavailable. Keep validation and
  installation status distinct from transfer completion.
- Decide how `xaga-cli embeddings download-model` invokes the shared downloader,
  which currently lives in the Tauri app. Implement delegation or a shared host,
  or explicitly retire the CLI download command; remove its independent reqwest
  acquisition path and update CLI guidance. If the command remains, preserve its
  explicit `--no-proxy` escape hatch.
- Verify successful multi-file installation, checksum rejection, interrupted and
  timed-out downloads, size-limit rejection, lock contention, retry, and service
  startup failure. Check progress and error visibility, staging cleanup, and
  preservation of an existing installation. Confirm neither model entry point
  retains reqwest fetching; update durable model and fetching documentation.

## References

- [Model acquisition and manifest](../../../crates/xaga-core/src/embeddings/model.rs)
- [App startup](../../../src-tauri/src/lib.rs)
- [CLI model commands](../../../crates/xaga-cli/src/cmd/embeddings.rs)
- [Loader download consumers](../../../src-tauri/src/loader_download.rs)
- [Injectable validation and staging](../../../src-tauri/src/resource_download.rs)
- [Article loader lifecycle](../../../src-tauri/src/article_loader.rs)
- [Wry navigation policy](https://github.com/reedable/wry/blob/566f7b984147d58e05e69c8dcf8cf6336cc465bb/src/wkwebview/navigation.rs)
- [Domain events](../../../crates/xaga-core/src/events.rs)
- [Tauri event funnel](../../../src-tauri/src/events.rs)
- [Platform events](../../../src/platform/events.ts)
- [Model trust](../../../docs/architecture/security/embedding_model_trust.md)
- [Remote content policy](../../../docs/architecture/security/fetching_remote_content.md)
- [First-run model consent](compliance__embedding-model--oobe-download-consent.md)
