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
    use tokio::io::AsyncWriteExt;

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
        let mut written: u64 = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?
        {
            if written + chunk.len() as u64 > MAX_FILE_BYTES {
                return Err(EmbeddingsError::ModelDownload(format!(
                    "{filename}: response exceeds the {MAX_FILE_BYTES}-byte cap"
                )));
            }
            file.write_all(&chunk)
                .await
                .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: write: {e}")))?;
            written += chunk.len() as u64;
        }
        log::info!("downloaded {filename} ({written} bytes)");
    }
    Ok(())
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
