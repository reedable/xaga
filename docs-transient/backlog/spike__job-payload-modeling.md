# Define embedding job payload rules

## Summary

Embedding jobs carry different forms according to their work: authoritative domain objects, typed entity references and state patches, or relationship IDs. The rule governing those choices is implicit, and content-carrying jobs contribute to fresh-recovery memory growth.

## Requirements

- Carry authoritative objects when work depends on the producer's committed view and rereading could observe different state.
- Carry intent or typed identity when the consumer must apply a narrow mutation to its current row.
- Keep payloads type-safe across entity kinds; consistency does not require collapsing semantically different jobs.
- Account for queue memory, ownership, retries, and stale-data behavior.

## Remaining work

- Classify each job by authoritative input, mutation intent, and required consumer state.
- Identify raw identifiers that should become typed relationship payloads.
- Compare per-kind variants with generic variants without erasing kind-specific invariants.
- Coordinate the payload rule with bounded fresh-recovery dispatch.
- Present the model and migration cost for decision.

## References

- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/pipeline.rs`
- `docs/principles/carry_intent_end_to_end.md`
- [Bound fresh-reindex memory](defect__fresh-reindex-memory-ceiling.md)
