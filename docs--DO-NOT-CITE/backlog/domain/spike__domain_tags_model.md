# Domain has no model of tags; row tags synthesized from booleans

## Current state

Domain types expose only the derived booleans (`is_unread`/`is_bookmarked`/`is_pinned`); there is no domain model of `tags`. Consequently the LanceDB row builders (`from_article`/`from_topic`/`from_feed`, `crates/xaga-core/src/embeddings/row.rs`) synthesize the `tags` column from those booleans. The original code copied raw frontmatter `tags`, so any non-state tag in a file is no longer mirrored into LanceDB.

## Decisions

Whether non-state tags should be mirrored (revert to copying frontmatter tags), and whether the domain should model `tags` at all.
