# Validate and package reader typography

## Summary

The reader imports the exact `lliwio-css` release tarball pinned in
`package.json`. Typography still needs application-level visual verification.

## Requirements

- Keep reader CSS bundled without runtime stylesheet requests.
- Preserve selection, annotation, find-highlight, link, and scrolling behavior.
- Support light, dark, and automatic themes, including an explicit theme that differs from the OS.
- Use a reproducible dependency source for development and release builds.
- Verify supported webviews and narrow reader panes.

## Remaining work

- Review prose, headings, quotations, code, tables, figures, multilingual text, and feed summaries in the bundled application.
- Review font and spacing controls and title typography.
- Verify interaction styling and supported CSS functions in each supported webview.
- Add targeted visual regression coverage.

## References

- `src/text/buildEntryHtml.ts`
- `src/components/content/HtmlReader.tsx`
- `src/components/content/ItemView.tsx`
