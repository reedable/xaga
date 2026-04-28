use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, ListArray, RecordBatch, RecordBatchIterator,
    RecordBatchReader, StringArray,
};
use arrow_array::builder::{ListBuilder, StringBuilder};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::DistanceType;
use lancedb::query::{ExecutableQuery, QueryBase};

use super::EmbeddingsError;
use crate::types::{AnnotationSummary, EmbeddedEntry, Entry};

const EMBEDDING_DIM: i32 = 384;

/// Cosine distance threshold used when the caller does not specify one.
/// Cosine distance = 1 − cosine_similarity; range [0, 1] for unit vectors.
/// 0.5 corresponds to cosine similarity ≥ 0.5.
const DEFAULT_MAX_COSINE_DISTANCE: f32 = 0.5;

/// Converts `min_similarity` / `max_similarity` into a `(lower, upper)` cosine
/// distance range for use with `distance_range(lower, upper)`.
///
/// - `min_similarity` → upper bound on distance (items must be *at most* this far)
/// - `max_similarity` → lower bound on distance (items must be *at least* this far)
///
/// When both are `None`, the default threshold is applied as the upper bound.
fn similarity_bounds(
    min_similarity: Option<f32>,
    max_similarity: Option<f32>,
) -> (Option<f32>, Option<f32>) {
    match (min_similarity, max_similarity) {
        (None, None) => (None, Some(DEFAULT_MAX_COSINE_DISTANCE)),
        (Some(min), None) => (None, Some(1.0 - min)),
        (None, Some(max)) => (Some(1.0 - max), None),
        (Some(min), Some(max)) => (Some(1.0 - max), Some(1.0 - min)),
    }
}

pub struct EmbeddingEntry {
    pub id: String,
    pub source: String,
    pub title: String,
    pub created_at: String,
    pub published_at: String,
    pub updated_at: String,
    pub checksum: String,
    pub vector: Vec<f32>,
    /// For annotation rows: the parent article ID. Empty for regular entries.
    pub reference: String,
    /// For annotation rows: the highlight color. Empty for regular entries.
    pub color: String,
    pub tags: Vec<String>,
}

// ── Arrow helpers ─────────────────────────────────────────────────────────────

fn make_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("published_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
        Field::new("checksum", DataType::Utf8, false),
        Field::new("reference", DataType::Utf8, false),
        Field::new("color", DataType::Utf8, false),
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        ),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            false,
        ),
    ]))
}

fn entry_to_batch(
    entry: EmbeddingEntry,
    schema: &Arc<Schema>,
) -> Result<RecordBatch, EmbeddingsError> {
    let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
    let vector_values = Arc::new(Float32Array::from(entry.vector));
    let vector_array =
        FixedSizeListArray::try_new(vector_field, EMBEDDING_DIM, vector_values, None)?;

    let mut tags_builder = ListBuilder::new(StringBuilder::new());
    for tag in &entry.tags {
        tags_builder.values().append_value(tag);
    }
    tags_builder.append(true);
    let tags_array = tags_builder.finish();

    Ok(RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![entry.id])),
            Arc::new(StringArray::from(vec![entry.source])),
            Arc::new(StringArray::from(vec![entry.title])),
            Arc::new(StringArray::from(vec![entry.created_at])),
            Arc::new(StringArray::from(vec![entry.published_at])),
            Arc::new(StringArray::from(vec![entry.updated_at])),
            Arc::new(StringArray::from(vec![entry.checksum])),
            Arc::new(StringArray::from(vec![entry.reference])),
            Arc::new(StringArray::from(vec![entry.color])),
            Arc::new(tags_array),
            Arc::new(vector_array),
        ],
    )?)
}

