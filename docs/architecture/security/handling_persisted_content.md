# Handling persisted content: reading storage back as untrusted input

Storage is a boundary. Anything loaded from storage is untrusted content even when what was written was sanitized at ingest — the write-side sanitization that happens in [handling_remote_content.md](handling_remote_content.md) and [handling_user_supplied_content.md](handling_user_supplied_content.md) is not relied on here. This document covers what happens when previously-stored content is read back and rendered; it applies uniformly regardless of whether the content originally arrived by remote fetch, user import, or IPC (an annotation typed by the user, for instance) — once it's on disk, the provenance no longer matters and the same controls apply to all of it.

Code: `src/text/markdownPipeline.ts`, `src/text/buildEntryHtml.ts`, `src/components/content/HtmlReader.tsx`, `src-tauri/tauri.conf.json`.

## Rendering (markdown → reader)

The rendering path applies its own controls and does not depend on ingest-time sanitization having happened or having been sufficient.

- **Rendered output is sanitized with DOMPurify at render.** `markdownToHtml` (`markdownPipeline.ts`) parses embedded HTML (`remark-rehype` with `allowDangerousHtml` + `rehype-raw`) so HTML-bearing fields (e.g. feed summaries, which are stored as feed-supplied HTML) render, and passes the serialized output through `DOMPurify.sanitize` before it reaches `buildEntryHtml`. DOMPurify is the explicit, render-side sanitization control; it does not rely on ingest-time sanitization.
- **The pipeline is inert up to the sanitizer.** Every stage before DOMPurify (remark/rehype, including `rehype-raw`'s parse5) transforms strings and plain JS syntax trees — no DOM node construction, no `Image`, no capability to execute content or emit a network request at any stage (the configured pipeline runs in plain Node, where any DOM API access throws). DOMPurify's own parse uses a document with no browsing context, which loads no subresources and executes no scripts. Sanitization is placed **last**, closest to the sink: sanitizing before the markdown transforms would re-parse sanitized output through different parsers (the mXSS pattern). Same assumption and fallback as ingest (`entryWorker.ts`): if any pre-sanitizer stage is ever replaced with one that renders or executes content, sanitization must move before it.
- **The reader is a sandboxed, opaque-origin iframe.** `sandbox="allow-scripts"` only (`HtmlReader.tsx:340`) — no `allow-same-origin` — so the reader document has an opaque origin: no access to the app's DOM, IPC, or storage. Content is delivered as a `Blob` URL via `location.replace` and the object URL is revoked after load.
- **The reader's own script is CSP-hash-pinned.** The bundled CSP (`tauri.conf.json`) has `script-src 'self' 'sha256-…'` with no `unsafe-inline`; the single inline reader script is byte-stable and its hash is allow-listed (regenerate with `node scripts/csp-hash.mjs` when it changes). No other inline script can execute in the app's pages.
- **Annotations enter the reader through a JSON data island**, a non-executed `<script type="application/json">` block (`buildEntryHtml.ts:89-102`). Every `<` in the JSON is escaped to `\u003c`, so annotation text containing `</script>` or `<!--` cannot break out of the island element.
- **`postMessage` traffic is source-checked.** The parent only accepts messages whose `source` is the reader iframe's own `contentWindow` (`HtmlReader.tsx:273-276`), so a sibling frame (e.g. the PDF viewer) or any window holding a reference cannot spoof reader events such as link opens.
- **No raw-HTML sinks in the app UI.** The React tree contains no `dangerouslySetInnerHTML` and no `srcdoc`; feed titles/summaries render as React text nodes, which are escaped by construction.
- The app shell CSP also sets `object-src 'none'`, `base-uri 'self'`, `form-action 'none'`, and restricts `connect-src` to the IPC endpoints.

## Testing caveat

The CSP is enforced only in the bundled Tauri app, not under the Vite dev server. Any verification of CSP/iframe/sandbox behavior must use `tauri build`, not `tauri dev`.
