# Inspector for Articles and Annotations

## Summary

Extend the Inspector to handle both Articles and Annotations, and finish
verification of its native-app presentation and accessibility. Define the
interaction details from the existing code, documented contracts, project
principles, and agreed shared-Inspector behavior.

## Requirements

- Provide the same Inspector sections for Articles and Annotations.
- Use the new Inspector as the single inspection surface for both target types,
  with one authoritative current target and shared section implementations.
  Opening it from an Annotation tooltip retargets that Inspector; do not retain
  a parallel Article Inspector or introduce a separate Annotation-only drawer.
  Use the existing responsive panel/drawer presentation for either target.
- Provide an Open Inspector action in the Annotation tooltip that currently
  offers Annotation color selection. Opening it inspects that tooltip's
  Annotation, independently of the reader route's selected Article or Annotation.
- Replace the old Add to Topics button and its dedicated drawer with the
  Inspector's Topics section once Annotation inspection is available. Preserve
  Topic assignment, removal, filtering, sorting, and inline creation there.
- When inspecting an Annotation, section values and assignments apply to that
  Annotation independently of its parent Article. Related results and Topic
  membership use the inspected Annotation as their target.
- Preserve Article inspection with values and assignments applying to the
  inspected Article. Retain the session state, Topic membership, and responsive
  Article presentation contracts in `docs/architecture/article_inspector.md`,
  updating its targeting contract for the agreed Annotation behavior.
- Keep tag creation/assignment and storage changes outside this story. Tags
  remains disabled with its explanation until the Entry-assignment story
  supplies that view; shared sections do not themselves define Annotation tag
  assignment.

## Remaining work

- Connect route selection and the Annotation tooltip action to the shared
  Inspector target, including returning inspection to the Article. Preserve the
  distinction between navigation state and the tooltip's explicit target.
- Integrate Annotation inspection with existing dismissal and focus behavior,
  follow the replacement event's old-to-new identity when recoloring, and clear
  stale Annotation state on removal using the existing removal notification.
  Preserve the tooltip's edit-save path when opening the Inspector.
- Implement the agreed targeting and shared sections, reusing existing Related
  and Topic controls where their contracts apply.
- Remove the superseded Add to Topics button, dedicated drawer, and unused
  supporting code after their functionality is available in the Inspector.
- Verify that inspecting or changing an Annotation never reads or changes its
  parent Article's assignments in place of the Annotation's, and that Article
  inspection still targets the Article.
- Check the wide panel and narrow toolbar-opened drawer in the macOS app,
  including resizing across the 700px reader-container boundary while open.
- Check Related selection/navigation and Topic filtering, sorting, assignment,
  removal, and inline creation. Confirm committed changes stay selected when
  reopening the Inspector before indexing catches up.
- Check read-only membership display with mutation controls disabled.
- Check article versus annotation targeting, selected-view retention across
  navigation and reopening, unchanged list scope on dismissal, and retained
  reader scroll position while opening, closing, and resizing.
- Check the disabled Tags explanation and reduced-motion behavior.
- Update the durable Inspector design and affected documentation alongside the
  implementation; the current design explicitly keeps the parent Article as the
  target during Annotation selection.

## References

- `docs/architecture/article_inspector.md`
- `docs/conventions/motion.md`
- [Single Source of Truth](../../docs/principles/single_source_of_truth.md)
- [DRY](../../docs/principles/dry.md)
- [Organization model](../../docs/architecture/organization_model.md)
- [Annotation Inspector idea](../ideas/annotation-inspector.md) — interaction
  context; its separate-drawer proposal is superseded by this story.
- [Entry Tag Assignment](../available/enhancement__tags--entry-assignment.md)
- `src/components/content/ArticleInspector.tsx`
- `src/components/content/ItemView.tsx`
- `src/components/content/TopicMenuButton.tsx`
- `src/components/content/AnnotationTooltip.tsx`
