# Reading progress and stats

## Summary

Define whether Xaga should track reading progress or statistics such as time spent and articles read. These are behavioral data, so collection purpose, accuracy, retention, and user control must be decided before implementation.

## Requirements

- Define a user-facing purpose for every collected value.
- Default collection and persistence according to the secure-by-default principle.
- Distinguish content being open from active reading and document expected measurement error.
- Provide inspection, deletion, and disable controls for persisted behavioral data.
- Keep statistics local unless a separate data-sharing decision is made.
- Bound storage and retention.

## Remaining work

- Identify candidate product behaviors and the minimum events each requires.
- Compare no tracking, session-only progress, durable per-item progress, and aggregate statistics.
- Define idle, background-window, reread, partial-read, and multi-window semantics.
- Present privacy, accuracy, and storage tradeoffs for decision.
- Design persistence and UI only after the collected data is selected.

## References

- `src/components/content/ItemView.tsx`
- `src/hooks/useRouter.tsx`
- `docs/principles/secure_by_default.md`
