# Secure the embedding model download (pending)

The embedding model is downloaded at startup when missing, by
`download_model_if_missing` (`crates/xaga-core/src/embeddings/model.rs`), over
the **unproxied** `http_client` (`src-tauri/src/lib.rs`, setup closure). The
download URL is a compile-time constant —
`https://huggingface.co/Qdrant/all-MiniLM-L6-v2-onnx/resolve/main/{file}` for
five files (`model.onnx`, `tokenizer.json`, `config.json`,
`special_tokens_map.json`, `tokenizer_config.json`) — so no
attacker-influenced input reaches this client and SSRF does not apply. The
download is staged and atomically renamed into place, and runs only when files
are missing.

The unaddressed risks are about the *content*, which is neither pinned nor
verified, and which is executed-adjacent: `model.onnx` and the tokenizer files
are parsed by native code (onnxruntime via fastembed,
`Embedder::load`, model.rs).

## Risks

1. **`resolve/main` is a moving ref.** The app downloads whatever the
   `Qdrant/all-MiniLM-L6-v2-onnx` repo's main branch holds at that moment.
   That repo is a third party (the Qdrant org on Hugging Face). A compromised
   or force-pushed repo changes what every fresh install receives, silently.
2. **No integrity verification.** No checksum, no signature, no size sanity
   check — bytes go straight from `.bytes()` to disk (`fetch_all_files`,
   model.rs) and then into onnxruntime. A malicious `model.onnx` is not just
   "wrong embeddings": it is attacker-controlled input to a native parser.
   `tokenizer.json` likewise.
3. **The transitive trust set is wider than huggingface.co.**
   `resolve/main` redirects to HF's LFS CDN; the default reqwest policy
   follows up to 10 redirects wherever the server points, with no
   `https_only` guard. The real trust set is HF, its CDN, the CA ecosystem,
   and every hop between.
4. **No timeout.** The client is `reqwest::Client::new()` — a stalled server
   hangs the setup-time model step indefinitely.
5. **Unbounded buffering.** Each file (model.onnx ≈ 90 MB) is buffered fully
   in memory via `.bytes()` before being written.
6. **Privacy.** A fresh install (or wiped model dir) contacts Hugging Face at
   app startup, revealing install/launch timing and IP to a third party. Runs
   only when files are missing.

## Hardening options

Options A–C address content trust (risks 1–3); which risks each one covers is
stated per option. Option D addresses client behavior (risks 3–5) and
combines with any of them.

### Option A — pin the HF revision

Change `resolve/main` to `resolve/<commit-sha>` in the URL.

- Pro: removes the moving-ref problem (risk 1); HF commit shas are immutable
  identifiers. Estimated change: the URL format string in `fetch_all_files`
  (model.rs).
- Con: still trusts HF's servers (and the redirect chain) to serve honest
  content for that sha — no client-side verification (risks 2–3 remain).
- Con: model upgrades require a code change (also true of B).

### Option B — pinned SHA-256 checksums, verified after download

Embed the known SHA-256 of each of the five files in the binary; after
`fetch_all_files` writes the staging dir, hash each file and compare before
the rename into place. Mismatch: delete staging, fail with a distinct error.

- Pro: server compromise, CDN compromise, redirect manipulation, and TLS
  interception all reduce to "download fails" (risks 1–3).
- The file set is five files, fixed at compile time. Estimated runtime cost:
  hashing ~90 MB once per download.
- Con: a model upgrade requires updating five hashes in code.
- Con: does not protect files already on disk from later local tampering
  (same as every option here; load-time verification would be a further
  step with its own startup-latency cost at ~90 MB).

### Option C — bundle the model with the app

Ship the five files inside the app bundle; delete the download path entirely.

- Pro: removes the network trust question, the startup dependency on HF
  availability, and the privacy signal (risks 1–6 all moot).
- Pro: integrity becomes the app's own code-signing story (see
  docs/_backlog/build/defect__macos_code_signing.md).
- Con: bundle grows by ~90 MB; every app update re-ships the model.
- Con: model upgrades are coupled to app releases.
- Con: the CLI (`xaga-cli embeddings download-model`) has its own download
  path that would need the same decision.

### Option D — client hygiene (combines with A/B/C; moot under C)

- `.timeout(...)` / `.connect_timeout(...)` on the client (risk 4).
- Stream to disk instead of `.bytes()` (risk 5).
- `.https_only(true)` and a lower redirect cap (narrows risk 3).
- Estimated change: confined to `fetch_all_files` and the client builder.
- Con: none of it verifies content; on its own it leaves risks 1–2 untouched.

## Related

- `docs/architecture/security/fetching_untrusted_content.md` — Known gaps points here;
  update that bullet when an option is implemented.
- The CLI download command (`download_model`,
  `crates/xaga-cli/src/cmd/embeddings.rs`) is a separate implementation with
  the same unverified-download shape; it additionally takes the repo as a
  command-line argument and writes directly into the live model dir without
  staging. Both writers feed the same model directory, and the reader checks
  file existence only, so a mechanism implemented in one writer does not
  constrain what the app loads — see
  `docs/_backlog/embeddings/refactor__embedding_model_download.md` for the structure,
  history, and candidate solutions.
