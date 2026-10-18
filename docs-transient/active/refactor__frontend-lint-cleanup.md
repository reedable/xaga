# Resolve the frontend lint baseline

## Summary

The frontend currently has ten ESLint errors across production components,
providers, and tests. Resolve the complete baseline in one bounded change so
`npm run lint` provides a clean repository-wide signal again.

## Requirements

- Resolve every current ESLint error without changing intended application
  behavior.
- Correct render-time ref mutation and synchronous effect-state patterns rather
  than suppressing the React rules.
- Move shared hooks or values out of component-only provider modules where
  required by Fast Refresh.
- Refactor test capture helpers so they observe hooks outside render without
  disabling purity rules.
- Do not weaken or broadly disable lint configuration; any narrow suppression
  requires a demonstrated false positive and an explanatory comment.
- Leave `npm run lint`, the frontend test suite, and the production frontend
  build passing.

## Remaining work

- Resolve the unused callback in `ArticleReader.test.tsx`.
- Correct the React rule violations in `ItemView.tsx`, `Toolbar.tsx`, and
  `AllItemsCountsProvider.tsx`.
- Separate non-component exports from `AllItemsCountsProvider.tsx` and
  `AppearanceProvider.tsx`.
- Replace render-time external assignments in `useContentLoader.test.tsx` and
  `ErrorSinkProvider.test.tsx` with effect-based test capture.
- Run the complete lint, test, and build commands.

## References

- `eslint.config.js`
- `src/components/content/ItemView.tsx`
- `src/components/content/Toolbar.tsx`
- `src/providers/AllItemsCountsProvider.tsx`
- `src/providers/AppearanceProvider.tsx`
