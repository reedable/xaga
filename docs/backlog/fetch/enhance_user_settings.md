# Settings UI for fetch-policy settings

These user-controllable fetch settings exist without UI;
they are settable only by editing `settings.json` until this item is done.

- **HTTP allowed** — boolean, default `false`. When `false`, plaintext `http://` fetches
  are refused (feeds, PDF probe, article loader).
- **Maximum redirects** — integer, range 0–10, default `3`. `0` means the initial request
  only, honoring no redirects. `3` means the initial request plus up to 3 redirect hops. `3` is not the most secure default, but for UX reasons, the project owner decided on this number on 2 July 2026.
- **Referer allowed** (`referer_allowed`) — boolean, default `false`. When `false`, the
  `Referer` header is withheld on redirect hops (reqwest fetch paths), so redirect
  targets don't learn the original feed URL.
- **Gzip allowed** (`gzip_allowed`) — boolean, default `false`. When `false`, no
  `Accept-Encoding` header is sent and responses are never decompressed. When `true`,
  both reqwest fetch paths (feed XML, PDF probe) send `Accept-Encoding: gzip` and
  decompress transparently; transfers shrink substantially, and every fetched host
  learns the client negotiates gzip (one more property narrowing which software the
  client could be) — see docs/architecture/security/fetching_untrusted_content.md, "Encoding
  negotiation".

Work: expose both in the settings UI with get/set commands, following the existing
settings command pattern (`src-tauri/src/commands/settings.rs`).

Related fact: `feed_timeout_secs` and `feed_max_body_bytes` are also settings.json-only
today (no setter command, no UI); whether they join this UI is an open decision.

Per docs/principles/secure_by_default.md, the UI should state the consequence of
enabling HTTP, of raising the redirect cap, of allowing the Referer, and of enabling
gzip negotiation. The default of 3 redirects is an
exception to that principle, granted 2026-07-02 (recorded in
docs/architecture/security/fetching_untrusted_content.md).
