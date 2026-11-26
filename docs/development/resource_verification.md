# Resource persistence verification

## Automated coverage

`npm run test:headless` includes the library-writer resource lifecycle tests and
the Toolbar/ArticleReader presentation tests. Writer tests use temporary libraries
and the real serialized writer, checking shared favicon replacement, failed
acquisition, legacy-reference refresh, failed destination writes, and reopening
after writer shutdown. Presentation tests check local-only favicon sources and
the unavailable presentation for missing hash-only images.

`npm run test:gui` includes `persisted_resources_webview`. Run it alone with:

```sh
cargo test -p xaga --test persisted_resources_webview --locked
```

The native test uses a temporary library, production recovery/save commands,
the real writer, and controlled local image downloads. It checks acquisition
counts, persisted bytes and references, disabled recovery, cached reuse,
an edit during acquisition, read-only recovery and refused saves, and staging
cleanup. Its local server and gates are shared with `loader_lifecycle_webview`.
Both require a macOS desktop session.

`loader_lifecycle_webview` also exercises WebKit-selected `srcset` and `<picture>`
URLs with offline image capture disabled and enabled. It checks serialized
selection metadata, successful capture pairings, alternative/title preservation,
and that disabled capture's HTML snapshot adds no image request. A separate
case turns off remote image loading and checks that WebKit makes no image-element,
CSS image, or favicon requests while article text still reaches Xaga. The
persisted resources fixture checks that turning off either image control prevents
recovery. Conversion and
presentation regressions in `article_loader.images.test.ts` cover failed capture,
missing browser selection, relative fallback sources, all HTML delivery paths,
and unavailable-image rendering. Browser capture-mode workflows do not run the
native article loader; they supplement these loader-specific checks.

The UI-network and SVG rendering probes show temporary test windows. Hidden
fixture views on the tested WebKit did not complete their image/timer milestones;
visibility does not change the probes' proxy configuration or CSP assertions.

The test grants its exact random localhost origin the existing loader callbacks
and gives a deny-all proxy that local endpoint exception. It does not start the
normal app, read user settings, or change production permissions. A native run
does not establish macOS Downloads authorization behavior for the bundled app.
Local-only WebKit presentation under the exact configured app CSP is separately
covered by the SVG probe in `test:gui`; component tests alone cannot establish
that browser policy.

## Native image attachment smoke cases

The `image-attachment` and `malformed-image-attachment` cases of
`resource_download_webview` serve an `image/png` attachment after a redirect.
Their assertions require WebKit to deliver a download, the image validator to
accept the complete PNG and reject the truncated PNG, and staging to be removed
after either result. Both cases run in `npm run test:gui`.

To run one case in a macOS desktop session:

```sh
XAGA_DOWNLOAD_SMOKE_CASE=image-attachment cargo test -p xaga --test resource_download_webview --locked
```

Set `XAGA_DOWNLOAD_SMOKE_CASE=malformed-image-attachment` for the rejection
case. The opt-in `html-as-image-attachment` case serves HTML with the same
declared image type and checks that the validator rejects the bytes and
removes staging. The fixture uses the loader WebKit configuration, a controlled proxy,
and a fixture download callback that calls the production validator.
Image-document cases explicitly enable offline image capture, matching the
production loader's enabled-capture initialization without reading user settings.

## Native image rendering smoke cases

The opt-in `relative-image` and `cross-origin-image` cases load an HTML
document after a redirect. The document refers to a PNG by a root-relative
URL or by a URL on a second origin. The fixture serves both origins through
its controlled proxy and supplies no CORS headers. Each case checks that
WebKit reports a successful image load and the proxy sees one image request
to the expected origin. The test rejects download callbacks received before
the image-load signal ends the run. By default, the cases use a hidden
fixture window with the loader WebKit configuration. Production capture, destination
refusal, and main/settings window policy require their own evidence.

The opt-in `missing-image` case returns HTTP 404 for the image. It checks
that WebKit reports an image loading error, the surrounding article text
remains visible to the document, and no download callback fires before the
test exits. The opt-in `data-image` and `blob-image` cases embed the same
PNG in the document and check that it loads without a download callback.
The `file-image` and `unsupported-image-scheme` cases check that the
HTTP document reports an image loading error for a fixture-owned file URL
and an unsupported scheme, respectively.
Set `XAGA_DOWNLOAD_SMOKE_VISIBLE=1` to run a case with a visible loader
fixture window. For rendering cases, set `XAGA_DOWNLOAD_SMOKE_SUBFRAME=1`
to load the case in a same-origin child frame of the fixture window.
The same option supports attachment cases, including `pdf-attachment`.
Those cases start the child navigation after the parent document loads
and check the redirected download URL, validation result, and staging cleanup.

