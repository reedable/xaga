pub mod db;
pub mod lock;
pub mod model;
pub mod pipeline;
pub mod service;

// ── Error ─────────────────────────────────────────────────────────────────────

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lock file already held by another instance — embeddings are read-only")]
    LockNotAcquired,
    #[error("Model download failed: {0}")]
    ModelDownload(String),
    #[error("Model error: {0}")]
    Model(String),
    #[error("Tokenization error: {0}")]
    Tokenize(String),
    #[error("LanceDB error: {0}")]
    LanceDb(String),
    #[error("Arrow error: {0}")]
    Arrow(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

impl From<lancedb::Error> for EmbeddingsError {
    fn from(e: lancedb::Error) -> Self {
        EmbeddingsError::LanceDb(e.to_string())
    }
}

impl From<arrow_schema::ArrowError> for EmbeddingsError {
    fn from(e: arrow_schema::ArrowError) -> Self {
        EmbeddingsError::Arrow(e.to_string())
    }
}

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use db::{EmbeddingEntry, EmbeddingsDb, KindStatus};
pub use lock::LockFile;
pub use model::{default_model_dir, download_model_if_missing, load_default_embedder, model_files_present, Embedder, DEFAULT_MODEL_REPO, MODEL_FILES};
pub use pipeline::embed_file;
pub use service::{EmbeddingService, EntityRef, Job, JobSender, ShutdownReason, StatePatch, Tier};