async fn create_empty_documents_table(
    connection: &lancedb::Connection,
    schema: &Arc<Schema>,
) -> Result<(), EmbeddingsError> {
    let empty_vector = FixedSizeListArray::try_new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        EMBEDDING_DIM,
        Arc::new(Float32Array::from(Vec::<f32>::new())),
        None,
    )?;
    let empty_tags = ListBuilder::new(StringBuilder::new()).finish();
    let empty_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())), // id
            Arc::new(StringArray::from(Vec::<&str>::new())), // source
            Arc::new(StringArray::from(Vec::<&str>::new())), // title
            Arc::new(StringArray::from(Vec::<&str>::new())), // created_at
            Arc::new(StringArray::from(Vec::<&str>::new())), // published_at
            Arc::new(StringArray::from(Vec::<&str>::new())), // updated_at
            Arc::new(StringArray::from(Vec::<&str>::new())), // checksum
            Arc::new(StringArray::from(Vec::<&str>::new())), // reference
            Arc::new(StringArray::from(Vec::<&str>::new())), // color
            Arc::new(empty_tags),                            // tags
            Arc::new(empty_vector),
        ],
    )?;
    let reader: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
        vec![Ok(empty_batch)],
        schema.clone(),
    ));
    connection
        .create_table("documents", reader)
        .execute()
        .await?;
    Ok(())
}

/// Holds typed column references extracted from a `RecordBatch` for building
/// `Entry` values.  Created once per batch to avoid repeated downcasts.
struct BatchSummaryColumns<'a> {
    ids: &'a StringArray,
    sources: &'a StringArray,
    titles: &'a StringArray,
    created_ats: &'a StringArray,
    published_ats: Option<&'a StringArray>,
    updated_ats: Option<&'a StringArray>,
    references: Option<&'a StringArray>,
    colors: Option<&'a StringArray>,
    tags: Option<&'a ListArray>,
}

impl<'a> BatchSummaryColumns<'a> {
    fn from_batch(batch: &'a RecordBatch) -> Option<Self> {
        Some(Self {
            ids: batch
                .column_by_name("id")?
                .as_any()
                .downcast_ref::<StringArray>()?,
            sources: batch
                .column_by_name("source")?
                .as_any()
                .downcast_ref::<StringArray>()?,
            titles: batch
                .column_by_name("title")?
                .as_any()
                .downcast_ref::<StringArray>()?,
            created_ats: batch
                .column_by_name("created_at")?
                .as_any()
                .downcast_ref::<StringArray>()?,
            published_ats: batch
                .column_by_name("published_at")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
            updated_ats: batch
                .column_by_name("updated_at")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
            references: batch
                .column_by_name("reference")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
            colors: batch
                .column_by_name("color")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
            tags: batch
                .column_by_name("tags")
                .and_then(|c| c.as_any().downcast_ref::<ListArray>()),
        })
    }

    fn tags_at(&self, i: usize) -> Vec<String> {
        let Some(arr) = self.tags else { return vec![] };
        let item = arr.value(i);
        let Some(strings) = item.as_any().downcast_ref::<StringArray>() else { return vec![] };
        (0..strings.len()).map(|j| strings.value(j).to_string()).collect()
    }

    fn summary_at(&self, i: usize) -> Entry {
        fn opt_str(col: Option<&StringArray>, i: usize) -> Option<String> {
            col.and_then(|a| {
                let v = a.value(i);
                if v.is_empty() { None } else { Some(v.to_string()) }
            })
        }
        let tags = self.tags_at(i);
        let is_unread = tags.contains(&"unread".to_string());
        let is_bookmarked = tags.contains(&"bookmark".to_string());
        Entry {
            id: self.ids.value(i).to_string(),
            url: self.sources.value(i).to_string(),
            title: self.titles.value(i).to_string(),
            created_at: self.created_ats.value(i).to_string(),
            published_at: opt_str(self.published_ats, i),
            updated_at: opt_str(self.updated_ats, i),
            summary: None,
            is_unread,
            is_bookmarked,
            tags,
        }
    }

