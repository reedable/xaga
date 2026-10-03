# Proposal: PDF downloads through the article loader

Status: **for review; not approved for implementation**. Written 2026-10-02. Updated 2026-10-03.

This records the approach discussed with the project owner, the implementation scope, and unresolved decisions. It does not change the current architecture or authorize a fork, dependency change, or migration. Durable documentation should describe this behavior only after implementation and verification.

## Findings that revise the patch scope (2026-10-03)

**The proposal remains unapproved. No Wry PDF patch has been implemented.**
The upstream baseline now uses Tauri/runtime-wry 2.12.1 and Wry 0.57.0.
The following findings supersede the earlier assumption that omitting an
application download handler disables downloads.

### Wry behavior and required design changes

Wry 0.57.0's default attributes install
`download_started_handler: Some(Box::new(|_, _| true))`. Thus “a handler exists”
is not evidence that Xaga explicitly enabled downloads and is not a sufficient
way to scope a compile-time PDF policy to article loaders.

On macOS, `wkwebview/download.rs` selects the system Downloads directory and
checks the proposed destination's existence **before** calling the application
handler. Returning false prevents accepting the download, but cannot prevent
that preceding filesystem check.

Revise the patch/integration design to:

1. Let Xaga reject a request or choose an app-owned staging destination before
   any existence check or filesystem access to a default Downloads path. The
   exact API/compile-time mechanism remains to be designed; do not assume a
   MIME-policy-only patch is sufficient or that no Tauri plumbing is needed.
2. Track each loader's native purpose/phase explicitly. Only article downloads
   may enter article PDF validation, persistence, display, or external opening.
3. Scope forced PDF conversion to intended article navigations. Inventory main,
   settings, hidden-loader, visible-loader, and subframe behavior; the default
   handler means registration alone cannot select these cases.
4. Preserve native ownership of the chosen staging path. On macOS Tauri's
   completion callback can report no path, so retain the assigned destination
   per download instead of relying on completion to return it or scanning a
   directory. Correlation for concurrent downloads remains a design question.

### Instrumentation now available

Article loaders now explicitly reject downloads and record native URL,
proposed destination, loader identity, source/current URL context, and completion
status in bounded diagnostics/release logs. The frontend schema was corrected
so Copy diagnostics preserves this evidence. See
`docs/architecture/article_loading_diagnostics.md` for collection limits and
sensitive URL/path handling. These callbacks do not provide response headers,
MIME type, HTTP status, or initiating-frame attribution. A current webview URL
is context, not proof of the download initiator.

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

## Wry patch and dependency strategy

The initial investigation inspected Wry 0.54.4; the upgraded upstream baseline is now Wry 0.57.0. The upstream investigation through the 0.57.0 changelog and `dev` response handler did not find an exposed response-policy callback that solves this flow.

In Wry's WebKit backend, `navigation_policy_response` currently chooses download for a response WebKit cannot display when a download handler exists. Otherwise it allows display. The proposed small patch adds PDF MIME-type recognition to that download decision. It targets the **response** policy, where MIME type is available, rather than the request navigation policy.

This is fixed behavior in the patched build. Xaga registers the existing Tauri download callback on the loader. The original MIME-only design did not require a new runtime policy API through Tauri and tauri-runtime-wry; the destination-ordering and scoping requirements above must now be assessed before retaining that claim. A Cargo feature could make the patched behavior build-time opt-in, but is optional and would not itself select individual windows.

Proposed development workflow:

1. Fork Wry and maintain a small, documented patch against a compatible upstream version.
2. Clone locally for development and exercise Xaga with a local Cargo override.
3. Pin the reviewed fork commit using workspace `[patch.crates-io]` so Tauri's transitive Wry dependency resolves to the patched source. Verify resolution in `Cargo.lock` and `cargo tree`.
4. Keep the upstream base and patch rationale recorded so future upgrades can reapply or retire the patch.

The condition “download handler registered” does not adequately scope the behavior: Wry installs an accepting handler by default, and the callback is not a PDF-only contract. Other downloads can use the same callback. We must inventory other windows with download handlers and define visible-loader behavior explicitly.

A WebKit-backend patch alone is not a cross-platform implementation. Equivalent behavior in Windows and Linux backends must be investigated before claiming platform-neutral support. The product-level flow can remain shared while Wry owns backend differences.

Upstream references inspected during discussion:

- https://github.com/tauri-apps/wry/blob/dev/src/wkwebview/navigation.rs
- https://github.com/tauri-apps/wry/blob/dev/CHANGELOG.md

These are moving references; record the exact base commit when implementing.

## Xaga implementation scope

- Remove the PDF reqwest probe and its HTML fallback request. This does not remove reqwest from unrelated feed or model-download paths.
- Register loader download callbacks, choose staging paths, handle completion/failure/cancellation, and preserve proxy lifetime until download completion.
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

The reading-pane rejection copy has already been changed independently to:

> This download can’t be displayed in the reading pane because its format isn’t supported.

This avoids falsely claiming that another content type was advertised as PDF or that the remote site attempted a bait-and-switch. It does not disclose the particular rejected content type.

The earlier edit to Privacy settings still describes the current probe behavior; replace it as part of the implemented change, not as a claim that the probe has already been removed.

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
| Downloads permission initially undecided or denied | No access to the user Downloads directory, including pre-callback existence checks; no permission prompt. |
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

## Decisions for review

1. Accept maintaining a small pinned Wry fork, or defer this architecture pending upstream support?
2. Implement and verify macOS first with an explicit platform scope, or require equivalent Windows/Linux behavior before shipping?
3. Approve persistent PDFs keyed by normalized URL hash; choose their storage location and relationship to library metadata.
4. Approve retaining the display preference, moving it to General, and opening validated local PDFs in the default app when inline viewing is off.
5. Resolve visible-loader downloads, read-only storage, content-type changes, and redirect-control differences during a focused implementation design pass.

If approved, first demonstrate the smallest Wry patch with controlled HTML/PDF responses, proxy/redirect checks, and Tauri's existing callbacks. Then implement persistence and Xaga integration, followed by UI/settings migration and durable documentation. This validates the critical dependency before committing to the broader storage changes.
