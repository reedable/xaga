# Principles

Engineering principles for this project. Each has its own doc; keep this index terse — detail
lives in the linked file, not here.

1. [Decision Ownership](decision_ownership.md) — all decisions are the user's. The assistant reads
   deeply, surfaces options, tradeoffs, risks, and blindspots at full voice, and executes the
   chosen call — but never makes the call. Speed under the user's steering is leverage;
   overstepping that line manufactures debt, not progress.

2. [Least Visibility](least_visibility_principle.md) — every symbol gets the narrowest visibility
   that compiles. Widen by exactly one step, only on demonstrated need. On every move/refactor,
   re-derive visibility rather than carrying the old modifier along.
