# Topic/Collection storage and identity

## The question

Whether Topic should be stored as `.atom` (like feeds) and distinguished from Feed by a branded `__kind` rather than by a structurally distinct type.

## Design considerations

- Feed and Topic are both `Collection`s; the proposed differences mostly reduce to extrinsic/role or implementation: provenance (extrinsic); "body content" vs feed `subtitle` (both are the collection's description); typed heterogeneous membership (a shadow of the curation role). So Topic may be `Collection & __kind:"topic"`.
- The one possibly-intrinsic divider is `url` / web-resource-ness: a Feed _is_ a fetchable web document (url = identity, relocation-invariant); a Topic has no external existence — unless published, at which point it genuinely becomes feed-like and `.atom` fits.
- Do not mint synthetic URIs for topics (`file://` = storage location, fails relocation-invariance as identity; `xaga-topic://id` = the id in costume).

## Modeling problems

1. Topic identity is derived from the filename (`item_id(path)`) instead of the declared id — identity should be declared, location derived.
2. `topic_path` bakes in `.md` (`format!("{id}.md")`) — format is assumed, not declared/discovered; the first break point if topic formats go mixed.

Related standing rule: `docs/architecture/type_identity.md` (brand for extrinsic roles only). This item is exploration — no decision has been made.
