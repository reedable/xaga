# Refactor: duplicated embedding-model download logic (pending)

The model-download logic exists twice — once in xaga-core, once in xaga-cli —
against an architecture in which core provides logic and the CLI owns
presentation. This document records the verified current state, the verified
history, hypotheses (clearly marked as hypotheses) for why the duplication
might exist or be kept, and candidate solutions. Decisions are open.

Related: `docs/_backlog/embeddings/secure_embedding_model_download.md` (security risks of
the download content; any verification mechanism chosen there interacts with
the structure described here).

## Current state (verified 2026-07-02)

Two implementations:

- **App path:** `download_model_if_missing`
  (`crates/xaga-core/src/embeddings/model.rs`). Repo fixed at compile time
  (`DEFAULT_MODEL_REPO`). Fetches all five `MODEL_FILES` into a staging
  directory and atomically renames it into place only when every file
  succeeded; a leftover staging directory from a failed attempt is removed
  first. Silent (log-based); returns `Result`. No-op when files are present.
- **CLI path:** `download_model` with its own `fetch_file` helper and its own
  reqwest client (`crates/xaga-cli/src/cmd/embeddings.rs`). Repo is a
  `--repo` argument defaulting to `DEFAULT_MODEL_REPO`. Writes each file
  directly into the live model directory with no staging; skips files that
  already exist (per-file resume); prints per-file progress and exits the
  process on error.

The CLI does not call the core function. What the CLI imports from core is
definitions only: `DEFAULT_MODEL_REPO`, `MODEL_FILES`, `default_model_dir`.

Shared state and its consequence: both writers feed one model directory. The
reader — `load_default_embedder` (model.rs), used by both the app and the
CLI — checks only that the five files exist before feeding them to
onnxruntime. It cannot distinguish which writer produced the files. A
mechanism implemented in only one writer does not constrain what the app
loads: the effective guarantee of the model directory is set by whichever
writer wrote last.

## History (verified from git)

- The CLI downloader (`fetch_file`) was present in the initial creation
  commit (96d847e, 2026-03-27).
- Core's `download_model_if_missing` was added a month later (f1485b4,
  2026-04-26).
- The CLI was never repointed to the core function after it appeared. The
  commit messages record no intent either way.

## Hypotheses — NOT facts

Why the duplication might exist or might have been kept. These are stated for
examination during solution design; none of them is recovered intent.

1. **Historical accretion, no decision at all.** Core's function was added
   for app startup, it satisfied that need, and the CLI was never revisited.
   This is the only hypothesis with evidential support (the commit dates
   above); even so, the absence of a recorded decision is not proof that no
   decision occurred.
2. **The contracts differ, and unifying them changes one side.** The CLI
   accepts an arbitrary `--repo`; the app path is fixed-repo by construction.
   A shared function taking a repo parameter would give the app's production
   path an arbitrary-repo capability it currently lacks; a second core entry
   point would keep two download functions in core.
3. **Presentation is interleaved with the loop.** The CLI prints per-file
   progress and exits mid-loop on error. For the CLI to call core and keep
   this behavior, core would need progress callbacks or an event stream —
   presentation concerns entering the logic crate.
4. **The differing failure semantics could be intended UX rather than
   drift.** Per-file skip lets an interrupted large download resume on rerun;
   staged-atomic all-or-nothing avoids a partially-populated live directory
   during unattended startup. If both semantics are correct for their
   contexts, one shared implementation must support both modes.
5. **Insulating the production path from tooling churn.** A developer-facing
   command pointed at experimental repos is subject to casual modification;
   keeping it out of core means such modifications cannot alter app startup
   behavior.

## Candidate solutions

Facts and consequences only; ordering carries no ranking.

### Option A — CLI calls `download_model_if_missing` as-is

- The duplicated fetch logic in the CLI would be deleted.
- The `--repo` argument would be removed or ignored (core's function is
  fixed-repo).
- Per-file progress output and per-file resume would be lost; the CLI would
  gain core's staging and atomic rename.
- Hypotheses 2–4, if they reflect real requirements, would surface as
  regressions.

### Option B — core exposes a parameterized download API; both paths call it

- One implementation of fetching/staging in core; the CLI passes its repo and
  (if progress output is kept) a progress callback.
- Core's API would carry a repo parameter and/or callback — the capability
  and presentation surface named in hypotheses 2 and 3 would move into core.
- The app path could keep a fixed-repo wrapper so its call site is unchanged.
- Behavior on interrupted downloads (staged vs per-file resume) would need a
  single defined answer, or a mode parameter (hypothesis 4).

### Option C — keep both implementations; document the divergence

- No code change; the duplication and the behavioral differences remain.
- The weakest-writer property of the model directory remains: a verification
  mechanism (see the security backlog document) implemented in one writer
  would not constrain the other.
- The divergence would be recorded (e.g., in module docs) so future changes
  to one path are made with knowledge of the other.

### Option D — verification at load time, independent of writers

- Orthogonal to A–C and combinable with any of them: if integrity checking
  from `secure_embedding_model_download.md` were implemented in
  `load_default_embedder` (the single reader) instead of, or in addition to,
  the writers, it would cover files regardless of which writer produced
  them.
- This changes what the security options there protect: writer-side checks
  gate what lands on disk; reader-side checks gate what reaches onnxruntime.
- Load-time hashing of ~90 MB adds work to every startup on the load path;
  writer-side checks add it only at download time.

## Open questions for the design

- Which hypotheses (2–5) describe real requirements, and which are
  after-the-fact rationales? Each one that is real constrains the solution
  shape; each one that is not can be dropped.
- Is `--repo` (arbitrary-repo download) a capability the CLI is meant to
  have? Its answer determines whether Option A is a regression or a cleanup.
- Where is the integrity boundary meant to sit — disk (writers) or load
  (reader)? This couples the choice here to the option chosen in
  `secure_embedding_model_download.md`.
