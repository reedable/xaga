# Secure the embedding model download (pending)

The embedding model is downloaded at startup when missing, by
`download_model_if_missing` (`crates/xaga-core/src/embeddings/model.rs`),
shared by the app (`src-tauri/src/lib.rs`, proxied client) and the CLI
(`download_model`, `crates/xaga-cli/src/cmd/embeddings.rs`, own proxy
instance), serialized by the account-wide lock. Fetches go through the SSRF
proxy, stream to disk under a 256 MB per-file cap, are verified against the
pinned manifest (revision + per-file SHA-256, model.rs; trust model:
`docs/architecture/security/embedding_model_trust.md`), and commit via
`fs::replace_dir_atomic`.

## Pending

1. **Stall handling.** The download has no timeout; a stalled server hangs
   the model step indefinitely. Discussion of 2026-07-09: the app degrades
   gracefully during download (UI usable, embeddings reads return empty), so
   a timeout alone converts an invisible hang into an equally invisible
   permanently-degraded session — download/service failures are logged only,
   nothing reaches the UI, nothing retries in-session, and `is_cache_ready`
   cannot distinguish "downloading" from "failed" from "read-only (lock
   held)". Direction undecided: timeout, in-session retry, UI-visible
   download/failure state, on-demand re-trigger, or a combination. New
   download-status events would be `DomainEvent` variants
   (`crates/xaga-core/src/events.rs`), emitted through the funnel in
   `src-tauri/src/events.rs`, named in `src/platform/events.ts`. Overlaps
   `docs-transient/backlog/ui/compliance__oobe_model_download_consent.md`.
