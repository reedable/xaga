# Handling untrusted remote content: parsing, processing, persistence, display

How Xaga secures remote bytes _after_ they arrive — feed XML and article HTML
through parsing, transformation, storage, and rendering. The network side
(which hosts can be reached at all) is covered in
[fetching_untrusted_content.md](fetching_untrusted_content.md).

## Feed XML (Atom)

Code: `crates/xaga-core/src/atom.rs`,
`crates/xaga-core/src/persistence/feeds.rs`, `crates/xaga-core/src/util.rs`.

### Parsing

- **No XXE, no entity expansion.** Both parsers — `feed_rs` for incoming
  fetched XML and `atom_syndication` for locally stored `.atom` files — sit on
  quick-xml, which does not resolve DTDs or external entities. There is no
  path from feed XML to local-file reads or request amplification.
- **Custom entity references are neutralized before parsing.**
  `fix_bare_ampersands` (`atom.rs`) rewrites any `&` that does not begin one
  of the five predefined entities or a character reference — so `&xxe;`
  becomes the inert text `&amp;xxe;`. (Its purpose is lenient parsing of
  publisher XML with unencoded ampersands; neutralizing custom entities is a
  property it provides on the way.) CDATA sections are copied through
  verbatim, never modified.
- Parse errors are propagated as strings; malformed feeds fail the refresh,
  they don't crash it.

### Identity and file naming

We defend against path traversal by never using remote data as a filename.
Feeds and entries are stored as `{hash12}.atom` / `{hash12}.md`, where
`hash12` is the first 12 hex characters of SHA-256 over the normalized URL
(`url_hash12`, `util.rs`) — the filename alphabet is lowercase hex only.
Entry ids from the feed (`<entry><id>`) are used as _logical_ ids only; when
absent, the id falls back to the hashed link (`feed_entry_id`).

`normalize_url` also upgrades `http://` to `https://` at add time, so a feed's
canonical fetch URL is https.

### What a hostile feed cannot rewrite

- **Its own fetch URL.** Refresh fetches the stored `.atom`'s
  `<link rel="self">`, which is seeded from the user-entered URL;
  `apply_incoming_xml` merges id/title/entries from the fetched document but
  never overwrites feed-level links. A feed cannot repoint where its next
  refresh goes.
- **App-local state.** The `pinned` flag and tags live in Atom `<category>`
  elements under the app's own scheme (`TAG_CATEGORY_SCHEME =
https://xaga.app/ns/tag`), so publisher-supplied categories can never
  collide with them.

### Persistence

- **`.atom` re-serialization is escape-aware.** Text nodes are escaped by the
  quick-xml writer; link hrefs — which `atom_syndication` writes verbatim —
  are pre-encoded by `xml_attr_encode` (`atom.rs`) so a crafted href cannot
  inject markup into the stored XML that would parse differently on the next
  read.
- **`.md` frontmatter is serializer-built, not templated.** `render_entry`
  (`persistence/entries.rs`) serializes attacker-controlled title and summary
  through `serde_yaml`; a title containing `---\ntags: [bookmark]` is escaped,
  not interpreted. Frontmatter injection cannot flip tags or add keys.
- **Writes are atomic and crash-ordered.** `.atom` files are written via
  `write_atomic_async`; on refresh, entry `.md` files are persisted _before_
  the `.atom` membership (`write_atom`, `persistence/feeds.rs`). After a crash
  between the two, the next refresh re-adds the missing membership from the
  live feed and skips the already-written `.md` files; no content is lost.
  The reverse order could lose content.

## Article HTML

Code: `src/text/entryWorker.ts`, `src/text/markdownPipeline.ts`,
`src/text/buildEntryHtml.ts`, `src/components/content/HtmlReader.tsx`,
`src-tauri/tauri.conf.json`.

### Processing (capture → markdown)

The loader webview returns the page's raw DOM HTML. `processEntry`
(`entryWorker.ts`) then:

1. Parses it with `DOMParser` and runs Readability — both operate on an inert,
   non-rendered document; no scripts execute, no event handlers fire, no
   subresources load. (This inertness assumption is documented at the call
   site; if it ever stops holding, sanitization must move _before_ parsing.)
2. **Sanitizes Readability's output with DOMPurify** — the explicit security
   boundary for article bodies (`entryWorker.ts:68-79`). This strips
   `<script>`, event-handler attributes, `javascript:`/`data:` URIs, etc. The
   downstream HTML→markdown conversion is _not_ relied on to drop dangerous
   markup — that would be a side effect, not a guarantee, and a dependency
   upgrade could quietly change it.
3. Converts the sanitized HTML to markdown (rehype→remark), which is what gets
   persisted.

### Persistence

Article markdown is stored in the same `{hash12}.md` files as feed entries,
with the same serde_yaml frontmatter boundary for title/summary/annotations
(see above). What is on disk has already crossed the DOMPurify boundary.

### Rendering (markdown → reader)

Storage is a boundary. Anything loaded from storage is untrusted content
even when what was written was sanitized at ingest. Stored content handling
falls under this document. The rendering path applies its own controls and does
not depend on ingest-time sanitization having happened or having been
sufficient.

- **Raw HTML in markdown is dropped at render.** `markdownToHtml`
  (`markdownPipeline.ts`) uses `remark-rehype` without `allowDangerousHtml`,
  so HTML embedded in stored markdown is discarded, not emitted.
- **The reader is a sandboxed, opaque-origin iframe.**
  `sandbox="allow-scripts"` only (`HtmlReader.tsx:340`) — no
  `allow-same-origin` — so the reader document has an opaque origin: no access
  to the app's DOM, IPC, or storage. Content is delivered as a `Blob` URL via
  `location.replace` and the object URL is revoked after load.
- **The reader's own script is CSP-hash-pinned.** The bundled CSP
  (`tauri.conf.json`) has `script-src 'self' 'sha256-…'` with no
  `unsafe-inline`; the single inline reader script is byte-stable and its hash
  is allow-listed (regenerate with `node scripts/csp-hash.mjs` when it
  changes). No other inline script can execute in the app's pages.
- **Annotations enter the reader through a JSON data island**, a non-executed
  `<script type="application/json">` block (`buildEntryHtml.ts:89-102`). Every
  `<` in the JSON is escaped to `\u003c`, so annotation text containing
  `</script>` or `<!--` cannot break out of the island element.
- **`postMessage` traffic is source-checked.** The parent only accepts
  messages whose `source` is the reader iframe's own `contentWindow`
  (`HtmlReader.tsx:273-276`), so a sibling frame (e.g. the PDF viewer) or any
  window holding a reference cannot spoof reader events such as link opens.
- **No raw-HTML sinks in the app UI.** The React tree contains no
  `dangerouslySetInnerHTML` and no `srcdoc`; feed titles/summaries render as
  React text nodes, which are escaped by construction.
- The app shell CSP also sets `object-src 'none'`, `base-uri 'self'`,
  `form-action 'none'`, and restricts `connect-src` to the IPC endpoints.

### Testing caveat

The CSP is enforced only in the bundled Tauri app, not under the Vite dev
server. Any verification of CSP/iframe/sandbox behavior must use
`tauri build`, not `tauri dev`.
