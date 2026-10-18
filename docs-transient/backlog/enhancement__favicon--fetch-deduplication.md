# Deduplicate concurrent favicon acquisition

## Summary

Concurrent article loads that discover the same normalized favicon URL may perform duplicate remote acquisition. Shared-resource writes are atomic, so this wastes work without corrupting the cache.

## Requirements

- Coalesce in-flight favicon acquisition by normalized URL identity.
- Deliver the validated result to every waiting article load.
- Preserve independent cancellation and diagnostics for each load.
- Do not let one cancelled waiter cancel work still required by another.
- Keep read-only processes from initiating a cache write.

## Remaining work

- Choose ownership and lifetime for the in-flight request registry.
- Define cancellation and failure fan-out.
- Add concurrent same-identity acquisition tests.

## References

- `src-tauri/src/article_loader.rs`
- `src-tauri/src/loader_download.rs`
- `crates/xaga-core/src/persistence/images.rs`
