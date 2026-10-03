# Compliance: consolidate favicon acquisition, storage, and rendering

Status: **open; investigation and implementation design required**.
Consolidated 2026-10-03. This is a self-contained backlog item, not a claim
that the proposed behavior is implemented. Security concerns give this item
the `compliance__` label; related correctness and enhancement work is included
so the favicon lifecycle can be reviewed together.

## Intended outcome

Acquire favicons only through the controlled remote-content context, store
validated local image bytes, and render them without remote requests from the
privileged main window. Missing or unusable icons must not trigger an implicit
file download, access the user's Downloads folder, or unnecessarily delay an
already captured article. Favicon work must remain distinct from article/PDF
processing even when the same webview performs both.

## Current implementation

- `src-tauri/src/article_loader.js` selects a declared icon, preferring exact
  `32x32`, then `16x16`, then the first icon link. It resolves against
  `document.baseURI` and falls back to `/favicon.ico`.
- `src-tauri/src/commands/articles.rs::receive_html` retains captured HTML and,
  for eligible hidden loaders, navigates the loader to the favicon URL. This
  permits image-document canvas readback without depending on cross-origin
  CORS permission. Visible loaders and some PDF/fallback paths skip this step.
- The loader script reads the image document and re-encodes it to PNG on a
  square canvas capped at 64 pixels. `receive_favicon` completes the pending
  capture. Article HTML survives favicon failure or the overall deadline.
- `src/text/entryWorker.ts::extractFavicon` independently selects an icon.
  Unlike the loader, it resolves against the supplied article URL and has no
  conventional `/favicon.ico` fallback. These paths can disagree.
- `crates/xaga-core/src/persistence/articles.rs` and `images.rs` localize
  favicon references into the entry's image storage. Missing bytes can produce
  a local reference with no file, retaining the source URL in the URL map.
  Failure therefore does not necessarily mean a newly saved remote reference.
- `src/components/content/Toolbar.tsx` reads local references through IPC but
  ignores other stored favicon values, showing the existing globe icon instead.
  The direct remote-URL rendering fallback was removed on 2026-10-03.

## Confirmed outstanding issues

### 1. Favicon navigation can become an unsolicited download

Loading `https://con.racket-lang.org/` without clicking within the page caused
a macOS Downloads permission prompt. A process sample showed Wry's
`download_policy` accepting a destination, followed by WebKit sandbox-extension
creation. Instrumented reproduction `1a101eb5293-1` identified the resource:

| Observation | Value |
| --- | --- |
| Original article | `https://con.racket-lang.org/` |
| HTML delivered / favicon started | 1784 ms |
| Download callback | 1813 ms |
| Download URL | `https://con.racket-lang.org/favicon.ico` |
| Proposed destination | `~/Downloads/favicon.ico` |
| Instrumented policy | Rejected |
| Completion callback | 1814 ms, `success: false`, no path |
| Favicon wait ended | 10025 ms, overall deadline |

The download was triggered by Xaga's favicon acquisition navigation. It was
not evidence of a user-requested download, a PDF, malware, or server deception.
Response headers were not captured: why WebKit classified this response as a
download (including MIME type or Content-Disposition) remains unverified.

Wry 0.57.0 installs an accepting download handler by default. Omitting an Xaga
handler does not disable downloads. Xaga now explicitly rejects loader
downloads and records diagnostic evidence, but this is not a complete favicon
solution. Wry checks the proposed Downloads destination for existence before
calling the application handler. Rejection cannot prevent that earlier access.

### 2. Rejected favicon download leaves article capture waiting

The observed download rejection does not complete the pending favicon phase.
The article's HTML is already available, yet completion waits for the overall
load deadline. Known terminal favicon failures should settle favicon work
immediately, preserving the article and resolving the load only once.

### 3. Legacy remote references: rendering bypass removed; recovery undecided

The toolbar's direct rendering of non-local stored favicon values was removed
on 2026-10-03. Only persisted local references resolved through IPC supply an
image; other values use the existing globe icon without a favicon request.
Stored metadata is not rewritten by this change. Recovery for older or edited
entries remains undecided: controlled reacquisition, explicit refresh, or a
placeholder. Any recovery must preserve the local-only rendering boundary.

