use super::*;

impl EmbeddingsDb {
    pub(crate) async fn resolve_tag_names(
        &self,
        names: &[String],
    ) -> Result<Vec<String>, EmbeddingsError> {
        self.tag_resolver
            .resolve(names)
            .await
            .map_err(|e| std::io::Error::other(e).into())
    }

    /// Queued snapshots may carry identities retired since their producer resolved them.
    pub(crate) async fn retain_current_tag_ids(
        &self,
        ids: &mut Vec<String>,
    ) -> Result<Vec<String>, EmbeddingsError> {
        self.tag_resolver
            .retain_ids(ids)
            .await
            .map_err(|e| std::io::Error::other(e).into())
    }

    /// The set of entry ids currently marked unread — read via the `is_unread` Bitmap index
    /// (no full scan), selecting only `id`. Used for sidebar unread counts.
    pub async fn unread_ids(&self) -> Result<HashSet<String>, EmbeddingsError> {
        self.state_ids("is_unread = true").await
    }

    /// Entry pin identities, also used to detect pin-only drift during recovery.
    pub(crate) async fn pinned_ids(&self, kind: Kind) -> Result<HashSet<String>, EmbeddingsError> {
        self.ids_matching(&format!("kind = '{}' AND is_pinned = true", kind.as_str()))
            .await
    }

