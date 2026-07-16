# Domain has no model of tags; row tags synthesized from booleans (pending — unapproved change awaiting decision)

## Current state

Domain types expose only the derived booleans (`is_unread`/`is_bookmarked`/`is_pinned`); there is no domain model of `tags`. Consequently the LanceDB row builders (`from_article`/`from_topic`/`from_feed`, `crates/xaga-core/src/embeddings/row.rs`) synthesize the `tags` column from those booleans. The original code copied raw frontmatter `tags`, so any non-state tag in a file is no longer mirrored into LanceDB.

## Provenance

The boolean-synthesis change was made during the persistence↔embeddings decoupling without approval (flagged in a report; never approved). The modeling gap couples row-building to "only three tags exist."

## Open decisions

Whether non-state tags should be mirrored (revert to copying frontmatter tags), and whether the domain should model `tags` at all. Related: `define_tag_collection_bookmark_concepts.md`.
