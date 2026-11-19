# Report invalid list items and event payloads

## Summary

`TauriPlatform` silently discards invalid list/search items and event payloads.
Expose incomplete or failed results without hiding valid items or presenting
validation failures as genuinely empty collections.

## Requirements

- Include persisted-record identity validation failures, where a document's
  saved ID disagrees with the filename lookup ID. These records fail to load;
  define their user-facing reporting alongside other rejected results. Cover
  backend failures and omissions as well as payload validation in the adapter,
  so rejection before IPC does not disappear from the reporting contract.
- Before this story is ready for implementation, obtain the user's decision on
  the UI presentation and record it here. Define where and how partial-result
  status, all-invalid results, and request failures appear, including retry
  controls and what remains visible after notifications disappear. The outcome
  table below does not settle that presentation.
- Use one shared validation/reporting path. Log each rejected payload's operation
  or event name, schema diagnostics, and identity when safely available, without
  dumping document content.
- For list/search requests, preserve these distinct outcomes:

  | Outcome            | Presentation                                                         |
  | ------------------ | -------------------------------------------------------------------- |
  | Valid empty result | Existing empty state                                                 |
  | Some invalid items | Valid items plus one shared-sink notification with omitted count     |
  | All items invalid  | Persistent “Items couldn't be displayed” state plus one notification |
  | Request failure    | Loading-failure state with retry                                     |

- Retain result status after notifications disappear. Do not add per-item error
  placeholders or notify once per rejected item.
- Reject malformed events without mutating the last valid displayed state.
  Consolidate repeated event-validation notifications through the existing sink;
  retain per-event diagnostics. Do not bypass runtime validation.

## Remaining work

- Define presentation for identity-conflict failures in direct loads, list/search
  results, and recovery reporting. Preserve the distinction between an absent
  record and a rejected record. Diagnostics must identify the file and both IDs
  without dumping content; keep technical details out of primary UI copy.
- Groom the UI presentation with the user: present alternatives for the affected
  list and search views, agree on placement and behavior for each outcome, and
  record the chosen design and its verification criteria before implementation.
- Replace silent list/search and event parse failures with shared reporting,
  carrying result status through platform consumers to list presentation.
- Test valid, empty, mixed, all-invalid, and rejected requests; verify omitted
  counts, one notification per request, persistent failure state, and retry.
  Cover malformed and repeated events preserving valid state without alert floods.
- Run relevant platform/component tests and `npm run test:browser`, with browser
  coverage for partial and all-invalid presentation. Record any native verification
  gap and document the result/error contract in durable docs.

## References

- [Persisted entity identity](../../docs/architecture/persisted_entity_identity.md)
- [Platform adapters](../../src/platform/TauriPlatform.ts)
- [Shared error sink](../../src/providers/ErrorSinkProvider.tsx)
- [Logger](../../src/utils/Logger.ts)
- [Type identity](../../docs/architecture/type_identity.md)
