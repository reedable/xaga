# Make article bookmark colors easier to distinguish in the item list

## Summary

Make article bookmark colors easier to scan in `EntityList`. Scope is article
items only; annotation and other list presentations are unchanged.

## Requirements

- Replace the lower-right bookmark icon with a 0.25rem left border in the assigned
  color, spanning the full item height with no top or bottom inset.
- Follow the [UI styling convention](../../docs/conventions/ui_styling.md): use
  decimal rem lengths in increments of `0.0625rem`.
- Reserve equal space on unbookmarked articles to keep text aligned.
- Add screen-reader-only text such as “Bookmarked, Blue” to marked articles,
  supplementing the article title. Unmarked articles need no bookmark label.
- Preserve unread, selection, and focus cues and the shared annotation palette.
- Reflect assignment, recoloring, and removal while preserving current list
  membership under the existing ListView contract.

## Remaining work

- Review the full-height border presentation and accessible labels. Native WebKit
  and screen-reader sessions have not been verified; Chromium capture tests cover
  the browser presentation and accessible names.

## References

- [Item list](../../src/components/list/EntityList.tsx)
- [Item list styles](../../src/components/list/EntityList.module.css)
- [Shared entity status](../../src/components/EntityStatus.tsx)
- [Shared color palette](../../src/colors.module.css)
- [Bookmark model](../../docs/architecture/organization_model.md#reading-state-bookmarks-and-color)
- [ListView behavior](../../docs/architecture/list_view.md)
- [UI and UX testing](../../docs/conventions/ui_testing.md)
