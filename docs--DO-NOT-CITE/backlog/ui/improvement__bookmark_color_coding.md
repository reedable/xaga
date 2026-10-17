# Color code bookmarks and bookmark icons

## Summary

Entries support an optional bookmark color, but the intended controls and presentation for colored bookmarks are undefined.

## Requirements

- Define what bookmark color means and where it is visible.
- Keep an uncolored bookmark valid.
- Preserve accessible non-color cues for bookmark state.
- Define how color changes interact with add, remove, import, lists, search results, and read-only mode.
- Persist color through the authoritative entry representation and derived cache.

## Remaining work

- Identify the workflows that assign, change, and clear bookmark color.
- Present color-palette and presentation options with accessibility and density tradeoffs.
- Decide whether icons, rows, or both carry the color.
- Implement the selected interaction and verify persistence and event propagation.

## References

- `src/domain.ts`
- `src/components/content/Toolbar.tsx`
- `src/components/list/ItemList.tsx`
- `crates/xaga-core/src/domain.rs`
