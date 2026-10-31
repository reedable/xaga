# Inspector for Articles and Annotations

## Summary

Verify the shared Article/Annotation Inspector in the full macOS app with a
library, including native accessibility and the shared color palette layout.

## Requirements

- Verify the behavior specified in the [Inspector design](../../docs/architecture/article_inspector.md).
- Confirm the shared Inspector applies Annotation assignments to the Annotation
  independently of its parent Article, with no standalone Add to Topics drawer.
- Annotation inspection opens the shared drawer at every width, sliding only
  when motion is permitted. Resizing keeps it in the drawer; closing leaves
  the Article panel as it was. The tooltip and Article panel remain in place while the
  drawer opens, with unchanged reader geometry, reading position, and Article
  panel tab.
- Swatch colors slightly overlap when the Remove confirmation appears, leaving
  its full label visible, including when reduced motion is requested. Motion
  changes only transitions, not layout.

## Remaining work

- Check full-app Article and Annotation navigation and opening the Inspector
  from the Annotation tooltip with persisted library data, including Related
  selection and Topic filtering, sorting, assignment, removal, and inline
  creation. Confirm committed memberships survive reopening before indexing
  catches up and that recoloring follows the replacement Annotation.
- Check read-only inspection and deletion of the inspected Annotation.
- Check VoiceOver names/states and keyboard access in the full app, including
  tab switching, drawer containment, Escape/Close/backdrop dismissal, and focus
  return to the toolbar or retained tooltip opener, with the reader as fallback.
- Check full-app resizing across the 700px reader-container boundary, retained
  selected view and reading position, and swatch compaction under the macOS
  Reduce Motion setting.

## References

- [Inspector design](../../docs/architecture/article_inspector.md)
- [Motion convention](../../docs/conventions/motion.md)
- [Single Source of Truth](../../docs/principles/single_source_of_truth.md)
- [DRY](../../docs/principles/dry.md)
- [Organization model](../../docs/architecture/organization_model.md)
- [Entry Tag Assignment](../available/enhancement__tags--entry-assignment.md)
- `src/components/content/Inspector.tsx`
- `src/components/content/ItemView.tsx`
- `src/components/content/TopicMembership.tsx`
- `src/components/content/AnnotationTooltip.tsx`
- `src/components/content/ColorSwatches.tsx`
