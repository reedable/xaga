# Wry PDF response routing

Xaga's workspace overrides Wry with the sibling `../wry` checkout. The patch
base is Wry 0.57.0, revision `792d0359ba6501a4fc360ece17de2ae42329a47c`.
The local override requires that checkout; it is not a reproducible remote pin.

## Build features

Wry's default-off `macos-download-pdf` feature changes the macOS navigation
response policy. Xaga's default-enabled `wry-pdf-downloads` feature forwards to it
through a macOS-targeted direct dependency. Tauri uses the same resolved Wry
package; no Tauri source modifications are involved.

Xaga connects article downloads to the shared staging pipeline with `PdfValidator`.
There is no preliminary reqwest PDF request. Main and settings windows reject
downloads; article loaders select the PDF consumer before HTML capture and the
image consumer during favicon capture.

## Persistence and presentation

Validated PDF bytes are committed through the library writer to
`pdfs/{url_hash12(normalize_url(source_url))}.pdf`. The source URL, rather than a
redirect destination, defines identity. The PDF and Markdown operations share
the entry's writer lane. PDF replacement preserves existing Markdown and its
annotations; a successful HTML save removes the associated PDF. A failed PDF
validation leaves the previous stored PDF intact.

Local lookup precedes article ingestion. Stored PDFs are size-bounded and their
signature is rechecked before use. Explicit refresh bypasses local lookup.
New downloads require a writable library; cached files remain readable in a
read-only library. PDF files are associated documents, not separately indexed
library entries.

The General setting **Show PDFs in the reading pane** controls presentation.
When enabled, the frontend displays validated bytes through a local Blob URL.
Otherwise, `open_stored_pdf` resolves and validates the library file from its
source URL and asks the default application to open it. This trusted command
does not accept a caller-supplied filesystem path. The setting does not change
the remote retrieval path.

PDF acceptance requires a `%PDF-` prefix and a maximum size of 64 MiB. This is a
format check, not proof that a PDF parser can process the document safely.
Download staging is removed on completion, cancellation, and window cleanup.
The loader deadline bounds waiting; the disk-size monitor samples every 50 ms
and can overshoot between checks. WebKit downloads retain the loader's proxy
and session configuration. The reqwest redirect-count setting does not govern
WebKit navigation; the proxy still enforces destination policy on each connection.

## Policy

When enabled on macOS, the policy chooses download if a handler exists and either
WebKit cannot display the response or all of these conditions hold:

- Main-frame response.
- HTTP or HTTPS response URL.
- MIME type `application/pdf`, case-insensitive, ignoring MIME parameters.

The PDF predicate does not use URL extensions. Missing MIME or URL does not
match. Local Blob/file URLs and displayable subframes retain their normal
policy. The feature is build-wide and includes visible loaders; it does not
select windows by label. Applications must reject downloads where they have
no resource consumer.

MIME is only a routing hint. Wry does not validate PDF bytes or choose Xaga's
storage layout. The consumer assigns temporary storage, validates the bytes,
and persists accepted output through the library writer.

## Native verification

`src-tauri/tests/resource_download_webview.rs` serves controlled resources
through a local proxy using a reserved external hostname. Its cases are `icon`,
`pdf`, `invalid-pdf`, `html`, and `blob-pdf`. Run each in a macOS GUI session:

```sh
XAGA_DOWNLOAD_SMOKE_CASE=pdf cargo test -p xaga --test resource_download_webview --features wry-pdf-downloads --locked --offline
```

The download cases verify redirect delivery, one retrieval of the final resource,
retained staging paths when the callback returns no path, injected validation,
and cleanup. The PDF fixture is a one-page PDF; the invalid fixture is labelled
PDF but fails the signature validator. The test uses the production validator with the current
`%PDF-` signature contract; it does not establish parser safety.

The rendering cases check HTML document and local PDF iframe load signals with
no download callback. They do not visually inspect PDF pixels or establish
all bundled application behaviors. The proxy fixture verifies routing through
a configured proxy; forbidden-destination enforcement is covered separately.

Wry's `response_policy.rs` includes predicate and decision unit tests, with
feature-enabled and feature-disabled builds covering the compile-time gate.
