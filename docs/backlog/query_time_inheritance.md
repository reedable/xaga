# Query-time inheritance in semantic search (pending)

## The problem it addresses

An annotation whose own embedding is meaningless (e.g. highlight "#$%^&*" filed under a topic
"Prompt Injection") never surfaces for a query like "prompt engineering" — its vector carries
no signal, but its curation does.

## Recorded mechanism (designed, unimplemented)

Late fusion / graph expansion in `search_by_text` (`src-tauri/src/commands/embeddings.rs`),
which already retrieves topic rows:

- For topics above the similarity floor, expand `topic.items` into candidates with
  `inherited = topic_sim × λ`, where λ is per-edge-class damping (annotation-member vs
  entry-member vs annotation→source-article).
- Merge with direct hits by max, dedupe.
- No re-embedding, no stored-vector changes, no `rank_topics_by_item` circularity.

As recorded, this answers the weak-signal magnitude question by sourcing weight from curation
edges rather than vector magnitude (see `weak_signal_detection.md`). The λ values and the
similarity floor are undecided.
