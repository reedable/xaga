# Handling untrusted remote content: parsing, processing, ingest persistence

How Xaga secures remote bytes _after_ they arrive — feed XML and article HTML through parsing, transformation, and the write into storage. The network side (which hosts can be reached at all) is covered in [fetching_remote_content.md](fetching_remote_content.md). What happens once content is in storage — including reading it back for rendering — is a separate boundary, covered in [handling_persisted_content.md](handling_persisted_content.md). Content supplied by the user rather than fetched (OPML/bookmarks import) is covered in [handling_user_supplied_content.md](handling_user_supplied_content.md).

## Feed XML (Atom)

Code: `crates/xaga-core/src/atom.rs`, `crates/xaga-core/src/persistence/feeds.rs`, `crates/xaga-core/src/util.rs`.

### Parsing

- **No XXE, no entity expansion.** Both parsers — `feed_rs` for incoming fetched XML and `atom_syndication` for locally stored `.atom` files — sit on quick-xml, which does not resolve DTDs or external entities. There is no path from feed XML to local-file reads or request amplification.
- **Custom entity references are neutralized before parsing.** `fix_bare_ampersands` (`atom.rs`) rewrites any `&` that does not begin one of the five predefined entities or a character reference — so `&xxe;` becomes the inert text `&amp;xxe;`. (Its purpose is lenient parsing of publisher XML with unencoded ampersands; neutralizing custom entities is a property it provides on the way.) CDATA sections are copied through verbatim, never modified.
- Parse errors are propagated as strings; malformed feeds fail the refresh, they don't crash it.

### Identity and file naming

We defend against path traversal by never using remote data as a filename. Feeds and entries are stored as `{hash12}.atom` / `{hash12}.md`, where `hash12` is the first 12 hex characters of SHA-256 over the normalized URL (`url_hash12`, `util.rs`) — the filename alphabet is lowercase hex only. Entry ids from the feed (`<entry><id>`) are used as _logical_ ids only; when absent, the id falls back to the hashed link (`feed_entry_id`).

`normalize_url` also upgrades `http://` to `https://` at add time, so a feed's canonical fetch URL is https.

### What a hostile feed cannot rewrite

- **Its own fetch URL.** Refresh fetches the stored `.atom`'s `<link rel="self">`, which is seeded from the user-entered URL; `apply_incoming_xml` merges id/title/entries from the fetched document but never overwrites feed-level links. A feed cannot repoint where its next refresh goes.
- **Pin membership.** Pins live in the library’s separate `pins.yaml` store. Runtime Feed parsing does not interpret categories as pin state, so publisher categories cannot create or remove pins.

### Persistence

- **`.atom` re-serialization is escape-aware.** Text nodes are escaped by the quick-xml writer; link hrefs — which `atom_syndication` writes verbatim — are pre-encoded by `xml_attr_encode` (`atom.rs`) so a crafted href cannot inject markup into the stored XML that would parse differently on the next read.
- **`.md` frontmatter is serializer-built, not templated.** Entry persistence uses the shared document upsert and `md::patch_document`, which serialize attacker-controlled title and summary through `noyalib`; a title containing `---\ntags: [bookmark]` is escaped, not interpreted. Frontmatter injection cannot flip tags or add keys.
- **Writes are atomic and crash-ordered.** `.atom` files are written via `write_atomic_async`; on refresh, entry `.md` files are persisted _before_ the `.atom` membership (`commit_via_writer`, `persistence/feeds.rs`, which awaits the entry writes before the `.atom` write). After a crash between the two, the next refresh re-adds the missing membership from the live feed and skips the already-written `.md` files; no content is lost. The reverse order could lose content.

## Article HTML

Code: `src/text/entryWorker.ts`, `src/text/markdownPipeline.ts`, `src/text/buildEntryHtml.ts`, `src/components/content/HtmlReader.tsx`, `src-tauri/tauri.conf.json`.

### Processing (capture → markdown)

The loader webview returns a serialized snapshot of the page's DOM HTML. `processEntry` (`entryWorker.ts`) then:

1. Parses it with `DOMParser` and runs Readability — both operate on an inert, non-rendered document; no scripts execute, no event handlers fire, no subresources load. (This inertness assumption is documented at the call site; if it ever stops holding, sanitization must move _before_ parsing.)
2. **Sanitizes Readability's output with DOMPurify** — the step that removes unsafe markup from article bodies (`entryWorker.ts:68-79`). This strips `<script>`, event-handler attributes, `javascript:`/`data:` URIs, etc. The downstream HTML→markdown conversion is _not_ relied on to drop dangerous markup — that would be a side effect, not a guarantee, and a dependency upgrade could quietly change it.
3. Converts the sanitized HTML to markdown (rehype→remark), which is what gets persisted.

### Persistence

Article markdown is stored in the same `{hash12}.md` files as feed entries, with the same `noyalib` frontmatter validation and serialization for title/summary/annotations (see above). Captured article bodies have been sanitized with DOMPurify; feed-supplied summaries are persisted as supplied. Reading stored content back and rendering it is a separate boundary — see [handling_persisted_content.md](handling_persisted_content.md).

Before HTML leaves the loader, an inert document snapshot records each available
browser-selected `img.currentSrc` in `data-xaga-selected-src`. Normal, deadline,
visible capture-and-close, and SPA deliveries share this snapshot operation. It
does not change live image sources or initiate resource acquisition.

Before Readability, conversion uses the selected URL as `img src` when no
successful capture pairing exists, then removes the snapshot attribute. If no
selected URL is available, the existing `src` is retained; conversion does not
select a `srcset` or `<picture>` candidate itself. Readability resolves relative
sources against the article URL. Alternative text and titles survive conversion.

Successful capture's `data-xaga-url` plus data-URI bytes remains authoritative,
even if browser selection changes before serialization. The restored remote URL
keys the bytes sent to persistence. Failed or disabled capture retains the
selected remote URL in Markdown without supplying bytes. Only successful image
writes replace those remote references with local paths.

Feed summaries bypass article conversion and can retain responsive-image HTML.
Their rendering/recovery policy and externally edited Markdown are unchanged.

## Native PDF downloads

The article consumer stages downloads outside the library, applies the injected
PDF validator, and only then submits accepted bytes to the library writer.
The validator requires a `%PDF-` prefix and at most 64 MiB. MIME drives WebKit's
routing but does not authorize persistence. Invalid format produces a neutral
unsupported-download error; transport and staging errors retain failure details.
No PDF bytes supplied by renderer capture callbacks are accepted.

PDF writes share the normalized source URL's entry lane with Markdown writes.
Replacement is atomic; failed validation preserves the previous file. Saving a
PDF preserves Markdown and annotations. A successful HTML save removes its
associated PDF so subsequent URL lookup selects the HTML representation.
