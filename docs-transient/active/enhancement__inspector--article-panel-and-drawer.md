# Inspector for Articles and Annotations

## Summary

Verify the remaining persisted-data, accessibility, and native presentation
behavior of the Article panel and Annotation drawer in the full macOS app.

## Requirements

- Verify against the [Inspector design](../../docs/architecture/article_inspector.md);
  interaction and presentation decisions are specified there.
- Use persisted library data for membership and lifecycle checks. Component
  tests and isolated WebKit probes do not establish full-app persistence or
  VoiceOver behavior.

## Remaining work

- Verify Article and Annotation Topic assignment, removal, and inline creation
  affect the intended typed member in the Topic file. Confirm selection survives
  drawer reopening before indexing catches up and remains correct after reload.
- Verify Topic memberships and the open Inspector follow Annotation recoloring,
  including recoloring immediately after assignment before indexing catches up.
- Verify Related results use the inspected Article or Annotation with persisted
  data, and selecting a result preserves the documented navigation behavior.
- Verify read-only membership inspection disables mutation controls. Delete an
  inspected Annotation and confirm its drawer/tooltip state clears without
  recreating it or changing the parent Article's assignments.
- Check VoiceOver names/states and keyboard access in the full app: tab switching,
  drawer focus containment, Close/Escape/backdrop dismissal, and focus return to
  the retained tooltip opener or toolbar, with the reader as fallback.
- Check resizing across the 700px reader-container boundary: Annotation inspection
  stays in its drawer; Article inspection changes between panel and drawer.
  Confirm retained tab selections, tooltip, reader geometry, and reading position
  where the layout is unchanged. With macOS Reduce Motion enabled, verify static
  drawer opening and full Remove-label visibility with overlapping swatches.

## References

- [Inspector design](../../docs/architecture/article_inspector.md)
- [Motion convention](../../docs/conventions/motion.md)
- [Single Source of Truth](../../docs/principles/single_source_of_truth.md)
- [Organization model](../../docs/architecture/organization_model.md)
- `src/components/content/Inspector.tsx`
- `src/components/content/ItemView.tsx`
- `src/components/content/TopicMembership.tsx`
- `src/components/content/AnnotationTooltip.tsx`
- `src/components/content/HtmlReader.tsx`
- `src/components/content/ColorSwatches.tsx`