### 4. Discovery paths differ and failed local captures need a recovery policy

The loader and extraction worker duplicate selection logic with different base
URL and fallback behavior. Consolidate their contract and test both execution
contexts. Missing image files after failed capture also need an explicit
placeholder/retry/refresh policy; do not reintroduce remote rendering to repair
them. Visible-loader and timeout paths must have intentional outcomes when
they provide an icon URL without bytes.

## Historical issues, not assumed current

A July 16 investigation recorded a toolbar `new Image()` probe followed by
remote image rendering, both outside the loader. Its promise also lacked an
`onerror` handler. The historical issue file was deleted in commit `e29d622`
on July 27. The current toolbar no longer uses that probe: acquisition moved
into the loader and persistence gained local image storage. Do not reopen the
old unresolved-promise defect as though that implementation still exists.
The subsequent legacy remote-rendering fallback has now also been removed;
recovery for entries lacking local bytes remains open.

## Design decisions and enhancement investigation

These are questions to resolve, not verified defects or approved features:

- Define supported formats and selection/fallback policy: multi-value `sizes`,
  `sizes="any"`, ICO/PNG/SVG, touch icons, and optionally manifest icons.
  Establish whether failure should try another declared candidate.
- Define byte, decoded-dimension, time, redirect, and candidate-count limits.
  The 64-pixel output canvas does not by itself bound network bytes or the
  source image's decode allocation. Audit actual enforcement before claiming
  these limits exist.
- Review relative URLs, `<base>`, redirects, cross-origin icons, unusual
  schemes, malformed responses, and author-controlled icon metadata.
- Decide whether per-entry storage remains sufficient or origin-level reuse
  is useful. Specify freshness, negative caching, explicit refresh, concurrent
  loads, deletion, and read-only-library behavior before adding a cache.
- Define image normalization behavior, including nonsquare icons, and safe
  handling of unsupported or active formats. Never infer image safety from a
  filename or declared MIME type alone.
- Determine an acquisition method that preserves proxy/session containment
  without a new observable probe or remote fetching in the main window.

## Requirements affecting a future Wry PDF patch

Track article and favicon phases in native state, rather than treating every
download callback as an article PDF candidate. A favicon must never enter PDF
persistence, inline rendering, or default-app opening. Download destination
policy must run before default-directory filesystem checks; select app-owned
staging if downloading becomes the chosen acquisition method. No implicit
access to the user's Downloads directory is acceptable.

Handler presence alone cannot scope a compile-time PDF conversion patch,
because Wry supplies a default handler. Audit hidden/visible loaders, other
windows, and subframes. Retain native ownership of assigned staging paths:
macOS completion callbacks may omit the path. Any staged-file design must
define correlation, cleanup, cancellation, and concurrent-request behavior.
The exact Wry/Tauri API or patch scope remains undecided.

## Diagnostics and verification

Existing native download evidence records URL, proposed destination, loader
identity, original/current URL context, and completion status. These fields
are preserved in copied diagnostics and bounded release logs. URLs and local
paths may be sensitive; review before sharing. Current URL context does not
prove initiating-frame identity. Response headers, MIME type, HTTP status,
and initiating frame are unavailable through the current Tauri callback.
See `docs/architecture/article_loading_diagnostics.md` for the implemented
diagnostic contract.

Use controlled endpoints and native webview tests, covering:

- Declared and conventional icons; differing base URLs and selection paths.
- Cross-origin icons without CORS; redirects and forbidden proxy destinations.
- Displayable images, attachment responses, generic binary MIME, HTML error
  responses, malformed images, oversized images, missing icons, and stalls.
- Terminal favicon failure completes promptly without losing article HTML;
  no double completion, orphaned loader, lingering proxy, or partial file.
- No Downloads access or prompt with permission either undecided or denied.
- No remote main-window favicon requests for new, legacy, or edited entries.
- Local reuse, missing files, refresh, read-only storage, and concurrent loads.
- Favicon downloads cannot enter the article PDF pipeline; visible-loader and
  other-window behavior is explicit.

Before closure, update durable acquisition, persistence, rendering, and
diagnostics documentation to match the verified implementation. This item
stands alone; no obsolete backlog documents need to be consulted or updated.
