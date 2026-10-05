# OOBE: user chooses whether to download the embedding model

## The concern

A fresh install (or wiped model dir) contacts Hugging Face at app startup, unprompted — revealing install/launch timing and IP to a third party the user never chose. The download mechanism itself is not the problem (the CLI invocation of the same path is user-initiated); the problem is the app deciding to make network contact without the user. It is currently the app's only unconditional unprompted outbound contact: the other automatic fetch, startup feed refresh, is opt-in (`refresh_on_startup`, default false) and goes to user-chosen hosts.

## Work

An out-of-box experience: on first run (model files missing), the app asks before downloading — e.g. "Xaga uses a local embedding model (~91 MB, downloaded once from huggingface.co) for related-item search. Download now?" — with decline leaving the app in its existing degraded mode (embeddings reads return empty; everything else works) and a way to trigger the download later.

## Facts available to the implementation

- The download is consent-shaped already: `model_present` gates it (no-op when files are present) and `download_model` (`crates/xaga-core/src/embeddings/model.rs`) does the fetch, returning a typed `DownloadOutcome`. The **caller** holds the account-wide lock across it (the app via an in-process account lease, the CLI via a direct flock). It is callable at any time — deferring it from startup to a user-triggered moment is a call-site change, not a mechanism change.
- Download-status/consent events have a home: `DomainEvent` (`crates/xaga-core/src/events.rs`) → emit funnel (`src-tauri/src/events.rs`) → `src/platform/events.ts`.
- The app already degrades gracefully without the model (UI usable, embeddings reads empty; `is_cache_ready` polling exists) — decline needs no new degraded mode, only an honest status surface.
- Provide visible download state and an on-demand trigger alongside the consent choice.
- `menu:open-settings` currently has a TS listener and no rust emitter (found 2026-07-11); if OOBE/consent lands near settings UI, that orphan is adjacent material.
