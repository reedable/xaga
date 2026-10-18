# Compact the reader title while scrolling

## Summary

After the article title is rendered inside the reader iframe, keep it available
as context while reclaiming reading space by shrinking its sticky `<h1>` into a
compact title during downward scrolling.

## Requirements

- Depend on the reader document having a sibling `<header><h1>` before
  `<article>`, as defined by the title-in-iframe story.
- Keep the reader header sticky at the top of the iframe's scroll container.
- Begin with the normal lliwio-css `h1` presentation and progressively compact
  the title over a bounded initial scroll distance.
- Keep a readable compact title visible after the transition; do not make the
  title disappear.
- Give the sticky header an opaque reader-theme background so scrolling article
  content does not show through it.
- Constrain long titles in the compact state so they cannot remain a tall,
  multi-line obstruction. Preserve access to the complete title through an
  accessible mechanism rather than deleting it from the document.
- Preserve light and dark themes and the user's reader font family, font size,
  and line-height preferences.
- Respect reduced-motion preferences. Provide a stable compact or non-sticky
  fallback rather than animated interpolation when motion is reduced.
- Provide a deliberate fallback when scroll-driven CSS animations are not
  supported by the app's WebKit version. The fallback must not leave the full
  large heading permanently sticky.
- Keep the header outside `<article>` and outside every annotation traversal and
  selection boundary. This story must not change stored annotation identity,
  occurrence matching, or highlight placement.
- Ensure the sticky header does not cover destinations reached through existing
  annotation scrolling, find-result navigation, or normal anchor navigation.
- Do not add scroll-event JavaScript unless CSS cannot provide the required
  behavior on a supported platform. Any executable iframe-script change must
  follow the reader CSP-hash update and security-documentation procedure.
- Decide during implementation whether this behavior is a reusable lliwio-css
  pattern or Xaga-specific reader styling. Keep the behavioral contract and
  tests the same in either location; do not copy competing implementations into
  both stylesheets.

## Remaining work

- Complete `enhancement__article-reader--title-inside-iframe.md` first.
- Evaluate the experimental pattern in `../lliwio-css/public/index.css` against
  the reader's structure, supported WebKit runtime, appearance variables, and
  reduced-motion behavior.
- Choose and document ownership in lliwio-css or Xaga based on whether the
  pattern is a general typography primitive or application-specific reader
  behavior.
- Implement the sticky header, bounded scroll-linked compaction, opaque
  background, long-title treatment, reduced-motion behavior, and unsupported-
  feature fallback.
- Account for the compact header's occupied height in programmatic and anchor
  scrolling.
- Cover initial and compact states, long titles, appearance preferences,
  reduced motion, unsupported scroll-animation behavior, and light/dark themes.
- Add regressions proving existing annotation restoration and scrolling still
  target the same body passage, including an annotation near the beginning of
  the article, and verify find destinations are not obscured.
- Verify the behavior in the native macOS WebKit reader in addition to frontend
  unit coverage.
- Run frontend formatting, lint, tests, and the production build.
- Update reader and persisted-content documentation with the chosen CSS
  ownership, fallback, and scrolling behavior.

## References

- `docs-transient/available/enhancement__article-reader--title-inside-iframe.md`
- `../lliwio-css/public/index.css`
- `../lliwio-css/src/index.css`
- `src/text/buildEntryHtml.ts`
- `src/components/content/HtmlReader.tsx`
- `docs/architecture/security/handling_persisted_content.md`
