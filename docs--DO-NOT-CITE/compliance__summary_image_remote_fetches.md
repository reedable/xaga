# Compliance: feed summaries make remote fetches (tracking / read receipts)

An `<img>`, or a `style="background-image: url(…)"`, in a feed's `<summary>`
fetches when the entry is opened. The summary
is stored as feed HTML in the frontmatter `summary:` key and never passes
through `extract_images`; at render, `ArticleReader.tsx`'s summary branch skips
`inlineEntryImages`.

Strip remote URLs from the summary at ingestion.
