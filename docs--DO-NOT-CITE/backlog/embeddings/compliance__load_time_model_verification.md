# Verify the embedding model against the manifest at load time

## The gap

Files already on disk are checked for existence only (`model_files_present`, `crates/xaga-core/src/embeddings/model.rs`); the manifest check runs during download and never again. A local actor who can write the model dir can substitute content after installation, and every subsequent load feeds the substituted bytes to onnxruntime unverified.

## Direction

`Embedder::load` verifies each file against the pinned manifest (`MODEL_FILES`, model.rs) before handing bytes to fastembed/onnxruntime.

Facts for the implementation:

- `Embedder::load` already reads all five files fully into memory (`std::fs::read`), so verification is one SHA-256 pass over buffers already in RAM — no additional I/O. CPU cost: hashing ~91 MB once per load (estimate: on the order of 100 ms with hardware SHA; unmeasured).
- The manifest and `verify_checksum` already exist (model.rs); the load path can reuse both.

## Decision: mismatch semantics

Choose the behavior after a failed load-time check:

1. **Fail the load.** Error like the current missing-file path; session runs in the existing degraded mode; bad files remain on disk for inspection; recovery requires deleting the model dir and re-downloading (manual today; the stall-handling / OOBE work may later surface it).
2. **Self-heal.** Delete the failed files and invoke the existing verified download path (`download_model`, with the account right held by the caller). Session recovers without user action; costs: the on-disk evidence is destroyed, and an unprompted network fetch occurs — the same consent class as oobe model download consent.

## On completion

Update the Limits section of `docs/architecture/security/embedding_model_trust.md` (the doc currently describes download-time verification only) and the load-time bullet in `docs/architecture/security/fetching_remote_content.md`.
