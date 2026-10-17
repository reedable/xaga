# Read-only mode: complete the UI experience

## Summary

The backend rejects library mutations with `E_READ_ONLY`, but most frontend write affordances do not reflect the mode. Only the reader and sidebar consume `useLibraryReadOnly`; other controls can invite writes that will be refused.

## Requirements

- Keep the backend IPC gate authoritative; UI state is presentation and defense in depth.
- Apply one hide or disable-with-explanation policy to equivalent write controls.
- Map residual `E_READ_ONLY` failures to a consistent user message.
- Keep account-scoped settings, export, clear-history, and library switching available when they do not mutate the open library.
- Do not imply that the read-only view receives live changes from the writer process.

## Remaining work

- Inventory annotation, bookmark, read-state, Topic-membership, feed, Topic, import, and menu actions against the current command policy.
- Decide hide versus disable-with-explanation for each interaction class.
- Apply `useLibraryReadOnly` consistently and centralize refused-write error presentation.
- Decide whether to disclose snapshot age or provide an explicit view refresh; live cross-process synchronization is out of scope.
- Test writer-to-read-only mode transitions and every remaining reachable write path.

## References

- `src/hooks/useLibraryReadOnly.tsx`
- `src-tauri/src/commands/mod.rs`
- `src/components/content/HtmlReader.tsx`
- `src/components/sidebar/Sidebar.tsx`
- `docs/architecture/single_writer_library_lock.md`
