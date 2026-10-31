# Annotation Inspector

The annotation popup serves as its toolbar, both for creating an annotation from
selected text and for acting on an existing highlight. Text selection shows the
popup; it does not immediately open a drawer. Clicking an existing highlight
also shows its popup. An explicit popup action opens that annotation's Inspector
as a drawer, regardless of window width.

The article Inspector remains associated with the article. Opening an annotation
Inspector does not retarget the article's panel. The opening action establishes
which annotation is being inspected.

Article Inspector behavior, Entry tagging, tag identity and lifecycle, color-backed
bookmarks, pin storage/navigation, and Collection/Site definitions are specified
separately. Their requirements are not open questions in this idea. Initial
Entry tagging does not include annotations; broader tagging UX is explored in
[Tagging Beyond Entries](tagging-other-entities.md). Durable domain definitions
are in [Organization Model](../../docs/architecture/organization_model.md).

## Annotation interactions to refine

- Allocate existing annotation actions between the popup and drawer: color,
  comment editing, Topic membership, and removal. Related inspection belongs to
  the annotation target; annotation tag assignment requires the separate broader
  tagging design.
- Determine whether selecting an annotation from a list should only scroll to
  its passage or also reveal controls. Do not silently equate route selection
  with the popup's local annotation context.
- Define comment save timing and what commits a newly selected passage as an
  annotation. Preserve copying, native context menus, and dragging text across
  existing highlights without accidentally invoking highlight-click actions.
- Define popup/drawer dismissal, Escape, focus restoration, and reading-position
  preservation. Resolve interaction between article and annotation drawers on
  narrow windows and selected-view retention for annotation inspection.
- Account for annotation replacement during recoloring: preserve the open
  Inspector's target continuity and existing Topic relationships. Investigate
  the current identity contract before proposing a change to it.
- Explore Undo for annotation removal as an optional idea, not an agreed
  requirement.

## Implementation context

List navigation currently carries `annotationId` and its parent `entryId` and
scrolls to the passage without opening the popup. Highlight clicks open local
popup state without updating route selection, so those contexts can differ.
The proposed drawer must have an explicit annotation target.

Recoloring currently replaces the annotation under a new ID. Annotation comments
and Topic controls already exist in the popup; moving controls must preserve
their behavior rather than imply those capabilities are new.

Relevant code:

- `src/hooks/useRouter.tsx`: annotation navigation.
- `src/components/content/ArticleReader.tsx`: parent article and reader context.
- `src/components/content/HtmlReader.tsx`: selections, popup state, annotation
  creation/replacement/removal, and scroll messages.
- `src/components/content/AnnotationTooltip.tsx`: annotation controls and Topics.
- `src/text/buildEntryHtml.ts`: iframe text-selection and highlight events.
