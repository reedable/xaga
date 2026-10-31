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

## Bundled macOS Downloads authorization check

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
favicon; explicit article refresh may reacquire resources supplied by the page.
