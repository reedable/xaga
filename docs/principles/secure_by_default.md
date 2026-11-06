# Secure by Default

**The app starts with secure settings. If a security measure limits usability, give the user narrowly scoped controls and explain the consequences of changing them.**

Three parts, in order of precedence:

1. **Secure by default.** The out-of-the-box behavior is the secure behavior. Security is never something the user must remember to turn on.
2. **Progressive, user-controllable trade-offs.** When a security measure limits usability, keep the secure default and let the user relax it explicitly. Limit the change to the affected operation or resource as narrowly as the mechanism allows.
3. **Informed consequences.** When the user exercises such a control (or an action carries a security consequence), the app tells them what the consequence is. The user must have that information before deciding.

## Existing instances in the code

Each of these is verifiable in the source:

- **Loader webviews are incognito by default** — no cookies or site data written to disk. The user-controlled `allowed_domains` setting relaxes this per domain: an allow-listed domain gets a persistent, per-domain `WKWebsiteDataStore` (`article_loader.rs`). Persistence is enabled only by the user’s choice for the named domain.
- **The inline PDF reader is off by default** (`inline_pdf_reader`, default `false`). PDFs use the same validated local download path for either presentation choice; when off, the stored PDF opens in the default app.
- **The network proxy rejects private, loopback, and link-local destinations for requests subject to its SSRF policy. This does not guarantee those destinations are unreachable from a webview.** The native `ui_network_webview` test demonstrated HTTP image requests to `127.0.0.1` reaching a local server directly from both UI test webviews, bypassing their configured proxies. That fixture does not establish exposure under the bundled UI’s CSP or in the article loader; those checks remain outstanding. See the [network enforcement gap and reproduction command](../architecture/security/fetching_remote_content.md#main-and-settings-network-policy).
- **Timeouts are on by default and user controllable** — `loader_timeout_secs` and `feed_timeout_secs` (both default 10 s, settings.json).
- **Unsupported downloads are reported neutrally**: an article download lacking the PDF signature is rejected with `article:unsupported-download`, without claiming that the server misrepresented its content.

## Placement of sanitization checks

- This principle does not prescribe one sanitization check per format or any other fixed count. Sanitize untrusted content at the points required by each mechanism; decide the number and placement from the actual risks.
