# Improvement: article reload progress and failure visibility

Make article reload progress visible while retrieval is in flight. Preserve the
saved article when refresh fails, but keep the refresh result available to the
user: rendering the old article can report a missing image and replace the
refresh-error toast and its diagnostic link.

- Provide an in-progress indication for the article reload action.
- Keep the refresh failure and its diagnostics accessible when subsequent
  rendering reports a separate image error. Use the existing reporting contract.
- Verify successful reload, failed retrieval with retained content, and failed
  retrieval followed by a missing-image error. Do not discard the saved article
  or automatically retry network requests.

Relevant code: `src/components/content/ItemView.tsx`,
`src/hooks/useContentLoader.ts`, `src/components/content/ArticleReader.tsx`, and
`src/providers/ErrorSinkProvider.tsx`.
