# Define weak-signal behavior in semantic search

## Summary

Some saved items have text that carries little semantic signal while their Topic membership carries user-authored meaning. Stored vectors are L2-normalized, so vector magnitude cannot identify these items. Decide whether search should use score-distribution rules, Topic membership inheritance, or leave direct similarity unchanged.

## Requirements

- Do not use stored vector magnitude as a signal-strength measure.
- Preserve direct-result ranking semantics unless the selected design explicitly changes them.
- Prevent membership expansion from recursively amplifying Topics or duplicating results.
- Make thresholds and edge weights explainable and testable across entries and annotations.
- Measure relevance and latency before adopting additional query-time work.

## Remaining work

- Define representative weak-signal and strong-signal evaluation cases.
- Compare direct similarity, similarity-distribution filtering, and query-time Topic expansion.
- If inheritance is selected, decide eligible edges, similarity floor, damping, deduplication, and result-limit ordering.
- Prototype the selected option and measure relevance and query latency.
- Record the chosen search semantics in durable documentation before implementation is considered complete.

## References

- `crates/xaga-core/src/embeddings/model.rs`
- `crates/xaga-core/src/query/items.rs`
- `src-tauri/src/commands/embeddings.rs`
- `docs-transient/backlog/spike__saved-vs-relevant-concept.md`
