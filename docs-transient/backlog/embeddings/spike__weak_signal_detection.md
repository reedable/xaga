# Weak-signal detection in search results and Topics (pending — idea needs rework)

## The idea as recorded

Use the magnitude of embeddings to identify "weak signals" and consider suppressing them in search results, and likewise within Topics.

## The recorded correction

As written, it won't work: `crates/xaga-core/src/embeddings/model.rs` returns L2-normalised vectors, so stored magnitude is always 1.0. Recorded options:

- Capture the pre-normalization magnitude at embed time (a stored-schema change).
- Redefine "weak signal" via the similarity distribution instead of magnitude.

Either way the idea needs rewording or dropping before any implementation. Related: `query_time_inheritance.md` answers the same weak-signal question from curation edges rather than vectors.
