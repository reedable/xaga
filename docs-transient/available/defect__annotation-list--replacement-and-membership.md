# Keep route selection valid after annotation replacement or deletion

## Summary

Recoloring transfers selected-annotation state only in article-scoped lists.
Deletion removes the annotation without clearing its route selection. Keep the
current route valid in article-scoped, All items, feed, and Topic lists.

## Requirements

- When the selected annotation is replaced by recoloring, replace the current
  route state with the new annotation ID.
- When the selected annotation is deleted, replace the current route state with
  the parent article selected and `annotationId` cleared.
- Preserve the parent article, list, filters, reading context, and documented
  ListView stability, including article-scope refresh and passage ordering.
- Unselected-annotation changes and late events must not overwrite newer selection.

## Remaining work

- Fix replacement and deletion route handling.
- Cover selected/unselected annotations and late events across all four scopes
  in component and browser tests, asserting preserved list context.
- Run relevant component checks and `npm run test:browser`. Check native
  replacement/removal event ordering and report any unverified behavior.

## References

- [Item list](../../src/components/list/EntityList.tsx)
- [Routing](../../src/hooks/useRouter.tsx)
- [ListView behavior](../../docs/architecture/list_view.md)
