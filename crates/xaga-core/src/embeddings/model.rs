use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::EmbeddingsError;

const MODEL_DOWNLOAD_BASE: &str = "https://huggingface.co";

/// Per-file cap on downloaded bytes, counted after decompression (the client
/// negotiates gzip; chunks arrive inflated). Bounds what a hostile server can
/// write to the staging dir. Largest legitimate file is model.onnx ≈ 90 MB.
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

// ── The pinned model manifest ─────────────────────────────────────────────────
//
// The single source of content trust for the model download: the repo, the
// immutable HF revision the URL pins (`resolve/{MODEL_REVISION}`), and the
// SHA-256 of every file, verified before the staging dir is committed. The
// revision keeps fresh installs working if the repo's `main` is force-pushed;
// the checksums verify what actually arrived regardless of what any server
// claims — a mismatch is unambiguous and never "the repo updated."
//
// Values anchored 2026-07-11:
// - MODEL_REVISION = the repo's `main` sha from the HF API
//   (`/api/models/{repo}`, lastModified 2025-06-09).
// - model.onnx sha256 = the published Git-LFS oid from the HF tree API,
//   which agreed with two independent fetches (an earlier download of
//   `main` and a fresh fetch at the pinned revision).
// - The four JSON files are plain git blobs (no LFS oid published); their
//   sha256s were computed from the same two fetches, which agreed.
// - The anchored set was behavior-checked by loading the embedder.
//
// Model upgrade: pick the new revision sha, re-derive every hash the same
// way (HF tree API for LFS files, two-fetch self-computation for the rest),
// replace this block wholesale.
pub const DEFAULT_MODEL_REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
const MODEL_REVISION: &str = "5f1b8cd78bc4fb444dd171e59b18f3a3af89a079";
const MODEL_FILES: &[(&str, &str)] = &[
    (
        "model.onnx",
        "bbd7b466f6d58e646fdc2bd5fd67b2f5e93c0b687011bd4548c420f7bd46f0c5",
    ),
    (
        "tokenizer.json",
        "da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0",
    ),
    (
        "config.json",
        "1b4d8e2a3988377ed8b519a31d8d31025a25f1c5f8606998e8014111438efcd7",
    ),
    (
        "special_tokens_map.json",
        "5d5b662e421ea9fac075174bb0688ee0d9431699900b90662acd44b2a350503a",
    ),
    (
        "tokenizer_config.json",
        "bd2e06a5b20fd1b13ca988bedc8763d332d242381b4fbc98f8fead4524158f79",
    ),
];

/// Returns the platform-appropriate directory for storing downloaded models.
/// e.g. ~/Library/Application Support/xaga/models on macOS.
pub fn default_model_dir() -> PathBuf {
    crate::fs::data_root()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("models")
        .join(DEFAULT_MODEL_REPO.replace('/', "--"))
}

/// Returns true if all required model files are present in `model_dir`.
pub fn model_files_present(model_dir: &Path) -> bool {
    MODEL_FILES
        .iter()
        .all(|(name, _)| model_dir.join(name).is_file())
}

/// How [`download_model`] concluded. (The "another process holds the account lock" outcome
/// is handled by the caller, which owns the acquire, so it isn't a variant here.)
pub enum DownloadOutcome {
    /// The model files are present (already there, or downloaded just now).
    Ready,
}

/// Whether the model files are present — the caller's outer check before deciding to acquire
/// the account right and download.
pub fn model_present() -> bool {
    model_files_present(&default_model_dir())
}

