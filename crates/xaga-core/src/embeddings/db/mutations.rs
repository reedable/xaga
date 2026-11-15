use super::*;

impl EmbeddingsDb {
    /// Remove one identity from a bounded Entry batch, preserving every other column.
    pub(crate) async fn patch_tag_batch(
        &self,
        tag_id: &str,
        replacement: Option<&str>,
        ids: &[String],
    ) -> Result<(), EmbeddingsError> {
        if ids.is_empty() {
            return Ok(());
        }
        if ids.len() > 1024 {
            return Err(std::io::Error::other("tag batch exceeds 1024 Entries").into());
        }
        let predicate =
            Self::tag_predicate_for(&[tag_id.into()], crate::types::EntityFilter::All, None)?;
        let ids = ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let removed = format!("array_remove(tags, '{tag_id}')");
        let selected = format!("{predicate} AND (id IN ({ids}) OR reference IN ({ids}))");
        if let Some(replacement) = replacement {
            Self::tag_predicate(&[replacement.into()], crate::types::EntityFilter::All)?;
            // Two idempotent subsets avoid unsupported CASE expressions. A crash between
            // updates leaves the remaining losing IDs eligible for the persisted replay.
            let has_survivor = format!("array_has_all(tags, ['{replacement}'])");
            self.table
                .update()
                .only_if(format!("{selected} AND {has_survivor}"))
                .column("tags", removed.clone())
                .execute()
                .await?;
            self.table
                .update()
                .only_if(format!("{selected} AND NOT {has_survivor}"))
                .column("tags", format!("array_append({removed}, '{replacement}')"))
                .execute()
                .await?;
        } else {
            self.table
                .update()
                .only_if(selected)
                .column("tags", removed)
                .execute()
                .await?;
        }
        Ok(())
    }

    /// A merge can invalidate the identity carried by an older assignment delta.
    /// This reconciles that Entry's current source assignments without replacing other columns.
    pub(crate) async fn set_entry_tag_ids(
        &self,
        entry: &str,
        tags: &[String],
    ) -> Result<(), EmbeddingsError> {
        self.set_target_tag_ids(entry, None, tags).await
    }

    pub(crate) async fn set_annotation_tag_ids(
        &self,
        entry: &str,
        annotation: &str,
        tags: &[String],
    ) -> Result<(), EmbeddingsError> {
        self.set_target_tag_ids(annotation, Some(entry), tags).await
    }

    async fn set_target_tag_ids(
        &self,
        id: &str,
        parent: Option<&str>,
        tags: &[String],
    ) -> Result<(), EmbeddingsError> {
        Self::tag_predicate(tags, crate::types::EntityFilter::All)?;
        let value = if tags.is_empty() {
            "arrow_cast([], 'List(Utf8)')".to_owned()
        } else {
            format!(
                "[{}]",
                tags.iter()
                    .map(|id| format!("'{id}'"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let predicate = if let Some(parent) = parent {
            format!(
                "kind = 'annotation' AND reference = '{}' AND id = '{}'",
                parent.replace('\'', "''"),
                id.replace('\'', "''")
            )
        } else {
            format!("kind = 'entry' AND id = '{}'", id.replace('\'', "''"))
        };
        self.table
            .update()
            .only_if(predicate)
            .column("tags", value)
            .execute()
            .await?;
        Ok(())
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
    pub(crate) async fn add_batch(
        &self,
        entries: Vec<EmbeddingEntry>,
    ) -> Result<(), EmbeddingsError> {
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

    /// Deletes the row whose `id` column matches exactly.
    pub(crate) async fn delete_by_id(&self, id: &str) -> Result<(), EmbeddingsError> {
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
}
