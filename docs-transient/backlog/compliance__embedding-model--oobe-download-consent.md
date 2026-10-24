# Require consent before downloading the embedding model

## Summary

A fresh install (or wiped model dir) contacts Hugging Face at app startup, unprompted — revealing install/launch timing and IP to a third party the user never chose. The download mechanism itself is not the problem (the CLI invocation of the same path is user-initiated); the problem is the app deciding to make network contact without the user. It is currently the app's only unconditional unprompted outbound contact: the other automatic fetch, startup feed refresh, is opt-in (`refresh_on_startup`, default false) and goes to user-chosen hosts.

## Requirements

- Make no model-host network request until the user explicitly consents.
- Explain the source, approximate download size, local execution, and unavailable features before consent.
- Keep non-embedding features usable when the user declines or download fails.
- Provide visible download progress, failure state, retry, and a later user-triggered download action.
- Hold the account-wide resource right across download and installation.
- Persist only the consent state necessary to honor the user's choice, with a way to change it.

## Remaining work

- Decide whether consent is presented in first-run UI, settings, or both, and define dismissal behavior.
- Move startup download behind the persisted consent decision.
- Add an on-demand command and status events around the existing verified download path.
- Reuse or deliberately replace the startup-status delivery channel that exists
  when this work begins; do not introduce an independent readiness channel.
- Surface unavailable, downloading, ready, and failed states without blocking application startup.
- Test fresh install, decline, later consent, concurrent processes, failure, retry, and already-installed model paths.

## References

- `crates/xaga-core/src/embeddings/model.rs`
- `src-tauri/src/lib.rs`
- `crates/xaga-core/src/events.rs`
- `src-tauri/src/events.rs`
- `src/platform/events.ts`
- `docs/architecture/security/embedding_model_trust.md`
- `docs/architecture/security/fetching_remote_content.md`
