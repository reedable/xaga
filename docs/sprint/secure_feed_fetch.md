# Feed fetch (Atom) security & privacy hardening (pending)

Security audit of the remote-feed fetch path, 2026-07-01. Scope: everything between a
feed URL entering the app and fetched data landing on disk —
`crates/xaga-core/src/persistence/feeds.rs` (fetch/merge/persist),
`crates/xaga-core/src/atom.rs` (parsing), `crates/xaga-core/src/util.rs`
(URL normalization), `src-tauri/src/lib.rs` (client construction),
`src-tauri/src/ssrf_proxy.rs`, `src-tauri/src/commands/feeds.rs`, and
`crates/xaga-core/src/persistence/entries.rs` (what fetched data becomes on disk).

Existing defenses and fixed findings are documented in
`docs/security/fetching_untrusted_content.md` (network side),
`docs/security/handling_untrusted_content.md` (parsing, persistence, display),
and `docs/security/command_surface.md` (the invoke boundary).
Findings 1–3 and 5 are resolved (see those docs and git history); finding 4 is
partially resolved (only User-Agent remains). The numbering below is original.

## Pending findings

### 4. Privacy of the fetch itself

- **No `User-Agent` sent** (reqwest sends none by default). The absence of a UA is
  itself a signature, but a declared, identifiable UA discloses strictly more: app
  identity and version, to every feed host on every request. The claim that some hosts
  reject UA-less requests is unverified.
  - Option: identifiable UA (e.g., `xaga/<version>`) as an opt-in setting, off by
    default (docs/principles/secure_by_default.md). Consequence when enabled: every
    feed host learns the app and its version.
  - Option: a common browser UA string, as an opt-in setting, off by default (same
    principle). Consequence when enabled: hosts receive a browser identity rather
    than the app's, and the app is not named in the header. Whether other request
    properties (header set, request pattern) distinguish the app from a browser has
    not been investigated.
- **No `Accept-Encoding` negotiated** (`default-features = false`, only `rustls-tls`):
  no decompression-bomb risk, but also no compression. Informational; if gzip were
  enabled, reqwest would decompress before `chunk()` yields bytes, so the
  `feed_max_body_bytes` cap would count decompressed bytes — whether that holds in
  practice has not been tested.

Which findings to take, in what order, and the specific limits/values are open decisions.
