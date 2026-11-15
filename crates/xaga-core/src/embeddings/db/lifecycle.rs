use super::*;

impl EmbeddingsDb {
    /// Bulk CLI maintenance uses the same compaction/pruning as the service,
    /// then catches scalar indexes up with newly written fragments.
    pub(crate) async fn maintain(&self) -> Result<(), EmbeddingsError> {
        self.reload().await?;
        if self.count().await? == 0 {
            return Ok(());
        }
        self.compact().await?;
        self.ensure_indexes().await?;
        self.table
            .optimize(lancedb::table::optimize::OptimizeAction::Index(
                Default::default(),
            ))
            .await?;
        self.reload().await
    }

    /// Opens an existing embeddings table without creating or migrating it.
    pub async fn open_readonly(library_dir: &Path) -> Result<Self, EmbeddingsError> {
        let schema = make_schema();
        let db_path = crate::library::index_path(library_dir);
        // Connecting to a missing local database may create its directory.
        // Read-only startup must not initialize a cache for an older library.
        if !tokio::fs::metadata(&db_path).await?.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "embeddings path is not a directory",
            )
            .into());
        }
        let connection = lancedb::connect(db_path.to_str().ok_or_else(|| {
            EmbeddingsError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "non-UTF-8 path",
            ))
        })?)
        .execute()
        .await?;
        let table = connection.open_table("documents").execute().await?;

        Ok(Self {
            tag_resolver: crate::tags::CachedResolver::new(library_dir),
            _connection: connection,
            table,
            schema,
        })
    }

    /// Opens the embeddings table for mutation. Possession of the library lock is required for
    /// creation and schema migration as well as subsequent writes through the returned handle.
    pub async fn open_writer(
        library_dir: &Path,
        _lock: &FileLock,
    ) -> Result<Self, EmbeddingsError> {
        Self::open_mutating(library_dir).await
    }

    /// Test databases are isolated and do not participate in the process-level library lock.
    #[cfg(test)]
    pub(crate) async fn open(library_dir: &Path) -> Result<Self, EmbeddingsError> {
        Self::open_mutating(library_dir).await
    }

    /// Opens the `embeddings.lance` database inside `library_dir`, creating an empty table only
    /// on first run (when it does not exist yet).
    ///
    /// If the table exists but cannot be opened, this returns the open error rather than recreating
    /// it. A transient open failure (corruption, fd exhaustion, an IO hiccup) must never be
    /// "repaired" by overwriting or destroying a populated index — doing so has wiped the entire
    /// embeddings mirror. Clearing a corrupt DB is a separate, explicit operation (the `reset`
    /// command / [`Self::destroy`]).
    async fn open_mutating(library_dir: &Path) -> Result<Self, EmbeddingsError> {
        let schema = make_schema();
        let db_path = crate::library::index_path(library_dir);
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
                        "EmbeddingsDb: adding 'items' column to existing table (metadata-only)"
                    );
                    let entities_schema = Arc::new(Schema::new(vec![Field::new(
                        "items",
                        DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                        true,
                    )]));
                    table
                        .add_columns(
                            lancedb::table::NewColumnTransform::AllNulls(entities_schema),
                            None,
                        )
                        .await?;
                }
                Ok(EmbeddingsDb {
                    tag_resolver: crate::tags::CachedResolver::new(library_dir),
                    _connection: connection,
                    table,
                    schema,
                })
            }
            // First run: the table genuinely does not exist yet — create it empty.
            Err(lancedb::Error::TableNotFound { .. }) => {
                create_empty_documents_table(&connection, &schema).await?;
                let table = connection.open_table("documents").execute().await?;
                Ok(EmbeddingsDb {
                    tag_resolver: crate::tags::CachedResolver::new(library_dir),
                    _connection: connection,
                    table,
                    schema,
                })
            }
            // The table exists but failed to open. Surface the error — never recreate/overwrite,
            // which would destroy a populated index on a merely transient failure.
            Err(e) => Err(e.into()),
        }
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
    pub fn destroy(library_dir: &Path, _lock: &FileLock) -> std::io::Result<()> {
        match std::fs::remove_dir_all(crate::library::index_path(library_dir)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    /// Creates scalar indexes when absent, idempotently: BTree on `id` (id-set retrieval) and
    /// `published_at` (ordered reads); Bitmap on low-cardinality state and kind columns
    /// `is_unread`/`color`/`kind`/`is_pinned` so state filters and counts are index-driven rather
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

        // Bitmap indexes serve the state id-sets, sidebar counts, and Saved membership.
        for col in ["is_unread", "is_pinned", "color", "kind"] {
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
        if !indexed.contains("tags") {
            self.table
                .create_index(
                    &["tags"],
                    lancedb::index::Index::LabelList(Default::default()),
                )
                .execute()
                .await?;
            log::info!("EmbeddingsDb: created LabelList index on `tags`");
        }
        Ok(())
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

        // Synchronously advance this handle — and every clone sharing its inner, including the
        // app's read handle — to the just-compacted version. Reads share this handle and do NOT
        // reload per-read, so this is the single point that makes post-compaction state visible to
        // them. Must stay inside compact() (not a separate async signal) so the version the readers
        // see never lags a prune.
        self.reload().await?;
        log::info!("EmbeddingsDb::compact: read handle reloaded to compacted version");
        Ok(())
    }
}
