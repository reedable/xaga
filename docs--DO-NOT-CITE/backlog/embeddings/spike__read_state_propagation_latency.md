# Define read-state propagation latency and consistency

## Summary

Marking an entry read persists the file before an Interactive `SyncState` updates LanceDB and emits state events. Item state and aggregate unread counts can therefore become visible at different times. The dominant latency and intended read-after-write contract are undefined.

## Requirements

- Preserve durable-file authority and backend-authoritative UI state.
- Define separate visibility guarantees for the changed item and derived aggregate counts.
- Avoid presenting a completed state that the backend has not accepted.
- Measure file write, queue wait, database update, event delivery, and count recomputation separately.

## Remaining work

- Instrument the mark-read path through item and sidebar updates.
- Define the required consistency and latency bounds for user-initiated state changes.
- Compare the current asynchronous cache update, awaiting cache write-through, and event-carried aggregate deltas.
- Evaluate failure and restart behavior for each option.
- Implement the selected contract and verify rapid repeated changes and large-library counts.

## References

- `src-tauri/src/commands/entries.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `src/platform/TauriPlatform.ts`
- `src/components/list/ItemList.tsx`
