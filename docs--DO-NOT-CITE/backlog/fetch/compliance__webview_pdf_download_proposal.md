# Proposal: PDF downloads through the article loader

## Problem and intended outcome

When inline PDF viewing is enabled, Xaga currently starts each ordinary, unsaved URL load with a reqwest GET. If the response is a PDF, its bytes are checked and displayed locally. If it is HTML, the response is discarded and the article loader navigates to the original URL again using WebKit. The first request has no User-Agent; the second has WebKit's User-Agent. A host can observe the matching URL, timing, and client differences. This is the probe-induced profiling signal we intend to remove; removing it does not make all Xaga traffic indistinguishable from other browsing.

The proposed flow uses the existing article loader for the initial navigation and branches on the navigation response. HTML continues through the existing capture path. PDFs become downloads from that navigation, are checked, stored locally, and displayed inline or opened in the default app. No main-window fetch, separate probing webview, or reqwest PDF fetch is involved.

Current source locations:

- `src-tauri/src/commands/articles.rs`: `try_fetch_pdf`, `load_article`, and loader capture commands.
- `src-tauri/src/article_loader.rs` and `article_loader.js`: proxied loader navigation, session isolation, capture, and lifetime.
- `src/hooks/useArticles.tsx`: PDF bytes currently return before article persistence.
- `src/hooks/useContentLoader.ts`: saved-article lookup, temporary PDF Blob, and external URL opening.
- `src/components/content/PdfReader.tsx`: local Blob display in an iframe.
- `crates/xaga-core/src/persistence/entries.rs`: normalized-URL hash filenames for `.md` entries.

## Proposed request and storage sequence

1. Normalize the requested URL using the existing identity rules and look up locally stored content.
2. If a saved PDF exists, use its local bytes without retrieving the source again. Saved HTML continues through its existing read path. Explicit refresh bypasses this lookup.
3. Otherwise, create the existing article loader with its SSRF proxy and appropriate ephemeral or per-domain session store.
4. Navigate to the source URL once. Normal redirects, page subresources, and favicon handling still occur; “once” means no separate preliminary probe, not exactly one HTTP request for the entire operation.
5. On an HTML response, continue navigation and the existing extraction/persistence flow.
6. On a PDF response, convert the navigation response to a download. Choose a unique, app-controlled staging destination. Do not derive local paths from the server's suggested filename.
7. On successful download completion, use the staging path retained by native download state (the macOS completion callback may omit it); do not scan a downloads directory. Read the bytes and retain the existing `%PDF-` check.
8. If the check fails, reject the download through the existing unsupported-content presentation and remove the staging file. A ZIP or another ordinary download may reach the same callback; rejection does not imply deception by the server.
9. If the check succeeds, atomically commit the PDF to its persistent location using the library's write ownership rules.
10. Use the display preference to either create the existing local PDF Blob for the reading pane or open the stored file in the default app.

The `%PDF-` check remains a format-signature check. This proposal neither removes it nor adds a new PDF inspection mechanism.

## Dependency and routing integration

Replace the local Wry workspace override with the reviewed fork's exact Git
revision for reproducible builds. Keep the 0.57.0 patch base.

Enable `wry-pdf-downloads` by default only after connecting the application PDF
consumer. Verify rejection in main/settings windows and intended visible-loader
behavior before enabling the build-wide rule. Extend native verification to
remote PDF subframes, bundled application viewing, forbidden redirects, and
session behavior. Evaluate Windows/Linux implementations separately before
claiming cross-platform PDF conversion.

## Xaga implementation scope

- Remove the PDF reqwest probe and its HTML fallback request. This does not remove reqwest from unrelated feed or model-download paths.
- Connect the PDF consumer to the shared download pipeline with the existing PDF byte validator and PDF persistence. Preserve proxy lifetime until PDF download completion.
- Reconcile download completion with existing capture callbacks and timeouts so only one outcome resolves a load. Do not capture an empty document or close the loader while its download is pending.
- Move byte validation to downloaded-file processing and return validated local content through the existing rendering flow.
- Remove obsolete `reqwest_error` PDF fallback behavior. Review the explicitly visible loader (`keep_open`) separately so downloads and closing that window have intentional behavior.
- Rename PDF-spoof-specific error codes, diagnostics, and frontend flags to neutral unsupported-download terms where appropriate. Preserve rejection and user notification behavior.
- Add PDF persistence and local lookup, refresh, replacement, and deletion behavior.
- Add a narrowly scoped external-open operation for a validated, app-owned stored PDF. The existing general URL opener rejects file URLs; do not relax it to accept arbitrary local paths from remote content.
- Keep storage mutations under the established library writer/lock rules. Define read-only-library behavior for newly downloaded PDFs.

## Persistence proposal

Reuse the existing normalized URL and `url_hash12` strategy: a PDF is stored as `{hash12}.pdf`. The exact directory remains a decision. Lookup is deterministic and does not require a separate URL-to-file index simply to find the bytes.

