# Render the article title inside the reader iframe

## Summary

Move the canonical saved article title from the surrounding application UI into
the styled reader document as a sibling `<header><h1>` before `<article>`, while
preserving every existing annotation's body-text target.

## Requirements

- Pass the canonical saved Entry title into the reader-document builder and
  render this structure inside the iframe:

  ```html
  <body>
    <header><h1>Article title</h1></header>
    <article>Article body</article>
  </body>
  ```

- Insert the title as escaped text, never as trusted or interpolated HTML.
- Keep `<header>` outside `<article>` so metadata remains structurally separate
  from the stored article body.
- Style the header through the iframe's lliwio-css document and current reader
  appearance settings. Align it with the article's readable content width.
- Remove the permanently visible external MUI article-title heading. Retain the
  existing loading progress indication.
- Always render the canonical title in the first version. Do not inspect,
  suppress, or rewrite a matching heading already present in the article body;
  occasional duplicate headings are accepted.
- Treat annotation compatibility as a hard acceptance condition: every stored
  annotation must resolve to the same article-body text and occurrence before
  and after title injection, without migration or rewriting of its ID, text, or
  occurrence values.
- Scope annotation occurrence counting, highlight restoration, selection, and
  lookup to the `<article>` element rather than `document.body`. Header text
  must never affect an annotation occurrence index or receive a restored
  highlight.
- Do not create an annotation from a selection wholly inside the header or from
  a selection crossing the header/article boundary.
- Preserve clicking, editing, removing, and scrolling to existing annotations
  after the header changes the iframe document's vertical layout.
- Apply the title consistently to summary and full-content reader views and to
  content rebuilt after image recovery or an accepted article update.
- Keep the iframe sandbox, content sanitization, CSP, link routing, find UI, and
  reader-history behavior unchanged except for the intentional title display.

## Remaining work

- Extend `buildEntryHtml` to accept the canonical title, encode it safely, and
  emit the sibling header and article structure with reader-native styling.
- Pass the current article title through every reader-document build path and
  remove the external title component and obsolete layout styles.
- Give annotation traversal and selection an explicit article-content root,
  including rejection of header-only and cross-boundary selections.
- Add focused builder and component coverage for escaped title text, summary and
  content views, appearance settings, accepted duplicate headings, loading, and
  rebuild paths.
- Add annotation regressions where the title contains the same phrase as an
  existing annotation, where the body contains that phrase multiple times, and
  where an existing annotation is clicked or scrolled into view. Assert that
  stored occurrence values continue to select the same body passage.
- Cover refused header-only and header-to-article selections.
- Run frontend formatting, lint, tests, and the production build.
- Update the persisted-content rendering documentation with the title's trust
  boundary and the `<article>` annotation scope.

## References

- `src/components/content/ArticleReader.tsx`
- `src/components/content/ArticleReader.module.css`
- `src/components/content/ArticleReader.test.tsx`
- `src/components/content/HtmlReader.tsx`
- `src/text/buildEntryHtml.ts`
- `src/text/markdownPipeline.ts`
- `../lliwio-css/src/index.css`
- `docs/architecture/security/handling_persisted_content.md`
