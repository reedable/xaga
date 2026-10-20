# Browse articles by Site

## Summary

Add a **Sites** sidebar section that derives one Site for every normalized
hostname represented by a stored Entry URL and lets the user browse that Site's
articles with the same list filters used for Feeds.

## Requirements

- Derive Site membership from each Entry's stored article URL, never from Feed
  URL or Feed membership.
- Accept only HTTP(S) Entry URLs with a valid DNS hostname; exclude malformed,
  hostname-less, non-HTTP(S), and IP-literal URLs.
- Normalize a Site identity from the URL hostname by applying URL-parser host
  normalization, removing a terminal dot, and treating one leading `www.` as
  equivalent to the hostname without it. Do not collapse any other subdomain.
- Use the normalized hostname as the Site's stable identity and displayed title;
  do not identify it as a publisher or infer ownership between hostnames.
- Return every represented Site, including Sites containing only one Entry.
- Return authoritative total and unread Entry counts for each Site.
- Add Sites as a fourth sidebar accordion section following the established Feed
  and Topic interaction pattern where practical.
- Selecting a Site lists only Entries whose stored URLs normalize to that Site,
  with the standard `unread`, `saved`, and `all` filters.
- Keep Site membership and counts derived and rebuildable; do not persist a
  second membership index as a source of truth.
- Handle Entry additions, URL changes, state changes, and removals without
  requiring an application relaunch.

## Remaining work

- Add the normalized-hostname value and shared normalization function in the
  core layer, with coverage for scheme, case, terminal dots, `www.`, subdomains,
  invalid URLs, and excluded hosts.
- Add focused Site summary and Site-entry query APIs without materializing
  embedding vectors or opening shared state once per Site.
- Carry the Site summary and list operations through the Tauri and TypeScript
  platform boundary.
- Add the Sites accordion, selection routing, counts, and filtered item list.
- Cover one-article Sites, multiple Sites, `www.` equivalence, distinct
  subdomains, Feed-independent membership, live updates, and each item filter.
- Run Rust and frontend formatting, lint, tests, and the production build.
- Add durable documentation for Site identity, membership, and derived-state
  ownership.

## References

- `crates/xaga-core/src/types.rs`
- `crates/xaga-core/src/query/items.rs`
- `crates/xaga-core/src/embeddings/db.rs`
- `src-tauri/src/commands/feeds.rs`
- `src/components/sidebar/Sidebar.tsx`
- `src/components/sidebar/FeedList.tsx`
- `src/components/list/ItemList.tsx`
- `src/domain.ts`
- `src/platform/Platform.ts`
- `docs/principles/single_source_of_truth.md`

