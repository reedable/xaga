# Keep the application visible during embedding startup

## Summary

The main window remains blank while the embedding model and database initialize,
and the frontend's 100 ms readiness polling produces repetitive IPC logs. Render
the ordinary application shell during initialization while presenting embedding
features as temporarily unavailable.

## Requirements

- Render the main application shell while embedding startup is `loading`; do not
  wait for a terminal embedding state before showing ordinary library features.
- Distinguish temporary loading from terminal unavailability in embedding-backed
  affordances and messaging.
- Disable semantic search, related-item lookup, and relevance ranking until
  embeddings become available.
- Show a restrained startup indication that does not obscure non-embedding
  workflows.
- Replace the 100 ms readiness polling with a state-change notification, or
  otherwise eliminate its repetitive IPC logging without hiding unrelated IPC
  diagnostics.
- Preserve the existing available and unavailable behavior after initialization
  completes.
- Cover loading, available, and unavailable UI transitions in tests.

## Remaining work

- Allow `App` to mount the main providers and layout while embedding startup is
  loading.
- Add loading-specific presentation and capability gating alongside the existing
  unavailable mode.
- Replace or quiet the readiness polling path and verify that state transitions
  still reach every window that consumes them.
- Run frontend lint, tests, and the production build.
- Update the durable startup documentation for the resulting behavior.

## References

- `src/App.tsx`
- `src/AppLayout.tsx`
- `src/providers/EmbeddingStartupProvider.tsx`
- `src/hooks/useEmbeddingStartupState.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/entries.rs`
- `docs/architecture/security/embedding_model_trust.md`