On macOS 27.2.1 (26B131), 25 top-frame cases and 13 child-frame
rendering and attachment cases passed in each visibility mode. The separate
`ui_network_webview` fixture observed main and Settings proxy refusals for
the tested remote images, stylesheet, frame, fetch, and WebSocket requests,
with only the loader reaching the upstream fixture. Those UI windows use a
custom fixture page. The bundled UI probe below exercises the production
window constructors with compiled assets. Loader refusal of policy-forbidden
redirects still requires direct evidence.

## Bundled UI download refusal

Build the frontend before running the native test in a macOS desktop session.

```sh
npm run ui:build
XAGA_VERIFY_BUNDLED_UI_ROUTING=1 cargo test -p xaga --features tauri/custom-protocol --test ui_network_webview --locked
```

The test opens main through the startup window constructor and Settings through
its menu handler. Both load the bundled asset page and call the production theme
command. Each then attempts a local blob download from its top frame and from a
same-origin blob child frame. On macOS 27.2.1 (26B131), both windows refused both
downloads and left staging unchanged. The fixture-page mode independently verifies
that closing both windows releases their proxy listeners.

Set `XAGA_VERIFY_BUNDLED_UI_ROUTING=data-top` or `data-child` to exercise
the equivalent data URL link from one frame at a time. Both windows sent
top-frame data URLs to the native refusal callback. The child-frame attempts
produced no native download callback during the observation window. Staging
stayed unchanged in both cases.

The bundled probe runs inside the native test executable with a temporary
library path and the theme command registered. The earlier desktop smoke covers
ordinary application startup and reader rendering.

## Native HTTPS routing smoke cases

Run the native resource fixture in a macOS desktop session. The HTTPS mode
uses the system OpenSSL executable to generate a certificate and key in a
temporary directory. The test callback accepts only the exact certificate
for the fixture's localhost port. The callback is part of the test executable.

```sh
XAGA_DOWNLOAD_SMOKE_HTTPS=1 XAGA_DOWNLOAD_SMOKE_CASE=pdf cargo test -p xaga --test resource_download_webview --locked
```

HTTPS mode supports `html`, `relative-image`, `missing-image`,
`image-attachment`, `malformed-image-attachment`, `html-as-image-attachment`,
`pdf-attachment`, `pdf`, and `invalid-pdf`. Use the same visibility and
child-frame options as the HTTP fixture. The inline `pdf` and `invalid-pdf`
cases are top-frame cases because Wry's inline PDF download override only
applies to top frames. To check pin rejection, use `html` with both
`XAGA_DOWNLOAD_SMOKE_HTTPS=1` and `XAGA_DOWNLOAD_SMOKE_WRONG_PIN=1`.

On macOS 27.2.1 (26B131), all 34 HTTPS runs passed. In each visibility
mode, nine top-frame cases and seven child-frame cases exercised rendering,
redirected attachment routing, validation, and the applicable staging checks.
Both wrong-pin controls cancelled the TLS challenge before the server saw an
HTTP request. The fixture exercises the native loader configuration and Wry
response handling. Default certificate trust, proxy containment, and the
production capture and persistence paths require separate evidence.

## Bundled macOS Downloads authorization check

Verification under independently established **undecided** and **denied**
Downloads access remains unperformed. The automated coverage above does not
establish behavior under those bundled-app permission states.

This check requires an installed bundle and independently established Downloads
permission states. Do not infer a state from a successful temporary-file test,
absence of a prompt, or an unbundled Cargo test. Do not modify the user's TCC
database or reset authorization as part of automated tests.

For each of **undecided** and **denied** Downloads access:

1. Record the macOS version, bundle path/version, bundle identifier, code-signing
   identity and designated requirement, and how the permission state was
   established. Use an isolated test account where necessary; keep the bundle
   identity stable between the state setup and the test.
2. In the bundled application, load a controlled article whose favicon becomes
   a native download, using an endpoint permitted by the existing network policy.
   Record its article diagnostic, including the assigned staging destination.
3. Verify that the staging destination is app-generated private temporary storage,
   not a server filename or the Downloads directory. Confirm that validated bytes
   reach the library, the saved article reopens with a local favicon, and no new
   Downloads authorization is needed for that staging operation.
4. Repeat with a rejected resource response. Verify retained article content,
   retained existing favicon bytes/reference where present, and removal of the
   assigned staging directory after completion.
5. Record pass/fail and the observed cleanup for each state. If either state
   cannot be established or observed, retain that as an unverified condition;
   automated tests are not a substitute for this evidence.

Automatic repair of missing hash-only resources is not provided. Their source
URLs are not retained. Cached reads present an unavailable image or absent
favicon. Explicit article refresh may reacquire resources supplied by the page
when both image permissions are on.
