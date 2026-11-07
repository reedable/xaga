# Embedding model content trust

How Xaga verifies the embedding model it downloads and loads, and which assumptions remain after verification. [Fetching remote content](fetching_remote_content.md) covers destination restrictions and resource limits. This document covers expected hashes, their origin, and dependence on the publisher and distributed app.

Code: the pinned manifest, download-time `write_capped`/`verify_checksum`, and
load-time `read_verified_model_files` in
`crates/xaga-core/src/embeddings/model.rs`; verified downloads are installed through `fs::replace_dir_atomic`.

## What verification establishes

Every download is hashed while streaming and compared with expected values
compiled into the binary. The download URL selects an immutable revision. Every
later load hashes the installed bytes again before fastembed or ONNX Runtime
receives them.

These checks depend on the expected values being trustworthy and on the app
binary containing the intended manifest. They establish that the bytes match a
chosen artifact; they do not establish that the artifact is harmless.

## The chain of trust

1. **Downloaded and installed bytes are checked against the manifest.** Download
   computes SHA-256 before committing its staging directory. A mismatch rejects
   installation and deletes staging. Load reads each installed file once and
   checks those same bytes before constructing fastembed. A load mismatch
   preserves installed files and neither initiates nor offers a download. The
   pinned revision means a remote `main` update is not an explanation for a
   mismatch.
2. **Expected hashes were compared before recording.** For `model.onnx`, the
   Hugging Face Git-LFS SHA-256 was compared with local hashes of two separate
   downloads at different times. The small non-LFS files have no published
   SHA-256, so their values came from the two local downloads. Agreement provides
   evidence against tampering affecting only one observation. It does not exclude
   an attacker controlling all of those observations consistently.
3. **The observations used one machine and network environment.** Both downloads
   and the API request used TLS from one machine. A persistent compromise of its
   network or trusted certificate authorities could produce matching false data.
   A genuinely independent network path would provide additional evidence; it
   was not used.
4. **The artifact still depends on its publisher, Qdrant.** Matching bytes remain
   whatever Qdrant uploaded. There is no cryptographic attestation that its ONNX
   conversion of `sentence-transformers/all-MiniLM-L6-v2` is faithful or safe, and
   no independent publisher of hashes for this artifact. Its use as fastembed’s
   default source, unchanged since mid-2025 and widely consumed, provides
   reputational evidence rather than independent verification.
5. **The claimed conversion from upstream has not been reproduced.** Reproducing
   the ONNX export would replace dependence on Qdrant’s conversion with dependence
   on upstream weights and the conversion tools.
6. **The checksums depend on the distributed app.** Someone able to replace the
   app binary can replace its manifest too. Authenticating that binary requires
   the code-signing and distribution process. The project has not established
   protection against a modified build shipping rewritten checksums. Repository
   users also depend on the integrity of Git history.

## What the behavior check proves

The recorded artifact produced normalized 384-dimensional output when loaded.
This demonstrates function, not safety. With the hash checks, subsequent loads
pass the same selected bytes to the model parser, preventing undetected changes
by the server or after installation. The parser still has to handle that
artifact safely.

## Load-time verification cost

Measured 2026-10-18 on arm64 macOS 27.0.1 with an optimized build. Hashing the
complete installed 91,102,069-byte model set took a median 258.50 ms across 21
samples (257.91–259.65 ms). Files were read before timing because model loading
already materializes those bytes; the measurement isolates the SHA-256 work added
by load-time verification and excludes disk reads and ONNX initialization. Re-run
`measure_installed_model_hashing` in release mode when the model set or hashing
implementation changes.

## Runtime failure behavior

Embedding startup has three states shared with the frontend: `loading`,
`available`, and `unavailable`. The main application waits only while startup is
`loading`. A model-verification or embeddings-database failure moves startup to
`unavailable`: the diagnostic cause is logged, the application shell opens, and
ordinary library workflows remain usable. Semantic search, related-entity lookup,
and relevance ranking are withheld and the UI explains that embedding features
are unavailable without exposing technical details.

The model and database must both initialize before startup becomes `available`.
Read-write instances obtain the query model from `EmbeddingService`; read-only
instances load a query-only model alongside their read-only database handle, so
their existing embedding reads and semantic search remain available without
starting a writer service.

Library write permission and embedding readiness are independent. Library mode selects the
initialization path, but either mode begins with embeddings `loading` and can end
with them `available` or `unavailable`:

| Library mode | Embedding state | Runtime capability                                                                                                            |
| ------------ | --------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Read-write   | `loading`       | Library writes are permitted; embedding resources are initializing and embedding-backed reads are not yet available.          |
| Read-write   | `available`     | Library writes and embedding-backed reads are available.                                                                      |
| Read-write   | `unavailable`   | Library writes remain available; embedding-backed reads are unavailable.                                                      |
| Read-only    | `loading`       | Library writes are refused; query-only embedding resources are initializing and embedding-backed reads are not yet available. |
| Read-only    | `available`     | Library writes are refused; embedding-backed reads are available from the read-only database and query model.                 |
| Read-only    | `unavailable`   | Library writes and embedding-backed reads are unavailable.                                                                    |

Non-embedding, file-backed reads remain available in every combination. An
embedding failure therefore does not change the library's read-write/read-only
mode, and library mode alone does not determine whether embedding-backed reads
are available.

## Origin of the pinned hashes

Values recorded on 2026-07-11:

- `MODEL_REVISION` = the repo's `main` sha from the HF API (`/api/models/{repo}`, lastModified 2025-06-09).
- `model.onnx` sha256 = the published Git-LFS oid from the HF tree API, which agreed with two independent fetches (an earlier download of `main` and a fresh fetch at the pinned revision).
- The four JSON files are plain git blobs (no LFS oid published); their sha256s were computed from the same two fetches, which agreed.
- The recorded set was checked by loading the embedder.

For a model upgrade, obtain and compare hashes using the same procedure, then update the manifest (`crates/xaga-core/src/embeddings/model.rs`) and this record together.

## Limits

- **The checksum does not prove parser safety.** onnxruntime parses the selected model natively. Checking its bytes does not establish that parsing them is safe.
- **Verification is refusal, not recovery.** A missing or mismatched installed file
  makes embedding-model loading fail. The failed files remain unchanged for
  diagnosis; this path neither deletes them nor obtains a replacement. The app
  continues in embedding-unavailable mode.
