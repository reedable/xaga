mod lifecycle;
mod mutations;
mod queries;
mod schema;
mod similarity;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use arrow_array::builder::{ListBuilder, StringBuilder};
use arrow_array::{
    Array, BooleanArray, FixedSizeListArray, Float32Array, ListArray, RecordBatch,
    RecordBatchIterator, RecordBatchReader, StringArray,
};
use arrow_buffer::NullBuffer;
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::DistanceType;
use lancedb::query::{ExecutableQuery, QueryBase, Select};

use super::EmbeddingsError;
use crate::fs::FileLock;
use crate::types::{EmbeddedEntry, Entry, Kind, PartialAnnotation};

use schema::{
    BatchSummaryColumns, create_empty_documents_table, entries_from_batches, entries_to_batch,
    entry_to_batch, list_entities_from_batches, make_schema,
};

impl Kind {
    /// SQL predicate selecting rows of this kind via the `kind` column. A DB-layer
    /// concern, kept here rather than on the domain type in `types.rs`.
    pub(crate) fn predicate(&self) -> String {
        format!("kind = '{}'", self.as_str())
    }
}

/// Per-kind row counts for `embeddings status`: how many rows of each kind exist
/// and how many already have a computed vector. `total - embedded` is the count of
/// rows inserted but still awaiting embedding (null vector).
pub struct KindStatus {
    pub kind: String,
    pub total: usize,
    pub embedded: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct AllEntitiesCounts {
    pub entry_count: usize,
    pub unread_count: usize,
}

pub struct EmbeddingEntry {
    pub id: String,
    /// Row kind discriminator. Stored as the `kind` Utf8 column via `Kind::as_str`.
    pub kind: Kind,
    pub source: String,
    pub title: String,
    pub created_at: String,
    pub published_at: String,
    pub updated_at: String,
    pub checksum: String,
    /// For annotation rows: the parent article ID. Empty for regular entries.
    pub reference: String,
    /// Article marking or annotation highlight color. Empty for unmarked Entries.
    pub color: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub topics: Vec<String>,
    /// The ids of the managed entities that belong to this row — a single membership meaning, agnostic
    /// to the entity kind (Topic/Entry/Annotation/…). Which relation it denormalizes depends only on
    /// the owning row:
    /// - **topic rows**: the topic's members — the prior-membership base for the reverse-index delta,
    ///   mirroring the topic `.md` `items:` field (the SoT).
    /// - **entry rows**: the entry's annotations — the recorded membership recovery diffs against the
    ///   `.md` (without a per-entry query) to gate the annotation reconcile. Written by
    ///   `sync_annotations` as its final step (after the annotation rows), so a crash leaves the rows
    ///   present and this stale — a harmless re-reconcile on the next pass.
    ///
    /// Empty for annotation/feed rows. Nullable in storage; reads back empty when absent/null.
    pub entities: Vec<String>,
    pub is_unread: bool,
    pub is_pinned: bool,
    /// `None` until the embedding is computed (row may be inserted before embedding).
    pub vector: Option<Vec<f32>>,
}

#[derive(Clone)]
pub struct EmbeddingsDb {
    tag_resolver: crate::tags::CachedResolver,
    _connection: lancedb::Connection,
    table: lancedb::Table,
    schema: Arc<Schema>,
}

#[cfg(test)]
mod tests;
