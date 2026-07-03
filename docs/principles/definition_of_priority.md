# Definition of Priority

**Priority is precedence: the order in which competing work is granted the resource, assigned by
the work's importance and urgency — how soon its outcome must be reflected, and for whom. It is
never assigned by the work's type, shape, size, cost, or effort. Cost is a different axis.**

## The confound this definition exists to kill

"Cheap = high priority, expensive = low priority." Cost and precedence feel correlated because the
common case often has both ("small state toggle, reflect now"), but the correlation is a
status-quo bias, not a definition. Cheap work can be deferrable; expensive work can be immediate.
A scheduler built on the confound mis-orders exactly the cases that matter.

## The disproof (reason from edge cases, not the common case)

**Shutdown is important *and* expensive *and* immediate.** It rides the highest tier (T0). One
edge case is enough: a definition of priority that assigns by cost cannot place Shutdown, so
cost is not the definition. The same test applies to any proposed assignment rule — find the
work item that is heavy and urgent, or light and deferrable, and see whether the rule survives.

## Application in this codebase

- **The tier is the priority; the Job is the action; the two are orthogonal.** The same Job may
  ride different tiers from different producers — `EmbedEntry` on Regular from a live save, on
  Maintenance from recovery — because the *urgency* differs while the action is identical
  (the `Tier` and `Job` docs, `crates/xaga-core/src/embeddings/service.rs`). Encoding the action
  into the tier (per-tier enums) re-couples the axes.
- **The assignment question is "how soon must this reflect, and who is waiting?"** — not "how
  heavy is it?" A user who clicked add_feed is waiting on the entry rows, so the upsert rides
  Interactive; the embedding that follows is not blocking anyone's view, so it rides Regular
  (`src-tauri/src/commands/feeds.rs`, tier comments).
- Tiers drain strictly highest-first: Control > Interactive > Regular > Maintenance (the biased
  `select!` in `run_loop`, `crates/xaga-core/src/embeddings/service.rs`).

## The failure this definition prevents

The cost⇒priority judgment was baked into a queue-tier comment ("cheap, no ONNX") and routed
user-clicked refresh/import to the lowest tier. It propagated across sessions, mis-scheduled the
whole app, and made it unusable for days; the fix was purely re-prioritization — zero capacity
change. The full account is in [decision_ownership.md](decision_ownership.md) (plot points, "the
'cheap = priority' saga") — that document records the failure mode and who owns such calls; this
document is the definition.
