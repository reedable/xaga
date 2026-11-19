use super::*;

pub(super) enum Op {
    RepairIdentity {
        path: PathBuf,
        kind: crate::persistence::identity::Record,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
    CreateTag {
        name: String,
        resp: oneshot::Sender<Result<crate::tags::TagCreation, WriterError>>,
    },
    MutateTags {
        mutation: crate::tags::Mutation,
        generations: Arc<std::sync::Mutex<HashMap<String, u64>>>,
        updates: flume::Sender<Result<crate::tags::TagSnapshot, WriterError>>,
        resp: oneshot::Sender<Result<crate::tags::Tags, WriterError>>,
    },
    ResolveEntityTags {
        id: String,
        resp: oneshot::Sender<Result<(crate::tags::Tags, Vec<String>), WriterError>>,
    },
    #[cfg(test)]
    DiscoverTags {
        names: Vec<String>,
        resp: oneshot::Sender<Result<crate::tags::Tags, WriterError>>,
    },
    OrganizePins {
        edit: crate::pins::PinEdit,
        resp: oneshot::Sender<Result<crate::pins::Pins, WriterError>>,
    },
    SetPin {
        target: crate::pins::PinTarget,
        pinned: bool,
        resp: oneshot::Sender<Result<crate::pins::Pins, WriterError>>,
    },
    RecoverImages {
        id: String,
        recovery: crate::persistence::image_recovery::ImageRecovery,
        data: std::collections::HashMap<String, String>,
        resp: oneshot::Sender<Result<Article, WriterError>>,
    },
    RepairSummary {
        id: String,
        summary: String,
        resp: oneshot::Sender<Result<bool, WriterError>>,
    },
    SavePdf {
        url: String,
        bytes: Option<Vec<u8>>,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
    SaveArticle {
        id: String,
        url: String,
        title: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        favicon: Option<String>,
        markdown: String,
        /// `url → data:` for the images the loader fetched, so the persistence
        /// layer can write local image files.
        url_data: std::collections::HashMap<String, String>,
        resp: oneshot::Sender<Result<Article, WriterError>>,
    },
    AddAnnotation {
        article_id: String,
        annotation: Annotation,
        resp: oneshot::Sender<Result<Article, WriterError>>,
    },
    ReplaceAnnotation {
        article_id: String,
        old_id: String,
        annotation: Annotation,
        resp: oneshot::Sender<Result<Article, WriterError>>,
    },
    RemoveAnnotation {
        article_id: String,
        annotation_id: String,
        resp: oneshot::Sender<Result<Article, WriterError>>,
    },
    PatchTagLabels {
        id: String,
        old_names: Vec<String>,
        new_name: Option<String>,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
    ResolveAnnotationTags {
        id: String,
        resp: oneshot::Sender<
            Result<
                (
                    crate::tags::Tags,
                    Vec<Annotation>,
                    crate::tags::AnnotationTagIds,
                ),
                WriterError,
            >,
        >,
    },
    SetAnnotationTag {
        article_id: String,
        annotation_id: String,
        tag_id: String,
        assigned: bool,
        resp: oneshot::Sender<Result<crate::tags::AnnotationTagChange, WriterError>>,
    },
    SetEntryTag {
        id: String,
        tag_id: String,
        assigned: bool,
        resp: oneshot::Sender<Result<crate::tags::EntryTagChange, WriterError>>,
    },
    SetEntryUnread {
        id: String,
        is_unread: bool,
        resp: oneshot::Sender<Result<Entry, WriterError>>,
    },
    SetEntryColor {
        id: String,
        color: Option<String>,
        resp: oneshot::Sender<Result<Entry, WriterError>>,
    },
    /// Bookmark an entry — the bookmark-import path — deciding existing-vs-new INSIDE the
    /// entry lane so a concurrent create (a feed carrying the same URL) cannot slip between
    /// the check and the write and cause the bookmark color to be silently dropped. If the
    /// entry exists, the bookmark color is ensured; otherwise a read (already-read) entry is
    /// created carrying the color.
    BookmarkEntry {
        url: String,
        title: String,
        created_at: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        color: Option<String>,
        resp: oneshot::Sender<Result<BookmarkOutcome, WriterError>>,
    },
    SaveTopic {
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
        entities: Vec<Reference>,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    CreateTopicWithMember {
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
        entity: Reference,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    /// Edits a topic's metadata without touching its member references. The existing
    /// items are re-read and written back inside the topic's lane, so a
    /// concurrent item add/remove cannot be lost.
    EditTopicMeta {
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    DeleteTopic {
        id: String,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
    AddTopicEntity {
        topic_id: String,
        entity: Reference,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    ReplaceTopicEntity {
        topic_id: String,
        old_id: String,
        entity: Reference,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    RemoveTopicEntity {
        topic_id: String,
        entity_id: String,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    RenameFeed {
        id: String,
        title: String,
        unread: HashSet<String>,
        bookmarked: HashSet<String>,
        resp: oneshot::Sender<Result<Feed, WriterError>>,
    },
    RemoveFeed {
        id: String,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
    /// Create-if-absent one feed entry, keyed to that ENTRY's lane (not the feed's), so the
    /// existence check and the write are atomic against every other write to the same entry
    /// file — a second feed carrying the same entry, or a user's mark-read on it. `None`
    /// means the entry already existed (left untouched); `Some` is the created Article.
    WriteFeedEntry {
        url: String,
        title: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        resp: oneshot::Sender<Result<Option<Article>, WriterError>>,
    },
    /// Write a feed's `.atom` (feed lane). Submitted by the orchestration only AFTER every
    /// `WriteFeedEntry` for the feed has completed, preserving the entries-before-atom
    /// ordering invariant across the per-entry lanes. `require_exists` gates the write on
    /// the feed still being present (a refresh must not resurrect a feed the user removed
    /// during the network fetch); it is `false` for `add_feed`, which creates the feed. The
    /// existence check and the write are atomic on the feed lane, serialized against
    /// `remove_feed`.
    WriteFeedAtom {
        id: String,
        atom: Box<Atom>,
        require_exists: bool,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
}

impl Op {
    /// The serialization lane: one per target file. Kind-prefixed so an entry and a
    /// topic sharing an id string can never collide onto one lane.
    pub(super) fn lane_key(&self) -> String {
        match self {
            Op::RepairIdentity { path, kind, .. } => kind.lane(path),
            #[cfg(test)]
            Op::DiscoverTags { .. } => "tags".into(),
            Op::ResolveEntityTags { .. }
            | Op::ResolveAnnotationTags { .. }
            | Op::CreateTag { .. }
            | Op::MutateTags { .. } => "tags".into(),
            Op::SetPin { .. } | Op::OrganizePins { .. } => "pins".into(),
            Op::RecoverImages { id, .. }
            | Op::RepairSummary { id, .. }
            | Op::SaveArticle { id, .. }
            | Op::AddAnnotation { article_id: id, .. }
            | Op::ReplaceAnnotation { article_id: id, .. }
            | Op::RemoveAnnotation { article_id: id, .. }
            | Op::PatchTagLabels { id, .. }
            | Op::SetAnnotationTag { article_id: id, .. }
            | Op::SetEntryTag { id, .. }
            | Op::SetEntryUnread { id, .. }
            | Op::SetEntryColor { id, .. } => format!("entry/{id}"),
            Op::SavePdf { url, .. }
            | Op::BookmarkEntry { url, .. }
            | Op::WriteFeedEntry { url, .. } => {
                // Same id the persistence layer derives, so this shares the entry's lane.
                let id = crate::util::normalize_url(url)
                    .map(|u| crate::util::url_hash12(&u))
                    .unwrap_or_else(|_| url.clone());
                format!("entry/{id}")
            }
            Op::SaveTopic { id, .. }
            | Op::CreateTopicWithMember { id, .. }
            | Op::EditTopicMeta { id, .. }
            | Op::DeleteTopic { id, .. }
            | Op::AddTopicEntity { topic_id: id, .. }
            | Op::ReplaceTopicEntity { topic_id: id, .. }
            | Op::RemoveTopicEntity { topic_id: id, .. } => format!("topic/{id}"),
            Op::RenameFeed { id, .. }
            | Op::RemoveFeed { id, .. }
            | Op::WriteFeedAtom { id, .. } => format!("feed/{id}"),
        }
    }

    /// Short name for shutdown-drop logging.
    pub(super) fn label(&self) -> &'static str {
        match self {
            Op::RepairIdentity { .. } => "repair_identity",
            Op::CreateTag { .. } => "create_tag",
            Op::MutateTags { .. } => "mutate_tags",
            Op::ResolveEntityTags { .. } => "resolve_entry_tags",
            Op::ResolveAnnotationTags { .. } => "resolve_annotation_tags",
            #[cfg(test)]
            Op::DiscoverTags { .. } => "discover_tags",
            Op::OrganizePins { .. } => "organize_pins",
            Op::SetPin { .. } => "set_pin",
            Op::RecoverImages { .. } => "recover_images",
            Op::RepairSummary { .. } => "repair_summary",
            Op::SavePdf { .. } => "save_pdf",
            Op::SaveArticle { .. } => "save_article",
            Op::AddAnnotation { .. } => "add_annotation",
            Op::ReplaceAnnotation { .. } => "replace_annotation",
            Op::RemoveAnnotation { .. } => "remove_annotation",
            Op::PatchTagLabels { .. } => "patch_tag_labels",
            Op::SetEntryTag { .. } => "set_entry_tag",
            Op::SetAnnotationTag { .. } => "set_annotation_tag",
            Op::SetEntryUnread { .. } => "set_entry_unread",
            Op::SetEntryColor { .. } => "set_entry_color",
            Op::BookmarkEntry { .. } => "bookmark_entry",
            Op::SaveTopic { .. } => "save_topic",
            Op::CreateTopicWithMember { .. } => "create_topic_with_member",
            Op::EditTopicMeta { .. } => "edit_topic_meta",
            Op::DeleteTopic { .. } => "delete_topic",
            Op::AddTopicEntity { .. } => "add_topic_entity",
            Op::ReplaceTopicEntity { .. } => "replace_topic_entity",
            Op::RemoveTopicEntity { .. } => "remove_topic_entity",
            Op::RenameFeed { .. } => "rename_feed",
            Op::RemoveFeed { .. } => "remove_feed",
            Op::WriteFeedEntry { .. } => "write_feed_entry",
            Op::WriteFeedAtom { .. } => "write_feed_atom",
        }
    }

    /// Refuse without executing (Regular backlog at shutdown).
    pub(super) fn refuse(self) {
        match self {
            Op::RepairIdentity { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::CreateTag { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::MutateTags { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ResolveAnnotationTags { resp, .. } => {
                drop(resp.send(Err(WriterError::ShuttingDown)))
            }
            Op::ResolveEntityTags { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            #[cfg(test)]
            Op::DiscoverTags { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetPin { resp, .. } | Op::OrganizePins { resp, .. } => {
                drop(resp.send(Err(WriterError::ShuttingDown)))
            }
            Op::RepairSummary { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SavePdf { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RecoverImages { resp, .. } | Op::SaveArticle { resp, .. } => {
                drop(resp.send(Err(WriterError::ShuttingDown)))
            }
            Op::AddAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ReplaceAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::PatchTagLabels { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetEntryTag { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetAnnotationTag { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetEntryUnread { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetEntryColor { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::BookmarkEntry { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SaveTopic { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::CreateTopicWithMember { resp, .. } => {
                drop(resp.send(Err(WriterError::ShuttingDown)))
            }
            Op::DeleteTopic { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::EditTopicMeta { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::AddTopicEntity { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ReplaceTopicEntity { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveTopicEntity { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RenameFeed { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveFeed { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteFeedEntry { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteFeedAtom { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
        }
    }

    /// Execute against `library_dir` and deliver the result through the responder.
    /// Runs inside the op's lane: nothing else touches this file concurrently.
    pub(super) async fn execute(self, library_dir: &std::path::Path, token: WriteToken) {
        match self {
            Op::RepairIdentity { path, kind, resp } => {
                let _ = resp.send(
                    crate::persistence::identity::repair(library_dir, &path, kind, token).await,
                );
            }
            Op::CreateTag { name, resp } => {
                let result = crate::tags::Tags::create_write(library_dir, &name, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::MutateTags {
                mutation,
                resp,
                updates,
                generations,
            } => {
                let notify = matches!(
                    mutation,
                    crate::tags::Mutation::Rename { .. }
                        | crate::tags::Mutation::Delete { .. }
                        | crate::tags::Mutation::Merge { .. }
                        | crate::tags::Mutation::Finish { .. }
                );
                let started = if let crate::tags::Mutation::Rename { id, .. } = &mutation {
                    Some(id.clone())
                } else {
                    None
                };
                let result = crate::tags::Tags::mutate_write(library_dir, mutation, token)
                    .await
                    .map_err(WriterError::from);
                if let Ok(store) = &result {
                    if let Some(id) = started {
                        *generations.lock().unwrap().entry(id).or_default() += 1;
                    }
                    if notify {
                        let _ = updates.send(Ok(store.snapshot()));
                    }
                }
                let _ = resp.send(result);
            }
            Op::ResolveAnnotationTags { id, resp } => {
                let result = async {
                    let article = articles::read_article(library_dir, &id).await?;
                    let labels: Vec<_> = article
                        .annotations
                        .iter()
                        .flat_map(|a| a.tags.iter().cloned())
                        .collect();
                    let store =
                        crate::tags::Tags::discover_entry_write(library_dir, &id, &labels, token)
                            .await?;
                    let resolver = store.resolver();
                    let ids = article
                        .annotations
                        .iter()
                        .map(|a| Ok((a.id.clone(), resolver.resolve(&a.tags)?)))
                        .collect::<Result<_, crate::tags::TagsError>>()?;
                    Ok((store, article.annotations, ids))
                }
                .await;
                let _ = resp.send(result);
            }
            Op::ResolveEntityTags { id, resp } => {
                let result = async {
                    let names = entries::read_entry_tags(library_dir, &id).await?;
                    let store =
                        crate::tags::Tags::discover_entry_write(library_dir, &id, &names, token)
                            .await?;
                    let ids = store.resolver().resolve(&names)?;
                    Ok((store, ids))
                }
                .await;
                let _ = resp.send(result);
            }
            #[cfg(test)]
            Op::DiscoverTags { names, resp } => {
                let result = crate::tags::Tags::discover_write(library_dir, &names, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::OrganizePins { edit, resp } => {
                let result = crate::pins::Pins::organize(library_dir, edit, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::SetPin {
                target,
                pinned,
                resp,
            } => {
                let result = crate::pins::Pins::set(library_dir, target, pinned, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::RepairSummary { id, summary, resp } => {
                let result = crate::persistence::summary_repair::repair_summary(
                    library_dir,
                    &id,
                    &summary,
                    token,
                )
                .await
                .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::SavePdf { url, bytes, resp } => {
                let result = crate::persistence::pdfs::save(library_dir, &url, bytes, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::RecoverImages {
                id,
                recovery,
                data,
                resp,
            } => {
                let result = crate::persistence::image_recovery::apply(
                    library_dir,
                    &id,
                    recovery,
                    data,
                    token,
                )
                .await
                .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::SaveArticle {
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
            } => {
                let r = articles::save_article(
                    library_dir,
                    &id,
                    &url,
                    &title,
                    published_at.as_deref(),
                    updated_at.as_deref(),
                    summary.as_deref(),
                    favicon.as_deref(),
                    &markdown,
                    &url_data,
                    token,
                )
                .await
                .map_err(WriterError::from);
                let r = match r {
                    Ok(article) => {
                        crate::persistence::pdfs::remove_after_html(library_dir, &url, token)
                            .await
                            .map(|_| article)
                            .map_err(WriterError::from)
                    }
                    Err(error) => Err(error),
                };
                let _ = resp.send(r);
            }
            Op::AddAnnotation {
                article_id,
                annotation,
                resp,
            } => {
                let r = articles::add_annotation(library_dir, &article_id, annotation, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::ReplaceAnnotation {
                article_id,
                old_id,
                annotation,
                resp,
            } => {
                let r = articles::replace_annotation(
                    library_dir,
                    &article_id,
                    &old_id,
                    annotation,
                    token,
                )
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveAnnotation {
                article_id,
                annotation_id,
                resp,
            } => {
                let r =
                    articles::remove_annotation(library_dir, &article_id, &annotation_id, token)
                        .await
                        .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::PatchTagLabels {
                id,
                old_names,
                new_name,
                resp,
            } => {
                let result = crate::tags::patch_file(
                    library_dir,
                    &id,
                    &old_names,
                    new_name.as_deref(),
                    token,
                )
                .await;
                let _ = resp.send(result);
            }
            Op::SetAnnotationTag {
                article_id,
                annotation_id,
                tag_id,
                assigned,
                resp,
            } => {
                let result = articles::set_annotation_tag(
                    library_dir,
                    &article_id,
                    &annotation_id,
                    &tag_id,
                    assigned,
                    token,
                )
                .await;
                let _ = resp.send(result);
            }
            Op::SetEntryTag {
                id,
                tag_id,
                assigned,
                resp,
            } => {
                let result =
                    entries::set_entry_tag(library_dir, &id, &tag_id, assigned, token).await;
                let _ = resp.send(result);
            }
            Op::SetEntryUnread {
                id,
                is_unread,
                resp,
            } => {
                let result = entries::set_entry_unread(library_dir, &id, is_unread, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(result);
            }
            Op::SetEntryColor { id, color, resp } => {
                let r = entries::set_entry_color(library_dir, &id, color, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::BookmarkEntry {
                url,
                title,
                created_at,
                published_at,
                updated_at,
                summary,
                color,
                resp,
            } => {
                // Existing-vs-new decided INSIDE the entry lane: a concurrent create of the
                // same URL (a feed) cannot slip between the check and the write, so the
                // bookmark color is never silently dropped.
                let r = async {
                    let id = crate::util::url_hash12(&crate::util::normalize_url(&url)?);
                    let outcome = if entries::find_entry_path(library_dir, &id).await?.is_some() {
                        let color = color.or_else(|| Some("blue".into()));
                        let entry =
                            entries::set_entry_color(library_dir, &id, color, token).await?;
                        BookmarkOutcome::Updated(entry)
                    } else {
                        let article = entries::write_read_entry(
                            library_dir,
                            &url,
                            &title,
                            &created_at,
                            published_at.as_deref(),
                            updated_at.as_deref(),
                            summary.as_deref(),
                            Some(color.as_deref().unwrap_or("blue")),
                            token,
                        )
                        .await?;
                        BookmarkOutcome::Added(Box::new(article))
                    };
                    Ok::<BookmarkOutcome, EntryError>(outcome)
                }
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::SaveTopic {
                id,
                title,
                description,
                threshold,
                entities,
                resp,
            } => {
                let r = topics::save_topic(
                    library_dir,
                    &id,
                    &title,
                    &description,
                    threshold,
                    entities,
                    token,
                )
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::CreateTopicWithMember {
                id,
                title,
                description,
                threshold,
                entity,
                resp,
            } => {
                let r = async {
                    let article_id = match &entity {
                        Reference::Entry { id } => id,
                        Reference::Annotation {
                            article_id: Some(article_id),
                            ..
                        } if !article_id.is_empty() => article_id,
                        Reference::Annotation { .. } | Reference::Note { .. } => {
                            return Err(WriterError::Entry(EntryError::NotFound));
                        }
                    };
                    let article = articles::read_article(library_dir, article_id).await?;
                    if let Reference::Annotation { id, .. } = &entity
                        && !article
                            .annotations
                            .iter()
                            .any(|annotation| annotation.id == *id)
                    {
                        return Err(WriterError::Entry(EntryError::NotFound));
                    }
                    topics::save_topic(
                        library_dir,
                        &id,
                        &title,
                        &description,
                        threshold,
                        vec![entity],
                        token,
                    )
                    .await
                    .map_err(WriterError::from)
                }
                .await;
                let _ = resp.send(r);
            }
            Op::EditTopicMeta {
                id,
                title,
                description,
                threshold,
                resp,
            } => {
                // Read the current items and rewrite with the edited metadata, all inside
                // the lane — no item change made concurrently can be lost.
                let r = async {
                    let entities = topics::read_topic_entities(library_dir, &id)
                        .await
                        .unwrap_or_default();
                    topics::save_topic(
                        library_dir,
                        &id,
                        &title,
                        &description,
                        threshold,
                        entities,
                        token,
                    )
                    .await
                }
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::DeleteTopic { id, resp } => {
                let r = topics::delete_topic(library_dir, &id, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }

            Op::AddTopicEntity {
                topic_id,
                entity,
                resp,
            } => {
                let r = topics::add_topic_entity(library_dir, &topic_id, entity, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::ReplaceTopicEntity {
                topic_id,
                old_id,
                entity,
                resp,
            } => {
                let r =
                    topics::replace_topic_entity(library_dir, &topic_id, &old_id, entity, token)
                        .await
                        .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveTopicEntity {
                topic_id,
                entity_id,
                resp,
            } => {
                let r = topics::remove_topic_entity(library_dir, &topic_id, &entity_id, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }

            Op::RenameFeed {
                id,
                title,
                unread,
                bookmarked,
                resp,
            } => {
                let r = feeds::rename_feed(library_dir, &id, &title, &unread, &bookmarked, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveFeed { id, resp } => {
                let r = feeds::remove_feed(library_dir, &id, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::WriteFeedEntry {
                url,
                title,
                published_at,
                updated_at,
                summary,
                resp,
            } => {
                // Existence check AND write inside the entry lane, so a concurrent create
                // (another feed carrying the same entry) or a user write to the same file
                // cannot interleave. The shared Markdown upsert also preserves existing
                // data if a creation caller encounters an existing file.
                let r = async {
                    let id = crate::util::url_hash12(&crate::util::normalize_url(&url)?);
                    if let Some(path) = entries::find_entry_path(library_dir, &id).await? {
                        crate::persistence::documents::repair_identity(&path, token).await?;
                        Ok(None)
                    } else {
                        entries::write_entry(
                            library_dir,
                            &url,
                            &title,
                            published_at.as_deref(),
                            updated_at.as_deref(),
                            summary.as_deref(),
                            token,
                        )
                        .await
                        .map(Some)
                    }
                }
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::WriteFeedAtom {
                id,
                atom,
                require_exists,
                resp,
            } => {
                let r = feeds::write_feed_atom(library_dir, &id, &atom, require_exists, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
        }
    }
}
