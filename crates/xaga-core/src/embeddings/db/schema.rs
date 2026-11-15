use super::*;

pub(super) const EMBEDDING_DIM: i32 = 384;

pub(super) fn make_schema() -> Arc<Schema> {
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

pub(super) fn entry_to_batch(
    entry: EmbeddingEntry,
    schema: &Arc<Schema>,
) -> Result<RecordBatch, EmbeddingsError> {
    entries_to_batch(vec![entry], schema)
}

/// Builds a record batch from many entries: one column per field, one row per entry.
/// A null vector is stored as `EMBEDDING_DIM` placeholder slots marked invalid via the
/// validity bitmap.
pub(super) fn entries_to_batch(
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
    let mut pinned = Vec::with_capacity(n);

    let mut tags_builder = ListBuilder::new(StringBuilder::new());
    let mut topics_builder = ListBuilder::new(StringBuilder::new());
    let mut entities_builder = ListBuilder::new(StringBuilder::new());

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
        pinned.push(entry.is_pinned);

        for tag in &entry.tags {
            tags_builder.values().append_value(tag);
        }
        tags_builder.append(true);

        for topic in &entry.topics {
            topics_builder.values().append_value(topic);
        }
        topics_builder.append(true);

        for entity in &entry.entities {
            entities_builder.values().append_value(entity);
        }
        entities_builder.append(true);

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
            Arc::new(entities_builder.finish()),
            Arc::new(BooleanArray::from(unread)),
            Arc::new(BooleanArray::from(pinned)),
            Arc::new(vector_array),
        ],
    )?)
}

pub(super) async fn create_empty_documents_table(
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
    let empty_entities = ListBuilder::new(StringBuilder::new()).finish();
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
            Arc::new(empty_entities),                        // items
            Arc::new(BooleanArray::from(Vec::<bool>::new())), // is_unread
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
pub(super) struct BatchSummaryColumns<'a> {
    pub(super) ids: &'a StringArray,
    pub(super) sources: &'a StringArray,
    pub(super) titles: &'a StringArray,
    pub(super) created_ats: &'a StringArray,
    published_ats: Option<&'a StringArray>,
    updated_ats: Option<&'a StringArray>,
    references: Option<&'a StringArray>,
    colors: Option<&'a StringArray>,
    summaries: Option<&'a StringArray>,
    unread: &'a BooleanArray,
    pinned: Option<&'a BooleanArray>,
    kinds: Option<&'a StringArray>,
}

impl<'a> BatchSummaryColumns<'a> {
    pub(super) fn from_batch(batch: &'a RecordBatch) -> Option<Self> {
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
            pinned: batch
                .column_by_name("is_pinned")
                .and_then(|c| c.as_any().downcast_ref::<BooleanArray>()),
            unread: batch
                .column_by_name("is_unread")?
                .as_any()
                .downcast_ref::<BooleanArray>()?,
            kinds: batch
                .column_by_name("kind")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>()),
        })
    }

    /// The row's domain kind. Defaults to `Entry` if the column is absent (it is
    /// part of `SUMMARY_COLUMNS` and the full row, so this should not occur).
    pub(super) fn kind_at(&self, i: usize) -> Kind {
        self.kinds
            .and_then(|a| a.value(i).parse().ok())
            .unwrap_or(Kind::Entry)
    }

    pub(super) fn summary_at(&self, i: usize) -> Entry {
        fn opt_str(col: Option<&StringArray>, i: usize) -> Option<String> {
            col.and_then(|a| {
                let v = a.value(i);
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            })
        }
        let is_unread = self.unread.value(i);
        Entry {
            tags: None,
            id: self.ids.value(i).to_string(),
            url: self.sources.value(i).to_string(),
            title: self.titles.value(i).to_string(),
            created_at: self.created_ats.value(i).to_string(),
            published_at: opt_str(self.published_ats, i),
            updated_at: opt_str(self.updated_ats, i),
            description: opt_str(self.summaries, i),
            is_unread,
            is_pinned: self.pinned.is_some_and(|values| values.value(i)),
            color: opt_str(self.colors, i),
        }
    }

    pub(super) fn reference_at(&self, i: usize) -> String {
        self.references
            .map_or(String::new(), |a| a.value(i).to_string())
    }

    pub(super) fn color_at(&self, i: usize) -> String {
        self.colors
            .map_or(String::new(), |a| a.value(i).to_string())
    }
}

/// Builds `Entry` values from query result batches (used by the list-read API).
pub(super) fn entries_from_batches(batches: &[RecordBatch]) -> Vec<Entry> {
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

pub(super) fn list_entities_from_batches(batches: &[RecordBatch]) -> Vec<crate::types::ListEntity> {
    use crate::types::ListEntity;
    let mut entities = Vec::new();
    for batch in batches {
        if let Some(cols) = BatchSummaryColumns::from_batch(batch) {
            for i in 0..batch.num_rows() {
                let entry = cols.summary_at(i);
                entities.push(ListEntity {
                    id: entry.id,
                    title: entry.title,
                    created_at: entry.created_at,
                    url: entry.url,
                    is_unread: entry.is_unread,
                    is_pinned: entry.is_pinned,
                    reference: (cols.kind_at(i) == Kind::Annotation).then(|| cols.reference_at(i)),
                    color: entry.color,
                });
            }
        }
    }
    entities
}
