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
