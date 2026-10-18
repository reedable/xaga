# Preserve source identity for stored PDFs

## Summary

Stop creating newly stored PDFs whose source URL cannot later be recovered.
Persist a small, versioned metadata sidecar for each PDF at save time, without
waiting for PDF parsing or the full PDF Entry representation.

## Requirements

- Store one metadata sidecar beside each PDF, using the same URL-derived stem:
  `pdfs/{hash12}.json` for `pdfs/{hash12}.pdf`.
- Store a schema version and the normalized HTTP(S) source URL. Do not add
  guessed title, extracted content, or other fields unavailable at the native
  PDF persistence boundary.
- Derive both filenames from the stored normalized URL and verify on read that
  hashing that URL produces the filename stem. Treat a mismatch as malformed
  metadata rather than accepting a second identity.
- Write or atomically replace the sidecar before atomically committing new PDF
  bytes. This ordering may leave a sidecar without a PDF after interruption,
  but a newly committed PDF must not be left without its reversible identity.
- On PDF removal, remove the PDF before removing its sidecar. Treat a missing
  file as an idempotent success and document the harmless sidecar-only crash
  state.
- Serialize PDF and sidecar mutation through the PDF's existing normalized-URL
  writer lane and require the active library writer.
- Preserve the last complete PDF and its matching sidecar if URL normalization,
  metadata serialization, PDF validation, or an atomic replacement fails.
- Keep existing stored-PDF lookup and presentation behavior compatible; the
  sidecar records identity for migration and recovery but is not a prerequisite
  for reading a legacy PDF by a URL the caller already knows.
- Bound sidecar reads, reject non-regular files and symlinked PDF directories,
  and treat persisted metadata as untrusted input.
- Do not attempt to reconstruct sidecars for legacy PDFs in this story. The
  explicit PDF migration CLI owns discovery and reporting for those files.
- Make the future PDF Entry migration consume this sidecar as authoritative
  durable evidence of the stored PDF's source URL.

## Remaining work

- Define the minimal versioned sidecar schema and shared PDF/sidecar path
  helpers in core persistence.
- Extend the library-writer PDF save and removal operation with the required
  sidecar ordering and failure behavior.
- Add bounded sidecar reading for recovery and migration without changing the
  current URL-keyed PDF presentation API.
- Cover URL normalization, filename verification, initial save, replacement,
  removal, missing files, malformed or oversized metadata, symlinks, simulated
  failures at each ordering boundary, and legacy PDF reads without sidecars.
- Update durable PDF identity, crash-consistency, single-writer, and persisted-
  content documentation.
- Run Rust formatting, lint, tests, and the native PDF download regression
  coverage.

## References

- `crates/xaga-core/src/persistence/pdfs.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `src-tauri/src/commands/articles.rs`
- `docs/architecture/wry_pdf_routing.md`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/single_writer_library_lock.md`
- `docs/architecture/security/handling_persisted_content.md`
- `docs-transient/backlog/enhancement__pdfs--library-entry-representation.md`
- `docs/principles/single_source_of_truth.md`