/// Downloads the model into a staging directory and atomically renames it into
/// place only once all files have been fetched.  A leftover staging directory
/// from a previous failed attempt is removed before starting.
///
/// **The caller must already hold the account-wide xaga resource right** (the flock on
/// settings.json) across this call — the app via its in-process account lease, the CLI via a
/// direct `fs::try_acquire_account_lock`. This function does NOT acquire it (xaga-core cannot
/// name the app's lease guard). It re-checks `model_files_present` under the held right (a
/// raced contender may have finished between the caller's check and its acquire), then
/// downloads + commits.
pub async fn download_model(
    client: &reqwest::Client,
) -> Result<DownloadOutcome, EmbeddingsError> {
    let model_dir = default_model_dir();

    // Re-check under the caller's held right: a raced contender may have completed the
    // download between the caller's outer check and its acquire.
    if model_files_present(&model_dir) {
        return Ok(DownloadOutcome::Ready);
    }

    let parent = model_dir.parent().ok_or_else(|| {
        EmbeddingsError::ModelDownload("model_dir has no parent".into())
    })?;

    let staging_dir = {
        let name = model_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        parent.join(format!("{name}-download"))
    };

    if staging_dir.exists() {
        std::fs::remove_dir_all(&staging_dir).map_err(|e| {
            EmbeddingsError::ModelDownload(format!("remove stale staging dir: {e}"))
        })?;
    }

    std::fs::create_dir_all(&staging_dir)
        .map_err(|e| EmbeddingsError::ModelDownload(format!("create staging dir: {e}")))?;

    if let Err(e) = fetch_all_files(client, &staging_dir).await {
        let _ = std::fs::remove_dir_all(&staging_dir);
        return Err(e);
    }

    // An existing model_dir here is a partial set (the complete-set check above
    // returned), so the removal window inside the swap never takes away a dir
    // readers consider valid.
    crate::fs::replace_dir_atomic_async(staging_dir, model_dir.clone())
        .await
        .map_err(|e| {
            EmbeddingsError::ModelDownload(format!("rename staging to model dir: {e}"))
        })?;

    log::info!("embedding model ready at {}", model_dir.display());
    Ok(DownloadOutcome::Ready)
}

async fn fetch_all_files(client: &reqwest::Client, dir: &Path) -> Result<(), EmbeddingsError> {
    for &(filename, expected_sha256) in MODEL_FILES {
        let url = format!(
            "{MODEL_DOWNLOAD_BASE}/{DEFAULT_MODEL_REPO}/resolve/{MODEL_REVISION}/{filename}"
        );
        log::info!("downloading {filename}");
        let mut response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?
            .error_for_status()
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?;

        // Streamed to disk: peak memory is one chunk, not the whole file
        // (model.onnx ≈ 90 MB). Durability of the written bytes is the commit
        // step's job (`replace_dir_atomic` fsyncs the staged files).
        let mut file = tokio::fs::File::create(dir.join(filename))
            .await
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: write: {e}")))?;
        let (written, digest) =
            write_capped(&mut response, &mut file, filename, MAX_FILE_BYTES).await?;
        verify_checksum(filename, expected_sha256, &digest)?;
        log::info!("downloaded {filename} ({written} bytes, sha256 verified)");
    }
    Ok(())
}

/// Compares a downloaded file's digest against its manifest entry. A mismatch
/// refuses installation — the caller's error path deletes the staging dir, so
/// unverified bytes never reach the model dir.
fn verify_checksum(
    filename: &str,
    expected_sha256: &str,
    digest_hex: &str,
) -> Result<(), EmbeddingsError> {
    if digest_hex != expected_sha256 {
        return Err(EmbeddingsError::ModelDownload(format!(
            "{filename}: checksum mismatch (expected {expected_sha256}, got {digest_hex}) — refusing to install"
        )));
    }
    Ok(())
}

/// A pull-source of response body chunks. Abstracts `reqwest::Response` so
/// [`write_capped`] is unit-testable without network I/O (no network calls
/// from unit tests). Errors are the source's own text; [`write_capped`] adds
/// the per-file context.
trait ChunkSource {
    async fn next_chunk(&mut self) -> Result<Option<bytes::Bytes>, String>;
}

impl ChunkSource for reqwest::Response {
    async fn next_chunk(&mut self) -> Result<Option<bytes::Bytes>, String> {
        self.chunk().await.map_err(|e| e.to_string())
    }
}

