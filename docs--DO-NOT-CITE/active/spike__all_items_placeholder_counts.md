# ALL_ITEMS placeholder counts

## Summary

`ALL_ITEMS` carries zero `entryCount` and `unreadCount` values only to satisfy the `Countable` shape used for selectable collections. Decide whether the shortcut should expose real counts or stop pretending to be an authoritative counted collection.

## Requirements

- Do not present placeholder values as real counts.
- Keep routing and collection selection type-safe.
- If real counts are added, define their source, freshness, and behavior while the embeddings cache is unavailable.
- Avoid forcing unrelated shortcuts to carry feed- or Topic-specific data.

## Remaining work

- Identify every consumer that relies on `ALL_ITEMS` being `Countable`.
- Compare real aggregate counts, a separate navigable-shortcut type, and a union accepted by routing.
- Present the type and UI consequences for decision.
- Implement the selected model and add guard and routing tests.

## References

- `src/Shortcuts.ts`
- `src/domain.ts`
- `src/types.ts`
- `src/hooks/useRouter.tsx`
- `src/components/sidebar/ShortcutList.tsx`
