# Embeddings write-lock participation audit

## Current state

The single-writer library lock (`docs/architecture/single_writer_library_lock.md`) is advisory — it protects only write paths that acquire it. Known participants: `EmbeddingService::start` (`crates/xaga-core/src/embeddings/service.rs`) and the CLI's `require_write_lock` (`crates/xaga-cli/src/cmd/embeddings.rs`).

## The work

Audit every `EmbeddingsDb` write path across the workspace (app + CLI) and gate any non-participant behind the lock. The original record of this item cited a CLI `topics migrate` bypass and a `lock.rs` module; both citations predate the current code layout (the migrate commands were deleted; the lock lives in `fs.rs`/`library.rs`), so the first step is re-establishing which write paths exist now and which take the lock.

Scope: xaga's own UI + CLI only — not third-party tools modifying LanceDB.
