# Compliance: legacy stored content still fetches at read time

Entries saved before ingestion started storing bytes locally still carry remote
URLs, and the main window fetches them when the entry is opened:

- bodies with `https:` image URLs — `inlineEntryImages` returns them unchanged
- frontmatter `favicon:` holding an `https:` URL — `Toolbar.tsx` uses it as-is

Reloading an article rewrites both. Decide whether that is enough, or whether a
migration or a prompt is wanted.
