# Model trust chain: untaken hardening steps (pending)

Raised 2026-07-16, moved out of `docs/architecture/security/embedding_model_trust.md`
"Limits". Security-relevant, hence `compliance__`. The chain itself is
described in that doc; these are the two known steps that would strengthen it
and have not been taken.

## 1. Anchoring used no independent network path

Both anchoring fetches (and the API call) rode TLS from one machine/network. A
persistent MitM on that machine's network or CA trust store at anchoring time
could have supplied self-consistent lies. Re-deriving the manifest values from
a genuinely independent network path (different machine/network) and comparing
against the recorded values would raise the bar from "tamper persistently on
one network" to "tamper across independent networks."

## 2. The Qdrant→upstream link is unreproduced

The artifact is presented as an ONNX conversion of
`sentence-transformers/all-MiniLM-L6-v2`; the derivation has not been
reproduced. Reproducing the export from upstream weights and comparing hashes
would replace the reputational trust in Qdrant with trust in upstream plus the
conversion toolchain.

## Open questions

- Whether either step is worth the effort relative to the existing chain
  (three-source agreement, pinned revision, per-file SHA-256) is undecided.
- Re-anchoring procedure and toolchain pinning for a reproducible export are
  unscoped.
