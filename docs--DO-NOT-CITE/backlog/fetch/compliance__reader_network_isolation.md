# Compliance: local-only UI rendering and article image capture

## Intended boundary

Block remote network access in the main and settings webviews. Remote retrieval
belongs in the article loader or the existing native fetch paths. Image handling
is the remaining functional dependency identified in the UI review; no direct
remote-request functionality was found in settings.

Enforce this at the webview boundary, including reader frames, rather than
relying on ingestion to remove every remote reference. A remote URL in Markdown,
embedded HTML, or CSS must not cause a remote request when viewed locally.
Locally persisted images, data URLs, and local PDF Blob presentation must continue
working. Preserve the local IPC connections needed by the application.

Remote resources intentionally added by the user follow the same rule: they do
not load. Document this as Xaga's behavior. Do not add CSS stripping to the
rendering pipeline solely to enforce this boundary.

## Current image handling

- The article loader already captures image bytes using JavaScript `fetch()`
  inside its proxied webview. Capture is currently enabled. This is separate
  from the native file-download pipeline used by PDFs and downloadable favicons.
- Ingestion persists captured images and rewrites article-body image references
  to local URL-hash paths. Even when bytes are unavailable, references become
  local paths; they do not fall back to remote retrieval.
- The reader resolves stored image references to local data URLs. Older or
  externally edited articles can still contain remote image references, which
  the current renderer preserves.
- Feed summaries are stored as HTML in frontmatter. They bypass article-body
  image extraction and the reader's local-image substitution. Their image URLs,
  including CSS background images, remain remote references when displayed.
- HTML-to-Markdown ingestion drops the tested inline background-image style.
  Rendering stored Markdown preserves that style when embedded HTML already
  contains it. Ingestion is therefore not a substitute for the network boundary.

The UI deny-all proxy rejects requests that reach it. However, the native
`ui_network_webview` regression demonstrates direct HTTP loopback requests from
both UI windows. Close this bypass before treating the webviews as network
isolated or changing image-reference persistence. Check HTTPS loopback,
navigation, redirects, child frames, and proxy unavailability as part of that
work. Refusals observed by the proxy already use the shared error sink.

Relevant code: `src-tauri/src/article_loader.js`, `src/text/entryWorker.ts`,
`crates/xaga-core/src/persistence/images.rs`, `src/text/inlineEntryImages.ts`,
`src/text/markdownPipeline.ts`, `src/components/content/ArticleReader.tsx`, and
`src-tauri/tauri.conf.json`.

## Privacy tradeoff: additional image capture requests

The page loads images normally, then Xaga issues additional fetches to obtain
their bytes for storage. Cache reuse may avoid another transfer, but is not
guaranteed. Extra requests and their request characteristics can expose
recognizable behavior to remote hosts. The network proxy controls destinations;
it does not conceal that behavior.

Capturing already-loaded DOM images through canvas would avoid Xaga's additional
fetches, but is not a lossless replacement. For a cross-origin image whose server
allows a credential-free CORS read:

| Page markup | Export existing image through canvas | Separate CORS fetch |
| --- | --- | --- |
| Ordinary `<img src="…">` | Blocked because the image was loaded without CORS mode | Can succeed |
| `<img crossorigin="anonymous" src="…">` | Allowed after a successful CORS load | Can succeed |

Ordinary cross-origin image markup is an important case to retain in evaluating
this tradeoff. If the server does not grant CORS access, the additional fetch
also fails. Do not set `crossorigin` and reload an image as a supposed way to
reuse the original load without changing request behavior.

Consider making additional image capture opt-in and off by default, consistent
with private-by-design. This is a proposed setting, not an implemented decision.
Describe the choice as saving article images for offline viewing, rather than
claiming it controls all image loading. Disabling capture must stop Xaga's extra
image fetches; the original page may still load images normally in the loader.
The existing “open in loader window” capability remains a way to view those
images without requiring capture or persistence.

## Image references: chosen approach

Establish and verify the UI network block before changing image-reference
persistence. Once that boundary is enforced:

- Rewrite an image reference to a local path only when its bytes have actually
  been persisted. Failed retrieval or persistence must not create a new dangling
  local reference.
- Keep the original remote URL in the Markdown when the image is not stored
  locally, including when capture is disabled. The webview boundary prevents
  that URL from causing a request during rendering.
- Treat remote image URLs identically whether introduced manually or retained
  after unsuccessful capture. Neither provenance requires special rendering or
  a remote-fetch exception.
- Preserve existing local images. A local file subsequently deleted or lost is
  a separate missing-file case, not a reason to fetch automatically.

The URL in the content preserves the information needed for a future explicit
image-only retrieval action. That action could use the controlled capture path,
persist successful results, and replace their references with local paths,
without reloading or replacing the article text. This matters when the original
article is no longer available but an image URL still works. The retrieval UI
and implementation are future work, not part of establishing the network block.

Review the frontmatter image map before removing it. It currently preserves
original URLs for both captured and uncaptured images; durable documentation
describes recovery as the reason for retaining them. The chosen representation
removes the need for a separate recovery mapping for uncaptured images. Check
other consumers and the needs of captured images before deciding whether the
map can be eliminated entirely.

Define conversion of existing missing local references using their frontmatter
URLs, preserving article text and working local images. Do not require a full
article reload to recover source URLs already present in the saved file. When
no source URL is available, leave that unresolved case explicit rather than
inventing one. Perform any library changes through the established writer.

## Saved articles and unavailable images

Blocking UI network access prevents older remote image references from loading.
Reloading an article can replace them with local images when capture is enabled
and retrieval succeeds. Reloading with capture disabled does not restore images.
Previously stored local image bytes should remain usable regardless of whether
future capture is enabled.

Under the chosen representation, uncaptured images and manually added remote
images both remain remote references that do not load. Decide how the reader
presents unavailable images and useful alt text without implying that enabling
UI network access is the remedy. No specific placeholder or notification design
has been chosen. This presentation choice does not require stripping embedded
CSS or distinguishing the origin of a remote URL.

If summary images should be available offline, define their capture and local
persistence behavior. They must not retain a render-time remote-fetch exception.

## Implementation and verification

- Close the demonstrated WebKit proxy bypass and verify enforcement for
  navigation, subresources, and child frames in a bundled application.
- Verify remote Markdown images, raw HTML images, CSS background images, summary
  images, and remote frame/navigation attempts cause no outbound request. Check
  local PDF viewer interactions as part of the boundary verification.
- Verify local article images, favicons, PDF Blobs, application assets, and IPC
  continue working, while article-loader retrieval remains functional.
- Only after verifying the network block, change persistence to retain remote
  URLs for uncaptured images and emit local references for successfully stored
  bytes. Cover disabled capture, CORS refusal, and failed image writes.
- Audit frontmatter-map consumers and define conversion of existing dangling
  references before removing any recovery metadata. Verify the conversion does
  not fetch remotely, replace article text, or disrupt working local images.
- Verify manually added and failed-capture remote references have the same
  rendering behavior and preserve their source URLs for future explicit retry.
- Decide the capture preference and disclosure. If introduced, verify it prevents
  additional capture requests when off, including in visible loaders, without
  promising that the page itself makes no image requests.
- Cover successful capture, CORS refusal, missing local files, older remote
  references, explicit reload, and already-persisted images with capture off.
- Update durable documentation with the enforced behavior and the chosen capture
  policy. Keep the network boundary independent of content provenance or edits.
