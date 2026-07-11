use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::EmbeddingsError;

const MODEL_DOWNLOAD_BASE: &str = "https://huggingface.co";

/// Per-file cap on downloaded bytes, counted after decompression (the client
/// negotiates gzip; chunks arrive inflated). Bounds what a hostile server can
/// write to the staging dir. Largest legitimate file is model.onnx ≈ 90 MB.
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

pub const DEFAULT_MODEL_REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
const MODEL_FILES: &[&str] = &[
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
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
    MODEL_FILES.iter().all(|f| model_dir.join(f).is_file())
}

/// How [`download_model_if_missing`] concluded.
pub enum DownloadOutcome {
    /// The model files are present (already there, or downloaded just now).
    Ready,
    /// Another xaga process holds the account-wide lock (e.g. it is
    /// downloading right now, or writing settings); this attempt was skipped.
    SkippedLockHeld,
}

/// Downloads the model into a staging directory and atomically renames it into
/// place only once all files have been fetched.  A leftover staging directory
/// from a previous failed attempt is removed before starting.
///
/// Returns immediately (no-op) if the model files are already present. The
/// download runs under the account-wide xaga resource lock (the flock on
/// settings.json — see `fs::try_acquire_account_lock`), held across
/// check + download + commit, so concurrent downloaders (app startup vs CLI)
/// cannot collide in the staging dir; a contender skips.
pub async fn download_model_if_missing(
    client: &reqwest::Client,
) -> Result<DownloadOutcome, EmbeddingsError> {
    let model_dir = default_model_dir();
    if model_files_present(&model_dir) {
        return Ok(DownloadOutcome::Ready);
    }

    let Some(_account_lock) = crate::fs::try_acquire_account_lock() else {
        log::info!("another xaga process holds the account lock — skipping model download");
        return Ok(DownloadOutcome::SkippedLockHeld);
    };

    // Re-check under the lock: a raced contender may have completed the
    // download between our first check and the acquire.
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
    for &filename in MODEL_FILES {
        let url = format!(
            "{MODEL_DOWNLOAD_BASE}/{DEFAULT_MODEL_REPO}/resolve/main/{filename}"
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
        let written = write_capped(&mut response, &mut file, filename, MAX_FILE_BYTES).await?;
        log::info!("downloaded {filename} ({written} bytes)");
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
/// bomb. Returns the byte count written.
async fn write_capped(
    source: &mut impl ChunkSource,
    file: &mut tokio::fs::File,
    filename: &str,
    cap: u64,
) -> Result<u64, EmbeddingsError> {
    use tokio::io::AsyncWriteExt;

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
        written += chunk.len() as u64;
    }
    // tokio's File buffers: without this, drop completes the remaining write
    // in the background with no ordering guarantee against the commit step's
    // open-by-path fsync (`replace_dir_atomic`) — a truncated file could be
    // committed. Flush awaits completion of every buffered write.
    file.flush()
        .await
        .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: write: {e}")))?;
    Ok(written)
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
        let written = write_capped(&mut chunks(&[b"aaaa", b"bb"]), &mut file, "f", 10)
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
        let written = write_capped(&mut chunks(&[b"aaaa", b"bbbb"]), &mut file, "f", 8)
            .await
            .unwrap();
        assert_eq!(written, 8);
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
        let err = write_capped(&mut Failing, &mut file, "tokenizer.json", 10)
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
