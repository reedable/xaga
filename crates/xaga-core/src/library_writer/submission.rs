use super::*;

impl LibraryWriter {
    pub(crate) async fn repair_identity(
        &self,
        path: PathBuf,
        kind: crate::persistence::identity::Record,
    ) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::RepairIdentity { path, kind, resp },
            rx,
        )
        .await
    }

    pub async fn save_pdf(&self, url: String, bytes: Option<Vec<u8>>) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(Tier::Interactive, Op::SavePdf { url, bytes, resp }, rx)
            .await
    }
    async fn submit<T>(
        &self,
        tier: Tier,
        op: Op,
        rx: oneshot::Receiver<Result<T, WriterError>>,
    ) -> Result<T, WriterError> {
        // Fast-fail once shutdown has begun, so a late submit is refused synchronously
        // instead of landing in the channel buffer the dispatcher will no longer drain
        // (a buffered Op holds its responder alive → the caller would await forever).
        if self.shutting_down.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(WriterError::ShuttingDown);
        }
        self.tx
            .send(Msg::Submit(tier, Box::new(op)))
            .map_err(|_| WriterError::Closed)?;
        rx.await.map_err(|_| WriterError::Closed)?
    }

    // ── Write API (mirrors the persistence write API, minus library_dir) ─────────────

    pub(crate) async fn repair_summary(
        &self,
        id: String,
        summary: String,
    ) -> Result<bool, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::RepairSummary { id, summary, resp },
            rx,
        )
        .await
    }

    pub async fn recover_images(
        &self,
        id: String,
        recovery: crate::persistence::image_recovery::ImageRecovery,
        data: std::collections::HashMap<String, String>,
    ) -> Result<Article, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::RecoverImages {
                id,
                recovery,
                data,
                resp,
            },
            rx,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn save_article(
        &self,
        tier: Tier,
        id: String,
        url: String,
        title: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        favicon: Option<String>,
        markdown: String,
        url_data: std::collections::HashMap<String, String>,
    ) -> Result<Article, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::SaveArticle {
            id,
            url,
            title,
            published_at,
            updated_at,
            summary,
            favicon,
            markdown,
            url_data,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    /// `create_article` = `save_article` with the id derived from the URL hash, exactly
    /// as the persistence layer derives it — the id must be known here to key the lane.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_article(
        &self,
        tier: Tier,
        url: String,
        title: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        favicon: Option<String>,
        markdown: String,
        url_data: std::collections::HashMap<String, String>,
    ) -> Result<Article, WriterError> {
        let normalized = crate::util::normalize_url(&url)
            .map_err(|e| WriterError::Entry(EntryError::from(e)))?;
        let id = crate::util::url_hash12(&normalized);
        self.save_article(
            tier,
            id,
            url,
            title,
            published_at,
            updated_at,
            summary,
            favicon,
            markdown,
            url_data,
        )
        .await
    }

    pub async fn add_annotation(
        &self,
        tier: Tier,
        article_id: String,
        annotation: Annotation,
    ) -> Result<Article, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::AddAnnotation {
            article_id,
            annotation,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn replace_annotation(
        &self,
        tier: Tier,
        article_id: String,
        old_id: String,
        annotation: Annotation,
    ) -> Result<Article, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::ReplaceAnnotation {
            article_id,
            old_id,
            annotation,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn remove_annotation(
        &self,
        tier: Tier,
        article_id: String,
        annotation_id: String,
    ) -> Result<Article, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::RemoveAnnotation {
            article_id,
            annotation_id,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn set_annotation_tag(
        &self,
        article_id: String,
        annotation_id: String,
        tag_id: String,
        assigned: bool,
    ) -> Result<crate::tags::AnnotationTagChange, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::SetAnnotationTag {
                article_id,
                annotation_id,
                tag_id,
                assigned,
                resp,
            },
            rx,
        )
        .await
    }

    pub async fn set_entry_tag(
        &self,
        id: String,
        tag_id: String,
        assigned: bool,
    ) -> Result<crate::tags::EntryTagChange, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::SetEntryTag {
                id,
                tag_id,
                assigned,
                resp,
            },
            rx,
        )
        .await
    }

    /// Sets reading state without changing organizational tags.
    pub async fn set_entry_unread(
        &self,
        tier: Tier,
        id: String,
        is_unread: bool,
    ) -> Result<Entry, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            tier,
            Op::SetEntryUnread {
                id,
                is_unread,
                resp,
            },
            rx,
        )
        .await
    }

    pub async fn set_entry_color(
        &self,
        tier: Tier,
        id: String,
        color: Option<String>,
    ) -> Result<Entry, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::SetEntryColor { id, color, resp };
        self.submit(tier, op, rx).await
    }

    /// Bookmark an entry (bookmark-import path): ensure it exists as a read entry and
    /// carries the bookmark color, deciding existing-vs-new inside the entry lane so a
    /// concurrent create cannot cause the marking to be dropped.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn bookmark_entry(
        &self,
        tier: Tier,
        url: String,
        title: String,
        created_at: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        color: Option<String>,
    ) -> Result<BookmarkOutcome, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::BookmarkEntry {
            url,
            title,
            created_at,
            published_at,
            updated_at,
            summary,
            color,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn save_topic(
        &self,
        tier: Tier,
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
        entities: Vec<Reference>,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::SaveTopic {
            id,
            title,
            description,
            threshold,
            entities,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    /// Edit a topic's metadata, preserving its current items (read + write inside the
    /// lane). Use this for UI topic edits; use `save_topic` only when setting an explicit
    /// member references.
    pub async fn edit_topic_meta(
        &self,
        tier: Tier,
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::EditTopicMeta {
            id,
            title,
            description,
            threshold,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    /// `create_topic` = `save_topic` under a fresh UUID, minted here to key the lane.
    pub async fn create_topic(
        &self,
        tier: Tier,
        title: String,
        description: String,
        threshold: Option<f32>,
    ) -> Result<Topic, WriterError> {
        let id = uuid::Uuid::new_v4().to_string();
        self.save_topic(tier, id, title, description, threshold, vec![])
            .await
    }

    /// Creates a topic with one already-persisted member in the same writer operation and
    /// topic-document write. The member is validated before the topic file is created.
    pub async fn create_topic_with_member(
        &self,
        tier: Tier,
        title: String,
        description: String,
        threshold: Option<f32>,
        entity: Reference,
    ) -> Result<Topic, WriterError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (resp, rx) = oneshot::channel();
        let op = Op::CreateTopicWithMember {
            id,
            title,
            description,
            threshold,
            entity,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn delete_topic(&self, tier: Tier, id: String) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::DeleteTopic { id, resp };
        self.submit(tier, op, rx).await
    }

    pub(crate) async fn mutate_tags(
        &self,
        mutation: crate::tags::Mutation,
    ) -> Result<crate::tags::Tags, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::MutateTags {
                mutation,
                resp,
                updates: self.tag_updates.clone(),
                generations: self.rename_generations.clone(),
            },
            rx,
        )
        .await
    }

    pub(crate) async fn wait_for_indexed_renames(&self) -> Result<(), WriterError> {
        self.resume_tag_renames().await?;
        loop {
            if self.is_shutting_down() {
                return Err(WriterError::ShuttingDown);
            }
            if self
                .rename_tasks
                .lock()
                .unwrap()
                .values()
                .all(|task| task.is_finished())
            {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    pub fn subscribe_to_tag_updates(
        &self,
    ) -> flume::Receiver<Result<crate::tags::TagSnapshot, WriterError>> {
        self.tag_updates_rx.clone()
    }

    pub(crate) fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Commit the reservation; the command then enqueues typed indexed cleanup.
    pub async fn delete_tag(&self, id: String) -> Result<crate::tags::TagSnapshot, WriterError> {
        Ok(self
            .mutate_tags(crate::tags::Mutation::Delete { id })
            .await?
            .snapshot())
    }

    pub(crate) fn report_tag_error(&self, error: WriterError) {
        let _ = self.tag_updates.send(Err(error));
    }

    pub async fn rename_tag(
        &self,
        id: String,
        name: String,
    ) -> Result<crate::tags::TagSnapshot, WriterError> {
        let store = self
            .mutate_tags(crate::tags::Mutation::Rename { id, name })
            .await?;
        self.resume_tag_renames().await?;
        Ok(store.snapshot())
    }

    pub async fn resume_tag_renames(&self) -> Result<(), WriterError> {
        crate::tags::rename::resume(self).await
    }

    pub(crate) async fn patch_tag_name(
        &self,
        id: String,
        old_name: String,
        new_name: Option<String>,
    ) -> Result<(), WriterError> {
        self.patch_tag_labels(id, vec![old_name], new_name).await
    }

    pub(crate) async fn patch_tag_labels(
        &self,
        id: String,
        old_names: Vec<String>,
        new_name: Option<String>,
    ) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::PatchTagLabels {
                id,
                old_names,
                new_name,
                resp,
            },
            rx,
        )
        .await
    }

    /// Resolve current source assignments in the registry lane before enqueuing an index job.
    pub async fn resolve_annotation_tags(
        &self,
        id: String,
    ) -> Result<(crate::tags::Tags, crate::tags::AnnotationTagIds), WriterError> {
        let (store, _, ids) = self.resolve_annotation_snapshot(id).await?;
        Ok((store, ids))
    }

    pub(crate) async fn resolve_annotation_snapshot(
        &self,
        id: String,
    ) -> Result<
        (
            crate::tags::Tags,
            Vec<Annotation>,
            crate::tags::AnnotationTagIds,
        ),
        WriterError,
    > {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::ResolveAnnotationTags { id, resp },
            rx,
        )
        .await
    }

    pub async fn resolve_entry_tags(
        &self,
        id: String,
    ) -> Result<(crate::tags::Tags, Vec<String>), WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(Tier::Interactive, Op::ResolveEntityTags { id, resp }, rx)
            .await
    }

    pub async fn create_tag(&self, name: String) -> Result<crate::tags::TagCreation, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(Tier::Interactive, Op::CreateTag { name, resp }, rx)
            .await
    }

    #[cfg(test)]
    pub(crate) async fn discover_tags(
        &self,
        names: Vec<String>,
    ) -> Result<crate::tags::Tags, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(Tier::Interactive, Op::DiscoverTags { names, resp }, rx)
            .await
    }

    pub async fn organize_pins(
        &self,
        edit: crate::pins::PinEdit,
    ) -> Result<crate::pins::Pins, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(Tier::Interactive, Op::OrganizePins { edit, resp }, rx)
            .await
    }

    pub async fn set_pin(
        &self,
        target: crate::pins::PinTarget,
        pinned: bool,
    ) -> Result<crate::pins::Pins, WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(
            Tier::Interactive,
            Op::SetPin {
                target,
                pinned,
                resp,
            },
            rx,
        )
        .await
    }

    pub async fn add_topic_entity(
        &self,
        tier: Tier,
        topic_id: String,
        entity: Reference,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::AddTopicEntity {
            topic_id,
            entity,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn replace_topic_entity(
        &self,
        tier: Tier,
        topic_id: String,
        old_id: String,
        entity: Reference,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::ReplaceTopicEntity {
            topic_id,
            old_id,
            entity,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn remove_topic_entity(
        &self,
        tier: Tier,
        topic_id: String,
        entity_id: String,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::RemoveTopicEntity {
            topic_id,
            entity_id,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn rename_feed(
        &self,
        tier: Tier,
        id: String,
        title: String,
        unread: HashSet<String>,
        bookmarked: HashSet<String>,
    ) -> Result<Feed, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::RenameFeed {
            id,
            title,
            unread,
            bookmarked,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn remove_feed(&self, tier: Tier, id: String) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::RemoveFeed { id, resp };
        self.submit(tier, op, rx).await
    }

    /// Create-if-absent one feed entry, on that entry's lane. `Some` = created; `None` =
    /// already existed. `pub(crate)`: the in-core feed orchestration calls this per new
    /// entry, then calls [`write_feed_atom`](Self::write_feed_atom) once the entries land.
    pub(crate) async fn write_feed_entry(
        &self,
        tier: Tier,
        url: String,
        title: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
    ) -> Result<Option<Article>, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::WriteFeedEntry {
            url,
            title,
            published_at,
            updated_at,
            summary,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    /// Write a feed's `.atom` on the feed lane. `pub(crate)`: the orchestration calls this
    /// only AFTER every `write_feed_entry` for the feed has completed (entries-before-atom).
    /// `require_exists` = update-only (a refresh): skip if the feed was removed concurrently.
    /// `Atom` is a crate-internal type, so this is unreachable from the command layer.
    pub(crate) async fn write_feed_atom(
        &self,
        tier: Tier,
        id: String,
        atom: Atom,
        require_exists: bool,
    ) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::WriteFeedAtom {
            id,
            atom: Box::new(atom),
            require_exists,
            resp,
        };
        self.submit(tier, op, rx).await
    }
}
