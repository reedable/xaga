# Improvement: lliwio-css typography in the article reader

## Remaining evaluation

- Review the reader with lliwio typography in the actual Tauri application:
  prose, headings, quotations, code, tables, figures, and multilingual text in
  both article bodies and feed summaries.
- Check light, dark, and automatic themes, including an explicit Xaga theme
  that differs from the OS. The iframe's `color-scheme` follows Xaga's resolved
  theme; lliwio supplies its existing color and semantic-color rules.
- Review font and spacing controls, narrow panes, and the CSS mathematical
  functions in the supported webviews.
- Check selection, annotations, find highlights, links, and scrolling visually.
- Assess title typography after reviewing the body. The title remains outside
  the iframe.

List indentation and spacing remain non-blocking as lliwio-css evolves.

The integration imports the sibling lliwio-css checkout at build time through
`src/text/buildEntryHtml.ts`. Vite embeds the CSS in the reader document, so no
runtime stylesheet fetch is needed. Decide how to distribute that dependency
when moving beyond the local integration exercise.

Relevant code: `src/text/buildEntryHtml.ts`,
`src/components/content/HtmlReader.tsx`, and the sibling library's
`src/index.css`. The reader uses the existing library entry point directly,
without maintaining a separate selection of styles.
