# Type Identity: fields and `__kind`

**Check an object’s fields to establish its structure. Use `__kind` to distinguish
roles that can share that structure, and check the fields as well as the kind.
Create kind values when converting native payloads into UI domain objects.**

## Structure and role

Some types, such as Annotation and Topic, have distinguishing fields. Others,
such as Feed and Entry, share the `WebResource` structure and need a role
indicator. The project uses `__kind` for that indicator; this document also calls
it a brand.

A brand cannot override missing or incompatible fields. Where a type has a
brand, its guard checks both the expected structure and the brand. The domain
guards in `src/domain.ts` include:

- `isEntry(entity) = isManagedEntity(entity) && entity.__kind === "entry" && typeof entity.url === "string"`
- `isFeed(entity) = isManagedEntity(entity) && entity.__kind === "feed" && Array.isArray(entity.members)`

The native-payload adapters (`toEntry`, `toFeed`, and `toAnyEmbeddedEntity`)
establish the kind as an object enters the UI domain model. Do not assign a new
kind later merely to make an object pass a guard.

Capability types and generic result wrappers do not need new kinds. `Countable`
adds counts; `Ranked` adds similarity. A Site has its own kind because collection
navigation must distinguish it from Feed and Topic.

## Entity and collection guards

The base guards check fields: `isEntity` requires string `id` and `title`;
`isManagedEntity` additionally requires a string `createdAt`. These are shared
requirements rather than concrete entity kinds.

`isSite` requires Entity fields, `__kind: "site"`, and numeric summary counts.
It does not require managed-lifecycle fields. `isManagedCollection` requires
Feed or Topic fields and numeric summary counts; `isCollection` accepts either a
managed collection or a Site. An Entry or built-in shortcut does not become a
collection merely because it has counts.

The platform adapter supplies Entry, Feed, Topic, Annotation, and Site kinds from
validated native payloads. See
[Collections and identity](organization_model.md#collections-and-identity) for
the type hierarchy.
