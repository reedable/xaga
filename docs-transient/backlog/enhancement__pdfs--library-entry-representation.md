# Represent stored PDFs as library Entries

## Summary

Give every successfully stored PDF a durable Markdown Entry counterpart so it
has an identity and user-managed state in the library. Enrich that Entry with
bounded text extraction when possible so PDF content can participate in
embedding-backed discovery without making successful parsing a prerequisite
for bookmarking or manual Topic membership.

## Requirements

- Create or preserve one `.md` Entry for the normalized source URL whenever a
  PDF is successfully persisted, using the same URL-derived identity and writer
  lane as the stored PDF.
- Treat the `.md` Entry as the durable source of truth for PDF metadata and user
  state, including title, source URL, read state, bookmark state, and manual
  Topic membership. Keep the PDF file as the durable source document.
- Allow a stored PDF to be bookmarked and manually added to or removed from a
  Topic even when no text could be extracted from it.
- Preserve an existing Entry's metadata, tags, annotations, and other authored
  fields when a PDF is downloaded or refreshed for its URL.
- Maintain one Markdown identity record per normalized source URL. Prefer and
  preserve `entries/{hash12}.md` when it exists; only create
  `pdfs/{hash12}.md` when the Entry is absent. Until activation is designed,
  keep PDF companions outside normal Entry discovery.
- Represent the Entry as a PDF explicitly; do not infer its representation only
  from a filename extension or from the continued presence of extracted text.
- Keep PDF presentation backed by the validated stored PDF bytes rather than
  rendering extracted Markdown as if it were the original document.
- Attempt bounded text and metadata extraction from validated PDF bytes. When
  extraction succeeds, persist normalized text suitable for indexing without
  presenting it as a lossless visual conversion.
- When extraction is unsupported or fails, retain the PDF and metadata-only
  Entry, report the extraction outcome through the established diagnostics, and
  do not make the document disappear from the library.
- Embed a PDF Entry only when it has meaningful extracted text or another
  explicitly defined embedding input. Do not invent semantic content from its
  URL or a placeholder body.
- Treat the PDF parser as a hostile-content boundary. Enforce input, time,
  memory, output, and page-count limits; define isolation and cancellation; and
  reject parser-produced active or external content from the Markdown output.
- Define behavior for encrypted, password-protected, malformed, image-only, and
  partially extractable PDFs. OCR is not implicitly part of the first version.
- Preserve read-only behavior: existing PDF Entries and stored PDFs remain
  readable, but creating the companion or refreshing extraction requires the
  active library writer.
- Keep recovery able to detect stored PDFs without either form of Markdown
  identity record and repair them when sufficient metadata is available,
  without silently discarding the PDF.
- Provide an explicit CLI migration for existing libraries. It must support a
  non-writing dry run, acquire the library write lock before mutation, be safe
  to rerun after interruption, avoid network access, and report migrated,
  already represented, failed, and unresolved PDFs separately.
- Recover a legacy PDF's source identity only from durable library evidence,
  including its valid Entry-format PDF companion or an existing Entry or Feed item
  whose normalized URL produces the PDF filename's hash. A legacy filename
  contains only a one-way URL hash; do not invent a source URL or silently
  create an incorrectly identified Entry when no matching evidence exists.

## Remaining work

- Define the PDF-specific Entry metadata and Markdown-body contract, including
  how a companion is distinguished from an HTML article and how extraction
  provenance or status is recorded.
- Define activation and repair for the two valid layouts: a feed-discovered PDF
  may use a richer `entries/{hash12}.md`, while a manually visited PDF may use a
  provisional `pdfs/{hash12}.md`. Normal writers do not create both. Recovery
  must nevertheless detect duplicates introduced by older builds or external
  edits and reconcile them without double indexing, split future edits, or loss
  of metadata, tags, annotations, Topic membership, timestamps, or content.
  The duplicate-reconciliation mechanism remains deliberately unresolved.
- Decide the authoritative title and timestamps when the load begins with only
  a URL, when a Feed Entry already exists, and when the PDF supplies document
  metadata.
- Redefine the current PDF/Markdown replacement rule: a PDF Entry requires both
  files to coexist, while a later response that is genuinely HTML may still
  replace the PDF representation. Specify ordering, hard-kill windows, and
  recovery for the two-file commit.
- Extend the library writer's PDF operation to activate or enrich the selected
  identity record without clobbering existing user state, then enqueue state and
  embedding work only after the durable representation lands.
- Evaluate PDF extraction libraries and their transitive licenses, supported
  formats, native distribution cost, parser safety history, and ability to run
  under enforceable resource limits outside the UI process.
- Implement deterministic, bounded extraction and normalization, including an
  explicit no-text outcome for image-only documents.
- Carry the represented Entry through list, bookmark, manual Topic membership,
  event, count, and recovery paths. Keep PDF lookup and viewing keyed by its
  source URL.
- Add an `xaga-cli articles` migration command that inventories legacy PDFs,
  builds a bounded URL-candidate map from durable Entry and Feed data, previews
  its actions under `--dry-run`, and creates or enriches companion Entries
  through the same core migration logic used by recovery. Produce a stable
  per-file outcome and final summary, including actionable unresolved cases.
- Define whether the migration performs text extraction immediately or creates
  metadata-only companions first and leaves extraction or embedding to a
  separate resumable pass. Do not require re-downloading the source PDF.
- Decide whether PDF annotation is separate follow-up work; do not imply that a
  text-extraction representation provides coordinate-stable PDF annotation.
- Cover PDFs reached through Feeds and otherwise, pre-existing Entry metadata,
  metadata-only fallback, successful extraction and embedding, refresh, format
  changes between PDF and HTML, parser limits and failure, crash recovery,
  bookmark state, manual Topic membership, and read-only libraries.
- Run Rust and frontend formatting, lint, tests, production build, and native
  PDF download and presentation verification.
- Update durable documentation for PDF identity, representation ownership,
  parsing security, indexing eligibility, and crash consistency.

## References

- `crates/xaga-core/src/persistence/pdfs.rs`
- `crates/xaga-core/src/persistence/entries.rs`
- `crates/xaga-core/src/library_writer/mod.rs`
- `crates/xaga-core/src/embeddings/row.rs`
- `crates/xaga-core/src/recovery/`
- `crates/xaga-cli/src/cmd/articles.rs`
- `src-tauri/src/commands/articles.rs`
- `docs/architecture/wry_pdf_routing.md`
- `docs/architecture/crash_consistency.md`
- `docs/architecture/single_writer_library_lock.md`
- `docs/architecture/security/handling_remote_content.md`
- `docs/architecture/security/handling_persisted_content.md`
- `docs/principles/single_source_of_truth.md`
