# Principles

Engineering principles for this project. Each has its own doc; keep this index terse — detail lives in the linked file, not here.

1. [Decision Ownership](decision_ownership.md) — all decisions are the user's. The assistant reads deeply, surfaces options, tradeoffs, risks, and blindspots at full voice, and executes the chosen call — but never makes the call. Speed under the user's steering is leverage; overstepping that line manufactures debt, not progress.

2. [Least Visibility](least_visibility_principle.md) — every symbol gets the narrowest visibility that compiles. Widen by exactly one step, only on demonstrated need. On every move/refactor, re-derive visibility rather than carrying the old modifier along.

3. [Secure by Default](secure_by_default.md) — the out-of-the-box behavior is the secure behavior. Where security is at odds with UX, provide a progressive, user-controllable security/UX trade-off, and inform the user of the consequences of their actions.

4. [Single Source of Truth](single_source_of_truth.md) — every datum has exactly one source of truth; every other holder is a derived, rebuildable mirror with enumerated producers. A write from outside the enumeration is drift.

5. [Carry Intent End-to-End](carry_intent_end_to_end.md) — the layer that knows what the user did carries that intent, typed and prioritized, to every consumer. Intent is never flattened into a generic event and re-derived downstream.

6. [Definition of Priority](definition_of_priority.md) — priority is precedence, assigned by importance and urgency (how soon the outcome must reflect, and for whom). Never by type, shape, size, cost, or effort — cost is a different axis. Disproof of the confound: Shutdown is important, expensive, and immediate.

7. [DRY — Consolidate Parallel Implementations](dry.md) — a piece of logic has one implementation; deferred consolidation is recorded as debt with fact and location. The code-side sibling of Single Source of Truth.
