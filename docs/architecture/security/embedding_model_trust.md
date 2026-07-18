# Embedding model content trust

How Xaga decides to trust the embedding model it downloads and executes. Transport containment (proxy, caps) is covered in [fetching_remote_content.md](fetching_remote_content.md); this document covers the _content_: why the bytes fed to onnxruntime are trusted, which links in that chain are cryptographic and which are reputational, and where the chain ends.

Code: the pinned manifest, `write_capped` (incremental SHA-256), and `verify_checksum` in `crates/xaga-core/src/embeddings/model.rs`; the verify-then-commit seam is `fs::replace_dir_atomic`.

## Intent

Trust is **moved in time, not eliminated**. No download trusts the server at fetch time: every file is hashed as it streams and compared against values compiled into the binary, and the URL pins an immutable revision. All trust is thereby relocated to a single **anchoring event** — the moment the manifest's values were derived and recorded (see "The anchoring record" below). The security question "is this download honest?" reduces permanently to "was the anchoring event honest?"

## The chain of trust

Ordered from download backwards. Each link states what secures it.

1. **Every download → the manifest.** _Cryptographic._ SHA-256 over the streamed bytes, compared before the staging dir is committed; mismatch refuses installation and deletes staging. The pinned revision makes a server-side change of the repo's `main` irrelevant to installs — a hash mismatch is unambiguous evidence of a serving-path problem, never "the repo updated."

2. **The manifest → the anchoring event.** _Procedural, with cryptographic elements._ The anchored values are required to agree across three sources before recording: the SHA-256 published by Hugging Face as the Git-LFS oid (computed by HF's infrastructure at upload time — available for LFS-tracked files only, i.e. `model.onnx`), and two independent fetches separated in time, hashed locally (the sole source for the small non-LFS files, which have no published SHA-256). This agreement rules out transient tampering — a one-time MitM, a poisoned cache, a compromised CDN node — but not tampering that was _consistent_ across upload time, both fetches, and the API.

3. **The anchoring event → the anchoring environment.** _Infrastructural._ Both fetches and the API call rode TLS from one machine: a persistent man-in-the-middle on that machine's network or CA trust store could have supplied self-consistent lies. Time-separated fetches raise the bar from "tamper once" to "tamper persistently"; fetching from a genuinely independent network path would raise it further and was not done.

4. **The content → the publisher (Qdrant).** _Reputational — the irreducible link._ With perfect integrity everywhere, the bytes are still whatever the Qdrant organization uploaded. No cryptographic attestation exists that their ONNX conversion of `sentence-transformers/all-MiniLM-L6-v2` is faithful or benign, and no independent party publishes hashes for this artifact. The evidence is reputational: the repo is fastembed's default model source, unchanged since mid-2025, consumed broadly by that ecosystem — a compromise would have a wide blast radius and many observers. That is a likelihood argument, not a verification.

5. **The publisher → upstream.** _Claimed, unverified._ The artifact is presented as a conversion of the upstream sentence-transformers model; the derivation has not been reproduced. Reproducing the ONNX export from upstream weights would replace link 4's trust in Qdrant with trust in upstream plus the conversion toolchain.

6. **The manifest → the binary.** _Deferred to the platform._ Users receive the manifest inside the app; whoever can modify the shipped binary can rewrite the checksums. The manifest's forward integrity is therefore exactly the app's code-signing/distribution story (docs-transient/backlog/build/defect\_\_macos_code_signing.md), and, for this repository, the integrity of its git history.

## What the behavior check proves

Loading the embedder against the anchored set (normed 384-dim output) proves the artifact _functions_ — it does not prove benignity. Its actual security value: combined with link 1, every install forever receives exactly the bytes that existed at the audited moment, so the onnxruntime parser surface is frozen at one fixed artifact rather than re-exposed to whatever the server serves each day.

## The anchoring record

Values anchored 2026-07-11:

- `MODEL_REVISION` = the repo's `main` sha from the HF API (`/api/models/{repo}`, lastModified 2025-06-09).
- `model.onnx` sha256 = the published Git-LFS oid from the HF tree API, which agreed with two independent fetches (an earlier download of `main` and a fresh fetch at the pinned revision).
- The four JSON files are plain git blobs (no LFS oid published); their sha256s were computed from the same two fetches, which agreed.
- The anchored set was behavior-checked by loading the embedder.

A model upgrade re-derives every value the same way and replaces the manifest block (`crates/xaga-core/src/embeddings/model.rs`) and this record in the same motion.

## Limits

- **The parser is trusted with the frozen artifact.** onnxruntime parses the model natively; the checksum pins _which_ bytes it parses, not that parsing them is safe.
