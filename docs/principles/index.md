# Principles

Engineering principles and the decisions they govern.

1. [Decision Ownership](decision_ownership.md) — the user decides; the assistant investigates, explains trade-offs, and executes the chosen direction.
2. [Least Visibility](least_visibility_principle.md) — expose symbols only as widely as existing callers need; recheck visibility when moving code.
3. [Secure by Default](secure_by_default.md) — retain secure defaults and explain the consequences of user-controlled exceptions.
4. [Single Source of Truth](single_source_of_truth.md) — identify the authoritative data and the permitted paths that update its rebuildable copies.
5. [Carry Intent End-to-End](carry_intent_end_to_end.md) — commands pass typed actions and saved values to consumers rather than making them infer changes from file events.
6. [Definition of Priority](definition_of_priority.md) — assign precedence by importance and urgency, not by cost or size.
7. [DRY — Consolidate Parallel Implementations](dry.md) — share implementations of the same logic or document retained duplication and its location.
