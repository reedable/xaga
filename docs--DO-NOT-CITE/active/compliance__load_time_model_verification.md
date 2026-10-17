# Verify the embedding model against the manifest at load time

## Summary

Files already on disk are checked for existence only (`model_files_present`, `crates/xaga-core/src/embeddings/model.rs`); the manifest check runs during download and never again. A local actor who can write the model dir can substitute content after installation, and every subsequent load feeds the substituted bytes to onnxruntime unverified.

## Requirements

- Verify every model file against the pinned manifest before passing it to fastembed or ONNX Runtime.
- Reuse the bytes already read by `Embedder::load`; verification must not require a second file read.
- Refuse to execute mismatched content.
- Do not delete evidence or initiate a replacement download without an explicit recovery policy.
- Keep download-time verification unchanged.

## Remaining work

- Measure load-time hashing cost for the complete model.
- Decide whether a mismatch only fails loading or may offer a separately authorized verified download; preserve the failed files unless deletion is explicitly chosen.
- Add verification to `Embedder::load` using `MODEL_FILES` and the existing checksum comparison.
- Test valid, missing, and mismatched files.
- Update the durable trust and remote-content documentation.

## References

- `crates/xaga-core/src/embeddings/model.rs`
- `docs/architecture/security/embedding_model_trust.md`
- `docs/architecture/security/fetching_remote_content.md`
