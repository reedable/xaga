# Keep the application visible during embedding startup

## Summary

The main window remains blank while the embedding model and database initialize,
and the frontend's 100 ms readiness polling produces repetitive IPC logs. Render
the ordinary application shell during initialization and deliver the existing
three-state startup lifecycle to the frontend through a state-change event.

## Requirements

- Render the main application shell while embedding startup is `loading`; do not
  wait for a terminal embedding state before showing ordinary library features.
- Present embedding-backed affordances as initializing while startup is `loading`,
  distinct from terminal `unavailable` behavior.
- Disable semantic search, related-item lookup, and relevance ranking until
  embeddings become available.
- Show a restrained startup indication that does not obscure non-embedding
  workflows.
- Replace the 100 ms readiness polling with a state-change event carrying the
  existing `loading`, `available`, or `unavailable` state. Preserve the backend's
  current transition points and readiness timing.
- Subscribe to state changes before reading the current startup-state snapshot so
  a window cannot miss a transition between loading and subscription. Consumers
  must tolerate receiving the same state from both paths.
- Preserve the existing available and unavailable behavior after initialization
  completes.
- Cover loading, available, and unavailable UI transitions in tests.
- Do not add lifecycle states or change model acquisition, consent, progress,
  failure detail, retry, or the work required before startup becomes available;
  those changes remain with the referenced backlog tasks.

## Remaining work

- Allow `App` to mount the main providers and layout while embedding startup is
  loading.
- Add loading-specific presentation and capability gating alongside the existing
  unavailable mode.
- Add the typed startup-state event to the existing Tauri event funnel and platform
  event surface. Emit it at the same assignments that currently make startup
  `available` or `unavailable`.
- Replace readiness polling with subscribe-then-read initialization and verify
  transitions and duplicate delivery for every consuming window.
- Run frontend lint, tests, and the production build.
- Update the durable startup documentation for the resulting behavior.

## References

- `src/App.tsx`
- `src/AppLayout.tsx`
- `src/providers/EmbeddingStartupProvider.tsx`
- `src/hooks/useEmbeddingStartupState.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/events.rs`
- `src-tauri/src/commands/entries.rs`
- `src/platform/events.ts`
- `docs/architecture/security/embedding_model_trust.md`
- `docs-transient/backlog/compliance__embedding-model--oobe-download-consent.md`
- `docs-transient/backlog/compliance__embedding-model--download-consolidation.md`