The file should be durable across view changes and app restarts, not a temporary download deleted after display. This enables repeated inline viewing and “open in default app” without fetching again. Stage and validate before atomic replacement so a failed refresh leaves the previous valid copy intact.

Additional storage decisions remain:

- Is the PDF a library entry with metadata, listing, bookmarks, and deletion, or a persisted document associated with an existing entry? A hash filename alone does not supply title/source metadata or library integration.
- How is active content type represented if both `.md` and `.pdf` exist for the same URL? A `.md` file can contain useful metadata or user-authored annotations; do not blindly delete it when refreshing to a PDF.
- Which URL defines identity after redirects? Prefer consistency with existing article identity; any final-URL aliases need an explicit policy.
- How are missing or invalid stored files handled? Stored bytes remain untrusted; check before handing them to the PDF viewer or default app.
- How do deletion, abandoned staging-file cleanup, and concurrent loads interact with the writer and external viewers?

No text extraction, PDF annotation system, or PDF search indexing is implied by this proposal.

## UI behavior and copy

Retain the display switch. Both choices would use the same download and validation path; the switch would only decide where to display the local PDF. Preserve the existing preference value during migration unless a different default is chosen.

Proposed placement: General settings rather than Privacy, because the fetch-pattern tradeoff would be removed.

Proposed label: **Show PDFs in the reading pane**

Proposed explanation: **When off, PDFs open in your default app.**

Replace the current probe-specific Privacy settings explanation when removing the probe.

## Verification and acceptance criteria

Use controlled endpoints and request logs, plus focused persistence tests. A native download smoke test is necessary; frontend tests alone cannot verify proxy inheritance.

| Scenario | Expected result |
| --- | --- |
| Fresh HTML URL | No preliminary reqwest request; existing capture and persistence work. |
| Fresh PDF, inline enabled | Loader navigation becomes download; bytes validated, stored, and shown locally. |
| Fresh PDF, inline disabled | Same download/validation/storage flow; stored file opens in default app. |
| Reopen PDF, including after restart | Uses stored file without a retrieval request to the source. |
| Explicit refresh | Downloads replacement; failure preserves old valid content. |
| Invalid PDF bytes or ordinary non-PDF download | Existing neutral rejection UI; no inline display or external open. |
| Proxy and redirects | Download traffic stays through the configured proxy; forbidden destinations remain blocked, including download redirects. |
| Session settings | Incognito and per-domain persistent-session behavior remain intentional through download conversion. |
| Timeout, cancellation, network failure, window close | One resolved outcome; no orphaned active download, lingering proxy, or accepted partial file. |
| Repeated/concurrent requests | No staging collision, torn persistent file, or overwrite outside the writer rules. |
| Content changes between HTML and PDF | Explicit active-type behavior; existing metadata and user-authored content preserved. |
| Non-article download | Native phase routing excludes it from article PDF validation, persistence, display, and external opening. |
| Downloads permission initially undecided or denied | Download to app-owned staging; no Downloads permission prompt. |
| macOS completion omits destination | Native state identifies the assigned staging file without directory scanning. |
| Visible loader and other webviews | Their download and capture behavior remains intentional. |

Reqwest-specific redirect limits do not automatically transfer to WebKit. Audit the effective limits and preserve or explicitly resolve differences before claiming no loss of existing controls. Likewise, proxy inheritance on native download conversion is expected from the architecture but must be demonstrated in the running integration.

## Durable documentation changes after implementation

- `docs/principles/secure_by_default.md`: replace the probe-based inline-PDF rationale with the verified behavior.
- `docs/architecture/security/fetching_remote_content.md`: remove PDF-probe descriptions, document response-to-download routing, proxy/session behavior, and actual redirect/timeout enforcement. Keep feed/model reqwest descriptions where still true.
- `docs/architecture/security/handling_remote_content.md`: describe staging, signature validation, unsupported downloads, and persistence.
- `docs/architecture/security/handling_persisted_content.md`: document stored-PDF lookup, validation, local rendering, and external opening.
- `docs/architecture/security/performance_tradeoff.md`: remove obsolete PDF-probe examples and explain local reuse without claiming all network characteristics disappear.
- `docs/architecture/article_loading_diagnostics.md`: replace probe/fallback steps with download, validation, persistence, and local-open steps.
- Update writer/crash-consistency documentation where the PDF storage design extends those contracts. Record the fork commit and maintenance procedure in appropriate durable build documentation.

## Design decisions

1. Confirm the build-wide main-frame PDF rule for visible loaders; per-window exceptions would require additional configuration.
2. Choose the storage location and relationship to library metadata for PDFs keyed by normalized URL hash.
3. Decide whether to move the retained PDF display preference to General settings.
4. Resolve visible-loader downloads, read-only storage, content-type changes, and redirect-control differences during a focused implementation design pass.

## Implementation sequence

Connect PDF validation and persistence to the application download consumer.
Verify the full loader flow before enabling PDF routing and removing the probe.
Then apply settings changes and update durable documentation.
