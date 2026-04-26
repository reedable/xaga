use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::EmbeddingsError;

const MODEL_DOWNLOAD_BASE: &str = "https://huggingface.co";

pub const DEFAULT_MODEL_REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
pub const MODEL_FILES: &[&str] = &[
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];

/// Returns the platform-appropriate directory for storing downloaded models.
/// e.g. ~/Library/Application Support/xaga/models on macOS.
pub fn default_model_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xaga")
        .join("models")
        .join(DEFAULT_MODEL_REPO.replace('/', "--"))
}

/// Returns true if all required model files are present in `model_dir`.
pub fn model_files_present(model_dir: &Path) -> bool {
    MODEL_FILES.iter().all(|f| model_dir.join(f).is_file())
}

/// Downloads the model into a staging directory and atomically renames it into
/// place only once all files have been fetched.  A leftover staging directory
/// from a previous failed attempt is removed before starting.
///
/// Returns immediately (no-op) if the model files are already present.
pub async fn download_model_if_missing(client: &reqwest::Client) -> Result<(), EmbeddingsError> {
    let model_dir = default_model_dir();
    if model_files_present(&model_dir) {
        return Ok(());
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

    if model_dir.exists() {
        std::fs::remove_dir_all(&model_dir).map_err(|e| {
            EmbeddingsError::ModelDownload(format!("remove partial model dir: {e}"))
        })?;
    }

    std::fs::rename(&staging_dir, &model_dir).map_err(|e| {
        EmbeddingsError::ModelDownload(format!("rename staging to model dir: {e}"))
    })?;

    log::info!("embedding model ready at {}", model_dir.display());
    Ok(())
}

async fn fetch_all_files(client: &reqwest::Client, dir: &Path) -> Result<(), EmbeddingsError> {
    for &filename in MODEL_FILES {
        let url = format!(
            "{MODEL_DOWNLOAD_BASE}/{DEFAULT_MODEL_REPO}/resolve/main/{filename}"
        );
        log::info!("downloading {filename}");
        let bytes = client
            .get(&url)
            .send()
            .await
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?
            .error_for_status()
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?
            .bytes()
            .await
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: {e}")))?;
        std::fs::write(dir.join(filename), &bytes)
            .map_err(|e| EmbeddingsError::ModelDownload(format!("{filename}: write: {e}")))?;
        log::info!("downloaded {filename} ({} bytes)", bytes.len());
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
    pub fn load(model_dir: &Path) -> Result<Self, EmbeddingsError> {
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
