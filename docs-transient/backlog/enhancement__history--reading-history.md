# Reading history

## Summary

Provide a history of previously read entries, distinct from the navigation stack used by the forward and back buttons. Make recent reading discoverable through time-based lists such as Today, Last three days, and Last one week.

## Requirements

- Define which interaction places an entry in reading history.
- Show each entry once and order entries by their most recent qualifying interaction.
- Support useful recent time windows, including Today, Last three days, and Last one week.
- Define whether reading history is one sidebar list with selectable time windows or a set of lists that can be pinned to the sidebar.
- Provide a way to clear reading history.
- Keep reading history local and out of logs and diagnostics.
- Keep reading history distinct from forward and back navigation history even if they share storage or events.

## Remaining work

- Decide which time windows to provide and whether their boundaries use calendar days or elapsed time.
- Decide how reading history appears in the sidebar and how pinning works.
- Decide how repeat visits, deleted entries, multiple libraries, and clearing are handled.
- Determine whether reading history and persistent navigation history should share any underlying representation.
- Implement and test recording, listing, time windows, pinning, and clearing.

## References

- `src/hooks/useRouter.tsx`
- `docs-transient/backlog/enhancement__history--navigation-persistence.md`
- `docs/principles/secure_by_default.md`
