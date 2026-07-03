# Secure by Default

**The app is secure by default. Where security is at odds with UX, we provide a progressive
security/UX trade-off that is user controllable. Our principle is to inform the user of the
consequences of their actions.**

Three parts, in order of precedence:

1. **Secure by default.** The out-of-the-box behavior is the secure behavior. Security is never
   something the user must remember to turn on.
2. **Progressive, user-controllable trade-offs.** Where a security measure costs UX, the resolution
   is not to weaken the default — it is to give the user a control that relaxes the measure by an
   explicit, bounded step, scoped as narrowly as the mechanism allows.
3. **Informed consequences.** When the user exercises such a control (or an action carries a
   security consequence), the app tells them what the consequence is. The user decides; the app
   does not decide for them, and does not let them decide blind.

## Existing instances in the code

Each of these is verifiable in the source today:

- **Loader webviews are incognito by default** — no cookies or site data written to disk. The
  user-controlled `allowed_domains` setting relaxes this per domain: an allow-listed domain gets a
  persistent, per-domain `WKWebsiteDataStore` (`article_loader.rs`). Default secure; relaxation
  explicit, user-owned, and scoped to the named domain.
- **The inline PDF reader is off by default** (`inline_pdf_reader`, default `false`). When off, the
  probe request is never made — the comment at the call site records the reason: not making the
  request avoids letting servers profile us (`commands/articles.rs`).
- **All fetching of untrusted remote content routes through the SSRF proxy** — loader webviews,
  feed XML, and the PDF probe (see `docs/architecture/security/fetching_untrusted_content.md`). Private,
  loopback, and link-local destinations are unreachable by default.
- **Timeouts are on by default and user controllable** — `loader_timeout_secs` and
  `feed_timeout_secs` (both default 10 s, settings.json).
- **Blocked actions are reported as what they are** — a spoofed PDF (claimed `application/pdf`,
  body not a PDF) is dead-ended with the distinct `E_SPOOFED_PDF` code so the frontend can tell
  the user it was blocked, rather than showing a generic failure (`commands/articles.rs`).

## What this principle is not

- It is not "exactly one sanitization boundary per format" or any other count of boundaries. All
  untrusted content must go through sanitization; the number and placement of boundaries is a
  design question decided per mechanism, not doctrine.
