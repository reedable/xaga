# Definition of Priority

**Priority determines which competing work receives a resource first. Assign it
by importance and urgency: who needs the result and how soon. The work’s type,
size, cost, or effort does not determine its priority.**

## Why cost does not determine priority

A small state change often needs an immediate result, but this does not mean all
cheap work is urgent or all expensive work can wait. Shutdown is important,
expensive, and immediate; it uses the highest tier (T0). A scheduling rule based
on cost cannot handle that case correctly.

## Application in this codebase

- **`Tier` specifies priority; `Job` specifies the action.** The same job can use
  different tiers depending on its producer. A live save queues `EmbedEntry` as
  Regular; recovery queues it as Maintenance. The action is the same, but its
  urgency differs. Keep the enums separate rather than defining actions by tier
  (`crates/xaga-core/src/embeddings/service.rs`).
- **Ask who is waiting for the result.** A user who called `add_feed` is waiting
  for article rows to appear, so the database upsert uses Interactive. The
  following embedding work does not block that view and uses Regular
  (`src-tauri/src/commands/feeds.rs`).
- The service processes tiers strictly highest-first: Control > Interactive >
  Regular > Maintenance, using biased `select!` in `run_loop`
  (`crates/xaga-core/src/embeddings/service.rs`).

## Previous scheduling failure

A queue comment equated Interactive work with “cheap, no ONNX.” User-requested
refresh and import work was then routed to the lowest tier. This made the app
unusable for days; correcting the priorities restored responsiveness without
adding capacity. [Decision Ownership](decision_ownership.md) records the
associated decision-making failures.
