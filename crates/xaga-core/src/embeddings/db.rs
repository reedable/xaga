use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, BooleanArray, FixedSizeListArray, Float32Array, ListArray, RecordBatch,
    RecordBatchIterator, RecordBatchReader, StringArray,
};
use arrow_array::builder::{ListBuilder, StringBuilder};
use arrow_buffer::NullBuffer;
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::DistanceType;
use lancedb::query::{ExecutableQuery, QueryBase, Select};

use super::EmbeddingsError;
use crate::types::{PartialAnnotation, EmbeddedEntry, Entry, Kind};

const EMBEDDING_DIM: i32 = 384;

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
    /// For annotation rows: the highlight color. Empty for regular entries.
    pub color: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub topics: Vec<String>,
    /// For topic rows: the ids of the topic's members (entries/annotations) — the prior-membership
    /// base for the reverse-index delta, mirroring the topic `.md` `items:` field (the SoT). Empty
    /// for entry/annotation/feed rows. Nullable in storage; reads back empty when absent/null.
    pub items: Vec<String>,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    pub is_pinned: bool,
    /// `None` until the embedding is computed (row may be inserted before embedding).
    pub vector: Option<Vec<f32>>,
}

// ── Arrow helpers ─────────────────────────────────────────────────────────────

fn make_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("published_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
        Field::new("checksum", DataType::Utf8, false),
        Field::new("reference", DataType::Utf8, false),
        Field::new("color", DataType::Utf8, false),
        Field::new("summary", DataType::Utf8, false),
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        ),
        Field::new(
            "topics",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        ),
        // Nullable so it can be added to a pre-existing table via `add_columns(AllNulls)`
        // (metadata-only); existing rows read back empty.
        Field::new(
            "items",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new("is_unread", DataType::Boolean, false),
        Field::new("is_bookmarked", DataType::Boolean, false),
        Field::new("is_pinned", DataType::Boolean, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            true,
        ),
    ]))
}

fn entry_to_batch(
    entry: EmbeddingEntry,
    schema: &Arc<Schema>,
) -> Result<RecordBatch, EmbeddingsError> {
    entries_to_batch(vec![entry], schema)
}

/// Builds a record batch from many entries: one column per field, one row per entry.
/// A null vector is stored as `EMBEDDING_DIM` placeholder slots marked invalid via the
/// validity bitmap.
fn entries_to_batch(
    entries: Vec<EmbeddingEntry>,
    schema: &Arc<Schema>,
) -> Result<RecordBatch, EmbeddingsError> {
    let n = entries.len();
    let dim = EMBEDDING_DIM as usize;

    let mut ids = Vec::with_capacity(n);
    let mut kinds = Vec::with_capacity(n);
    let mut sources = Vec::with_capacity(n);
    let mut titles = Vec::with_capacity(n);
    let mut created_ats = Vec::with_capacity(n);
    let mut published_ats = Vec::with_capacity(n);
    let mut updated_ats = Vec::with_capacity(n);
    let mut checksums = Vec::with_capacity(n);
    let mut references = Vec::with_capacity(n);
    let mut colors = Vec::with_capacity(n);
    let mut summaries = Vec::with_capacity(n);
    let mut unread = Vec::with_capacity(n);
    let mut bookmarked = Vec::with_capacity(n);
    let mut pinned = Vec::with_capacity(n);

    let mut tags_builder = ListBuilder::new(StringBuilder::new());
    let mut topics_builder = ListBuilder::new(StringBuilder::new());
    let mut items_builder = ListBuilder::new(StringBuilder::new());

    let mut vec_values: Vec<f32> = Vec::with_capacity(n * dim);
    let mut vec_validity: Vec<bool> = Vec::with_capacity(n);

    for entry in entries {
        ids.push(entry.id);
        kinds.push(entry.kind.as_str().to_string());
        sources.push(entry.source);
        titles.push(entry.title);
        created_ats.push(entry.created_at);
        published_ats.push(entry.published_at);
        updated_ats.push(entry.updated_at);
        checksums.push(entry.checksum);
        references.push(entry.reference);
        colors.push(entry.color);
        summaries.push(entry.summary);
        unread.push(entry.is_unread);
        bookmarked.push(entry.is_bookmarked);
        pinned.push(entry.is_pinned);

        for tag in &entry.tags {
            tags_builder.values().append_value(tag);
        }
        tags_builder.append(true);

        for topic in &entry.topics {
            topics_builder.values().append_value(topic);
        }
        topics_builder.append(true);

        for item in &entry.items {
            items_builder.values().append_value(item);
        }
        items_builder.append(true);

        match entry.vector {
            Some(v) => {
                vec_values.extend_from_slice(&v);
                vec_validity.push(true);
            }
            None => {
                vec_values.extend(std::iter::repeat(0.0f32).take(dim));
                vec_validity.push(false);
            }
        }
    }

    let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
    let vector_array = FixedSizeListArray::try_new(
        vector_field,
        EMBEDDING_DIM,
        Arc::new(Float32Array::from(vec_values)),
        Some(NullBuffer::from(vec_validity)),
    )?;

    Ok(RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(kinds)),
            Arc::new(StringArray::from(sources)),
            Arc::new(StringArray::from(titles)),
            Arc::new(StringArray::from(created_ats)),
            Arc::new(StringArray::from(published_ats)),
            Arc::new(StringArray::from(updated_ats)),
            Arc::new(StringArray::from(checksums)),
            Arc::new(StringArray::from(references)),
            Arc::new(StringArray::from(colors)),
            Arc::new(StringArray::from(summaries)),
            Arc::new(tags_builder.finish()),
            Arc::new(topics_builder.finish()),
            Arc::new(items_builder.finish()),
            Arc::new(BooleanArray::from(unread)),
            Arc::new(BooleanArray::from(bookmarked)),
            Arc::new(BooleanArray::from(pinned)),
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
    let empty_topics = ListBuilder::new(StringBuilder::new()).finish();
    let empty_items = ListBuilder::new(StringBuilder::new()).finish();
    let empty_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())), // id
            Arc::new(StringArray::from(Vec::<&str>::new())), // kind
            Arc::new(StringArray::from(Vec::<&str>::new())), // source
            Arc::new(StringArray::from(Vec::<&str>::new())), // title
            Arc::new(StringArray::from(Vec::<&str>::new())), // created_at
            Arc::new(StringArray::from(Vec::<&str>::new())), // published_at
            Arc::new(StringArray::from(Vec::<&str>::new())), // updated_at
            Arc::new(StringArray::from(Vec::<&str>::new())), // checksum
            Arc::new(StringArray::from(Vec::<&str>::new())), // reference
            Arc::new(StringArray::from(Vec::<&str>::new())), // color
            Arc::new(StringArray::from(Vec::<&str>::new())), // summary
            Arc::new(empty_tags),                            // tags
            Arc::new(empty_topics),                          // topics
            Arc::new(empty_items),                           // items
            Arc::new(BooleanArray::from(Vec::<bool>::new())), // is_unread
            Arc::new(BooleanArray::from(Vec::<bool>::new())), // is_bookmarked
            Arc::new(BooleanArray::from(Vec::<bool>::new())), // is_pinned
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
    summaries: Option<&'a StringArray>,
    tags: Option<&'a ListArray>,
    kinds: Option<&'a StringArray>,
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
            summaries: batch
                .column_by_name("summary")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
            tags: batch
                .column_by_name("tags")
                .and_then(|c| c.as_any().downcast_ref::<ListArray>()),
            kinds: batch
                .column_by_name("kind")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
        })
    }

    /// The row's domain kind. Defaults to `Entry` if the column is absent (it is
    /// part of `SUMMARY_COLUMNS` and the full row, so this should not occur).
    fn kind_at(&self, i: usize) -> Kind {
        self.kinds
            .and_then(|a| a.value(i).parse().ok())
            .unwrap_or(Kind::Entry)
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
        let is_unread = crate::tags::is_unread(&tags);
        let is_bookmarked = crate::tags::is_bookmarked(&tags);
        Entry {
            id: self.ids.value(i).to_string(),
            url: self.sources.value(i).to_string(),
            title: self.titles.value(i).to_string(),
            created_at: self.created_ats.value(i).to_string(),
            published_at: opt_str(self.published_ats, i),
            updated_at: opt_str(self.updated_ats, i),
            description: opt_str(self.summaries, i),
            is_unread,
            is_bookmarked,
            color: None,
        }
    }

    fn reference_at(&self, i: usize) -> String {
        self.references.map_or(String::new(), |a| a.value(i).to_string())
    }

    fn color_at(&self, i: usize) -> String {
        self.colors.map_or(String::new(), |a| a.value(i).to_string())
    }
}