    fn reference_at(&self, i: usize) -> String {
        self.references.map_or(String::new(), |a| a.value(i).to_string())
    }

    fn color_at(&self, i: usize) -> String {
        self.colors.map_or(String::new(), |a| a.value(i).to_string())
    }
}

// ── EmbeddingsDb ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct EmbeddingsDb {
    _connection: lancedb::Connection,
    table: lancedb::Table,
    schema: Arc<Schema>,
}

impl EmbeddingsDb {
    /// Opens (or creates) the `embeddings.lance` database inside `data_dir`.
    ///
    /// If the directory is in an inconsistent state (open says "not found" while
    /// create says "already exists"), the directory is destroyed and recreated once.
    pub async fn open(data_dir: &Path) -> Result<Self, EmbeddingsError> {
        let schema = make_schema();
        for attempt in 0..2u8 {
            let db_path = data_dir.join("embeddings.lance");
            let connection = lancedb::connect(db_path.to_str().ok_or_else(|| {
                EmbeddingsError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "non-UTF-8 path",
                ))
            })?)
            .execute()
            .await?;

            match connection.open_table("documents").execute().await {
                Ok(table) => {
                    return Ok(EmbeddingsDb {
                        _connection: connection,
                        table,
                        schema,
                    });
                }
                Err(_) => match create_empty_documents_table(&connection, &schema).await {
                    Ok(()) => {
                        let table = connection.open_table("documents").execute().await?;
                        return Ok(EmbeddingsDb {
                            _connection: connection,
                            table,
                            schema,
                        });
                    }
                    Err(e) if attempt == 0 => {
                        // Inconsistent state: open failed and create failed.
                        // Destroy the directory and retry once with a clean slate.
                        log::warn!("EmbeddingsDb::open: inconsistent state ({e}), resetting DB");
                        drop(connection);
                        Self::destroy(data_dir)?;
                    }
                    Err(e) => return Err(e),
                },
            }
        }
        unreachable!()
    }

    /// Drops and recreates the "documents" table, erasing all indexed data.
    /// The `EmbeddingsDb` instance remains valid and ready for new inserts.
    pub async fn reset(&mut self) -> Result<(), EmbeddingsError> {
        self._connection.drop_table("documents", &[]).await?;

        create_empty_documents_table(&self._connection, &self.schema).await?;
        self.table = self._connection.open_table("documents").execute().await?;
        Ok(())
    }

    /// Deletes the `embeddings.lance` directory entirely.
    /// Used when restarting the Reconciler with a fresh DB.
    /// Silently succeeds if the directory does not exist.
    pub fn destroy(data_dir: &Path) -> std::io::Result<()> {
        match std::fs::remove_dir_all(data_dir.join("embeddings.lance")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    /// Returns the set of all checksums currently in the database.
    /// Used to build the in-memory guard shared across feed entry embedding.
    pub async fn all_checksums(
        &self,
    ) -> Result<std::collections::HashSet<String>, EmbeddingsError> {
        Ok(self.all_entries_meta().await?.into_values().map(|(checksum, _)| checksum).collect())
    }

    /// Inserts an embedding row, replacing any existing row with the same id.
    pub async fn upsert(&self, entry: EmbeddingEntry) -> Result<(), EmbeddingsError> {
        // Delete before insert rather than using merge_insert.
        //
        // merge_insert performs a relational join between the incoming row and all
        // matching rows in the table.  If the table already contains two rows with
        // the same id (from a previous bug or interrupted recovery), merge_insert
        // emits one updated output row per match — doubling the duplicates on every
        // subsequent upsert instead of cleaning them up.
        //
        // An explicit delete-then-insert guarantees exactly one row per id
        // regardless of prior state.  The service processes one message at a time
        // on a single thread, so this two-step operation is effectively atomic from
        // the application's perspective.
        let escaped = entry.id.replace('\'', "''");
        self.table
            .delete(&format!("id = '{escaped}'"))
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let batch = entry_to_batch(entry, &self.schema)?;

        let reader: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
            vec![Ok(batch)],
            self.schema.clone(),
        ));

        self.table
            .add(reader)
            .execute()
            .await
            .map(|_| ())
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))
    }

    /// Returns the total number of embedded documents.
    pub async fn count(&self) -> Result<usize, EmbeddingsError> {
        let n = self
            .table
            .count_rows(None)
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
        Ok(n)
    }

    /// Returns a map of `id → (checksum, reference)` for every row in the database.
    /// `reference` is non-empty only for annotation rows (it holds the parent article ID).
    /// Used by recovery to determine which files are already embedded and whether
    /// content has changed.
    pub async fn all_entries_meta(&self) -> Result<HashMap<String, (String, String)>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut map = HashMap::new();

        for batch in &batches {
            let ids = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let checksums = batch
                .column_by_name("checksum")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let references = batch
                .column_by_name("reference")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(ids), Some(checksums)) = (ids, checksums) {
                for i in 0..batch.num_rows() {
                    let reference = references.map_or(String::new(), |a| a.value(i).to_string());
                    map.insert(ids.value(i).to_string(), (checksums.value(i).to_string(), reference));
                }
            }
        }
        Ok(map)
    }

    /// Returns a map of `id → checksum` for all annotation rows belonging to `article_id`.
    pub async fn annotation_meta_for_article(
        &self,
        article_id: &str,
    ) -> Result<HashMap<String, String>, EmbeddingsError> {
        let escaped = article_id.replace('\'', "''");
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(format!("reference = '{escaped}'"))
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut map = HashMap::new();
        for batch in &batches {
            let ids = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let checksums = batch
                .column_by_name("checksum")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            if let (Some(ids), Some(checksums)) = (ids, checksums) {
                for i in 0..batch.num_rows() {
                    map.insert(ids.value(i).to_string(), checksums.value(i).to_string());
                }
            }
        }
        Ok(map)
    }

    /// Returns the stored checksum for `id`, or `None` if not indexed.
    pub async fn get_checksum(&self, id: &str) -> Result<Option<String>, EmbeddingsError> {
        let escaped = id.replace('\'', "''");
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(format!("id = '{escaped}'"))
            .limit(1)
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        Ok(batches.iter().find_map(|b| {
            let col = b.column_by_name("checksum")?;
            let arr = col.as_any().downcast_ref::<StringArray>()?;
            if b.num_rows() == 0 {
                None
            } else {
                Some(arr.value(0).to_string())
            }
        }))
    }

    /// Returns a fully populated `EmbeddingEntry` for `id` so the caller can
    /// mutate metadata (e.g. tags) and upsert without re-running the embedder.
    /// Returns `None` if the entry is not in the database.
    pub async fn get_entry(&self, id: &str) -> Result<Option<EmbeddingEntry>, EmbeddingsError> {
        let escaped = id.replace('\'', "''");
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(format!("id = '{escaped}'"))
            .limit(1)
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        for batch in &batches {
            if batch.num_rows() == 0 { continue; }
            let get_str = |name: &str| -> String {
                batch.column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(0).to_string())
                    .unwrap_or_default()
            };
            let tags = batch
                .column_by_name("tags")
                .and_then(|c| c.as_any().downcast_ref::<ListArray>())
                .map(|arr| {
                    let item = arr.value(0);
                    item.as_any().downcast_ref::<StringArray>()
                        .map(|s| (0..s.len()).map(|j| s.value(j).to_string()).collect())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let vector = batch
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
                .map(|arr| {
                    let values = arr.value(0);
                    values.as_any().downcast_ref::<Float32Array>()
                        .map(|f| f.values().to_vec())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            return Ok(Some(EmbeddingEntry {
                id: get_str("id"),
                source: get_str("source"),
                title: get_str("title"),
                created_at: get_str("created_at"),
                published_at: get_str("published_at"),
                updated_at: get_str("updated_at"),
                checksum: get_str("checksum"),
                reference: get_str("reference"),
                color: get_str("color"),
                tags,
                vector,
            }));
        }
        Ok(None)
    }

    /// Deletes the row whose `id` column matches exactly.
    pub async fn delete_by_id(&self, id: &str) -> Result<(), EmbeddingsError> {
        let escaped = id.replace('\'', "''");
        self.table.delete(&format!("id = '{escaped}'")).await?;
        Ok(())
    }

    /// Deletes all rows whose `id` is in `ids` using a single operation.
    /// Skips the call entirely when `ids` is empty.
    pub async fn delete_by_ids(&self, ids: &[String]) -> Result<(), EmbeddingsError> {
        if ids.is_empty() {
            return Ok(());
        }
        let in_list: Vec<String> = ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        self.table
            .delete(&format!("id IN ({})", in_list.join(", ")))
            .await?;
        Ok(())
    }

    /// Compacts small fragment files and prunes all old versions.
    /// Call this after bulk ingestion to keep the on-disk layout tidy and
    /// prevent Lance's auto-cleanup from blocking the next write.
    pub async fn compact(&self) -> Result<(), EmbeddingsError> {
        use lancedb::table::optimize::{CompactionOptions, Duration, OptimizeAction};

        log::info!("EmbeddingsDb::compact: compacting fragments");
        self.table
            .optimize(OptimizeAction::Compact {
                options: CompactionOptions::default(),
                remap_options: None,
            })
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        log::info!("EmbeddingsDb::compact: pruning old versions");
        self.table
            .optimize(OptimizeAction::Prune {
                older_than: Some(Duration::zero()),
                delete_unverified: Some(true),
                error_if_tagged_old_versions: Some(false),
            })
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        log::info!("EmbeddingsDb::compact: done");
        Ok(())
    }

    /// Returns all embedded documents sorted by source URL.
    pub async fn list(&self) -> Result<Vec<Entry>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut summaries = Vec::new();
        for batch in &batches {
            let Some(cols) = BatchSummaryColumns::from_batch(batch) else {
                continue;
            };
            for i in 0..batch.num_rows() {
                summaries.push(cols.summary_at(i));
            }
        }

        summaries.sort_by(|a, b| a.url.cmp(&b.url));
        Ok(summaries)
    }

    /// Returns all annotation rows (those with a non-empty `reference` column),
    /// sorted by `created_at`.
    pub async fn list_annotations(&self) -> Result<Vec<AnnotationSummary>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if("reference != ''")
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut results = Vec::new();
        for batch in &batches {
            let Some(cols) = BatchSummaryColumns::from_batch(batch) else {
                continue;
            };
            for i in 0..batch.num_rows() {
                results.push(AnnotationSummary {
                    id: cols.ids.value(i).to_string(),
                    reference: cols.reference_at(i),
                    title: cols.titles.value(i).to_string(),
                    color: cols.color_at(i),
                    created_at: cols.created_ats.value(i).to_string(),
                    url: cols.sources.value(i).to_string(),
                });
            }
        }

        results.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(results)
    }

    /// Full-scan similarity search returning scores alongside entries.
    ///
    /// ANN (`nearest_to`) cannot efficiently find items at distance ≥ N because
    /// it optimizes for the *closest* neighbors. When `max_similarity` is set we
    /// fall back to a full table scan, compute cosine similarities in Rust, and
    /// filter/sort manually. For typical corpus sizes (tens of thousands of
    /// documents) this is fast enough.
    ///
    /// Results are returned sorted by similarity descending (most similar first).
    async fn full_scan_scored(
        &self,
        query_vector: &[f32],
        exclude_ids: &HashSet<String>,
        min_similarity: f32,
        max_similarity: f32,
        limit: Option<usize>,
    ) -> Result<Vec<(f32, Entry, String, String)>, EmbeddingsError> {
        use arrow_array::Array;

        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let query_norm: f32 = query_vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        let mut scored: Vec<(f32, Entry, String, String)> = Vec::new();
        for batch in &batches {
            let Some(id_col) = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            else {
                continue;
            };
            let Some(vec_col) = batch
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            else {
                continue;
            };
            let Some(cols) = BatchSummaryColumns::from_batch(batch) else {
                continue;
            };

            for i in 0..batch.num_rows() {
                if exclude_ids.contains(id_col.value(i)) {
                    continue;
                }

                let Some(vec_values) = vec_col
                    .value(i)
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .map(|a| a.iter().map(|v| v.unwrap_or(0.0)).collect::<Vec<f32>>())
                else {
                    continue;
                };

                let dot: f32 = query_vector
                    .iter()
                    .zip(vec_values.iter())
                    .map(|(a, b)| a * b)
                    .sum();
                let vec_norm: f32 = vec_values.iter().map(|x| x * x).sum::<f32>().sqrt();
                let similarity = if query_norm > 0.0 && vec_norm > 0.0 {
                    dot / (query_norm * vec_norm)
                } else {
                    0.0
                };

                if similarity >= min_similarity && similarity <= max_similarity {
                    scored.push((similarity, cols.summary_at(i), cols.reference_at(i), cols.color_at(i)));
                }
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(n) = limit {
            scored.truncate(n);
        }

        Ok(scored)
    }

    async fn full_scan_by_similarity(
        &self,
        query_vector: &[f32],
        exclude_ids: &HashSet<String>,
        min_similarity: f32,
        max_similarity: f32,
        limit: Option<usize>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        Ok(self
            .full_scan_scored(
                query_vector,
                exclude_ids,
                min_similarity,
                max_similarity,
                limit,
            )
            .await?
            .into_iter()
            .map(|(similarity, entry, reference, color)| EmbeddedEntry { entry, similarity, reference, color })
            .collect())
    }

    /// Samples up to `sample_size` items from the corpus and returns every
    /// pairwise cosine similarity ≥ `min_similarity` between each sample item
    /// and all other items in the database.
    ///
    /// Uses a single full table scan for efficiency: O(sample_size × n) dot
    /// products rather than O(sample_size) full scans.
    pub async fn sample_similarities(
        &self,
        sample_size: usize,
        min_similarity: f32,
    ) -> Result<Vec<f32>, EmbeddingsError> {
        use arrow_array::Array;

        // Single full scan — collect all (id, vector) pairs.
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut all_items: Vec<Vec<f32>> = Vec::new();
        for batch in &batches {
            let Some(vec_col) = batch
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            else {
                continue;
            };

            for i in 0..batch.num_rows() {
                let Some(vec_values) = vec_col
                    .value(i)
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .map(|a| a.iter().map(|v| v.unwrap_or(0.0)).collect::<Vec<f32>>())
                else {
                    continue;
                };
                all_items.push(vec_values);
            }
        }

        let n = all_items.len();
        if n < 2 {
            return Ok(vec![]);
        }

        // Stride-based sample: spread evenly across the corpus.
        let sample_indices: Vec<usize> = if n <= sample_size {
            (0..n).collect()
        } else {
            let stride = n / sample_size;
            (0..sample_size).map(|i| i * stride).collect()
        };

        // Pre-compute norms once.
        let norms: Vec<f32> = all_items
            .iter()
            .map(|v| v.iter().map(|x| x * x).sum::<f32>().sqrt())
            .collect();

        let mut scores: Vec<f32> = Vec::new();
        for &si in &sample_indices {
            let query_norm = norms[si];
            if query_norm == 0.0 {
                continue;
            }
            let qv = &all_items[si];
            for j in 0..n {
                if j == si {
                    continue;
                }
                let vec_norm = norms[j];
                if vec_norm == 0.0 {
                    continue;
                }
                let tv = &all_items[j];
                let dot: f32 = qv.iter().zip(tv.iter()).map(|(a, b)| a * b).sum();
                let sim = dot / (query_norm * vec_norm);
                if sim >= min_similarity {
                    scores.push(sim);
                }
            }
        }

        Ok(scores)
    }

    /// Returns items whose vectors are most similar to the centroid of the
    /// items identified by `ids`, excluding those items themselves.
    ///
    /// - `limit`: cap on results; `None` returns all matches above the threshold.
    /// - `min_similarity`: cosine similarity floor in [0, 1]; `None` uses
    ///   `DEFAULT_MAX_COSINE_DISTANCE` (0.5 similarity).
    ///
    /// Returns an empty vec if none of the provided IDs are indexed.
    pub async fn find_related_to_many(
        &self,
        ids: &[String],
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        // 1. Fetch vectors for all provided IDs in one query.
        let in_list: Vec<String> = ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        let rows: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(format!("id IN ({})", in_list.join(", ")))
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        // 2. Collect source vectors (union approach — no centroid averaging).
        let mut source_vectors: Vec<Vec<f32>> = Vec::new();
        for batch in &rows {
            use arrow_array::Array;
            let Some(col) = batch.column_by_name("vector") else {
                continue;
            };
            let Some(list) = col.as_any().downcast_ref::<FixedSizeListArray>() else {
                continue;
            };
            for i in 0..batch.num_rows() {
                if let Some(values) = list
                    .value(i)
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .map(|a| a.iter().map(|v| v.unwrap_or(0.0)).collect::<Vec<f32>>())
                {
                    source_vectors.push(values);
                }
            }
        }

        if source_vectors.is_empty() {
            return Ok(vec![]);
        }

        // Pre-compute source norms.
        let source_norms: Vec<f32> = source_vectors
            .iter()
            .map(|v| v.iter().map(|x| x * x).sum::<f32>().sqrt())
            .collect();

        // 3. Full scan: for each candidate, score by max similarity to any source vector.
        let effective_min = min_similarity.unwrap_or(1.0 - DEFAULT_MAX_COSINE_DISTANCE);
        let effective_max = max_similarity.unwrap_or(f32::INFINITY);
        let exclude: HashSet<String> = ids.iter().cloned().collect();

        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut scored: Vec<(f32, Entry, String, String)> = Vec::new();
        for batch in &batches {
            use arrow_array::Array;
            let Some(id_col) = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            else {
                continue;
            };
            let Some(vec_col) = batch
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            else {
                continue;
            };
            let Some(cols) = BatchSummaryColumns::from_batch(batch) else {
                continue;
            };

            for i in 0..batch.num_rows() {
                if exclude.contains(id_col.value(i)) {
                    continue;
                }

                let Some(tv) = vec_col
                    .value(i)
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .map(|a| a.iter().map(|v| v.unwrap_or(0.0)).collect::<Vec<f32>>())
                else {
                    continue;
                };

                let vec_norm: f32 = tv.iter().map(|x| x * x).sum::<f32>().sqrt();
                if vec_norm == 0.0 {
                    continue;
                }

                let max_sim = source_vectors
                    .iter()
                    .zip(source_norms.iter())
                    .filter_map(|(sv, &snorm)| {
                        if snorm == 0.0 {
                            return None;
                        }
                        let dot: f32 = sv.iter().zip(tv.iter()).map(|(a, b)| a * b).sum();
                        Some(dot / (snorm * vec_norm))
                    })
                    .fold(f32::NEG_INFINITY, f32::max);

                if max_sim >= effective_min && max_sim <= effective_max {
                    scored.push((max_sim, cols.summary_at(i), cols.reference_at(i), cols.color_at(i)));
                }
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(n) = limit {
            scored.truncate(n);
        }

        Ok(scored
            .into_iter()
            .map(|(similarity, entry, reference, color)| EmbeddedEntry { entry, similarity, reference, color })
            .collect())
    }

    /// Returns items whose vectors are most similar to the item identified by
    /// `id`, excluding that item itself.
    ///
    /// - `limit`: cap on results; `None` returns all matches above the threshold.
    /// - `min_similarity`: cosine similarity floor in [0, 1]; `None` uses
    ///   `DEFAULT_MAX_COSINE_DISTANCE` (0.5 similarity).
    pub async fn find_related(
        &self,
        id: &str,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        // 1. Fetch the query vector for this id.
        let escaped = id.replace('\'', "''");
        let rows: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(format!("id = '{escaped}'"))
            .limit(1)
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let query_vector: Vec<f32> = rows
            .iter()
            .find_map(|b| {
                use arrow_array::Array;
                let col = b.column_by_name("vector")?;
                let list = col.as_any().downcast_ref::<FixedSizeListArray>()?;
                let values = list
                    .value(0)
                    .as_any()
                    .downcast_ref::<Float32Array>()?
                    .iter()
                    .map(|v| v.unwrap_or(0.0))
                    .collect::<Vec<f32>>();
                Some(values)
            })
            .ok_or_else(|| EmbeddingsError::LanceDb(format!("id not found: {id}")))?;

        // 2. Search, excluding the item itself.
        if max_similarity.is_some() {
            let exclude = HashSet::from([id.to_string()]);
            return self
                .full_scan_by_similarity(
                    &query_vector,
                    &exclude,
                    min_similarity.unwrap_or(f32::NEG_INFINITY),
                    max_similarity.unwrap(),
                    limit,
                )
                .await;
        }

        let effective_limit = match limit {
            Some(n) => n,
            None => self.count().await?.saturating_sub(1),
        };
        let (_, upper_distance) = similarity_bounds(min_similarity, max_similarity);
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?
            .distance_type(DistanceType::Cosine)
            .distance_range(None, upper_distance)
            .only_if(format!("id != '{escaped}'"))
            .limit(effective_limit)
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut results = Vec::new();
        for batch in &batches {
            let Some(cols) = BatchSummaryColumns::from_batch(batch) else {
                continue;
            };
            let distances = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
            for i in 0..batch.num_rows() {
                let entry = cols.summary_at(i);
                let similarity = distances.map_or(0.0, |d| 1.0 - d.value(i));
                results.push(EmbeddedEntry { entry, similarity, reference: cols.reference_at(i), color: cols.color_at(i) });
            }
        }

        Ok(results)
    }

    /// Returns items whose vectors are most similar to the given query vector.
    /// Unlike `find_related`, no ID lookup is performed and nothing is excluded.
    pub async fn find_by_vector(
        &self,
        query_vector: Vec<f32>,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        if max_similarity.is_some() {
            let exclude = HashSet::new();
            return self
                .full_scan_by_similarity(
                    &query_vector,
                    &exclude,
                    min_similarity.unwrap_or(f32::NEG_INFINITY),
                    max_similarity.unwrap(),
                    limit,
                )
                .await;
        }

        let effective_limit = match limit {
            Some(n) => n,
            None => self.count().await?,
        };
        let (_, upper_distance) = similarity_bounds(min_similarity, max_similarity);
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?
            .distance_type(DistanceType::Cosine)
            .distance_range(None, upper_distance)
            .limit(effective_limit)
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut results = Vec::new();
        for batch in &batches {
            let Some(cols) = BatchSummaryColumns::from_batch(batch) else {
                continue;
            };
            let distances = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
            for i in 0..batch.num_rows() {
                let entry = cols.summary_at(i);
                let similarity = distances.map_or(0.0, |d| 1.0 - d.value(i));
                results.push(EmbeddedEntry {
                    entry,
                    similarity,
                    reference: cols.reference_at(i),
                    color: cols.color_at(i),
                });
            }
        }
        Ok(results)
    }
}
