# Secure the embedding model download (pending)

The embedding model is downloaded at startup when missing, by
`download_model_if_missing` (`crates/xaga-core/src/embeddings/model.rs`),
shared by the app (`src-tauri/src/lib.rs`, proxied client) and the CLI
(`download_model`, `crates/xaga-cli/src/cmd/embeddings.rs`, own proxy
instance). Fetches go through the SSRF proxy (per-hop address validation,
plaintext refused), stream to disk under a 256 MB per-file cap (decompressed
bytes; gzip stays on), and commit via `fs::replace_dir_atomic`. The URL is a
compile-time constant — `https://huggingface.co/Qdrant/all-MiniLM-L6-v2-onnx/
resolve/main/{file}` for five files.

## Pending

1. **Content trust (decision open: Option A or B below).** `resolve/main` is
   a moving ref — a compromised or force-pushed third-party repo (Qdrant org
   on HF) changes what every fresh install receives, silently. And nothing
   verifies the bytes: no checksum, no signature — `model.onnx` and the
   tokenizer files are attacker-controlled input to a native parser
   (onnxruntime via fastembed, `Embedder::load`) if HF, its CDN, or the CA
   ecosystem is compromised. The proxy validates where bytes come from, not
   what they are.

2. **Stall handling.** The download has no timeout; a stalled server hangs
   the model step indefinitely. Discussion of 2026-07-09: the app degrades
   gracefully during download (UI usable, embeddings reads return empty), so
   a timeout alone converts an invisible hang into an equally invisible
   permanently-degraded session — download/service failures are logged only,
   nothing reaches the UI, nothing retries in-session, and `is_cache_ready`
   cannot distinguish "downloading" from "failed" from "read-only (lock
   held)". Direction undecided: timeout, in-session retry, UI-visible
   download/failure state, on-demand re-trigger, or a combination. The
   event registry now exists (2026-07-11): new download-status events would
   be `DomainEvent` variants (`crates/xaga-core/src/events.rs`), emitted
   through the funnel in `src-tauri/src/events.rs`, named in
   `src/platform/events.ts`.

3. **Cap enforcement is untested.** The 256 MB cap can't be exercised in a
   test because `fetch_all_files` hardcodes the download URL — unlike the
   feed path's injectable URL (whose cap is pinned by
   `gzip_cap_counts_decompressed_bytes`). Verified by inspection only;
   testing it requires URL injection.

4. **Privacy.** A fresh install (or wiped model dir) contacts Hugging Face
   at app startup, revealing install/launch timing and IP to a third party.
   Runs only when files are missing.

5. ~~**Concurrent-writer staging collision.**~~ Resolved 2026-07-10 by the
   account-wide xaga resource lock (the flock on settings.json,
   `fs::try_acquire_account_lock`): `download_model_if_missing` holds it
   across check + download + commit; a contender logs and skips (CLI prints
   the skip). The same mechanism serializes settings.json writes through the
   `update_settings` funnel (fail-fast on contention, stale-view check per
   the owner's ts-at-read design), fixing the settings lost-update defect
   that shared the root cause.

## Options for item 1

### Option A — pin the HF revision

Change `resolve/main` to `resolve/<commit-sha>` in the URL.

- Pro: removes the moving-ref problem; HF commit shas are immutable
  identifiers. Estimated change: the URL format string in `fetch_all_files`
  (model.rs).
- Con: still trusts HF's servers (and the redirect chain) to serve honest
  content for that sha — no client-side verification.
- Con: model upgrades require a code change (also true of B).

### Option B — pinned SHA-256 checksums, verified after download

Embed the known SHA-256 of each of the five files in the binary; after
`fetch_all_files` writes the staging dir, hash each file and compare before
the commit rename. Mismatch: delete staging, fail with a distinct error.
The streamed-download loop can hash incrementally (no extra read pass), and
`fs::replace_dir_atomic` is the natural verify-then-commit seam.

- Pro: server compromise, CDN compromise, redirect manipulation, and TLS
  interception all reduce to "download fails". Subsumes A's protection.
- The file set is five files, fixed at compile time. Estimated runtime cost:
  hashing ~90 MB once per download.
- Con: a model upgrade requires updating five hashes in code.
- Con: does not protect files already on disk from later local tampering
  (true of A as well; load-time verification would be a further step with
  its own startup-latency cost at ~90 MB).

## Related

- `docs/architecture/security/fetching_remote_content.md` — its Limits
  section carries the content-trust and timeout gaps; update it as items
  close.
