## Small (hours)

- [x] Bookmark icons do not appear in ItemList/RelatedItemList
- [x] CollectionList header: shortcuts hover highlight collides with traffic lights
- [x] Refresh button location is awkward
- [x] [Logger.ts](src/utils/Logger.ts): Context tracking is broken
- [x] Color code annotation quotes icon
- [ ] Color code bookmarks and bookmark icons
- [x] Move "open in browser" to a new UI location (favicon was repurposed for Show Original Site)
- [ ] [ItemList.tsx](src/components/list/ItemList.tsx): Item list does not reload after adding or removing a manual item via TopicMenuButton
- [ ] [ItemList.tsx](src/components/list/ItemList.tsx): `onAnnotationChanged` only patches items in place — does not add new annotations to or remove deleted annotations from the list
- [ ] [topics.rs](crates/xaga-core/src/topics.rs): `list_entry_topics` scans all topic files O(n) — called on every annotation removal and every TopicMenuButton open; needs a reverse index
- [x] [TopicMenuButton.tsx](src/components/content/TopicMenuButton.tsx): Long topic lists make the "Add to Topic" menu unwieldy — replace MUI Menu with a right-side Drawer containing a filter input and scrollable topic list

## Medium (days)

- [ ] [TauriPlatform.ts](src/platform/TauriPlatform.ts): Invalid items silently dropped in ~10 list operations — add unified error reporting
- [x] Collection menu display is awkward and overlaps with annotation tooltip
- [ ] Sidebar is crowded/visually congested
- [ ] [feed_entry.rs](crates/xaga-core/src/feeds/feed_entry.rs): `created_at` should come from the `.md` Entry on disk, not derived elsewhere
- [ ] [AppearanceProvider.tsx](src/providers/AppearanceProvider.tsx): Add `systemFontFamily` and `systemFontSize` support
- [ ] [articles.rs](crates/xaga-core/src/articles.rs): Trigger annotation embedding for semantic search
- [ ] Favicon deduplication — currently stored per-entry in frontmatter, could bloat `.md` files over time
- [ ] Tag and Collection concepts are not clearly defined, and neither is bookmarks
- [ ] Settings: "Keep history" toggle (off by default) — persist navigation history across sessions (storage TBD); include a clear history action
- [ ] Handle PDF display in ItemView/ArticleReader

## Large / uncertain

- [ ] Back/forward button does not work well with filter (Unread/Saved/All) — consider taking filter out of routing
- [ ] [entryWorker.ts](src/text/entryWorker.ts): Move back into a Web Worker once the `document` variable issue in the remark pipeline is resolved
- [ ] Review UI → Tauri → xaga-core and xaga-cli → xaga-core call paths and identify unnecessary code
- [ ] Export annotations and notes
- [ ] Keyboard navigation
- [ ] Support showing the original site (not reader mode) and let the user control when to use it
- [ ] Give the user a chance to interact with the original site before running it through reader
- [ ] Semantic search UI — LanceDB and embeddings are wired up but there's no search interface yet
- [ ] Track centroid for saved items per Topic
- [ ] Split the concept of "Saved" and "Relevant". Saved items are manually curated items, and relevant items are cosine similarity based on the running centroid of the topic + saved.
- [ ] Reading progress and stats (time spent, articles read per feed, etc.)
