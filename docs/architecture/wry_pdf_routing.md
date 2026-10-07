# Wry PDF response routing

Xaga's workspace overrides Wry with the sibling `../wry` checkout. The patch
base is Wry 0.57.0, revision `792d0359ba6501a4fc360ece17de2ae42329a47c`.
The local override requires that checkout; it is not a reproducible remote pin.

## Build features

Wry's default-off `macos-download-pdf` feature changes the macOS navigation
response policy. Xaga's default-off `wry-pdf-downloads` feature forwards to it
through a macOS-targeted direct dependency. Tauri uses the same resolved Wry
package; no Tauri source modifications are involved.

Ordinary Xaga builds retain upstream response routing. The application adapter
currently accepts only favicon downloads. The PDF feature is for integration
testing until a PDF consumer and persistence are connected and the probe is
removed.

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
PDF but fails the signature validator. The test validator uses the current
`%PDF-` signature contract; it does not establish parser safety.

The rendering cases check HTML document and local PDF iframe load signals with
no download callback. They do not visually inspect PDF pixels or establish
all bundled application behaviors. The proxy fixture verifies routing through
a configured proxy; forbidden-destination enforcement is covered separately.

Wry's `response_policy.rs` includes predicate and decision unit tests, with
feature-enabled and feature-disabled builds covering the compile-time gate.
