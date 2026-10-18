# Article and Entry carrier boundary

## Summary

Feed discovery and bookmark import use `Article` as a carrier for entries whose article data is incomplete. This violates the domain definition of `Article` as a loaded, content-bearing entry. Determine how these workflows should represent and carry the available data.

## Requirements

- Do not construct or carry an `Article` without complete article data.
- Support persistence and indexing of entries created from metadata-only sources.
- Keep domain types and job payloads consistent with their declared meaning.

## Remaining work

- Trace where feed discovery and bookmark import create incomplete `Article` values and where consumers depend on article-only fields.
- Identify options for representing and indexing the available entry data, with their effects on persistence return types and embedding job payloads.
- Implement the chosen representation consistently across the affected workflows.

## References

- `crates/xaga-core/src/domain.rs`
- `crates/xaga-core/src/persistence/entries.rs`
- `crates/xaga-core/src/embeddings/service.rs`
- `crates/xaga-core/src/embeddings/row.rs`
