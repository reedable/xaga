use super::*;

/// Cosine distance threshold used when the caller does not specify one.
/// Cosine distance = 1 − cosine_similarity; range [0, 1] for unit vectors.
/// 0.5 corresponds to cosine similarity ≥ 0.5.
const DEFAULT_MAX_COSINE_DISTANCE: f32 = 0.5;

/// Converts `min_similarity` / `max_similarity` into a `(lower, upper)` cosine
/// distance range for use with `distance_range(lower, upper)`.
///
/// - `min_similarity` → upper bound on distance (entities must be *at most* this far)
/// - `max_similarity` → lower bound on distance (entities must be *at least* this far)
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

impl EmbeddingsDb {
    /// Full-scan similarity search returning scores alongside entries.
    ///
    /// ANN (`nearest_to`) cannot efficiently find entities at distance ≥ N because
    /// it optimizes for the *closest* neighbors. When `max_similarity` is set we
    /// fall back to a full table scan, compute cosine similarities in Rust, and
    /// filter/sort manually.
    ///
    /// Results are returned sorted by similarity descending (most similar first).
    async fn search_scored(
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
                    scored.push((
                        similarity,
                        cols.summary_at(i),
                        cols.kind_at(i),
                        cols.reference_at(i),
                        cols.color_at(i),
                    ));
                }
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(n) = limit {
            scored.truncate(n);
        }

        Ok(scored)
    }

    async fn search_by_similarity(
        &self,
        query_vector: &[f32],
        exclude_ids: &HashSet<String>,
        min_similarity: f32,
        max_similarity: f32,
        limit: Option<usize>,
        row_kind: Option<&Kind>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        Ok(self
            .search_scored(
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
                if !color.is_empty() {
                    entry.color = Some(color);
                }

                EmbeddedEntry {
                    entry,
                    kind,
                    similarity,
                    reference,
                }
            })
            .collect())
    }

    /// Samples up to `sample_size` entities from the corpus and returns every
    /// pairwise cosine similarity ≥ `min_similarity` between each sample entity
    /// and all other entities in the database.
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

        let mut all_entities: Vec<Vec<f32>> = Vec::new();

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

                all_entities.push(vec_values);
            }
        }

        let n = all_entities.len();

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
        let norms: Vec<f32> = all_entities
            .iter()
            .map(|v| v.iter().map(|x| x * x).sum::<f32>().sqrt())
            .collect();

        let mut scores: Vec<f32> = Vec::new();

        for &si in &sample_indices {
            let query_norm = norms[si];

            if query_norm == 0.0 {
                continue;
            }

            let qv = &all_entities[si];

            for j in 0..n {
                if j == si {
                    continue;
                }

                let vec_norm = norms[j];

                if vec_norm == 0.0 {
                    continue;
                }

                let tv = &all_entities[j];
                let dot: f32 = qv.iter().zip(tv.iter()).map(|(a, b)| a * b).sum();
                let sim = dot / (query_norm * vec_norm);

                if sim >= min_similarity {
                    scores.push(sim);
                }
            }
        }

        Ok(scores)
    }

    /// Returns entities whose vectors are most similar to the entity identified by
    /// `id`, excluding that entity itself.
    ///
    /// - `limit`: cap on results; `None` returns all matches above the threshold.
    /// - `min_similarity`: cosine similarity floor in [0, 1]; `None` uses
    ///   `DEFAULT_MAX_COSINE_DISTANCE` (0.5 similarity).
    pub(crate) async fn search_related(
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

        // 2. Search, excluding the entity itself.
        if max_similarity.is_some() {
            let exclude = HashSet::from([id.to_string()]);

            return self
                .search_by_similarity(
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
                Some(k) => format!(
                    "id != '{escaped}' AND {} AND vector IS NOT NULL",
                    k.predicate()
                ),
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

                if !color.is_empty() {
                    entry.color = Some(color);
                }

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

    /// Returns entities whose vectors are most similar to the given query vector.
    /// Unlike `search_related`, no ID lookup is performed and nothing is excluded.
    pub(crate) async fn search_by_vector(
        &self,
        query_vector: Vec<f32>,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
        row_kind: Option<Kind>,
    ) -> Result<Vec<EmbeddedEntry>, EmbeddingsError> {
        if max_similarity.is_some() {
            let exclude = HashSet::new();

            return self
                .search_by_similarity(
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
            .only_if(match row_kind {
                Some(kind) => format!("{} AND vector IS NOT NULL", kind.predicate()),
                None => "vector IS NOT NULL".into(),
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

                if !color.is_empty() {
                    entry.color = Some(color);
                }

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
