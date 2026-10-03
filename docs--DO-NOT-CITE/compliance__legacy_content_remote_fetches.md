# Compliance: legacy stored article bodies still fetch images at read time

Entries saved before ingestion started storing image bytes locally can still
carry `https:` image URLs in their bodies. `inlineEntryImages` returns those
URLs unchanged, allowing requests when the entry is opened.

Reloading an article rewrites these body-image references. Decide whether that
is sufficient or whether migration or a prompt is needed.
