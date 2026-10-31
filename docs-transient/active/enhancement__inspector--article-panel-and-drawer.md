# Article Inspector Panel and Drawer

## Summary

Verify native-app presentation and accessibility for the article Inspector panel
and narrow-layout drawer.

## Requirements

- Follow the article targeting, session state, Topic membership, and responsive
  behavior in `docs/architecture/article_inspector.md`.
- Verify native WebKit and assistive-technology behavior directly rather than
  inferring it from DOM tests.
- Keep tag creation/assignment, annotation Inspector implementation, and storage
  changes outside this story. Tags remains disabled with its explanation until
  the Entry-assignment story supplies that view.

## Remaining work

- Check the wide panel and narrow toolbar-opened drawer in the macOS app,
  including resizing across the 700px reader-container boundary while open.
- Check Related selection/navigation and Topic filtering, sorting, assignment,
  removal, and inline creation. Confirm committed changes stay selected when
  reopening the Inspector before indexing catches up.
- Check read-only membership display with mutation controls disabled.
- Check keyboard tab switching, VoiceOver names/states, drawer focus containment,
  Escape/Close/backdrop dismissal, and focus return to the opener or wide panel.
- Check article versus annotation targeting, selected-view retention across
  navigation and reopening, unchanged list scope on dismissal, and retained
  reader scroll position while opening, closing, and resizing.
- Check the disabled Tags explanation and reduced-motion behavior.

## References

- `docs/architecture/article_inspector.md`
- `docs/conventions/motion.md`
- [Entry Tag Assignment](../available/enhancement__tags--entry-assignment.md)
- `src/components/content/ArticleInspector.tsx`
- `src/components/content/ItemView.tsx`
- `src/components/content/TopicMenuButton.tsx`
