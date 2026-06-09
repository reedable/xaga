## Small (hours)

- [ ] Color code bookmarks and bookmark icons
- [ ] [ItemList.tsx](src/components/list/ItemList.tsx): Item list does not reload after adding or removing a manual item via TopicMenuButton
- [ ] [ItemList.tsx](src/components/list/ItemList.tsx): `onAnnotationChanged` does not add new annotations to the list
- [ ] [topics.rs](crates/xaga-core/src/topics.rs): `list_entry_topics` scans all topic files O(n) — called on every annotation removal and every TopicMenuButton open; needs a reverse index
- [ ] [Shortcuts.ts](src/Shortcuts.ts): `ALL_ITEMS` has hardcoded `entryCount: 0` / `unreadCount: 0` placeholders to satisfy the `Collection` type — replace with real counts or restructure the type
- [ ] [csp-hash.mjs](scripts/csp-hash.mjs): Fail the build on CSP hash drift — add an npm script (run in `beforeBuildCommand`) that recomputes the reader script's sha256 and errors if it doesn't match `script-src` in [tauri.conf.json](src-tauri/tauri.conf.json); currently the hash is regenerated/pasted by hand and rot is silent (blank reader in the bundle)

## Medium (days)

- [ ] [PdfReader.tsx](src/components/content/PdfReader.tsx): Inline PDF fetch must move behind Rust. The renderer's `fetch(src)` works in the dev server but is blocked in the bundle by CSP `connect-src` (and bypasses the SSRF proxy regardless). Add a Rust command that scheme-guards `http|https`, fetches through the existing SSRF proxy (`reqwest` with `.proxy(...)` → `ssrf_proxy_port`), validates Content-Type / `%PDF-` magic bytes, and returns the bytes over IPC; the renderer rebuilds `new Blob([bytes], { type: "application/pdf" })` and keeps the existing blob-URL approach. Open choice: base64 over `invoke` (simple, ~33% bloat) vs. a custom URI-scheme handler that streams (better for large PDFs). On Rust fetch failure (e.g. `reqwest` blocked, or an auth/cookie-gated PDF the bare client can't reach), fall back to the article loader webview ([article_loader.rs](src-tauri/src/article_loader.rs)), which carries per-domain cookies / named data store and can render it
- [ ] [TauriPlatform.ts](src/platform/TauriPlatform.ts): Invalid items silently dropped in ~10 list operations — add unified error reporting
- [ ] Sidebar is crowded/visually congested
- [ ] [feed_entry.rs](crates/xaga-core/src/feeds/feed_entry.rs): `created_at` should come from the `.md` Entry on disk, not derived elsewhere
- [ ] [AppearanceProvider.tsx](src/providers/AppearanceProvider.tsx): Add `systemFontFamily` and `systemFontSize` support
- [ ] [articles.rs](crates/xaga-core/src/articles.rs): Trigger annotation embedding for semantic search
- [ ] Favicon deduplication — currently stored per-entry in frontmatter, could bloat `.md` files over time
- [ ] Tag and Collection concepts are not clearly defined, and neither is bookmarks
- [ ] Settings: "Keep history" toggle (off by default) — persist navigation history across sessions (storage TBD); include a clear history action

## Large (uncertain)

- [ ] Back/forward button does not work well with filter (Unread/Saved/All) — consider taking filter out of routing
- [ ] [entryWorker.ts](src/text/entryWorker.ts): Move back into a Web Worker once the `document` variable issue in the remark pipeline is resolved
- [ ] Review UI → Tauri → xaga-core and xaga-cli → xaga-core call paths and identify unnecessary code
- [ ] Export annotations
- [ ] Keyboard navigation
- [ ] Semantic search UI — LanceDB and embeddings are wired up but there's no search interface yet
- [ ] Track centroid for saved items per Topic
- [ ] Split the concept of "Saved" and "Similar/Relevant"? Saved items are manually curated items, and relevant items are cosine similarity based on the running centroid of the topic + saved.
- [ ] Reading progress and stats (time spent, articles read per feed, etc.)

## Deferred

- [ ] Support showing the original site (not reader mode) and let the user control when to use it
