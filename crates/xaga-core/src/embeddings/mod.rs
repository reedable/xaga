pub mod db;
pub mod model;
pub mod pipeline;
pub mod row;
pub mod service;

// ── Error ─────────────────────────────────────────────────────────────────────

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
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

impl EmbeddingsError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            EmbeddingsError::Io(_) => "embeddings:io",
            EmbeddingsError::ModelDownload(_) => "embeddings:model-download",
            EmbeddingsError::Model(_) => "embeddings:model",
            EmbeddingsError::Tokenize(_) => "embeddings:tokenize",
            EmbeddingsError::LanceDb(_) => "embeddings:lancedb",
            EmbeddingsError::Arrow(_) => "embeddings:arrow",
            EmbeddingsError::Json(_) => "embeddings:json",
            EmbeddingsError::Parse(_) => "embeddings:parse",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            EmbeddingsError::Io(_) => "Couldn't read or write search data.",
            EmbeddingsError::ModelDownload(_) => "Couldn't download the search model.",
            EmbeddingsError::Model(_) => "The search model failed.",
            EmbeddingsError::Tokenize(_) => "Couldn't process text for search.",
            EmbeddingsError::LanceDb(_) | EmbeddingsError::Arrow(_) => {
                "The search database failed."
            }
            EmbeddingsError::Json(_) | EmbeddingsError::Parse(_) => "Couldn't process search data.",
        }
    }
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

pub use db::{AllItemsCounts, EmbeddingEntry, EmbeddingsDb, KindStatus};
pub use model::{
    DEFAULT_MODEL_REPO, DownloadOutcome, Embedder, default_model_dir, download_model,
    load_default_embedder, model_files_present, model_present,
};
pub use pipeline::{embed_entry_vector, embed_topic_vector, sync_entry_row, sync_topic_row};
pub use service::{EmbeddingService, EntityRef, Job, ShutdownReason, StatePatch, Tier};
