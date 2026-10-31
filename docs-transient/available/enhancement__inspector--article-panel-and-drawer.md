# Article Inspector Panel and Drawer

## Summary

Turn the article reader's Related sidebar into an Inspector with switchable
Related, Topics, and Tags views. Keep it beside the reader on wide layouts and
open it through an article-toolbar button as a drawer when the panel is hidden.

## Requirements

- Keep the Inspector associated with the current article. Annotation navigation
  or popup interactions must not change its target to the annotation.
- Preserve existing Related results and selection behavior in the Related view.
- Provide article Topic membership controls in the Topics view, reusing existing
  membership operations and preserving read-only behavior.
- Provide the Tags view integration surface for the dependent Entry-assignment
  story; tag creation and assignment are outside this story.
- Use the same view content in the wide panel and narrow drawer. Opening from
  the article toolbar establishes article context without requiring a large
  title/quotation target header.
- Preserve the selected Inspector view across article navigation, resizing, and
  drawer reopening within the current app session. Drawer dismissal must not
  change the article or list scope.
- Support keyboard view switching, accessible drawer opening/closing, and focus
  return to the opener. Preserve reader position when opening or closing.
- Keep annotation Inspector implementation, annotation creation behavior,
  tagging operations, and storage-model changes outside this story.

## Remaining work

- Replace the Related-only aside with the Inspector view container and integrate
  existing Related and article Topic membership controls.
- Add responsive panel/drawer presentation and the article-toolbar opener using
  the reader's existing narrow-layout breakpoint.
- Add session state for selected view and drawer visibility, keeping view state
  separate from article navigation identity.
- Connect the Tags integration surface with Entry assignment when implemented;
  do not present nonfunctional tag controls as a completed feature.
- Verify Related navigation, Topic membership, read-only access, article versus
  annotation targeting, view retention, responsive transitions, reader position,
  and keyboard/focus behavior.
- Run frontend type checking, lint, affected tests, and production build; update
  durable reader/Inspector behavior documentation.

## References

- [Entry Tag Assignment](enhancement__tags--entry-assignment.md) — follows this
  story for its UI surface; this story does not depend on tag storage.
- [Annotation Inspector](../ideas/annotation-inspector.md)
- `src/components/content/ArticleReader.tsx`
- `src/components/content/ArticleReader.module.css`
- `src/components/content/RelatedItemList.tsx`
- `src/components/content/TopicMenuButton.tsx`
- `src/components/content/Toolbar.tsx`
- `src/hooks/useRouter.tsx`
- `src/platform/Platform.ts`
