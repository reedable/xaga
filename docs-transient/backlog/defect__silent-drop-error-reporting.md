# Invalid items silently dropped in list operations

## Summary

Several `TauriPlatform` list and event adapters use `safeParse` and discard failures. Invalid backend payloads therefore disappear without diagnostics or a user-visible indication that a result is incomplete.

## Requirements

- Report every dropped payload with operation or event name and schema diagnostics.
- Avoid logging sensitive document content unnecessarily.
- Distinguish an incomplete list from a wholly failed request in the user-facing error contract.
- Keep one malformed item from preventing valid items from rendering unless atomic validation is explicitly chosen.
- Use one shared validation-and-reporting path.

## Remaining work

- Inventory list and event adapters that filter failed parses.
- Define the logging and user-notification behavior for partial results.
- Add a shared parser helper carrying operation context and item identity when safely available.
- Replace silent filters and test valid, mixed, and wholly invalid payloads.

## References

- `src/platform/TauriPlatform.ts`
- `src/providers/ErrorSinkProvider.tsx`
- `src/utils/Logger.ts`
- `docs/architecture/type_identity.md`
