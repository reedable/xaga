# Persisted entity identity

Entry and Topic Markdown `id` fields and the local Feed Atom extension `xaga:id`
own local identity. Filenames are indexes for direct lookup. A lookup miss does
not establish absence elsewhere; reads do not scan for relocated records.
Existing IDs, references, directory layout, Markdown and Atom formats remain in use.

## Validation and repair

Every indexed read validates a present saved identity against its filename index
before returning the record, changing it, or deriving cache data. Present empty,
null, nonstring or unsafe Markdown IDs are malformed; absent IDs are repairable.
Conflicting identities are validation failures. Preserve the source file and both
identities; do not retarget references or create a replacement. Diagnostics identify
the file, filename ID and saved ID without logging document contents.

An absent identity uses the filename index in memory. A read supplied with the
library writer repairs it through that record's existing lane. Repair rereads and
validates the current file inside the lane, preserving concurrent saved changes.
Identity-only Markdown repair preserves unrelated fields, including unsupported
metadata, and the exact body. Ordinary edits retain their existing field ownership
and cleanup rules. New records persist identity; ordinary writes fill missing
identity and reject attempts to replace an existing identity.

Repair failure is logged as unsaved; the missing identity can still be inferred
in memory. Read-only APIs and callers without a writer never repair storage.
Discovery and exports apply the same validation and optional repair. Explicit
library conversion retains its exclusive-lock exception to runtime writer lanes.
No bulk migration or version increment is required for this remediation: it
formalizes existing identity authority and repairs deficiencies incrementally.

Entry lookup and discovery use the library's ordered roots. An existing
`entries/{id}.md` takes precedence over `pdfs/{id}.md`, including when the canonical
record is rejected or unreadable. PDF companions follow the same identity rules
and their additional source-URL validation when PDF persistence uses them to
identify the PDF. HTML saves retain a selected Markdown source under `pdfs/`; PDF cleanup removes a companion only
when the selected source is canonical. Later PDF saves preserve existing article
content and annotations in either location. See [library layout](library_layout.md).

## Local Feed fields

The shared Atom implementation defines `urn:xaga:atom` as the stable namespace
URI for Xaga-owned extension fields in local `.atom` files. `xaga` is a conventional
prefix, not identity: matching uses namespace URI and local name. Alternate
prefixes and element-local declarations resolve to the same ownership. Namespace
shadowing must not make a foreign field locally authoritative.

The locally owned `id` extension stores the existing local Feed ID. The ordinary
Atom `id` is publisher reference data and may change during refresh without
changing local identity or references. Publisher refresh merges publisher fields
into the saved Atom document and retains Xaga-owned extension fields; publisher
extensions cannot redefine them. Pins remain owned by `pins.yaml`.

New Feed files include the local extension. Refresh, rename, and other Atom writes
fill missing local identity in the same serialized write. They validate both the
candidate and current saved record before replacing a file, so a stale refresh
cannot overwrite a currently conflicting identity. Explicit conversion writes
follow the same rule under their exclusive library lock.

## Derived data and discovery failure

Authoritative source writes precede derived index updates. Recovery treats rejected
identities and unreadable Entry, Topic or Feed discovery as unreliable and withholds
all absence-based deletions. Valid records can still be reconciled. A later reliable
scan can remove genuinely absent cache rows. See [crash consistency](crash_consistency.md).

## Performance observation limits

Recovery and discovery await identity repair one record at a time through
`persistence::identity::read_markdown` and the walks in `persistence::walk` and
`recovery::embeddings`. A corpus with missing IDs can therefore require an atomic
source-file replacement per record, with rereads for validation and serialization.
Repair throughput and startup latency have not been measured at the
[multi-million-Entry design target](design_scale.md). Correctness tests do not
establish those costs.
