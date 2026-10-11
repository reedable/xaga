# Improvement: unavailable images and feed-summary image handling

- Review the HTML-to-Markdown and Markdown-to-reader pipelines to establish
  what image information survives, including alt text, titles, and source URLs.
- Design the display of unavailable images using the preserved information.
  Distinguish useful alternative text from a broken-image presentation without
  requiring different handling for manual edits and capture failures.
- Decide how opt-in capture applies to feed-summary images and how those images
  are persisted and rendered locally. Summaries can carry the primary content,
  as with xkcd; do not assume their images are merely decorative.
- Cover HTML summary images and CSS image references without adding a
  render-time remote-fetch exception.
- Verify the resulting presentation for unavailable images, preserved alt text,
  and image-centric summaries, with capture enabled and disabled.

Relevant code: `src/text/markdownPipeline.ts`, `src/text/entryWorker.ts`, and
`src/components/content/ArticleReader.tsx`.

Establish any needed summary-image baseline when working on this task. Update
durable documentation with reader presentation and summary-image capture,
persistence, and rendering behavior.
