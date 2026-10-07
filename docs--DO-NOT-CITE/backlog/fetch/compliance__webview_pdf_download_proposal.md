# PDF download integration: remaining verification and distribution work

## Reproducible dependency

Replace the sibling Wry path override with the reviewed fork's exact Git revision.
Keep the 0.57.0 patch base. Confirm clean-checkout dependency resolution before
shipping a build that depends on the patch.

## Bundled application verification

Exercise the complete application on macOS with inline viewing enabled and disabled:

- Fresh PDF, saved PDF after restart, and explicit refresh.
- HTML-to-PDF and PDF-to-HTML transitions while retaining annotations.
- Invalid bytes, failed refresh, cancellation, window close, and read-only library.
- Incognito and per-domain persistent sessions through download conversion.
- Forbidden redirect destinations and Downloads permission initially undecided.
- Visible loaders, main/settings rejection, and remote PDF subframes.

The controlled native test covers WebKit download routing and validation, not all
of these bundled application interactions or successful PDF pixel rendering.

## Redirect and platform scope

The configured reqwest redirect-count limit does not apply to WebKit navigation.
Decide whether a separate WebKit redirect-count control is required. The proxy
continues enforcing destination policy and the loader deadline bounds waiting.

Evaluate Windows/Linux response-to-download support before claiming the macOS
PDF routing behavior on those platforms.

## Library document lifecycle

PDFs are URL-associated files, not independently indexed entries. Decide whether
they need listing, metadata, bookmarks, and explicit deletion. Define cleanup of
unreferenced persisted PDFs and interaction with files open in external viewers.
PDF text extraction, search indexing, and annotation support need separate scope.
