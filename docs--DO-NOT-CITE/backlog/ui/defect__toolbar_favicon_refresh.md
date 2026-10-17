# Refresh the toolbar favicon after article persistence

## Summary

The toolbar reads article metadata when the selected entry ID changes. Ingestion or refresh can persist a favicon without changing that ID, leaving the toolbar's article and favicon state stale until navigation causes another read.

## Requirements

- Show a newly persisted favicon without requiring navigation away and back.
- Render only persisted local favicon references in the privileged main window.
- Clear stale favicon state when navigation or refresh produces an article without a usable local icon.
- Avoid duplicate remote acquisition.

## Remaining work

- Reproduce first-ingestion and same-entry refresh behavior.
- Define how successful article persistence notifies or updates the toolbar's current article state.
- Refresh the local favicon bytes after that state changes.
- Test successful favicon persistence, missing favicon, failed acquisition, refresh replacement, and navigation races.

## References

- `src/components/content/Toolbar.tsx`
- `src/hooks/useArticles.tsx`
- `src/platform/TauriPlatform.ts`
- `crates/xaga-core/src/persistence/articles.rs`
