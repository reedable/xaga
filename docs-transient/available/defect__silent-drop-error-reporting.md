# Report invalid list items and event payloads

## Summary

`TauriPlatform` silently discards invalid list/search items and event payloads.
Expose incomplete or failed results without hiding valid items or presenting
validation failures as genuinely empty collections.

## Requirements

- Use one shared validation/reporting path. Log each rejected payload's operation
  or event name, schema diagnostics, and identity when safely available, without
  dumping document content.
- For list/search requests, preserve these distinct outcomes:

  | Outcome | Presentation |
  | --- | --- |
  | Valid empty result | Existing empty state |
  | Some invalid items | Valid items plus one shared-sink notification with omitted count |
  | All items invalid | Persistent “Items couldn't be displayed” state plus one notification |
  | Request failure | Loading-failure state with retry |

- Retain result status after notifications disappear. Do not add per-item error
  placeholders or notify once per rejected item.
- Reject malformed events without mutating the last valid displayed state.
  Consolidate repeated event-validation notifications through the existing sink;
  retain per-event diagnostics. Do not bypass runtime validation.

## Remaining work

- Replace silent list/search and event parse failures with shared reporting,
  carrying result status through platform consumers to list presentation.
- Test valid, empty, mixed, all-invalid, and rejected requests; verify omitted
  counts, one notification per request, persistent failure state, and retry.
  Cover malformed and repeated events preserving valid state without alert floods.
- Run relevant platform/component tests and `npm run test:browser`, with browser
  coverage for partial and all-invalid presentation. Record any native verification
  gap and document the result/error contract in durable docs.

## References

- [Platform adapters](../../src/platform/TauriPlatform.ts)
- [Shared error sink](../../src/providers/ErrorSinkProvider.tsx)
- [Logger](../../src/utils/Logger.ts)
- [Type identity](../../docs/architecture/type_identity.md)
