# Remove item filters from navigation history

## Summary

All/Saved/Unread changes currently create navigation-history entries. Move this
filter into component state so content Back/Forward does not step through filter
changes. Evaluate the resulting experience before adding special history behavior.

## Requirements

- Keep All/Saved/Unread in component state, preserving the current default and
  ordinary filter behavior, including continuity across collection changes.
- Filter changes create no history entries. Back/Forward restores article
  selection through existing navigation without restoring or resetting the filter.
- Preserve collection/article navigation, tag-selection history, and existing
  text-filter behavior. Do not add an All-on-history rule or persistent preference.

## Remaining work

- Remove the filter from navigation state and route comparisons; update filter
  consumers to use component state.
- Test Topic/article/filter sequences through Back/Forward, including articles
  marked read during browsing and retained current filters. Run relevant
  component checks and `npm run test:browser`.
- Record the implemented contract in durable docs. Before closing, obtain and
  record PO acceptance, or, if the PO is not ready to decide, create a backlog
  story describing the unresolved behavior and evaluation needed, linked to the
  durable contract.

## References

- [Routing](../../src/hooks/useRouter.tsx)
- [Item list](../../src/components/list/EntityList.tsx)
- [Content toolbar](../../src/components/content/Toolbar.tsx)
- [ListView contract](../../docs/architecture/list_view.md)
