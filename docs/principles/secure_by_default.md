# Secure by Default

**The app is secure by default. Where security is at odds with UX, we provide a progressive security/UX trade-off that is user controllable. Our principle is to inform the user of the consequences of their actions.**

Three parts, in order of precedence:

1. **Secure by default.** The out-of-the-box behavior is the secure behavior. Security is never something the user must remember to turn on.
2. **Progressive, user-controllable trade-offs.** Where a security measure costs UX, the resolution is not to weaken the default — it is to give the user a control that relaxes the measure by an explicit, bounded step, scoped as narrowly as the mechanism allows.
3. **Informed consequences.** When the user exercises such a control (or an action carries a security consequence), the app tells them what the consequence is. The user decides; the app does not decide for them, and does not let them decide blind.

## Existing instances in the code

Each of these is verifiable in the source:

- **Loader webviews are incognito by default** — no cookies or site data written to disk. The user-controlled `allowed_domains` setting relaxes this per domain: an allow-listed domain gets a persistent, per-domain `WKWebsiteDataStore` (`article_loader.rs`). Default secure; relaxation explicit, user-owned, and scoped to the named domain.
- **The inline PDF reader is off by default** (`inline_pdf_reader`, default `false`). PDFs use the same validated local download path for either presentation choice; when off, the stored PDF opens in the default app.
- **The network proxy rejects private, loopback, and link-local destinations for requests subject to its SSRF policy. This does not guarantee those destinations are unreachable from a webview.** The native `ui_network_webview` test demonstrated HTTP image requests to `127.0.0.1` reaching a local server directly from both UI test webviews, bypassing their configured proxies. That fixture does not establish exposure under the bundled UI’s CSP or in the article loader; those checks remain outstanding. See the [network enforcement gap and reproduction command](../architecture/security/fetching_remote_content.md#main-and-settings-network-policy).
- **Timeouts are on by default and user controllable** — `loader_timeout_secs` and `feed_timeout_secs` (both default 10 s, settings.json).
- **Unsupported downloads are reported neutrally**: an article download lacking the PDF signature is rejected with `article:unsupported-download`, without claiming that the server misrepresented its content.

## What this principle is not

- It is not "exactly one sanitization boundary per format" or any other count of boundaries. All untrusted content must go through sanitization; the number and placement of boundaries is a design question decided per mechanism, not doctrine.
