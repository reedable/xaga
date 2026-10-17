# Define persistent navigation history

## Summary

Navigation history is session-local. Define an opt-in setting for retaining it across launches, including storage, restoration, and deletion semantics.

## Requirements

- Default persistent history to off.
- Explain that retained history records reading behavior.
- Provide a clear-history action and honor disabling the setting by removing retained data.
- Bound retained history by count, age, or both.
- Restore only valid local navigation targets and avoid triggering remote fetches automatically.
- Keep persisted history out of logs and diagnostics.

## Remaining work

- Define which navigation states are retained and which fields are excluded.
- Compare account settings, library-local storage, and a dedicated history store.
- Decide retention limits and behavior when libraries, entries, or Topics disappear.
- Design settings and clear-history interactions.
- Implement and test opt-in, restoration, pruning, clearing, and multiple libraries.

## References

- `src/hooks/useRouter.tsx`
- `src-tauri/src/commands/settings.rs`
- `docs/principles/secure_by_default.md`
