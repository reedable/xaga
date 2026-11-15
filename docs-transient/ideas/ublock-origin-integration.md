# uBlock Origin integration

Explore integrating uBlock Origin into Xaga to block ads and trackers when
loading remote article content.

Potential benefits include fewer unwanted network requests and less distracting
content. Filtering could also break article loading or extraction, and would
introduce filter-list maintenance and compatibility concerns.

Questions to investigate:

- **Licensing:** Determine the obligations for using, modifying, or distributing
  uBlock Origin code, alternative filtering engines, and filter lists.
- **Trust:** Identify which upstream maintainers, dependencies, list publishers,
  and update channels Xaga would trust, and how their changes would be reviewed
  or verified.
- **Build-time versus runtime integration:** Compare bundling reviewed code and
  lists in an app release with loading or updating them at runtime. Consider
  update freshness, reproducibility, offline behavior, integrity verification,
  rollback, and whether executable code and filter data need different policies.
- **Safari/WebKit compatibility:** Do not assume uBlock Origin can plug directly
  into Safari or Xaga's embedded WebKit webviews. Verify the extension APIs and
  filtering hooks available in each environment. Investigate alternatives such
  as native content-blocking rules, translating a supported subset of filter
  syntax, embedding a compatible filtering engine, or filtering requests at the
  network proxy. Establish each approach's coverage and limitations, including
  cosmetic filtering and script-based rules.
- **Security implications:** Evaluate the attack surface introduced by filter
  parsing, executable filtering rules, dependencies, and updates. Consider
  malicious or compromised lists, update-request privacy, resource exhaustion,
  and interactions with Xaga's existing network isolation and content security
  boundaries. Filtering must not be assumed to replace those protections.
- **User control and compatibility:** Investigate exceptions, diagnostics, and
  recovery when filtering breaks article loading or extraction.

Whether to integrate uBlock Origin itself or support compatible filtering
remains an open question; no implementation approach or priority has been
selected.

Alternatives and supporting components to investigate, without ranking or
recommendation:

- [DuckDuckGo Tracker Radar](https://github.com/duckduckgo/tracker-radar): a
  third-party domain dataset with ownership and tracking metadata.
- [Disconnect tracking protection lists](https://github.com/disconnectme/disconnect-tracking-protection):
  curated tracking services and categories.
- [EasyPrivacy](https://easylist.to/): filtering rules targeting tracking
  requests.
- [AdGuard filters](https://github.com/AdguardTeam/AdguardFilters): filtering
  lists that include tracking protection.
- [Brave's adblock-rust](https://github.com/brave/adblock-rust): an embeddable
  Rust filtering engine that consumes lists.
- [AdGuard's SafariConverterLib](https://github.com/AdguardTeam/SafariConverterLib):
  a converter from AdGuard rules to Safari content-blocking rules; applicability
  to Xaga's embedded webviews remains to be verified.

Tracker identification data, filtering rules, matching engines, and WebKit
integration components serve different roles and may be combined. Evaluate
licensing, integration model, upstream trust and governance, update control,
security implications, coverage, and user control for each candidate and any
proposed combination. Technical compatibility alone does not determine the
choice. Identifying a known tracker and deciding whether to block a particular
request are separate questions.
