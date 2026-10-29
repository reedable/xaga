# Tags and the Inspector

## Organization belongs to the user

Tags let users organize articles along dimensions other than content. A
`read-later` tag can connect articles with no semantic similarity.

Users define their own organizational labels. Some people
organize their work as `active`, `working`, `reference`, and `waiting-for`,
which can be combined to `active working`, `active reference`, etc; others use
entirely different systems. Neither “bookmark” nor “read-later” maps cleanly
onto those categories, and those categories are examples, not a taxonomy the
app should prescribe.

Provide an initial ordinary, editable tag named `bookmark`. Users can rename it
and create other tags. Applying a tag is expected to be a two-click
interaction: open the relevant control or view, then choose a tag. The exact
controls remain to be designed.

Pin provides access to a target through Shortcuts. Articles can be pinned as
well as collections: an article shortcut opens a ListView containing the article
first, then its annotations in passage order. Unpinning preserves the open list.
Site identity and persistence still need further definition.

## Tags include visual labels

An article or annotation can have **zero or more tags**. Tags are optional and
can be combined; neither `bookmark` nor any other tag is required.

Abstractly, a tag is a label whose meaning the user supplies. Words and colors
can express such labels. Choosing red for an annotation labels it visually;
color is a form of tag. A color label can combine with other labels just as
`active` can combine with `waiting-for`.

Multiple color labels are theoretically possible, but presenting six colors on
one highlighted passage introduces visual friction. Color could also label Feeds
or Topics in principle, although the app does not support that today. Keep color
separate from general-purpose Tags; its presentation
constraints differ from freely combining named labels.

## Unread is article reading state

Unread applies to articles, not to Feeds, Topics, or annotations. Collection
unread counts summarize their articles. It has an app-managed lifecycle as well
as explicit user controls. Renaming it to `needs-attention`, for example, would
make automatic removal on reading conflict with the user's chosen meaning.

Unread is separate from organizational Tags. Its article-specific scope and
reading lifecycle differ from labels whose meaning users define.

## An Inspector for the article

The current Related panel could become an Inspector with switchable **Related**,
**Topics**, and **Tags** views. Tags would expose the user's organizational labels;
Topics and Related would retain their respective roles rather than being folded
into tagging.

When the window is wide enough, the article Inspector occupies the panel beside
the reader. When that panel is hidden on a narrow window, an article-toolbar
button opens it as a drawer.

Annotations should have the same inspection capabilities, but their Inspector
always opens as a drawer. The article Inspector remains associated with the
article rather than switching its target to an annotation.

The action that opens an Inspector establishes its target. Someone opening it
from the article toolbar has already declared an intention to work with the
article. Someone opening it from an annotation's controls has declared an
intention to work with that annotation.

## The annotation popup is its toolbar

The relationship is **popup : annotation :: toolbar : article**.

Text selection shows the popup rather than immediately opening an Inspector
drawer. The popup serves both creation from selected text and actions on an
existing annotation.

Clicking an existing highlight shows its popup; opening the annotation Inspector
is an explicit action from that popup. Color and other quick actions can remain
near the text, while the drawer provides the inspection surface. The exact
allocation of color, comment, removal, and other actions still needs agreement.

## Current selection behavior to account for

The current implementation has two distinct annotation interactions:

- Selecting an annotation from a list sets the route's `annotationId` and its
  parent `entryId`, opens the parent article, and scrolls to the highlight. It
  does not open the annotation popup.
- Clicking a highlight opens local popup state without updating the route's
  annotation selection. The annotation in the route and the annotation in the
  popup can therefore differ.

Related currently receives the parent `entryId`, even when the route includes an
annotation. The main toolbar's Topics control also targets the article; the
annotation popup has its own Topics control.

Relevant implementation:

- `src/hooks/useRouter.tsx`: `selectAnnotation`, `selectEntry`, and `selectItem`.
- `src/components/content/ArticleReader.tsx`: annotation URL resolution and the
  article's Related panel.
- `src/components/content/HtmlReader.tsx`: selection, highlight clicks, popup
  state, annotation creation/replacement/removal, and scroll messages.
- `src/components/content/AnnotationTooltip.tsx`: annotation controls and Topics.
- `src/components/content/Toolbar.tsx`: article controls.
- `src/text/buildEntryHtml.ts`: iframe selection/highlight events and scrolling.

## Questions and consequences still to work through

- What should selecting an annotation from a list do beyond scrolling to it:
  reveal its popup, open its Inspector, or preserve today's behavior?
- What commits creation, and when does temporary browser text selection clear?
  Preserve copying and the native context menu; selecting text alone must not be
  treated as a committed annotation.
- Distinguish dragging across existing highlights from clicking one, so selecting
  text does not accidentally open annotation controls. Work through right-click
  behavior on selected text and existing highlights.
- Define popup dismissal, drawer dismissal, Escape, focus restoration, and reading
  position. Decide how article and annotation drawers interact on narrow windows.
- Decide whether the selected Inspector view persists across articles,
  annotations, drawer reopening, and changes in window width.
- Define comment saving, recoloring, removal, and whether removal offers Undo.
  Explain what happens to tags and topic relationships when an annotation is
  removed or replaced.
- Recoloring currently replaces an annotation with a new ID in `HtmlReader`.
  Work through Inspector target continuity and preservation of relationships;
  do not assume recoloring is an in-place identity-preserving update.
- Define tag identity, renaming, deletion, defaults, import/export, and migration
  of existing bookmarks. An editable `bookmark` label must not conceal a special
  hard-coded identity contract that makes ordinary editing break behavior.
