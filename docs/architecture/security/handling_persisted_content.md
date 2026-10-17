# Handling persisted content: reading storage back as untrusted input

Storage is a boundary. Anything loaded from storage is untrusted content even when what was written was sanitized at ingest — the write-side sanitization that happens in [handling_remote_content.md](handling_remote_content.md) and [handling_user_supplied_content.md](handling_user_supplied_content.md) is not relied on here. This document covers what happens when previously-stored content is read back and rendered; it applies uniformly regardless of whether the content originally arrived by remote fetch, user import, or IPC (an annotation typed by the user, for instance) — once it's on disk, the provenance no longer matters and the same controls apply to all of it.

Code: `src/text/markdownPipeline.ts`, `src/text/buildEntryHtml.ts`, `src/components/content/HtmlReader.tsx`, `src-tauri/tauri.conf.json`.

## Rendering (markdown → reader)

The rendering path applies its own controls and does not depend on ingest-time sanitization having happened or having been sufficient.

- **Rendered output is sanitized with DOMPurify at render.** `markdownToHtml` (`markdownPipeline.ts`) parses embedded HTML (`remark-rehype` with `allowDangerousHtml` + `rehype-raw`) so HTML-bearing fields (e.g. feed summaries, which are stored as feed-supplied HTML) render, and passes the serialized output through `DOMPurify.sanitize` before it reaches `buildEntryHtml`. DOMPurify is the explicit, render-side sanitization control; it does not rely on ingest-time sanitization.
- **The pipeline is inert up to the sanitizer.** Every stage before DOMPurify (remark/rehype, including `rehype-raw`'s parse5) transforms strings and plain JS syntax trees — no DOM node construction, no `Image`, no capability to execute content or emit a network request at any stage (the configured pipeline runs in plain Node, where any DOM API access throws). DOMPurify's own parse uses a document with no browsing context, which loads no subresources and executes no scripts. Sanitization is placed **last**, closest to the sink: sanitizing before the markdown transforms would re-parse sanitized output through different parsers (the mXSS pattern). Same assumption and fallback as ingest (`entryWorker.ts`): if any pre-sanitizer stage is ever replaced with one that renders or executes content, sanitization must move before it.
- **The reader is a sandboxed, opaque-origin iframe.** `sandbox="allow-scripts"` only (`HtmlReader.tsx:340`) — no `allow-same-origin` — so the reader document has an opaque origin: no access to the app's DOM, IPC, or storage. Content is delivered as a `Blob` URL via `location.replace` and the object URL is revoked after load.
- **The reader's own script is CSP-hash-pinned.** The bundled CSP (`tauri.conf.json`) has `script-src 'self' 'sha256-…'` with no `unsafe-inline`; the single inline reader script is byte-stable and its hash is allow-listed. `node scripts/csp-hash.mjs --check` verifies the configured hash and runs before every production frontend build; run the command without `--check` to print a replacement value. No other inline script can execute in the app's pages.
- **Annotations enter the reader through a JSON data island**, a non-executed `<script type="application/json">` block (`buildEntryHtml.ts:89-102`). Every `<` in the JSON is escaped to `\u003c`, so annotation text containing `</script>` or `<!--` cannot break out of the island element.
- **`postMessage` traffic is source-checked.** The parent only accepts messages whose `source` is the reader iframe's own `contentWindow` (`HtmlReader.tsx:273-276`), so a sibling frame (e.g. the PDF viewer) or any window holding a reference cannot spoof reader events such as link opens.
- **No raw-HTML sinks in the app UI.** The React tree contains no `dangerouslySetInnerHTML` and no `srcdoc`; feed titles/summaries render as React text nodes, which are escaped by construction.
- The app shell CSP also sets `object-src 'none'`, `base-uri 'self'`, `form-action 'none'`, and restricts `connect-src` to the IPC endpoints.

## Images in summaries and bodies

Feed ingestion continues to store the supplied summary in entry frontmatter,
including remote image references. It does not acquire or localize those images.
Summary images, uncaptured body images, and manually inserted remote references
use the same recovery and presentation path when their content is viewed.

`prepareEntryImages` resolves local image references in HTML, inline CSS and embedded stylesheets to
local data URIs. Remote HTML `img` sources and responsive candidates are removed
before reaching the reader. CSS declarations with unavailable URL or
image-set resources and external stylesheet imports are removed. The bundled `img-src` permits only self and data
images. This policy does not rely on the UI proxy being a complete boundary.
An unavailable image displays its nonempty alternative text, then its title when
that adds content; without either, it displays “Image unavailable.” Decode failures
use the same presentation. Stored content is not changed to this fallback text.

Inline SVG is different from an SVG resource displayed through `img` or a CSS
image property. DOMPurify removes inline SVG scripts and event handlers, but
SVG resource references such as `image href`, `xlink:href`, and `feImage href`
can survive sanitization and image preparation. Their network loading is
restricted by the reader's CSP. Sanitization alone is not a no-network boundary:
the native no-CSP control requests external SVG images even after the production
sanitizer and preparation functions have run.

The native SVG probe described in
[fetching_remote_content.md](fetching_remote_content.md) verifies that the tested
references make no network requests under the exact app CSP. It also verifies
that SVG data resources used as HTML/CSS images load without executing scripts
or fetching their nested resources in the tested WebKit image contexts. This
does not imply that stored SVG bytes have been sanitized or that opening those
bytes as a top-level document has the same protections.

SVG favicons also use locally stored resources rendered as images in the toolbar,
not inline SVG markup in the main webview. Their original validated SVG bytes
are retained. Opening a cached article does not itself retry a failed favicon
acquisition; reloading the article retries its icon. A successful save and the
toolbar's refresh timing are separate: the current UI can require navigating away
and back before displaying the updated icon.

The account setting `load_remote_image` gates recovery and the article loader's
additional body-image capture fetches. The absent-value default is defined once
as `DEFAULT_LOAD_REMOTE_IMAGE`, currently `false`, for both new and existing
installations without a stored value; explicit values take precedence.
Settings → Privacy exposes “Save article images for offline viewing.” Changes
are persisted before the switch updates; failed writes display an error.
There is no eager default write. The preference applies to subsequent ingestion,
article reloads and reader recovery attempts. Loader windows snapshot the value
when created, including visible loaders; reopen existing windows after changing it.
Changing the default later does not override a stored preference. Disabling it
leaves already-stored local images usable and does not control a publisher page's
normal resource loading in an opened loader window. Opening the original page
in a loader window remains available without opting in. Enabling this setting
allows additional network requests; it does not grant network access to the main
or settings UI. Favicon acquisition is a separate phase and is not gated by this
body-image preference.

Capture diagnostics distinguish the preference from actual fetch attempts and
explain skipped capture, including when the page exhausts the loader deadline
before fetching begins. See [Article loading diagnostics](../article_loading_diagnostics.md#article-image-capture).

The reader displays local content before waiting for recovery. Only the currently
viewed field is recovered. Acquisition uses an isolated image-purpose loader;
neither the main webview nor the reader fetches the remote resource. Reference
spans are discovered with Markdown, HTML and CSS parsers, preserving the source
text outside the replaced destinations. The original remote URL determines image
storage identity, including when acquisition redirects.

Recovery discovers inline Markdown image destinations, HTML `img src`, and CSS
image references. It does not resolve reference-style Markdown image definitions
or candidates supplied only through `srcset` or `<picture><source>`. Rendering
removes responsive candidates and uses an available `img src`; without a usable
source, the image receives the common unavailable presentation.

`recover_entry_images` captures the library/writer at invocation. After loading,
`LibraryWriter::recover_images` rereads the entry inside its writer lane and
compares the targeted field with the snapshot that initiated recovery. A changed
field is returned without replacement. A successful write saves image files
before replacing their references with local paths. Failed acquisition or image
writes leave those references untouched. Summary-only recovery preserves the
body, and body-only recovery preserves the summary. Unrelated metadata,
annotations, read state and the stored PDF representation are preserved. Recovery
never creates a missing entry. Cleanup considers references throughout the saved
document. Reading in a read-only library does not initiate recovery downloads.

Validation covers exact source preservation, stale-field refusal, failed writes,
HTML/CSS local resolution, unavailable-image presentation and the preference gate.
Network and sandbox behavior require native/bundled checks; Node tests alone do
not establish those properties.

## Preserving frontmatter during writes

Markdown file framing is intentionally assembled by the application: an opening
`---` delimiter, frontmatter serialized by noyalib, a closing `---` delimiter,
and the Markdown body. This framing is accepted; YAML field syntax, quoting,
indentation, and collections are delegated to the serializer. The framing code
is `crates/xaga-core/src/md.rs::patch_document`.

Existing entry and topic documents are validated before modification. Malformed
frontmatter aborts the operation; a parse failure must not trigger replacement
with fresh defaults. Article saves validate before storing images as well.

Writes parse frontmatter into a mapping, apply supplied field values, and use
noyalib to serialize the complete mapping. YAML formatting and comments are not
retained by serialization. Block scalars are disabled in noyalib serialization
so embedded carriage returns are escaped in quoted strings and survive a
round trip; noyalib 0.0.15 block scalar output fails on these summaries. Article body saves preserve the values of tags, aliases,
annotations, and other omitted fields. The unsupported `images` frontmatter field is
removed on every save, as required by `crash_consistency.md`. Omitted summary, publication dates, and favicon
values preserve their existing values. Annotation changes edit the annotations
field; tag and pin changes edit tags. Metadata-only operations preserve the
exact Markdown body. Topic edits preserve unrelated frontmatter and body.
All Markdown persistence operations go through `persistence::documents::upsert`.
It reads the existing file, or starts with empty frontmatter if the file is absent,
and applies the supplied fields through the same patch operation. Omitted fields
remain untouched; supplied values (including empty values) are stored as supplied.
Read and validation failures abort the operation.
The generic atomic writer rejects `.md` paths. Its Markdown counterpart accepts
only a validated document constructed inside the upsert module.

Edits remain inside the existing library-writer lanes and use atomic file
replacement. The same strict parser policy validates both input and output.
Article image cleanup considers references throughout the saved document,
including frontmatter, so a body refresh does not delete summary-only images.

## Repairing summaries from local Atom

Run `xaga-cli --data-dir /path/to/library articles repair-summaries` to restore
article summaries lost from frontmatter. The command reads the library's local
`feeds/*.atom` and `entries/*.md`; it does not fetch remote content.

Only an absent `summary` key is eligible. Existing values, including an empty
string or null, are left untouched. Entries are matched by normalized source URL.
A nonempty local Atom summary is applied through the shared Markdown upsert under
the library lock and entry writer lane, preserving the body and other fields
(subject to the unsupported `images` removal rule above). The command does
not create missing articles. If no local summary is available, it leaves the
article unchanged.

Use `xaga-cli --dry-run --data-dir /path/to/library articles repair-summaries`
to run the same scan and report would-repair, skipped, unavailable, and error
counts without submitting any article writes. Before counting a repair, dry-run
uses the same frontmatter patch serializer and output parser as the write path;
serialization failures are reported as errors. The flag also works after the
subcommand. Dry-run uses the same library lock as repair.

The command prints repaired, skipped, unavailable, and error counts. Malformed
files are reported and left untouched; other articles continue. Errors produce
a nonzero exit status. Close the app before running if it holds the library lock.
Regression coverage lives in `persistence/summary_repair.rs`.

## Testing caveat

The CSP is enforced only in the bundled Tauri app, not under the Vite dev server. Any verification of CSP/iframe/sandbox behavior must use `tauri build`, not `tauri dev`.

Verified against the running `tauri dev` server (Vite at `http://localhost:1420`): the served document has no CSP `<meta>` tag (`document.querySelector('meta[http-equiv="Content-Security-Policy"]')` returns `undefined`) and no `Content-Security-Policy` response header. Tauri's v2 config reference states the `csp` value is also injected on dev when `devCsp` is unset; Xaga's dev setup does not appear to match that.

## Stored PDFs

PDF lookup derives `pdfs/{hash12}.pdf` from a normalized HTTP(S) source URL inside
the active library. Reads reject nonregular files and symlink directories, bound
the bytes to 64 MiB, and recheck the `%PDF-` signature. A missing file proceeds to
normal article lookup/ingestion; an invalid stored file reports an error. Refresh
can replace it after validation. These are format and path checks, not a guarantee
of PDF parser safety.

The trusted `read_stored_pdf` command returns validated bytes for local Blob
presentation. The trusted `open_stored_pdf` command derives and validates the same
file before opening it in the default app; it accepts a source URL, never a local
path. Loader pages cannot invoke either command. Cached reads work in read-only
libraries, while new PDF persistence requires the active library writer.
