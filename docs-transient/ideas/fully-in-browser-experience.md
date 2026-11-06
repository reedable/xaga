# Fully In-Browser Xaga

Explore running the actual Xaga UI entirely in a browser, with a
`BrowserPlatform` implementing `Platform` through web APIs rather than the Rust
backend. Users could read, annotate, and organize a real library without
installing the desktop application.

The browser capture `MockPlatform` can help reveal native assumptions in the UI.
It supplies synthetic data for captures; a future `BrowserPlatform` would handle
real data and operations. Keep capture work scoped to its own purpose rather
than adding speculative browser infrastructure.

Possible directions to investigate:

- Store a library in browser-managed storage, or use user-authorized file access
  where supported. Decide whether opening the desktop app's Markdown/YAML library
  directly is a requirement. Browser-managed storage simplifies access but needs
  an explicit export, backup, and recovery experience; direct file access could
  preserve interoperability but introduces permissions and compatibility concerns.
- Provide local queries and indexes through a browser database, potentially
  using WebAssembly. Preserve authoritative library data and rebuildable indexes
  rather than making the index the only saved copy.
- Run embeddings in a worker, investigating available browser inference and
  acceleration options, model download costs, memory, and responsiveness at the
  documented design scale.
- Replace native menus, window controls, and dialogs with browser interactions
  while retaining the shared reading and organization UI.

Remote content access is a central open question. Direct feed fetching and
article extraction depend on cross-origin permissions and browser restrictions;
the native HTTP client and loader webviews cannot simply be reproduced through
the same browser calls. Investigate what a strictly browser-local experience can
support. If a supporting fetch service is considered, explain the privacy,
security, hosting, and offline consequences: that would change the fully local
execution model rather than resolve it invisibly.

Other questions include supported browsers, storage durability and quotas,
offline availability, library portability, and coordination when multiple tabs
edit the same library. Browser API availability and implementation feasibility
need verification before choosing an architecture or promising desktop parity.

References:

- [Platform interface](../../src/platform/Platform.ts)
- [TauriPlatform](../../src/platform/TauriPlatform.ts)
- [Layer ownership](../../docs/architecture/layer_ownership.md)
- [Single Source of Truth](../../docs/principles/single_source_of_truth.md)
- [Design scale](../../docs/architecture/design_scale.md)
