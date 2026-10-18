# Keep the application usable when embeddings are unavailable

## Summary

Embedding initialization failure currently leaves cache readiness false forever,
so the main application window remains blank. Allow the non-embedding application
to finish startup when the embedding model or embeddings database is unavailable,
while clearly withholding embedding-dependent capabilities.

## Requirements

- Distinguish embedding initialization in progress from its terminal available
  and unavailable outcomes.
- Render the main application after embedding initialization reaches either
  terminal outcome; do not leave it waiting indefinitely after failure.
- Keep feeds, articles, bookmarks, annotations, settings, and other
  non-embedding workflows usable when embeddings are unavailable.
- Prevent embedding-dependent actions from behaving as though the capability is
  available, and give the user a concise, actionable indication of the
  unavailable capability.
- Preserve the existing refusal behavior for an untrusted embedding model: do
  not delete or replace model files and do not initiate recovery downloads.
- Log the diagnostic cause without exposing technical details in the UI.
- Cover loading, available, and unavailable startup outcomes in tests.

## Remaining work

- Replace the boolean cache-readiness contract with an explicit startup state
  shared by the backend and frontend.
- Decouple rendering the application shell from successful embedding-service
  startup.
- Gate or annotate semantic search and related-item features in unavailable
  mode.
- Verify that embedding initialization failures leave ordinary library workflows
  usable and report embedding-dependent operations appropriately.
- Update the durable architecture documentation for the resulting startup and
  capability behavior.

## References

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/entries.rs`
- `src/App.tsx`
- `src/hooks/useCacheReady.tsx`
- `src/components/search/SearchResultList.tsx`
- `src/components/content/RelatedItemList.tsx`
- `docs/architecture/security/embedding_model_trust.md`
