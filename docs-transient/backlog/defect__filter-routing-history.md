# Back/forward vs list filter routing

## Summary

List-filter changes are stored in navigation state and create browser-history entries. Back and forward can therefore step through filter changes instead of the content and collection navigation the user expects. The intended history semantics are undefined.

## Requirements

- Define whether filter changes are navigable history, replace the current entry, or live outside routing.
- Preserve the active filter across collection changes if that remains intended.
- Keep restored routes internally consistent with selected collection, item, search, and filter state.
- Make toolbar back and forward availability reflect the actual history stack.

## Remaining work

- Reproduce back and forward sequences across filters, collections, items, and search.
- Compare pushed filter routes, replacement navigation, and component or persisted preference state.
- Present the history and restoration tradeoffs for decision.
- Implement the selected model and add navigation-sequence tests.

## References

- `src/hooks/useRouter.tsx`
- `src/components/list/ItemList.tsx`
- `src/components/content/Toolbar.tsx`
