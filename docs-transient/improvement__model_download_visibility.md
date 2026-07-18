# Model download: no status, no recovery in-session (pending)

The startup model download is invisible to the user and unrecoverable within a session:

- Download and service-start failures are logged only (`src-tauri/src/lib.rs`, setup task); nothing reaches the UI.
- The download client sets no timeout: a stalled server leaves the session silently degraded until relaunch. (The account lock stays held for the stall's duration.)
- `is_cache_ready` is the UI's only readiness signal, a bare boolean that reports `false` identically for "downloading, ready shortly," "failed, never ready this session," and "read-only instance."
- Nothing retries in-session and no user-facing trigger exists; recovery is relaunching the app (or `xaga-cli embeddings download-model`).

Direction undecided: timeout, in-session retry, UI-visible download/failure status, on-demand re-trigger, or a combination.

Implementation facts for whichever direction is chosen:

- Download-status events have a home: `DomainEvent` variants (`crates/xaga-core/src/events.rs`) → emit funnel (`src-tauri/src/events.rs`) → `src/platform/events.ts`.
- The download (`model_present` gate + `download_model`) is re-invokable at any time (no-op when files are present, account-lock-serialized by the caller holding the right, checksum-verified).
- Overlaps `docs-transient/backlog/ui/compliance__oobe_model_download_consent.md` (first-run consent wants the same status surface and trigger).
