# Type Identity: intrinsic shape, extrinsic brand

**Shape is the source of truth for intrinsic types. A brand (`__kind`) is a derived distillation, legitimate only for extrinsic role distinctions that structure cannot resolve. A brand is minted from the source of truth and never trusted over shape.**

This document records the standing rule.

## The rule

- **Intrinsic types** (what a thing _is_ — e.g. Annotation, Topic) are identified by their structure; no discriminator is needed, and where a brand exists anyway it is cross-validated against shape at the boundary.
- **Extrinsic roles** (what part a thing plays — e.g. feed vs entry, which share the `WebResource` shape) are the one legitimate use of a brand: the shared shape cannot resolve the role, so `__kind` layers on top.
- **Identity is shape AND brand together — never either alone.** The domain guards (`src/domain.ts`):
  - `isEntry(entity) = isManagedEntity(entity) && entity.__kind === "entry" && typeof entity.url === "string"`
  - `isFeed(entity) = isManagedEntity(entity) && entity.__kind === "feed" && Array.isArray(entity.members)`

  The structural checks are the foundation; the brand resolves the role. Posing brand-only versus shape-only as a fork misstates the rule — the two are conjoined, not alternatives.

- **Brands are minted at the wire boundary** (`toEntry`/`toFeed`/`toAnyEmbeddedEntity`), from the source of truth, at the moment the object enters the typed world. A brand is never invented downstream and never used to overrule what the structure says.
- **Capability intersections and generic result wrappers do not introduce brands.**
  Countable adds counts; Ranked adds similarity. A derived domain collection such
  as Site retains its own kind so that collection navigation can identify it.

## Entity and collection guards

The base guards are structural: `isEntity` requires string `id` and `title`,
and `isManagedEntity` additionally requires a string `createdAt`. They describe
shared contracts rather than a concrete domain kind.

`isSite` requires Entity shape, the Site brand, and numeric summary counts. It
does not require managed lifecycle fields. `isManagedCollection` requires Feed
or Topic shape and numeric summary counts; `isCollection` accepts either a
managed collection or a Site. Count-bearing entries and built-in shortcuts do
not become collections merely by having counts.

The platform adapter mints Entry, Feed, Topic, Annotation, and Site brands from
validated native payloads. See [Collections and identity](organization_model.md#collections-and-identity)
for the Entity, ManagedEntity, and collection hierarchy.