    pub(crate) async fn feed_dates(
        &self,
    ) -> Result<HashMap<String, (String, String)>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(Kind::Feed.predicate())
            .select(Select::columns(&["id", "created_at", "updated_at"]))
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        let mut dates = HashMap::new();
        for batch in &batches {
            let column = |name| {
                batch
                    .column_by_name(name)
                    .and_then(|array| array.as_any().downcast_ref::<StringArray>())
                    .ok_or_else(|| {
                        EmbeddingsError::LanceDb(format!("invalid feed date column: {name}"))
                    })
            };
            let (ids, created, updated) =
                (column("id")?, column("created_at")?, column("updated_at")?);
            for index in 0..batch.num_rows() {
                dates.insert(
                    ids.value(index).to_owned(),
                    (
                        created.value(index).to_owned(),
                        updated.value(index).to_owned(),
                    ),
                );
            }
        }
        Ok(dates)
    }

    /// Pinned article summaries, without reading vectors.
    #[cfg(test)]
    pub(crate) async fn filter_pinned_entries(&self) -> Result<Vec<Entry>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if("kind = 'entry' AND is_pinned = true")
            .execute()
            .await?
            .try_collect()
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;
        Ok(entries_from_batches(&batches))
    }

    /// The set of entry ids currently bookmarked — read via the `color` Bitmap index.
    pub async fn get_bookmarked_ids(&self) -> Result<HashSet<String>, EmbeddingsError> {
        self.state_ids("color != ''").await
    }

    /// Projects entry-row ids matching a state `predicate` into a set, selecting only the `id`
    /// column so the read never touches the vector.
    async fn state_ids(&self, predicate: &str) -> Result<HashSet<String>, EmbeddingsError> {
        self.ids_matching(&format!("kind = 'entry' AND {predicate}"))
            .await
    }

    async fn ids_matching(&self, predicate: &str) -> Result<HashSet<String>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(predicate)
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
    pub(super) const SUMMARY_COLUMNS: [&'static str; 13] = [
        "id",
        "kind",
        "source",
        "title",
        "created_at",
        "published_at",
        "updated_at",
        "reference",
        "color",
        "summary",
        "is_unread",
        "is_pinned",
        "tags",
    ];

    /// Fetches entry rows for the given ids (e.g. a feed's atom-supplied id set),
    /// served by the scalar `id` index. Order is not guaranteed; callers sort.
    pub async fn list_entries_by_ids(&self, ids: &[String]) -> Result<Vec<Entry>, EmbeddingsError> {
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

    /// Mixed list membership is evaluated by indexed kind/color predicates in LanceDB.
    pub(crate) async fn filter_entities(
        &self,
        filter: crate::types::EntityFilter,
    ) -> Result<Vec<crate::types::ListEntity>, EmbeddingsError> {
        use crate::types::EntityFilter;
        let predicate = match filter {
            EntityFilter::All => "kind IN ('entry', 'annotation')",
            EntityFilter::Unread => "kind = 'entry' AND is_unread = true",
            EntityFilter::Bookmarked => "kind = 'entry' AND color != ''",
            EntityFilter::Saved => "(kind = 'entry' AND color != '') OR kind = 'annotation'",
        };
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if(predicate)
            .execute()
            .await?
            .try_collect()
            .await?;
        Ok(list_entities_from_batches(&batches))
    }

    pub(super) fn tag_predicate(
        ids: &[String],
        filter: crate::types::EntityFilter,
    ) -> Result<String, EmbeddingsError> {
        Self::tag_predicate_for(ids, filter, Some(Kind::Entry))
    }

    pub(super) fn tag_predicate_for(
        ids: &[String],
        filter: crate::types::EntityFilter,
        kind: Option<Kind>,
    ) -> Result<String, EmbeddingsError> {
        let mut predicate = kind
            .map(|kind| kind.predicate().to_owned())
            .unwrap_or_else(|| "kind IN ('entry', 'annotation')".into());
        let mut labels = Vec::new();
        for id in ids {
            let parsed = uuid::Uuid::parse_str(id)
                .map_err(|_| EmbeddingsError::Parse("invalid tag ID".into()))?;
            if parsed.get_version_num() != 4 || parsed.to_string() != *id {
                return Err(EmbeddingsError::Parse("invalid tag ID".into()));
            }
            labels.push(format!("'{id}'"));
        }
        if !labels.is_empty() {
            predicate.push_str(&format!(" AND array_has_all(tags, [{}])", labels.join(",")));
        }
        match filter {
            crate::types::EntityFilter::Unread => {
                predicate.push_str(" AND kind = 'entry' AND is_unread = true")
            }
            crate::types::EntityFilter::Saved if kind.is_none() => {
                predicate.push_str(" AND (kind = 'annotation' OR color != '')")
            }
            crate::types::EntityFilter::Bookmarked => {
                predicate.push_str(" AND kind = 'entry' AND color != ''")
            }
            crate::types::EntityFilter::Saved => predicate.push_str(" AND color != ''"),
            crate::types::EntityFilter::All => {}
        }
        Ok(predicate)
    }

    /// Direct article and annotation assignments matching every selected identity.
    pub async fn list_tag_entities(
        &self,
        ids: &[String],
        filter: crate::types::EntityFilter,
    ) -> Result<Vec<crate::types::ListEntity>, EmbeddingsError> {
        self.list_tag_entity_groups(
            &ids.iter().map(|id| vec![id.clone()]).collect::<Vec<_>>(),
            filter,
        )
        .await
    }

    /// Intersect selections, accepting either identity of an active merge per group.
    pub async fn list_tag_entity_groups(
        &self,
        groups: &[Vec<String>],
        filter: crate::types::EntityFilter,
    ) -> Result<Vec<crate::types::ListEntity>, EmbeddingsError> {
        let filter = if !groups.is_empty() && filter == crate::types::EntityFilter::Unread {
            crate::types::EntityFilter::All
        } else {
            filter
        };
        let predicate = Self::tag_group_predicate_for(groups, filter, None)?;
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if(predicate)
            .execute()
            .await?
            .try_collect()
            .await?;
        let mut entries = list_entities_from_batches(&batches);
        crate::query::filter::sort_entities(&mut entries);
        Ok(entries)
    }

    /// Bounded ordered candidate IDs for resumable rename; assignments keep their IDs.
    pub(crate) async fn tag_entry_id_batch(
        &self,
        tag_id: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, EmbeddingsError> {
        self.tag_entry_id_batch_any(&[tag_id.into()], after, limit)
            .await
    }

    pub(crate) async fn tag_entry_id_batch_any(
        &self,
        tag_ids: &[String],
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, EmbeddingsError> {
        self.reload().await?;
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut parents = Vec::new();
        // Candidate rows and retained parent IDs are both bounded.
        for (kind, column) in [(Kind::Entry, "id"), (Kind::Annotation, "reference")] {
            let alternatives = tag_ids
                .iter()
                .map(|id| {
                    Self::tag_predicate_for(
                        std::slice::from_ref(id),
                        crate::types::EntityFilter::All,
                        Some(kind),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut predicate = format!("(({}))", alternatives.join(") OR ("));
            if let Some(after) = after {
                predicate.push_str(&format!(" AND {column} > '{}'", after.replace('\'', "''")));
            }
            let batches: Vec<RecordBatch> = self
                .table
                .query()
                .select(Select::columns(&[column]))
                .only_if(predicate)
                .order_by(Some(vec![lancedb::query::ColumnOrdering {
                    column_name: column.into(),
                    ascending: true,
                    nulls_first: false,
                }]))
                .limit(limit)
                .execute()
                .await?
                .try_collect()
                .await?;
            for batch in batches {
                let values = batch
                    .column_by_name(column)
                    .unwrap()
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap();
                for index in 0..batch.num_rows() {
                    parents.push(values.value(index).to_owned());
                }
            }
        }
        parents.sort();
        parents.dedup();
        parents.truncate(limit);
        Ok(parents)
    }

    pub(crate) async fn count_tag_assignments(&self, id: &str) -> Result<u64, EmbeddingsError> {
        let predicate =
            Self::tag_predicate_for(&[id.into()], crate::types::EntityFilter::All, None)?;
        Ok(self.table.count_rows(Some(predicate)).await? as u64)
    }

    fn tag_group_predicate(
        groups: &[Vec<String>],
        filter: crate::types::EntityFilter,
    ) -> Result<String, EmbeddingsError> {
        Self::tag_group_predicate_for(groups, filter, Some(Kind::Entry))
    }

    pub(super) fn tag_group_predicate_for(
        groups: &[Vec<String>],
        filter: crate::types::EntityFilter,
        kind: Option<Kind>,
    ) -> Result<String, EmbeddingsError> {
        let mut predicate = Self::tag_predicate_for(&[], filter, kind)?;
        for group in groups {
            if group.is_empty() {
                return Err(EmbeddingsError::Parse("empty tag identity group".into()));
            }
            let alternatives = group
                .iter()
                .map(|id| {
                    Self::tag_predicate_for(
                        std::slice::from_ref(id),
                        crate::types::EntityFilter::All,
                        kind,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            predicate.push_str(&format!(" AND (({}))", alternatives.join(") OR (")));
        }
        Ok(predicate)
    }

    pub async fn count_tag_group_unread(
        &self,
        groups: &[Vec<String>],
    ) -> Result<usize, EmbeddingsError> {
        Ok(self
            .table
            .count_rows(Some(Self::tag_group_predicate(
                groups,
                crate::types::EntityFilter::Unread,
            )?))
            .await?)
    }

    /// Count the intersection once, independently of list/text filters.
    pub async fn count_tag_unread(&self, ids: &[String]) -> Result<usize, EmbeddingsError> {
        let predicate = Self::tag_predicate(ids, crate::types::EntityFilter::Unread)?;
        Ok(self.table.count_rows(Some(predicate)).await?)
    }

    /// All entry rows (`kind = 'entry'`).
    pub async fn list_entries(&self) -> Result<Vec<Entry>, EmbeddingsError> {
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
    pub async fn filter_bookmarked(&self) -> Result<Vec<Entry>, EmbeddingsError> {
        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&Self::SUMMARY_COLUMNS))
            .only_if(format!("{} AND color != ''", Kind::Entry.predicate()))
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
    pub async fn get_embedding_status(&self) -> Result<Vec<KindStatus>, EmbeddingsError> {
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

    /// Returns the total number of embedded documents.
    pub async fn count(&self) -> Result<usize, EmbeddingsError> {
        let n = self
            .table
            .count_rows(None)
            .await
            .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        Ok(n)
    }

    /// Counts the rows shown by All Entities and the unread entry subset without
    /// materializing rows or vectors.
    pub async fn all_entities_counts(&self) -> Result<AllEntitiesCounts, EmbeddingsError> {
        let (entry_count, unread_count) = tokio::try_join!(
            self.table
                .count_rows(Some("kind = 'entry' OR kind = 'annotation'".to_string())),
            self.table.count_rows(Some("is_unread = true".to_string())),
        )
        .map_err(|e| EmbeddingsError::LanceDb(e.to_string()))?;

        Ok(AllEntitiesCounts {
            entry_count,
            unread_count,
        })
    }

    /// Returns a map of `id → (checksum, reference, has_vector, kind)` for every row.
    /// `reference` is non-empty only for annotation rows (it holds the parent article ID).
    /// `has_vector` is false for rows inserted before their embedding was computed.
    /// `kind` is the row-kind discriminator ("entry" | "annotation" | "topic" | "feed").
    /// Used by recovery to determine which files need (re-)embedding.
    /// Returns `id → (checksum, reference, has_vector, kind, entities, is_unread)` for every row, in one full scan.
    /// `entities` is the row's membership list — for entry rows, the recorded annotation membership that
    /// recovery diffs against the `.md` to gate the annotation reconcile without a per-entry query.
    pub(crate) async fn list_entries_meta(
        &self,
    ) -> Result<
        HashMap<
            String,
            (
                String,
                String,
                bool,
                String,
                Vec<String>,
                bool,
                String,
                Vec<String>,
            ),
        >,
        EmbeddingsError,
    > {
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

            let colors = batch
                .column_by_name("color")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let unread = batch
                .column_by_name("is_unread")
                .and_then(|c| c.as_any().downcast_ref::<BooleanArray>());
            let tags = batch
                .column_by_name("tags")
                .and_then(|c| c.as_any().downcast_ref::<ListArray>());
            let entities = batch
                .column_by_name("items")
                .and_then(|c| c.as_any().downcast_ref::<ListArray>());

            if let (Some(ids), Some(checksums)) = (ids, checksums) {
                for i in 0..batch.num_rows() {
                    let reference = references.map_or(String::new(), |a| a.value(i).to_string());
                    let has_vector = vectors.map_or(false, |v| !v.is_null(i));
                    let kind = kinds.map_or(String::new(), |a| a.value(i).to_string());
                    let entity_ids = entities
                        .filter(|a| !a.is_null(i))
                        .map(|a| {
                            let row = a.value(i);
                            row.as_any()
                                .downcast_ref::<StringArray>()
                                .map(|s| (0..s.len()).map(|j| s.value(j).to_string()).collect())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    map.insert(
                        ids.value(i).to_string(),
                        (
                            checksums.value(i).to_string(),
                            reference,
                            has_vector,
                            kind,
                            entity_ids,
                            unread.is_some_and(|a| a.value(i)),
                            colors.map_or(String::new(), |a| a.value(i).to_owned()),
                            tags.filter(|a| !a.is_null(i))
                                .map(|a| {
                                    let values = a.value(i);
                                    values
                                        .as_any()
                                        .downcast_ref::<StringArray>()
                                        .map(|s| {
                                            (0..s.len()).map(|j| s.value(j).to_owned()).collect()
                                        })
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default(),
                        ),
                    );
                }
            }
        }

        Ok(map)
    }

    /// Returns a map of `id → checksum` for all annotation rows belonging to `article_id`.
    /// Returns `(checksum, color, tags)` per annotation id for the given article.
    pub(crate) async fn get_annotation_meta_for_article(
        &self,
        article_id: &str,
    ) -> Result<HashMap<String, (String, String, Vec<String>)>, EmbeddingsError> {
        let escaped = article_id.replace('\'', "''");

        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .select(Select::columns(&["id", "checksum", "color", "tags"]))
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
                        (
                            checksums.value(i).to_string(),
                            colors.value(i).to_string(),
                            {
                                let tags = batch
                                    .column_by_name("tags")
                                    .unwrap()
                                    .as_any()
                                    .downcast_ref::<ListArray>()
                                    .unwrap()
                                    .value(i);
                                let tags = tags.as_any().downcast_ref::<StringArray>().unwrap();
                                (0..tags.len()).map(|i| tags.value(i).to_owned()).collect()
                            },
                        ),
                    );
                }
            }
        }

        Ok(map)
    }

    /// Returns a fully populated `EmbeddingEntry` for `id` so the caller can
    /// mutate metadata (e.g. tags) and upsert without re-running the embedder.
    /// Returns `None` if the entry is not in the database.
    pub(crate) async fn get_entry(
        &self,
        id: &str,
    ) -> Result<Option<EmbeddingEntry>, EmbeddingsError> {
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
            if batch.num_rows() == 0 {
                continue;
            }

            let get_str = |name: &str| -> String {
                batch
                    .column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(0).to_string())
                    .unwrap_or_default()
            };

            let read_list = |name: &str| -> Vec<String> {
                batch
                    .column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<ListArray>())
                    .map(|arr| {
                        let entity = arr.value(0);
                        entity
                            .as_any()
                            .downcast_ref::<StringArray>()
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
                entities: read_list("items"),
                is_unread: get_bool("is_unread"),
                is_pinned: get_bool("is_pinned"),
                vector,
            }));
        }

        Ok(None)
    }

    /// Finds the topic ids recorded on an entity row without reading its embedding vector.
    pub async fn find_topic_ids_by_entity_id(
        &self,
        entity_id: &str,
    ) -> Result<Option<Vec<String>>, EmbeddingsError> {
        self.find_string_list_by_id(entity_id, "topics").await
    }

    /// Finds the entity ids recorded on a topic row without reading its embedding vector.
    pub(crate) async fn find_entity_ids_by_topic_id(
        &self,
        topic_id: &str,
    ) -> Result<Option<Vec<String>>, EmbeddingsError> {
        self.find_string_list_by_id(topic_id, "items").await
    }

    async fn find_string_list_by_id(
        &self,
        id: &str,
        column: &str,
    ) -> Result<Option<Vec<String>>, EmbeddingsError> {
        let Some(batch) = self.find_batch_by_id(id, &[column]).await? else {
            return Ok(None);
        };

        let values = batch
            .column_by_name(column)
            .and_then(|array| array.as_any().downcast_ref::<ListArray>())
            .filter(|array| !array.is_null(0))
            .map(|array| {
                let values = array.value(0);
                values
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .map(|strings| {
                        (0..strings.len())
                            .map(|index| strings.value(index).to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        Ok(Some(values))
    }

    pub(super) async fn find_batch_by_id(
        &self,
        id: &str,
        columns: &[&str],
    ) -> Result<Option<RecordBatch>, EmbeddingsError> {
        let escaped = id.replace('\'', "''");
        let mut batches = self
            .table
            .query()
            .select(Select::columns(columns))
            .only_if(format!("id = '{escaped}'"))
            .limit(1)
            .execute()
            .await?
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|error| EmbeddingsError::LanceDb(error.to_string()))?;

        Ok(batches.pop().filter(|batch| batch.num_rows() > 0))
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
}
