# Make article bookmark colors easier to distinguish in the item list

## Summary

Make article bookmark colors easier to scan in `EntityList`. Scope is article
items only; annotation and other list presentations are unchanged.

## Requirements

- Replace the lower-right bookmark icon with a 4 px left border in the assigned
  color, spanning the item height with small top and bottom insets.
- Reserve equal space on unbookmarked articles to keep text aligned.
- Add screen-reader-only text such as “Bookmarked, Blue” to marked articles,
  supplementing the article title. Unmarked articles need no bookmark label.
- Preserve unread, selection, and focus cues and the shared annotation palette.
- Reflect assignment, recoloring, and removal while preserving current list
  membership under the existing ListView contract.

## Remaining work

- Implement the presentation, choosing exact inset spacing during implementation.
- Update component and browser coverage for alignment, accessible text, and live
  bookmark changes; run relevant component checks and `npm run test:browser`.
- Review all six colors in light/dark themes across selected, unread, hovered,
  and focused states. Verify unchanged annotations and report verification gaps.

## References

- [Item list](../../src/components/list/EntityList.tsx)
- [Item list styles](../../src/components/list/EntityList.module.css)
- [Shared entity status](../../src/components/EntityStatus.tsx)
- [Shared color palette](../../src/colors.module.css)
- [Bookmark model](../../docs/architecture/organization_model.md#reading-state-bookmarks-and-color)
- [ListView behavior](../../docs/architecture/list_view.md)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