/// Builds `Entry` values from query result batches (used by the list-read API).
fn entries_from_batches(batches: &[RecordBatch]) -> Vec<Entry> {
    let mut out = Vec::new();
    for batch in batches {
        if let Some(cols) = BatchSummaryColumns::from_batch(batch) {
            for i in 0..batch.num_rows() {
                out.push(cols.summary_at(i));
            }
        }
    }
    out
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
            let db_path = crate::library::index_path(data_dir);
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
                    // Roll the `items` column onto a table created before it existed. `AllNulls`
                    // is metadata-only (no fragment rewrite, no re-embed) — see lance schema_evolution.
                    if table.schema().await?.field_with_name("items").is_err() {
                        log::info!(
                            "EmbeddingsDb::open: adding 'items' column to existing table (metadata-only)"
                        );
                        let items_schema = Arc::new(Schema::new(vec![Field::new(
                            "items",
                            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                            true,
                        )]));
                        table
                            .add_columns(
                                lancedb::table::NewColumnTransform::AllNulls(items_schema),
                                None,
                            )
                            .await?;
                    }
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
    pub(crate) async fn reset(&mut self) -> Result<(), EmbeddingsError> {
        self._connection.drop_table("documents", &[]).await?;

        create_empty_documents_table(&self._connection, &self.schema).await?;
        self.table = self._connection.open_table("documents").execute().await?;
        Ok(())
    }

    /// Deletes the `embeddings.lance` directory entirely.
    /// Used when restarting the Reconciler with a fresh DB.
    /// Silently succeeds if the directory does not exist.
    pub fn destroy(data_dir: &Path) -> std::io::Result<()> {
        match std::fs::remove_dir_all(crate::library::index_path(data_dir)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    /// Creates scalar indexes when absent, idempotently: BTree on `id` (id-set retrieval) and
    /// `published_at` (ordered reads); Bitmap on the low-cardinality state booleans
    /// `is_unread`/`is_bookmarked`/`is_pinned` so state filters and counts are index-driven rather
    /// than full scans. Intended to run after recovery has populated the rows, so indexes train
    /// over real data.
    pub(crate) async fn ensure_indexes(&self) -> Result<(), EmbeddingsError> {
        let indexed: HashSet<String> = self
            .table
            .list_indices()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?
            .into_iter()
            .flat_map(|idx| idx.columns)
            .collect();

        for col in ["id", "published_at"] {
            if indexed.contains(col) {
                continue;
            }
            self.table
                .create_index(&[col], lancedb::index::Index::BTree(Default::default()))
                .execute()
                .await
                .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
            log::info!("EmbeddingsDb: created BTree index on `{col}`");
        }

        // Low-cardinality state booleans: Bitmap is the right index (the unread/bookmarked id-sets
        // for sidebar counts, and the Saved list, become index-driven not scans).
        for col in ["is_unread", "is_bookmarked", "is_pinned"] {
            if indexed.contains(col) {
                continue;
            }
            self.table
                .create_index(&[col], lancedb::index::Index::Bitmap(Default::default()))
                .execute()
                .await
                .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
            log::info!("EmbeddingsDb: created Bitmap index on `{col}`");
        }
        Ok(())
    }

    /// The set of entry ids currently marked unread — read via the `is_unread` Bitmap index
    /// (no full scan), selecting only `id`. Used for sidebar unread counts.
    pub async fn unread_ids(&self) -> Result<HashSet<String>, EmbeddingsError> {
        self.state_ids("is_unread = true").await
    }

    /// The set of entry ids currently bookmarked — read via the `is_bookmarked` Bitmap index.
    pub async fn bookmarked_ids(&self) -> Result<HashSet<String>, EmbeddingsError> {
        self.state_ids("is_bookmarked = true").await
    }

    /// Projects entry-row ids matching a state `predicate` into a set, selecting only the `id`
    /// column so the read never touches the vector.
    async fn state_ids(&self, predicate: &str) -> Result<HashSet<String>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(format!("kind = 'entry' AND {predicate}"))
            .select(Select::columns(&["id"]))
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut ids = HashSet::new();
        for batch in &batches {
            if let Some(col) = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            {
                for i in 0..col.len() {
                    ids.insert(col.value(i).to_string());
                }
            }
        }
        Ok(ids)
    }

    /// Columns needed to build an `Entry` — everything except the vector, so these
    /// list reads never touch the large embedding column.
    const SUMMARY_COLUMNS: [&'static str; 11] = [
        "id", "kind", "source", "title", "created_at", "published_at", "updated_at",
        "reference", "color", "summary", "tags",
    ];

    /// Fetches entry rows for the given ids (e.g. a feed's atom-supplied id set),
    /// served by the scalar `id` index. Order is not guaranteed; callers sort.
    pub async fn entries_by_ids(&self, ids: &[String]) -> Result<Vec<Entry>, EmbeddingsError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let in_list: Vec<String> = ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if(format!("id IN ({})", in_list.join(", ")))
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
        Ok(entries_from_batches(&batches))
    }

    /// All entry rows (`kind = 'entry'`).
    pub async fn all_entries(&self) -> Result<Vec<Entry>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if(Kind::Entry.predicate())
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
        Ok(entries_from_batches(&batches))
    }

    /// Entry rows that are bookmarked.
    pub async fn bookmarked_entries(&self) -> Result<Vec<Entry>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if(format!("{} AND is_bookmarked = true", Kind::Entry.predicate()))
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
        Ok(entries_from_batches(&batches))
    }

    /// Read-only per-kind summary: total rows and how many are embedded (non-null
    /// vector). Counts are pushed down to LanceDB (no row materialization). Kinds
    /// with zero rows are omitted.
    pub async fn embedding_status(&self) -> Result<Vec<KindStatus>, EmbeddingsError> {
        let mut out = Vec::new();
        for kind in [Kind::Entry, Kind::Annotation, Kind::Topic, Kind::Feed] {
            let total = self
                .table
                .count_rows(Some(kind.predicate()))
                .await
                .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
            if total == 0 {
                continue;
            }
            let embedded = self
                .table
                .count_rows(Some(format!("{} AND vector IS NOT NULL", kind.predicate())))
                .await
                .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
            out.push(KindStatus {
                kind: kind.as_str().to_string(),
                total,
                embedded,
            });
        }
        Ok(out)
    }

    /// Inserts an embedding row, replacing any existing row with the same id.
    pub(crate) async fn upsert(&self, entry: EmbeddingEntry) -> Result<(), EmbeddingsError> {
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

    /// Inserts many rows in a single `add`, with no per-row `get_entry`/`delete`.
    /// Intended for bulk recovery into a table that has no pre-existing rows for
    /// these ids (e.g. a fresh post-`reset` recovery); it does not de-duplicate.
    pub(crate) async fn add_batch(&self, entries: Vec<EmbeddingEntry>) -> Result<(), EmbeddingsError> {
        if entries.is_empty() {
            return Ok(());
        }
        let batch = entries_to_batch(entries, &self.schema)?;
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

    /// Refreshes this handle to the latest committed table version, so reads see writes
    /// made by another connection (e.g. the embedding service). LanceDB table handles are
    /// pinned to the version they opened at until checked out.
    pub async fn reload(&self) -> Result<(), EmbeddingsError> {
        self.table
            .checkout_latest()
            .await
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

    /// Returns a map of `id → (checksum, reference, has_vector, kind)` for every row.
    /// `reference` is non-empty only for annotation rows (it holds the parent article ID).
    /// `has_vector` is false for rows inserted before their embedding was computed.
    /// `kind` is the row-kind discriminator ("entry" | "annotation" | "topic" | "feed").
    /// Used by recovery to determine which files need (re-)embedding.
    pub(crate) async fn all_entries_meta(
        &self,
    ) -> Result<HashMap<String, (String, String, bool, String)>, EmbeddingsError> {
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
            let vectors = batch
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>());
            let kinds = batch
                .column_by_name("kind")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(ids), Some(checksums)) = (ids, checksums) {
                for i in 0..batch.num_rows() {
                    let reference = references.map_or(String::new(), |a| a.value(i).to_string());
                    let has_vector = vectors.map_or(false, |v| !v.is_null(i));
                    let kind = kinds.map_or(String::new(), |a| a.value(i).to_string());
                    map.insert(
                        ids.value(i).to_string(),
                        (checksums.value(i).to_string(), reference, has_vector, kind),
                    );
                }
            }
        }
        Ok(map)
    }

    /// Returns a map of `id → checksum` for all annotation rows belonging to `article_id`.
    /// Returns `(checksum, color)` per annotation id for the given article.
    pub(crate) async fn annotation_meta_for_article(
        &self,
        article_id: &str,
    ) -> Result<HashMap<String, (String, String)>, EmbeddingsError> {
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
            let colors = batch
                .column_by_name("color")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            if let (Some(ids), Some(checksums), Some(colors)) = (ids, checksums, colors) {
                for i in 0..batch.num_rows() {
                    map.insert(
                        ids.value(i).to_string(),
                        (checksums.value(i).to_string(), colors.value(i).to_string()),
                    );
                }
            }
        }
        Ok(map)
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
            let read_list = |name: &str| -> Vec<String> {
                batch
                    .column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<ListArray>())
                    .map(|arr| {
                        let item = arr.value(0);
                        item.as_any().downcast_ref::<StringArray>()
                            .map(|s| (0..s.len()).map(|j| s.value(j).to_string()).collect())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default()
            };
            let get_bool = |name: &str| -> bool {
                batch
                    .column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<BooleanArray>())
                    .map(|a| a.value(0))
                    .unwrap_or(false)
            };
            // `None` when the vector column is null for this row (not yet embedded).
            let vector = batch
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
                .and_then(|arr| {
                    if arr.is_null(0) {
                        return None;
                    }
                    let values = arr.value(0);
                    values
                        .as_any()
                        .downcast_ref::<Float32Array>()
                        .map(|f| f.values().to_vec())
                });
            return Ok(Some(EmbeddingEntry {
                id: get_str("id"),
                kind: get_str("kind").parse().unwrap_or(Kind::Entry),
                source: get_str("source"),
                title: get_str("title"),
                created_at: get_str("created_at"),
                published_at: get_str("published_at"),
                updated_at: get_str("updated_at"),
                checksum: get_str("checksum"),
                reference: get_str("reference"),
                color: get_str("color"),
                summary: get_str("summary"),
                tags: read_list("tags"),
                topics: read_list("topics"),
                items: read_list("items"),
                is_unread: get_bool("is_unread"),
                is_bookmarked: get_bool("is_bookmarked"),
                is_pinned: get_bool("is_pinned"),
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
    pub(crate) async fn delete_by_ids(&self, ids: &[String]) -> Result<(), EmbeddingsError> {
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
    pub(crate) async fn compact(&self) -> Result<(), EmbeddingsError> {
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

        Ok(summaries)
    }

    /// Returns all annotation rows (those with a non-empty `reference` column),
    /// sorted by `created_at`.
    pub async fn list_annotations(&self) -> Result<Vec<PartialAnnotation>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(Kind::Annotation.predicate())
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
                results.push(PartialAnnotation {
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
        row_kind: Option<&Kind>,
    ) -> Result<Vec<(f32, Entry, Kind, String, String)>, EmbeddingsError> {
        use arrow_array::Array;

        let mut q = self.table.query();
        if let Some(kind) = row_kind {
            q = q.only_if(kind.predicate());
        }
        let batches: Vec<RecordBatch> = q
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let query_norm: f32 = query_vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        let mut scored: Vec<(f32, Entry, Kind, String, String)> = Vec::new();
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
                // Skip rows whose embedding hasn't been computed yet (null vector).
                if vec_col.is_null(i) {
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
                    scored.push((similarity, cols.summary_at(i), cols.kind_at(i), cols.reference_at(i), cols.color_at(i)));
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
        row_kind: Option<&Kind>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        Ok(self
            .full_scan_scored(
                query_vector,
                exclude_ids,
                min_similarity,
                max_similarity,
                limit,
                row_kind,
            )
            .await?
            .into_iter()
            .map(|(similarity, mut entry, kind, reference, color)| {
                if !color.is_empty() { entry.color = Some(color); }
                EmbeddedEntry { entry, kind, similarity, reference }
            })
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
                // Skip rows whose embedding hasn't been computed yet (null vector).
                if vec_col.is_null(i) {
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

    /// Returns items whose vectors are most similar to the item identified by
    /// `id`, excluding that item itself.
    ///
    /// - `limit`: cap on results; `None` returns all matches above the threshold.
    /// - `min_similarity`: cosine similarity floor in [0, 1]; `None` uses
    ///   `DEFAULT_MAX_COSINE_DISTANCE` (0.5 similarity).
    pub(crate) async fn find_related(
        &self,
        id: &str,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
        row_kind: Option<Kind>,
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
                // A pending (null-vector) query row has no usable vector to search by.
                if list.is_null(0) {
                    return None;
                }
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
                    row_kind.as_ref(),
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
            .bypass_vector_index()
            .distance_type(DistanceType::Cosine)
            .distance_range(None, upper_distance)
            .only_if(match &row_kind {
                Some(k) => format!("id != '{escaped}' AND {} AND vector IS NOT NULL", k.predicate()),
                None => format!("id != '{escaped}' AND vector IS NOT NULL"),
            })
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
                let mut entry = cols.summary_at(i);
                let color = cols.color_at(i);
                if !color.is_empty() { entry.color = Some(color); }
                let similarity = distances.map_or(0.0, |d| 1.0 - d.value(i));
                results.push(EmbeddedEntry { entry, kind: cols.kind_at(i), similarity, reference: cols.reference_at(i) });
            }
        }

        Ok(results)
    }

    /// Returns items whose vectors are most similar to the given query vector.
    /// Unlike `find_related`, no ID lookup is performed and nothing is excluded.
    pub(crate) async fn find_by_vector(
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
                    None,
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
            .bypass_vector_index()
            .distance_type(DistanceType::Cosine)
            .distance_range(None, upper_distance)
            .only_if("vector IS NOT NULL")
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
                let mut entry = cols.summary_at(i);
                let color = cols.color_at(i);
                if !color.is_empty() { entry.color = Some(color); }
                let similarity = distances.map_or(0.0, |d| 1.0 - d.value(i));
                results.push(EmbeddedEntry {
                    entry,
                    kind: cols.kind_at(i),
                    similarity,
                    reference: cols.reference_at(i),
                });
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, kind: &str, vector: Option<Vec<f32>>) -> EmbeddingEntry {
        EmbeddingEntry {
            id: id.to_string(),
            kind: kind.parse().expect("valid kind"),
            source: "https://example.com/a".to_string(),
            title: "Title".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            checksum: "abc".to_string(),
            reference: String::new(),
            color: String::new(),
            summary: "a summary".to_string(),
            tags: vec!["unread".to_string()],
            topics: vec!["topic-1".to_string(), "topic-2".to_string()],
            items: vec![],
            is_unread: true,
            is_bookmarked: false,
            is_pinned: true,
            vector,
        }
    }

    /// The schema round-trips a row both with a vector and with a null vector,
    /// preserving the new `kind`/state/`summary`/`topics` columns. This guards the
    /// Arrow null-validity buffer and column ordering before the single DB reset.
    #[tokio::test]
    async fn schema_round_trips_with_and_without_vector() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        db.upsert(sample("with-vec", "entry", Some(vec![0.5f32; EMBEDDING_DIM as usize])))
            .await
            .unwrap();
        db.upsert(sample("no-vec", "entry", None)).await.unwrap();

        assert_eq!(db.count().await.unwrap(), 2);

        let got = db.get_entry("with-vec").await.unwrap().expect("row present");
        assert_eq!(got.kind, Kind::Entry);
        assert_eq!(got.summary, "a summary");
        assert_eq!(got.topics, vec!["topic-1".to_string(), "topic-2".to_string()]);
        assert!(got.is_unread && got.is_pinned && !got.is_bookmarked);
        let v = got.vector.expect("vector present");
        assert_eq!(v.len(), EMBEDDING_DIM as usize);
        assert_eq!(v[0], 0.5);

        let pending = db.get_entry("no-vec").await.unwrap().expect("row present");
        assert!(pending.vector.is_none(), "null-vector row must read back as None");
        assert_eq!(pending.topics, vec!["topic-1".to_string(), "topic-2".to_string()]);
        assert!(pending.is_pinned);
    }

    /// The `items` column (the topic-membership reverse-index base) round-trips, and an
    /// empty `items` reads back empty — guards the new column's Arrow building and ordering.
    #[tokio::test]
    async fn items_column_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut topic = sample("topic-x", "topic", None);
        topic.items = vec!["m1".to_string(), "m2".to_string(), "m3".to_string()];
        db.upsert(topic).await.unwrap();
        db.upsert(sample("entry-x", "entry", None)).await.unwrap();

        let got = db.get_entry("topic-x").await.unwrap().expect("topic row present");
        assert_eq!(
            got.items,
            vec!["m1".to_string(), "m2".to_string(), "m3".to_string()]
        );

        let entry = db.get_entry("entry-x").await.unwrap().expect("entry row present");
        assert!(entry.items.is_empty(), "entry rows carry no items");
    }

    /// `reconcile_topic_membership` adds the topic to newly-listed members, removes it from
    /// dropped members (via the topic-row `items` base), and tracks the base for the next diff.
    #[tokio::test]
    async fn reconcile_topic_membership_adds_removes_and_tracks_base() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        // Topic row with empty base; two entry rows with empty topics.
        let mut topic = sample("topic-1", "topic", None);
        topic.topics = vec![];
        db.upsert(topic).await.unwrap();
        for id in ["entry-1", "entry-2"] {
            let mut e = sample(id, "entry", None);
            e.topics = vec![];
            db.upsert(e).await.unwrap();
        }

        // First reconcile: both entries become members.
        crate::embeddings::pipeline::reconcile_topic_membership(
            &db,
            "topic-1",
            &["entry-1".to_string(), "entry-2".to_string()],
        )
        .await
        .unwrap();

        assert!(db.get_entry("entry-1").await.unwrap().unwrap().topics.contains(&"topic-1".to_string()));
        assert!(db.get_entry("entry-2").await.unwrap().unwrap().topics.contains(&"topic-1".to_string()));
        assert_eq!(
            db.get_entry("topic-1").await.unwrap().unwrap().items,
            vec!["entry-1".to_string(), "entry-2".to_string()],
            "base tracks the propagated members"
        );

        // Second reconcile: entry-2 dropped from the topic.
        crate::embeddings::pipeline::reconcile_topic_membership(
            &db,
            "topic-1",
            &["entry-1".to_string()],
        )
        .await
        .unwrap();

        assert!(db.get_entry("entry-1").await.unwrap().unwrap().topics.contains(&"topic-1".to_string()));
        assert!(
            !db.get_entry("entry-2").await.unwrap().unwrap().topics.contains(&"topic-1".to_string()),
            "dropped member loses the topic"
        );
        assert_eq!(db.get_entry("topic-1").await.unwrap().unwrap().items, vec!["entry-1".to_string()]);
    }

    /// `add_topic_member` / `remove_topic_member` update just that one member's reverse-index
    /// `topics` column and the topic-row `items` base — the precise O(1) live path. Re-adding
    /// an existing member is a no-op.
    #[tokio::test]
    async fn single_member_add_remove_tracks_base() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut topic = sample("topic-1", "topic", None);
        topic.topics = vec![];
        topic.items = vec![];
        db.upsert(topic).await.unwrap();
        let mut e = sample("entry-1", "entry", None);
        e.topics = vec![];
        db.upsert(e).await.unwrap();

        crate::embeddings::pipeline::add_topic_member(&db, "topic-1", "entry-1").await.unwrap();
        assert!(db.get_entry("entry-1").await.unwrap().unwrap().topics.contains(&"topic-1".to_string()));
        assert_eq!(db.get_entry("topic-1").await.unwrap().unwrap().items, vec!["entry-1".to_string()]);

        // Idempotent re-add: base unchanged.
        crate::embeddings::pipeline::add_topic_member(&db, "topic-1", "entry-1").await.unwrap();
        assert_eq!(db.get_entry("topic-1").await.unwrap().unwrap().items, vec!["entry-1".to_string()]);

        crate::embeddings::pipeline::remove_topic_member(&db, "topic-1", "entry-1").await.unwrap();
        assert!(!db.get_entry("entry-1").await.unwrap().unwrap().topics.contains(&"topic-1".to_string()));
        assert!(db.get_entry("topic-1").await.unwrap().unwrap().items.is_empty());
    }

    /// `apply_state` write-through flips the boolean columns + tags from the carried values,
    /// preserves the vector/topics, and emits a change event only for columns that changed.
    #[tokio::test]
    async fn apply_state_writes_through_and_emits() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut e = sample("entry-1", "entry", Some(vec![0.1f32; EMBEDDING_DIM as usize]));
        e.tags = vec!["unread".to_string()];
        e.is_unread = true;
        e.is_bookmarked = false;
        e.topics = vec!["topic-1".to_string()];
        db.upsert(e).await.unwrap();

        let (unread_tx, unread_rx) = flume::unbounded();
        let (bm_tx, bm_rx) = flume::unbounded();

        // Mark read (is_unread -> false) and bookmark (-> true) in one patch.
        crate::embeddings::pipeline::apply_state(
            &db, "entry-1", Some(false), Some(true), None, &unread_tx, &bm_tx,
        )
        .await
        .unwrap();

        let row = db.get_entry("entry-1").await.unwrap().unwrap();
        assert!(!row.is_unread && row.is_bookmarked);
        assert!(!row.tags.contains(&"unread".to_string()));
        assert!(row.tags.contains(&"bookmark".to_string()));
        assert!(row.vector.is_some(), "vector preserved");
        assert_eq!(row.topics, vec!["topic-1".to_string()], "topics preserved");
        assert_eq!(unread_rx.len(), 1, "is_unread changed → one event");
        assert_eq!(bm_rx.len(), 1, "is_bookmarked changed → one event");

        // A no-op patch (same value) emits nothing.
        crate::embeddings::pipeline::apply_state(
            &db, "entry-1", Some(false), None, None, &unread_tx, &bm_tx,
        )
        .await
        .unwrap();
        assert_eq!(unread_rx.len(), 1, "no event when unchanged");
    }

    /// The reconcile is member-type-agnostic: an annotation-kind member gets its `topics`
    /// updated exactly like an entry (covers W2 — callers now pass annotation members too).
    #[tokio::test]
    async fn reconcile_handles_annotation_members() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut topic = sample("topic-a", "topic", None);
        topic.topics = vec![];
        db.upsert(topic).await.unwrap();
        let mut anno = sample("anno-1", "annotation", None);
        anno.topics = vec![];
        db.upsert(anno).await.unwrap();

        crate::embeddings::pipeline::reconcile_topic_membership(&db, "topic-a", &["anno-1".to_string()])
            .await
            .unwrap();

        let got = db.get_entry("anno-1").await.unwrap().expect("annotation row present");
        assert_eq!(got.kind, Kind::Annotation);
        assert!(got.topics.contains(&"topic-a".to_string()), "annotation member gets the topic");
    }

    /// A membership change larger than the chunk size is applied across multiple chunks: every
    /// member ends in the right state and the topic-row `items` base tracks the propagated set,
    /// for both the add and the remove direction.
    #[tokio::test]
    async fn reconcile_chunked_multi_chunk_add_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut topic = sample("topic-1", "topic", None);
        topic.topics = vec![];
        db.upsert(topic).await.unwrap();
        let members: Vec<String> = (0..5).map(|i| format!("entry-{i}")).collect();
        for id in &members {
            let mut e = sample(id, "entry", None);
            e.topics = vec![];
            db.upsert(e).await.unwrap();
        }

        // Add all 5 with chunk size 2 -> 3 chunks.
        crate::embeddings::pipeline::reconcile_topic_membership_chunked(&db, "topic-1", &members, 2)
            .await
            .unwrap();
        for id in &members {
            assert!(
                db.get_entry(id).await.unwrap().unwrap().topics.contains(&"topic-1".to_string()),
                "{id} tagged after multi-chunk add"
            );
        }
        let mut base = db.get_entry("topic-1").await.unwrap().unwrap().items;
        base.sort();
        assert_eq!(base, members, "base = all members after multi-chunk add");

        // Drop 3, keep 2 (removals span 2 chunks at chunk size 2).
        let keep = vec!["entry-0".to_string(), "entry-1".to_string()];
        crate::embeddings::pipeline::reconcile_topic_membership_chunked(&db, "topic-1", &keep, 2)
            .await
            .unwrap();
        for id in &keep {
            assert!(db.get_entry(id).await.unwrap().unwrap().topics.contains(&"topic-1".to_string()));
        }
        for id in ["entry-2", "entry-3", "entry-4"] {
            assert!(
                !db.get_entry(id).await.unwrap().unwrap().topics.contains(&"topic-1".to_string()),
                "{id} untagged after multi-chunk remove"
            );
        }
        let mut base = db.get_entry("topic-1").await.unwrap().unwrap().items;
        base.sort();
        assert_eq!(base, keep, "base = kept members after multi-chunk remove");
    }

    /// Simulates a crash mid-fan-out: some members already carry the topic and the base is only
    /// partially advanced. A re-run converges (all members tagged, base complete) and the
    /// already-correct members are not double-tagged (the per-member idempotency skip).
    #[tokio::test]
    async fn reconcile_chunked_resumes_from_partial_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let members: Vec<String> = (0..4).map(|i| format!("entry-{i}")).collect();
        for id in &members {
            let mut e = sample(id, "entry", None);
            // Pretend chunk 1 (entry-0, entry-1) was written before the crash.
            e.topics = if id == "entry-0" || id == "entry-1" {
                vec!["topic-1".to_string()]
            } else {
                vec![]
            };
            db.upsert(e).await.unwrap();
        }
        // Base advanced for only entry-0 (crash between entry-1's add and its base checkpoint).
        let mut topic = sample("topic-1", "topic", None);
        topic.topics = vec![];
        topic.items = vec!["entry-0".to_string()];
        db.upsert(topic).await.unwrap();

        // Re-run with the full membership, chunk size 2.
        crate::embeddings::pipeline::reconcile_topic_membership_chunked(&db, "topic-1", &members, 2)
            .await
            .unwrap();

        for id in &members {
            let topics = db.get_entry(id).await.unwrap().unwrap().topics;
            assert_eq!(
                topics.iter().filter(|t| t.as_str() == "topic-1").count(),
                1,
                "{id} ends tagged exactly once (no duplicate from the re-run)"
            );
        }
        let mut base = db.get_entry("topic-1").await.unwrap().unwrap().items;
        base.sort();
        assert_eq!(base, members, "base converges to the full membership");
    }

    /// Null-vector rows must be excluded from both similarity paths: the `nearest_to`
    /// ANN branch (via `vector IS NOT NULL`) and the full-scan branch (via `is_null`).
    /// Also verifies lancedb 0.27 accepts `vector IS NOT NULL` on a FixedSizeList column.
    #[tokio::test]
    async fn null_vector_rows_excluded_from_similarity() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        db.upsert(sample("embedded", "entry", Some(vec![1.0f32; EMBEDDING_DIM as usize])))
            .await
            .unwrap();
        db.upsert(sample("pending", "entry", None)).await.unwrap();

        // ANN path: nearest_to + `vector IS NOT NULL`.
        let ann = db
            .find_by_vector(vec![1.0f32; EMBEDDING_DIM as usize], None, None, None)
            .await
            .unwrap();
        let ann_ids: Vec<&str> = ann.iter().map(|e| e.entry.id.as_str()).collect();
        assert!(ann_ids.contains(&"embedded"), "embedded row should match");
        assert!(
            !ann_ids.contains(&"pending"),
            "null-vector row must be excluded from ANN search"
        );

        // Full-scan path: max_similarity set routes through full_scan_scored.
        let scan = db
            .find_by_vector(vec![1.0f32; EMBEDDING_DIM as usize], None, Some(-1.0), Some(1.0))
            .await
            .unwrap();
        let scan_ids: Vec<&str> = scan.iter().map(|e| e.entry.id.as_str()).collect();
        assert!(
            !scan_ids.contains(&"pending"),
            "null-vector row must be excluded from full scan"
        );
    }

    /// `ensure_indexes` builds the scalar indexes, is safe to call twice, and leaves
    /// id lookups and similarity queries working.
    #[tokio::test]
    async fn ensure_indexes_idempotent_and_queryable() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        db.upsert(sample("a", "entry", Some(vec![1.0f32; EMBEDDING_DIM as usize])))
            .await
            .unwrap();
        db.upsert(sample("b", "entry", Some(vec![0.2f32; EMBEDDING_DIM as usize])))
            .await
            .unwrap();

        db.ensure_indexes().await.unwrap();
        // Second call is a no-op (indexes already present).
        db.ensure_indexes().await.unwrap();

        assert!(db.get_entry("a").await.unwrap().is_some());
        let related = db
            .find_by_vector(vec![1.0f32; EMBEDDING_DIM as usize], None, None, None)
            .await
            .unwrap();
        assert!(related.iter().any(|e| e.entry.id == "a"));
    }

    /// The list-read API returns only the right rows: all_entries excludes feed and
    /// annotation rows, bookmarked_entries filters on state, entries_by_ids fetches a
    /// set, and projected reads still carry summary + state.
    #[tokio::test]
    async fn read_api_filters_by_kind_and_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut e1 = sample("e1", "entry", Some(vec![0.1f32; EMBEDDING_DIM as usize]));
        e1.is_bookmarked = true;
        e1.tags = vec!["bookmark".to_string()];
        e1.summary = "s1".to_string();
        let e2 = sample("e2", "entry", Some(vec![0.2f32; EMBEDDING_DIM as usize]));
        let feed = sample("feed1", "feed", Some(vec![0.3f32; EMBEDDING_DIM as usize]));
        let mut ann = sample("ann1", "annotation", Some(vec![0.4f32; EMBEDDING_DIM as usize]));
        ann.reference = "e1".to_string();

        db.upsert(e1).await.unwrap();
        db.upsert(e2).await.unwrap();
        db.upsert(feed).await.unwrap();
        db.upsert(ann).await.unwrap();

        let mut all: Vec<String> =
            db.all_entries().await.unwrap().into_iter().map(|e| e.id).collect();
        all.sort();
        assert_eq!(all, vec!["e1".to_string(), "e2".to_string()]);

        let bm: Vec<String> = db
            .bookmarked_entries()
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(bm, vec!["e1".to_string()]);

        let by_id: Vec<String> = db
            .entries_by_ids(&["e2".to_string()])
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(by_id, vec!["e2".to_string()]);

        let e1_read = db.entries_by_ids(&["e1".to_string()]).await.unwrap();
        assert_eq!(e1_read[0].description.as_deref(), Some("s1"));
        assert!(e1_read[0].is_bookmarked);
    }

    /// `embedding_status` counts per kind and distinguishes embedded from pending
    /// (null-vector) rows; kinds with no rows are omitted.
    #[tokio::test]
    async fn embedding_status_counts_by_kind_and_vector() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        db.upsert(sample("e1", "entry", Some(vec![0.1f32; EMBEDDING_DIM as usize])))
            .await
            .unwrap();
        db.upsert(sample("e2", "entry", None)).await.unwrap(); // pending
        db.upsert(sample("f1", "feed", Some(vec![0.2f32; EMBEDDING_DIM as usize])))
            .await
            .unwrap();

        let status = db.embedding_status().await.unwrap();

        let entry = status.iter().find(|s| s.kind == "entry").expect("entry kind");
        assert_eq!(entry.total, 2);
        assert_eq!(entry.embedded, 1);

        let feed = status.iter().find(|s| s.kind == "feed").expect("feed kind");
        assert_eq!(feed.total, 1);
        assert_eq!(feed.embedded, 1);

        assert!(status.iter().all(|s| s.kind != "annotation" && s.kind != "topic"));
    }

    /// `add_batch` inserts many rows in one call, round-tripping mixed Some/None
    /// vectors, and the null-vector rows stay excluded from similarity.
    #[tokio::test]
    async fn add_batch_inserts_many_rows_with_mixed_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        db.add_batch(vec![
            sample("a", "entry", Some(vec![0.1f32; EMBEDDING_DIM as usize])),
            sample("b", "entry", None),
            sample("c", "topic", Some(vec![0.3f32; EMBEDDING_DIM as usize])),
        ])
        .await
        .unwrap();

        assert_eq!(db.count().await.unwrap(), 3);
        assert!(db.get_entry("a").await.unwrap().unwrap().vector.is_some());
        assert!(db.get_entry("b").await.unwrap().unwrap().vector.is_none());
        assert_eq!(db.get_entry("c").await.unwrap().unwrap().kind, Kind::Topic);

        let related = db
            .find_by_vector(vec![0.1f32; EMBEDDING_DIM as usize], None, None, None)
            .await
            .unwrap();
        assert!(related.iter().any(|e| e.entry.id == "a"));
        assert!(!related.iter().any(|e| e.entry.id == "b"));
    }

    /// Ground-truth for the read-side cutover: does a second handle (e.g. AppState's
    /// read handle) see writes made by the service's handle? Establishes whether a
    /// handle is version-pinned (needs `reload`) or auto-tracks latest.
    #[tokio::test]
    async fn read_handle_visibility_across_connections() {
        let dir = tempfile::tempdir().unwrap();
        let writer = EmbeddingsDb::open(dir.path()).await.unwrap();
        let reader = EmbeddingsDb::open(dir.path()).await.unwrap();

        writer
            .add_batch(vec![sample("x", "entry", None)])
            .await
            .unwrap();

        let before = reader.count().await.unwrap();
        reader.reload().await.unwrap();
        let after = reader.count().await.unwrap();

        // The guarantee the cutover relies on: after reload, the reader sees the commit.
        assert_eq!(after, 1, "reader must see writer's commit after reload");
        // Baseline hypothesis: a handle is pinned until reloaded. If this assert fails,
        // handles auto-track latest and `reload` is unnecessary — adjust accordingly.
        assert_eq!(before, 0, "handle is pinned to its open-version until reloaded");
    }
}
