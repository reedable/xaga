# Type Identity: intrinsic shape, extrinsic brand

**Shape is the source of truth for intrinsic types. A brand (`__kind`) is a derived
distillation, legitimate only for extrinsic role distinctions that structure cannot resolve. A
brand is minted from the source of truth and never trusted over shape.**

Ratified during the Feed/Entry branding design discussion; this document records it as the
standing rule.

## The rule

- **Intrinsic types** (what a thing *is* — e.g. Annotation, Topic) are identified by their
  structure; no discriminator is needed, and where a brand exists anyway it is cross-validated
  against shape at the boundary.
- **Extrinsic roles** (what part a thing plays — e.g. feed vs entry, which share the
  `WebResource` shape) are the one legitimate use of a brand: the shared shape cannot resolve
  the role, so `__kind` layers on top.
- **Identity is shape AND brand together — never either alone.** The target guards
  (`src/types.ts`):
  - `isEntry(item) = isWebResource(item) && item.__kind === "entry"`
  - `isFeed(item)  = isWebResource(item) && item.__kind === "feed"`

  The shape check (`isWebResource`) is the structural foundation and is never removed; the brand
  resolves the role. Posing brand-only versus shape-only as a fork misstates the rule — the two
  are conjoined, not alternatives.
- **Brands are minted at the wire boundary** (`toEntry`/`toFeed`/`toAnyEmbeddedItem`), from the
  source of truth, at the moment the object enters the typed world. A brand is never invented
  downstream and never used to overrule what the structure says.
- **Composite/view types are not branded.**

## Current state

`Entry`/`Feed` are branded and minted at the wire boundary; `SavedItem` rows
(`toSavedItem`, `src/platform/TauriPlatform.ts`) are not yet branded, so the guards in
`src/types.ts` run an interim brand-first / shape-fallback until `toSavedItem` mints
`__kind` — at which point the guards switch to the brand-and-shape form above.
