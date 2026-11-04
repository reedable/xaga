# Clarify Collection and ManagedCollection

## Summary

Express Collection as the common domain abstraction for Feed, Topic, and Site.
Name the narrower shared structure of Feed and Topic ManagedCollection, aligning
UI types and terminology without changing collection behavior or storage.

## Requirements

- Collection encompasses Feed, Topic, and Site. Membership can be supplied or
  derived; a collection need not have its own persisted membership list.
- ManagedCollection describes Feed and Topic: collections with their own
  identity and managed lifecycle, independent of their current members. The
  name does not imply manual selection of every member.
- Site is a Collection derived from article hostnames, not a ManagedCollection.
  Preserve its existing normalized-hostname identity and derived membership.
- On the UI side, represent Site identity only as `id: string`. Add a code comment
  explaining that it holds the normalized hostname. Map the backend payload's
  `hostname` to `Site.id` at the platform boundary and remove the duplicate UI
  `Site.hostname` field. Keep the backend hostname representation unchanged.
- Move the current member-bearing `Collection<T>` shape to ManagedCollection
  where appropriate. Do not require Site to invent creation timestamps, loaded
  member references, or persisted entity content to satisfy the common type.
- Make UI collection guards and navigation signatures distinguish common
  collection operations from operations specific to managed collections.
- Preserve existing list loading, navigation, counts, filtering, Feed/Topic
  editing, Site browsing, and article shortcuts. An article shortcut is not made
  a Collection solely because it establishes a list scope.
- Keep persisted formats and library compatibility unchanged; this type and
  terminology refactor does not require a storage-model version increment.

## Remaining work

- Trace common and managed-only collection consumers and derive the minimal
  common contract from their actual needs.
- Update TypeScript domain declarations, Site's domain placement, imports,
  guards, platform mappings, and collection-navigation dispatch.
- Update Site navigation, list selection, keys, and platform calls to use
  `Site.id`; pass that normalized-hostname value to backend hostname parameters.
  Preserve any required hostname identity checks when replacing the existing
  `isSite` guard; do not identify arbitrary collections as Sites merely by `id`.
- Align Rust domain terminology and boundary representations where they express
  the same concepts; do not add unused traits merely for symmetry.
- Document Collection, ManagedCollection, and Site in durable domain/architecture
  documentation, including the supplied-versus-derived membership distinction.
- Verify all three collection kinds in list selection, sidebar and responsive
  navigation, and ensure managed-only editing remains limited to Feed/Topic.
- Run frontend type checking, lint, affected tests, and production build. If
  Rust source changes, run workspace formatting and affected Rust checks.

## References

- [Organization model](../../docs/architecture/organization_model.md)
- `src/domain.ts`
- `src/types.ts`
- `src/hooks/useRouter.tsx`
- `src/components/list/ItemList.tsx`
- `src/components/sidebar/Sidebar.tsx`
- `src/platform/Platform.ts`
- `src/platform/TauriPlatform.ts`
- `crates/xaga-core/src/domain.rs`
- `docs/architecture/sites.md`
- `docs/conventions/vocabulary/semantic_vocabulary.md`
