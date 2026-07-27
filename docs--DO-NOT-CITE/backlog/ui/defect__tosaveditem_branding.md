# toSavedItem does not mint \_\_kind (pending)

## Current state

The target guard model is brand-AND-shape (`docs/architecture/type_identity.md`), but `SavedItem` — the DTO for `listAllItems`/`listTopicItems` (the All-items and Topic lists) — is not branded: `toSavedItem` (`src/platform/TauriPlatform.ts`) does not mint `__kind`. Requiring the brand today would drop every saved row, so the guards in `src/types.ts` run an interim brand-first / shape-fallback.

## The work (recorded target)

1. `toSavedItem` mints `__kind:"entry"` on its bookmark/entry rows (annotation rows stay detected by `isAnnotation`'s shape).
2. Extract the shared shape check as an `isWebResource` helper (not yet extracted).
3. Switch the guards to `isEntry = isWebResource(item) && item.__kind === "entry"` (and the `isFeed` equivalent).
