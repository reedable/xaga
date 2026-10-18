# Show article reload progress and failures

## Summary

Article reload has no visible in-progress state, and a later rendering warning can replace the refresh failure and its diagnostic link even though the saved article remains displayed.

## Requirements

- Indicate reload progress without hiding the saved article.
- Preserve the saved article when retrieval fails.
- Keep the refresh failure and its diagnostics accessible when rendering reports a separate image problem.
- Do not automatically retry network requests.
- Clear or replace feedback only according to an explicit lifecycle.

## Remaining work

- Define reload progress and failure placement in the item view.
- Separate concurrent refresh and rendering diagnostics in the error sink.
- Implement the feedback lifecycle.
- Test successful reload, failed retrieval with retained content, and failed retrieval followed by a missing-image warning.

## References

- `src/components/content/ItemView.tsx`
- `src/hooks/useContentLoader.ts`
- `src/components/content/ArticleReader.tsx`
- `src/providers/ErrorSinkProvider.tsx`