/// Streams `source` to `file`, refusing before any write that would take the
/// total past `cap`. At the reqwest boundary chunks arrive *decompressed*
/// (pinned for reqwest by `persistence::feeds`'s
/// `gzip_cap_counts_decompressed_bytes`), so the cap bounds a decompression
/// bomb. Every chunk is also fed to a SHA-256 incrementally (no extra read
/// pass for checksum verification). Returns the byte count written and the
/// digest as lowercase hex.
async fn write_capped(
    source: &mut impl ChunkSource,
    file: &mut tokio::fs::File,
    filename: &str,
    cap: u64,
) -> Result<(u64, String), EmbeddingsError> {
    use sha2::Digest;
    use tokio::io::AsyncWriteExt;

    let mut hasher = sha2::Sha256::new();
    let mut written: u64 = 0;
    while let Some(chunk) = source
        .next_chunk()
        .await
        .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?
    {
        if written + chunk.len() as u64 > cap {
            return Err(EmbeddingsError::ModelDownload(format!(
                "{filename}: response exceeds the {cap}-byte cap"
            )));
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: write: {e}")))?;
        hasher.update(&chunk);
        written += chunk.len() as u64;
    }
    // tokio's File buffers: without this, drop completes the remaining write
    // in the background with no ordering guarantee against the commit step's
    // open-by-path fsync (`replace_dir_atomic`) — a truncated file could be
    // committed. Flush awaits completion of every buffered write.
    file.flush()
        .await
        .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: write: {e}")))?;
    Ok((written, hex::encode(hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted chunk source: yields the queued chunks, then `None`.
    struct Chunks(std::collections::VecDeque<bytes::Bytes>);

    impl ChunkSource for Chunks {
        async fn next_chunk(&mut self) -> Result<Option<bytes::Bytes>, String> {
            Ok(self.0.pop_front())
        }
    }

    fn chunks(parts: &[&[u8]]) -> Chunks {
        Chunks(parts.iter().map(|p| bytes::Bytes::copy_from_slice(p)).collect())
    }

    async fn temp_file(dir: &tempfile::TempDir) -> tokio::fs::File {
        tokio::fs::File::create(dir.path().join("out")).await.unwrap()
    }

    #[tokio::test]
    async fn under_cap_writes_everything_and_reports_the_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = temp_file(&dir).await;
        let (written, _) = write_capped(&mut chunks(&[b"aaaa", b"bb"]), &mut file, "f", 10)
            .await
            .unwrap();
        assert_eq!(written, 6);
        drop(file);
        assert_eq!(std::fs::read(dir.path().join("out")).unwrap(), b"aaaabb");
    }

    #[tokio::test]
    async fn exactly_at_cap_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = temp_file(&dir).await;
        let (written, _) = write_capped(&mut chunks(&[b"aaaa", b"bbbb"]), &mut file, "f", 8)
            .await
            .unwrap();
        assert_eq!(written, 8);
    }

    /// The digest is the SHA-256 of the concatenated chunks: "abc" split
    /// across chunks hashes to the well-known test vector.
    #[tokio::test]
    async fn digest_is_sha256_of_the_stream() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = temp_file(&dir).await;
        let (_, digest) = write_capped(&mut chunks(&[b"ab", b"c"]), &mut file, "f", 10)
            .await
            .unwrap();
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_checksum_accepts_match_and_refuses_mismatch() {
        assert!(verify_checksum("f", "aa", "aa").is_ok());
        let err = verify_checksum("model.onnx", "aa", "bb").expect_err("mismatch must refuse");
        let msg = err.to_string();
        assert!(msg.contains("model.onnx"), "names the file: {msg}");
        assert!(msg.contains("checksum mismatch"), "names the refusal: {msg}");
    }

    #[tokio::test]
    async fn over_cap_refuses_before_the_offending_write() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = temp_file(&dir).await;
        let err = write_capped(&mut chunks(&[b"aaaa", b"bbbb", b"cc"]), &mut file, "model.onnx", 9)
            .await
            .expect_err("third chunk would exceed the cap");
        let msg = err.to_string();
        assert!(msg.contains("model.onnx"), "names the file: {msg}");
        assert!(msg.contains("9-byte cap"), "names the cap: {msg}");
        // The refusal happened before the offending write: only the first two
        // chunks are on disk. (Error path returns without flushing; flush here
        // so the read below observes the completed writes.)
        use tokio::io::AsyncWriteExt;
        file.flush().await.unwrap();
        drop(file);
        assert_eq!(std::fs::read(dir.path().join("out")).unwrap(), b"aaaabbbb");
    }

    #[tokio::test]
    async fn source_error_carries_the_file_context() {
        struct Failing;
        impl ChunkSource for Failing {
            async fn next_chunk(&mut self) -> Result<Option<bytes::Bytes>, String> {
                Err("connection reset".into())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let mut file = temp_file(&dir).await;
        let err: EmbeddingsError = write_capped(&mut Failing, &mut file, "tokenizer.json", 10)
            .await
            .expect_err("source error must propagate");
        assert!(err.to_string().contains("tokenizer.json: connection reset"));
    }
}

/// Loads the default model from `default_model_dir()`.
/// Returns `Err` if model files are missing or loading fails.
pub async fn load_default_embedder() -> Result<Arc<Embedder>, EmbeddingsError> {
    let model_dir = default_model_dir();
    if !model_files_present(&model_dir) {
        return Err(EmbeddingsError::Model(format!(
            "model not found at {} — run `xaga-cli embeddings download-model`",
            model_dir.display()
        )));
    }
    let embedder = tokio::task::spawn_blocking(move || Embedder::load(&model_dir))
        .await
        .map_err(|e| EmbeddingsError::Model(e.to_string()))??;
    Ok(Arc::new(embedder))
}

// ── Embedder ──────────────────────────────────────────────────────────────────

pub struct Embedder {
    model: fastembed::TextEmbedding,
}

impl Embedder {
    /// Loads the embedding model from `model_dir`.
    /// All five required files must already be present — see `model_files_present()`.
    pub(crate) fn load(model_dir: &Path) -> Result<Self, EmbeddingsError> {
        let read = |name: &str| -> Result<Vec<u8>, EmbeddingsError> {
            std::fs::read(model_dir.join(name))
                .map_err(|e| EmbeddingsError::Model(format!("{name}: {e}")))
        };
        let model = fastembed::TextEmbedding::try_new_from_user_defined(
            fastembed::UserDefinedEmbeddingModel {
                onnx_file: read("model.onnx")?,
                tokenizer_files: fastembed::TokenizerFiles {
                    tokenizer_file: read("tokenizer.json")?,
                    config_file: read("config.json")?,
                    special_tokens_map_file: read("special_tokens_map.json")?,
                    tokenizer_config_file: read("tokenizer_config.json")?,
                },
            },
            fastembed::InitOptionsUserDefined::default(),
        )
        .map_err(|e| EmbeddingsError::Model(e.to_string()))?;
        Ok(Embedder { model })
    }

    /// Returns a human-readable label for the compute device.
    pub fn device_label(&self) -> &'static str {
        "ONNX CPU"
    }

    /// Embeds `title + body` and returns a 384-dimensional L2-normalised vector.
    pub fn embed(&self, title: &str, body: &str) -> Result<Vec<f32>, EmbeddingsError> {
        log::debug!("embed title {title}");
        let text = format!("{title}\n\n{body}");

        let mut vecs = self
            .model
            .embed(vec![text], None)
            .map_err(|e| EmbeddingsError::Model(e.to_string()))?;
        vecs.pop()
            .ok_or_else(|| EmbeddingsError::Model("embed returned empty".into()))
    }
}
