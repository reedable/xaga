# Annotation Inspector

The annotation popup serves as its toolbar, both for creating an annotation from
selected text and for acting on an existing highlight. Text selection shows the
popup; it does not immediately open a drawer. Clicking an existing highlight
also shows its popup. An explicit popup action opens the shared Inspector for
that Annotation. The Inspector shares content between an Article panel and an Annotation drawer. Annotation inspection always
opens its drawer, while Article presentation remains responsive, as specified in the
[Inspector design](../../docs/architecture/article_inspector.md).

Article Inspector behavior, Entry tagging, tag identity and lifecycle, color-backed
bookmarks, pin storage/navigation, and Collection/Site definitions are specified
separately. Their requirements are not open questions in this idea. Initial
Entry tagging does not include annotations; broader tagging UX is explored in
[Tagging Beyond Entries](tagging-other-entities.md). Durable domain definitions
are in [Organization Model](../../docs/architecture/organization_model.md).

## Annotation interactions to refine

- Explore whether color, comment editing, and removal should also be available
  in the shared Inspector. These controls currently remain in the popup; Topic
  membership and Related inspection use the Inspector's Annotation target.
  Annotation tag assignment requires the separate broader tagging design.
- Refine comment save timing and what commits a newly selected passage as an
  annotation. Preserve copying, native context menus, and dragging text across
  existing highlights without accidentally invoking highlight-click actions.
- Explore Undo for annotation removal as an optional idea, not an agreed
  requirement.

## Implementation context

List navigation currently carries `annotationId` and its parent `entryId` and
scrolls to the passage without opening the popup. Highlight clicks open local
popup state without updating route selection, so those contexts can differ.
Inspection opened from the popup must use its explicit Annotation target.

Recoloring currently replaces the annotation under a new ID. Annotation comments
remain in the popup. Topic membership is handled by the shared Inspector.
Future movement of controls must preserve their behavior rather than imply
those capabilities are new.

Relevant code:

- `src/hooks/useRouter.tsx`: annotation navigation.
- `src/components/content/ArticleReader.tsx`: parent article and reader context.
- `src/components/content/HtmlReader.tsx`: selections, popup state, annotation
  creation/replacement/removal, and scroll messages.
- `src/components/content/AnnotationTooltip.tsx`: annotation controls and Inspector opening.
- `src/text/buildEntryHtml.ts`: iframe text-selection and highlight events.
