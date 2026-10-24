# Keep the application visible during embedding startup

## Summary

The main window remains blank while the embedding model and database initialize,
and the frontend's 100 ms readiness polling produces repetitive IPC logs. Render
the ordinary application shell during initialization and replace readiness
polling with event-driven delivery of the existing terminal transitions.

## Requirements

- Render the main application shell while embedding startup is `loading`; do not
  wait for a terminal embedding state before showing ordinary library features.
- Present embedding-backed affordances as initializing while startup is `loading`,
  distinct from terminal `unavailable` behavior.
- Disable semantic search, related-item lookup, and relevance ranking until
  embeddings become available.
- While startup is `loading`, show the topic-sort dropdown disabled with **Most
  recent** selected and helper text explaining that the index is initializing.
  When startup becomes `available`, enable only the dropdown and remove its helper
  text. Keep **Most recent** selected and preserve the mounted topic list's items,
  ordering, selected article, and scroll position; the transition must not request,
  reconcile, re-sort, or replace list contents. A new request may occur only after
  the user selects **Most relevant** or navigates to another collection.
- When startup is `unavailable`, keep the topic-sort dropdown disabled with **Most
  recent** selected and show helper text explaining that relevance ranking is
  unavailable.
- Show an embedding-startup badge in the sidebar's existing status area while
  startup is `loading`. It may appear beside the independent **Read-only** badge
  and disappears at either terminal embedding state.
- Keep library mode and embedding startup as orthogonal state with their separate
  state paths and hooks; unify only their presentation in the sidebar status area.
- Replace the 100 ms readiness polling with state-change events for the existing
  transitions from `loading` to `available` or `unavailable`. Preserve the
  backend's current transition points and readiness timing; the current-state
  snapshot continues to represent all three states.
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
- Add the temporary sidebar badge and loading-specific capability gating alongside
  the existing unavailable mode; cover simultaneous read-only and embedding-loading
  presentation.
- Add the disabled **Most recent** topic-sort presentation with state-specific
  initializing or unavailable helper text. Handle `loading` to `available`
  surgically by enabling the control and removing its helper text without changing
  the selected sort or triggering any topic-list request or item-state update.
- Add the typed startup-state event to the existing Tauri event funnel and platform
  event surface. Emit it at the same assignments that currently make startup
  `available` or `unavailable`.
- Replace readiness polling with subscribe-then-read initialization and verify
  transitions and duplicate delivery for every consuming window.
- Verify that `loading` to `available` enables the topic-sort control without
  changing its **Most recent** selection, issuing another list request, or changing
  the rendered items, their order, the selected article, or scroll position.
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
