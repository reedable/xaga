## Small (hours)

- [ ] [ItemList.tsx](src/components/list/ItemList.tsx): Item list does not reload after adding or removing a manual item via TopicMenuButton
- [ ] [ItemList.tsx](src/components/list/ItemList.tsx): `onAnnotationChanged` does not add new annotations to the list
- [ ] [Shortcuts.ts](src/Shortcuts.ts): `ALL_ITEMS` has hardcoded `entryCount: 0` / `unreadCount: 0` placeholders to satisfy the `Collection` type — replace with real counts or restructure the type
- [ ] [csp-hash.mjs](scripts/csp-hash.mjs): Fail the build on CSP hash drift — add an npm script (run in `beforeBuildCommand`) that recomputes the reader script's sha256 and errors if it doesn't match `script-src` in [tauri.conf.json](src-tauri/tauri.conf.json); currently the hash is regenerated/pasted by hand and rot is silent (blank reader in the bundle)

## Medium (days)

- [x] [util.rs](crates/xaga-core/src/util.rs): `normalize_url` now also strips the fragment, rewrites http→https, and strips a leading `www.` — `url_hash12` IDs no longer match entry/feed/bookmark files saved under the old rules; add an xaga-cli migration command that re-keys existing files to the new normalization (done: `xaga-cli migrate [--dry-run]`; remove the command after running it against the real library)
- [ ] [util.rs](crates/xaga-core/src/util.rs): Reevaluate the forced https in `normalize_url` — its output isn't just hashed for IDs, it's also stored and fetched (`import_feeds` fetches the normalized URL; entries store it as `source`), so an http-only site would now be fetched over https and fail
- [ ] [topics.rs](crates/xaga-core/src/topics.rs): `list_entry_topics` scans all topic files O(n) — called on every annotation removal and every TopicMenuButton open; needs a reverse index
- [ ] [TauriPlatform.ts](src/platform/TauriPlatform.ts): Invalid items silently dropped in ~10 list operations — add unified error reporting
- [ ] [feed_entry.rs](crates/xaga-core/src/feeds/feed_entry.rs): `created_at` should come from the `.md` Entry on disk, not derived elsewhere
- [ ] [articles.rs](crates/xaga-core/src/articles.rs): Trigger annotation embedding for semantic search
- [ ] Favicon deduplication — currently stored per-entry in frontmatter, could bloat `.md` files over time
- [ ] Settings: "Keep history" toggle (off by default) — persist navigation history across sessions (storage TBD); include a clear history action

## Large (uncertain)

- [ ] Color code bookmarks and bookmark icons
- [ ] Sidebar is crowded/visually congested
- [ ] Tag and Collection concepts are not clearly defined, and neither is bookmarks
- [ ] Back/forward button does not work well with filter (Unread/Saved/All) — consider taking filter out of routing
- [ ] [entryWorker.ts](src/text/entryWorker.ts): Move back into a Web Worker once the `document` variable issue in the remark pipeline is resolved
- [ ] Review UI → Tauri → xaga-core and xaga-cli → xaga-core call paths and identify unnecessary code
- [ ] Export annotations
- [ ] Keyboard navigation
- [ ] Use magnitutde of embeddings to identify "weak signals" and consider suppressing them in results.
- [ ] Use magnitutde of embeddings to identify "weak signals" in Topics.
- [ ] Weak signals via magnitude won't work as written — [model.rs](crates/xaga-core/src/embeddings/model.rs) returns L2-normalised vectors, so stored magnitude is always 1.0. Options: capture pre-normalization magnitude at embed time, or redefine "weak signal" via similarity distribution. Either way the two items above need rewording or dropping.
- [ ] Track centroid for saved items per Topic
- [ ] Split the concept of "Saved" and "Similar/Relevant"? Saved items are manually curated items, and relevant items are cosine similarity based on the running centroid of the topic + saved.
- [ ] Reading progress and stats (time spent, articles read per feed, etc.)
