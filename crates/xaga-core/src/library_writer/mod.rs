//! The library writer: the single component through which every library `.md` write
//! flows in the one read-write instance (docs/architecture/single_writer_library_lock.md).
//!
//! Owns the library `FileLock` (no writer, no writes) and serializes writes per entity
//! file — the whole read-modify-write runs inside a lane, so two operations on the same
//! file can never interleave (the annotation-clobber class), while writes to different
//! files run in parallel. Peer to — not part of — the `EmbeddingService`: this component
//! owns `.md` writes; LanceDB writes stay with the service's queue. Files remain the
//! source of truth; LanceDB syncs downstream of the file write, exactly as before.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::oneshot;

use std::collections::HashSet;

use crate::atom::Atom;
use crate::fs::FileLock;
use crate::persistence::entries::EntryError;
use crate::persistence::feeds::FeedError;
use crate::persistence::topics::TopicsError;
use crate::persistence::{articles, entries, feeds, topics};
use crate::types::{Annotation, Article, CollectionItem, Entry, Feed, Topic};

/// Proof of the write right: a zero-sized capability that every `pub(crate)` persistence
/// write requires. Its field is private to this module, so it can be constructed ONLY here
/// (the writer mints one at `start`, having taken the library lock). A future in-crate
/// caller that tried to call a persistence write directly — bypassing the writer's per-file
/// serialization — could not, because it cannot obtain a `WriteToken`. C5 already made the
/// write functions `pub(crate)` (no src-tauri caller); this makes even an in-crate bypass a
/// compile error. Copy so the writer can hand it to each spawned lane task freely.
#[derive(Clone, Copy)]
pub struct WriteToken(());

impl WriteToken {
    /// Private: only the writer, holding the lock, mints tokens.
    fn new() -> Self {
        WriteToken(())
    }

    /// Test-only constructor so persistence-layer unit tests can call write functions
    /// directly without standing up a writer.
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        WriteToken(())
    }
}

/// Cap on concurrently executing lane writes. Writes are ms-scale file RMWs; the cap
/// bounds spawn pressure during bulk intake (feed refresh) without approaching the
/// design-scale corpus limits — lanes above the cap simply wait their turn.
const MAX_IN_FLIGHT: usize = 8;

/// Queue priority for a write request.
///
/// NOTE: Different from the embeddings queue's tiers.
///
/// - `Interactive`: user content (annotations, tag toggles, topic edits).
///   Served first. Drained to disk at shutdown.
/// - `Regular`: content a feed refresh can recreate. Dropped at shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Interactive,
    Regular,
}

/// Why a write did not execute. Persistence failures pass through wrapped; the
/// writer-level cases are the queue refusing or disappearing.
#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error(transparent)]
    Pins(#[from] crate::pins::PinsError),
    #[error("{0}")]
    Entry(#[from] EntryError),
    #[error("{0}")]
    Topic(#[from] TopicsError),
    #[error("{0}")]
    Feed(#[from] FeedError),
    /// The write was refused because the writer is shutting down (Regular backlog is
    /// dropped — re-derivable by definition of the tier).
    #[error("library writer is shutting down; write refused")]
    ShuttingDown,
    /// The writer is gone (channel closed) — should not happen while the app runs.
    #[error("library writer is not running")]
    Closed,
}

impl WriterError {
    /// Stable code the UI matches on. Wrapped errors delegate to the inner
    /// code — a write that fails on an entry surfaces as `entry:*`, not a
    /// generic writer code. Mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            WriterError::Pins(e) => e.code(),
            WriterError::Entry(e) => e.code(),
            WriterError::Topic(e) => e.code(),
            WriterError::Feed(e) => e.code(),
            WriterError::ShuttingDown => "writer:shutting-down",
            WriterError::Closed => "writer:closed",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            WriterError::Pins(e) => e.user_message(),
            WriterError::Entry(e) => e.user_message(),
            WriterError::Topic(e) => e.user_message(),
            WriterError::Feed(e) => e.user_message(),
            WriterError::ShuttingDown => "The app is shutting down; your change wasn't saved.",
            WriterError::Closed => "Couldn't save your change right now.",
        }
    }
}

/// The result of a [`LibraryWriter::bookmark_entry`]: whether the bookmark created a new
/// entry (carrying its Article, for indexing) or marked an existing one (its Entry). The
/// loaded Article payload is boxed rather than inlined in every outcome.
pub enum BookmarkOutcome {
    Added(Box<Article>),
    Updated(Entry),
}

/// One file-scoped write operation (the unit of work is the file; multi-file
/// commands submit N independent requests and cross-file intermediate states are
/// recovery's concern). Each variant mirrors a persistence write function and carries its
/// typed responder.
enum Op {
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
        items: Vec<CollectionItem>,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    CreateTopicWithMember {
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
        item: CollectionItem,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    /// Edits a topic's metadata without touching its item list. The existing
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
    AddTopicItem {
        topic_id: String,
        item: CollectionItem,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    ReplaceTopicItem {
        topic_id: String,
        old_id: String,
        item: CollectionItem,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    RemoveTopicItem {
        topic_id: String,
        item_id: String,
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
    fn lane_key(&self) -> String {
        match self {
            Op::SetPin { .. } => "pins".into(),
            Op::RecoverImages { id, .. }
            | Op::RepairSummary { id, .. }
            | Op::SaveArticle { id, .. }
            | Op::AddAnnotation { article_id: id, .. }
            | Op::ReplaceAnnotation { article_id: id, .. }
            | Op::RemoveAnnotation { article_id: id, .. }
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
            | Op::AddTopicItem { topic_id: id, .. }
            | Op::ReplaceTopicItem { topic_id: id, .. }
            | Op::RemoveTopicItem { topic_id: id, .. } => format!("topic/{id}"),
            Op::RenameFeed { id, .. }
            | Op::RemoveFeed { id, .. }
            | Op::WriteFeedAtom { id, .. } => format!("feed/{id}"),
        }
    }

    /// Short name for shutdown-drop logging.
    fn label(&self) -> &'static str {
        match self {
            Op::SetPin { .. } => "set_pin",
            Op::RecoverImages { .. } => "recover_images",
            Op::RepairSummary { .. } => "repair_summary",
            Op::SavePdf { .. } => "save_pdf",
            Op::SaveArticle { .. } => "save_article",
            Op::AddAnnotation { .. } => "add_annotation",
            Op::ReplaceAnnotation { .. } => "replace_annotation",
            Op::RemoveAnnotation { .. } => "remove_annotation",
            Op::SetEntryUnread { .. } => "set_entry_unread",
            Op::SetEntryColor { .. } => "set_entry_color",
            Op::BookmarkEntry { .. } => "bookmark_entry",
            Op::SaveTopic { .. } => "save_topic",
            Op::CreateTopicWithMember { .. } => "create_topic_with_member",
            Op::EditTopicMeta { .. } => "edit_topic_meta",
            Op::DeleteTopic { .. } => "delete_topic",
            Op::AddTopicItem { .. } => "add_topic_item",
            Op::ReplaceTopicItem { .. } => "replace_topic_item",
            Op::RemoveTopicItem { .. } => "remove_topic_item",
            Op::RenameFeed { .. } => "rename_feed",
            Op::RemoveFeed { .. } => "remove_feed",
            Op::WriteFeedEntry { .. } => "write_feed_entry",
            Op::WriteFeedAtom { .. } => "write_feed_atom",
        }
    }

    /// Refuse without executing (Regular backlog at shutdown).
    fn refuse(self) {
        match self {
            Op::SetPin { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RepairSummary { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SavePdf { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RecoverImages { resp, .. } | Op::SaveArticle { resp, .. } => {
                drop(resp.send(Err(WriterError::ShuttingDown)))
            }
            Op::AddAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ReplaceAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetEntryUnread { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetEntryColor { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::BookmarkEntry { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SaveTopic { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::CreateTopicWithMember { resp, .. } => {
                drop(resp.send(Err(WriterError::ShuttingDown)))
            }
            Op::DeleteTopic { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::EditTopicMeta { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::AddTopicItem { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ReplaceTopicItem { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveTopicItem { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RenameFeed { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveFeed { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteFeedEntry { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteFeedAtom { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
        }
    }

    /// Execute against `library_dir` and deliver the result through the responder.
    /// Runs inside the op's lane: nothing else touches this file concurrently.
    async fn execute(self, library_dir: &std::path::Path, token: WriteToken) {
        match self {
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
                    Ok(article) => crate::persistence::pdfs::save(library_dir, &url, None, token)
                        .await
                        .map(|_| article)
                        .map_err(WriterError::from),
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
                    let outcome = if entries::find_entry_path(library_dir, &id).await.is_some() {
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
                items,
                resp,
            } => {
                let r = topics::save_topic(
                    library_dir,
                    &id,
                    &title,
                    &description,
                    threshold,
                    items,
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
                item,
                resp,
            } => {
                let r = async {
                    let article_id = match &item {
                        CollectionItem::Entry { id } => id,
                        CollectionItem::Annotation { reference, .. } => reference,
                    };
                    let article = articles::read_article(library_dir, article_id).await?;
                    if let CollectionItem::Annotation { id, .. } = &item
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
                        vec![item],
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
                    let items = topics::read_topic_items(library_dir, &id)
                        .await
                        .unwrap_or_default();
                    topics::save_topic(
                        library_dir,
                        &id,
                        &title,
                        &description,
                        threshold,
                        items,
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

            Op::AddTopicItem {
                topic_id,
                item,
                resp,
            } => {
                let r = topics::add_topic_item(library_dir, &topic_id, item, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::ReplaceTopicItem {
                topic_id,
                old_id,
                item,
                resp,
            } => {
                let r = topics::replace_topic_item(library_dir, &topic_id, &old_id, item, token)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveTopicItem {
                topic_id,
                item_id,
                resp,
            } => {
                let r = topics::remove_topic_item(library_dir, &topic_id, &item_id, token)
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
                    if entries::find_entry_path(library_dir, &id).await.is_some() {
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

enum Msg {
    // `Op` is large (the SaveArticle variant carries several owned strings); box it so a
    // queued message stays small.
    Submit(Tier, Box<Op>),
    /// Begin shutdown: drain Interactive (backlog and in-flight complete), drop Regular
    /// backlog (each refused with `ShuttingDown`), then ack.
    Shutdown(oneshot::Sender<()>),
}

/// Handle to the running writer. Cloneable; all clones feed the one dispatcher.
#[derive(Clone)]
pub struct LibraryWriter {
    tx: flume::Sender<Msg>,
    lock: Arc<FileLock>,
    library_dir: PathBuf,
    /// Set once `shutdown()` begins; makes `submit` refuse new work synchronously so
    /// nothing lands in the channel after the dispatcher stops draining it.
    shutting_down: Arc<std::sync::atomic::AtomicBool>,
}

impl LibraryWriter {
    pub async fn save_pdf(&self, url: String, bytes: Option<Vec<u8>>) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        self.submit(Tier::Interactive, Op::SavePdf { url, bytes, resp }, rx)
            .await
    }
    /// Starts the writer for `library_dir`, taking ownership of the library lock — the
    /// writer existing IS the read-write mode; without the lock it cannot be constructed.
    /// Runs its dispatcher on a dedicated thread with its own runtime (the same pattern
    /// as `EmbeddingService`), so it is alive before and independent of the service.
    pub fn start(library_dir: PathBuf, lock: FileLock) -> Self {
        let lock = Arc::new(lock);
        let (tx, rx) = flume::unbounded::<Msg>();

        let dir = library_dir.clone();
        std::thread::Builder::new()
            .name("library-writer".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build library writer runtime");
                rt.block_on(dispatch_loop(dir, rx, WriteToken::new()));
            })
            .expect("failed to spawn library writer thread");

        Self {
            tx,
            lock,
            library_dir,
            shutting_down: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// The held library lock — shared with the `EmbeddingService`, which keeps its own
    /// `Arc` for its lifetime.
    pub fn lock(&self) -> Arc<FileLock> {
        Arc::clone(&self.lock)
    }

    pub fn library_dir(&self) -> &std::path::Path {
        &self.library_dir
    }

    /// Graceful shutdown: Interactive backlog drains to disk, Regular backlog is refused
    /// (re-derivable by tier contract), in-flight writes complete. Resolves when the
    /// last write is durable.
    pub async fn shutdown(&self) {
        // Close the inflow FIRST (submit now fast-fails), so the set of messages the
        // dispatcher must drain is bounded and it can reach a quiescent channel.
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let (ack_tx, ack_rx) = oneshot::channel();
        if self.tx.send(Msg::Shutdown(ack_tx)).is_ok() {
            let _ = ack_rx.await;
        }
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
    pub async fn bookmark_entry(
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
        items: Vec<CollectionItem>,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::SaveTopic {
            id,
            title,
            description,
            threshold,
            items,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    /// Edit a topic's metadata, preserving its current items (read + write inside the
    /// lane). Use this for UI topic edits; use `save_topic` only when setting an explicit
    /// item list.
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
        item: CollectionItem,
    ) -> Result<Topic, WriterError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (resp, rx) = oneshot::channel();
        let op = Op::CreateTopicWithMember {
            id,
            title,
            description,
            threshold,
            item,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn delete_topic(&self, tier: Tier, id: String) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::DeleteTopic { id, resp };
        self.submit(tier, op, rx).await
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

    pub async fn add_topic_item(
        &self,
        tier: Tier,
        topic_id: String,
        item: CollectionItem,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::AddTopicItem {
            topic_id,
            item,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn replace_topic_item(
        &self,
        tier: Tier,
        topic_id: String,
        old_id: String,
        item: CollectionItem,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::ReplaceTopicItem {
            topic_id,
            old_id,
            item,
            resp,
        };
        self.submit(tier, op, rx).await
    }

    pub async fn remove_topic_item(
        &self,
        tier: Tier,
        topic_id: String,
        item_id: String,
    ) -> Result<Topic, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::RemoveTopicItem {
            topic_id,
            item_id,
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

/// Per-file lane: FIFO queue of pending ops (same-file order is submission order,
/// regardless of tier — an Interactive op never jumps a Regular op targeting the SAME
/// file, because the earlier op may create the file the later one needs).
struct Lane {
    queue: VecDeque<(Tier, Op)>,
    busy: bool,
    /// Whether this lane's key currently sits in one of the ready lists.
    listed: bool,
}

async fn dispatch_loop(library_dir: PathBuf, rx: flume::Receiver<Msg>, token: WriteToken) {
    let mut lanes: HashMap<String, Lane> = HashMap::new();
    // Cross-lane priority: Interactive-headed lanes dispatch before Regular-headed ones.
    let mut ready_interactive: VecDeque<String> = VecDeque::new();
    let mut ready_regular: VecDeque<String> = VecDeque::new();
    let mut in_flight: usize = 0;
    let mut shutdown_ack: Option<oneshot::Sender<()>> = None;

    let (completion_tx, completion_rx) = flume::unbounded::<String>();

    loop {
        // While shutting down, drain any buffered messages BEFORE dispatch so the
        // interactive backlog reaches the lanes and gets dispatched below. Draining
        // first is essential: a message that was sent (ok) but not yet received still
        // holds its responder, so acking with messages left in the channel would strand
        // those callers forever. `submit` fast-fails once shutdown begins, so the inflow
        // is bounded and this converges. Regular is re-derivable (tier contract) — refused.
        if shutdown_ack.is_some() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    Msg::Submit(Tier::Interactive, op) => intake(
                        &mut lanes,
                        &mut ready_interactive,
                        &mut ready_regular,
                        Tier::Interactive,
                        *op,
                    ),
                    Msg::Submit(Tier::Regular, op) => {
                        log::info!("library writer: shutdown — refusing {}", op.label());
                        op.refuse();
                    }
                    Msg::Shutdown(_) => {}
                }
            }
        }

        // Dispatch every ready lane up to the concurrency cap, Interactive first.
        while in_flight < MAX_IN_FLIGHT {
            let Some(key) = ready_interactive
                .pop_front()
                .or_else(|| ready_regular.pop_front())
            else {
                break;
            };
            let lane = lanes.get_mut(&key).expect("listed lane exists");
            lane.listed = false;
            let (_, op) = lane.queue.pop_front().expect("listed lane has work");
            lane.busy = true;
            in_flight += 1;

            let dir = library_dir.clone();
            let done = completion_tx.clone();
            let lane_key = key.clone();
            tokio::spawn(async move {
                op.execute(&dir, token).await;
                let _ = done.send(lane_key);
            });
        }

        // Shutdown completes only once the drained interactive backlog has run out and
        // nothing is queued or in flight.
        if let Some(ack) = shutdown_ack.take() {
            if in_flight == 0 && lanes.values().all(|l| l.queue.is_empty() && !l.busy) {
                let _ = ack.send(());
                return;
            }
            shutdown_ack = Some(ack);
        }

        tokio::select! {
            msg = rx.recv_async() => match msg {
                Ok(Msg::Submit(tier, op)) => {
                    if shutdown_ack.is_some() && tier == Tier::Regular {
                        // Drop policy: Regular is re-derivable by tier contract.
                        log::info!("library writer: shutdown — refusing {}", op.label());
                        op.refuse();
                        continue;
                    }
                    intake(&mut lanes, &mut ready_interactive, &mut ready_regular, tier, *op);
                }
                Ok(Msg::Shutdown(ack)) => {
                    shutdown_ack = Some(ack);
                    purge_regular(&mut lanes, &mut ready_interactive, &mut ready_regular);
                }
                // Every handle dropped: exit once in-flight work lands. Pending queued
                // ops can have no listeners, but complete them anyway — they were
                // accepted, and durability beats speed on the way out.
                Err(_) => {
                    if in_flight == 0 && lanes.values().all(|l| l.queue.is_empty() && !l.busy) {
                        return;
                    }
                }
            },
            key = completion_rx.recv_async() => {
                let key = key.expect("completion channel outlives loop");
                in_flight -= 1;
                let lane = lanes.get_mut(&key).expect("completing lane exists");
                lane.busy = false;
                if lane.queue.is_empty() {
                    lanes.remove(&key);
                } else {
                    relist(&mut lanes, &mut ready_interactive, &mut ready_regular, key);
                }
            }
        }
    }
}

fn intake(
    lanes: &mut HashMap<String, Lane>,
    ready_interactive: &mut VecDeque<String>,
    ready_regular: &mut VecDeque<String>,
    tier: Tier,
    op: Op,
) {
    let key = op.lane_key();
    let lane = lanes.entry(key.clone()).or_insert_with(|| Lane {
        queue: VecDeque::new(),
        busy: false,
        listed: false,
    });
    lane.queue.push_back((tier, op));
    if !lane.busy && !lane.listed {
        lane.listed = true;
        match lane.queue.front().expect("just pushed").0 {
            Tier::Interactive => ready_interactive.push_back(key),
            Tier::Regular => ready_regular.push_back(key),
        }
    }
}

/// Re-list a lane whose head changed (previous op completed), by its new head's tier.
fn relist(
    lanes: &mut HashMap<String, Lane>,
    ready_interactive: &mut VecDeque<String>,
    ready_regular: &mut VecDeque<String>,
    key: String,
) {
    let lane = lanes.get_mut(&key).expect("relisting lane exists");
    lane.listed = true;
    match lane.queue.front().expect("relist requires work").0 {
        Tier::Interactive => ready_interactive.push_back(key),
        Tier::Regular => ready_regular.push_back(key),
    }
}

/// Shutdown drop: refuse every queued Regular op, then rebuild the ready lists around
/// what remains (all Interactive). In-flight ops are untouched — they complete.
fn purge_regular(
    lanes: &mut HashMap<String, Lane>,
    ready_interactive: &mut VecDeque<String>,
    ready_regular: &mut VecDeque<String>,
) {
    let mut dropped = 0usize;
    for lane in lanes.values_mut() {
        let mut kept = VecDeque::with_capacity(lane.queue.len());
        for (tier, op) in lane.queue.drain(..) {
            match tier {
                Tier::Interactive => kept.push_back((tier, op)),
                Tier::Regular => {
                    dropped += 1;
                    log::info!("library writer: shutdown — dropping queued {}", op.label());
                    op.refuse();
                }
            }
        }
        lane.queue = kept;
        lane.listed = false;
    }
    ready_interactive.clear();
    ready_regular.clear();
    lanes.retain(|_, lane| lane.busy || !lane.queue.is_empty());
    let keys: Vec<String> = lanes
        .iter()
        .filter(|(_, l)| !l.busy && !l.queue.is_empty())
        .map(|(k, _)| k.clone())
        .collect();
    for key in keys {
        relist(lanes, ready_interactive, ready_regular, key);
    }
    if dropped > 0 {
        log::info!("library writer: shutdown — dropped {dropped} Regular write(s) (re-derivable)");
    }
}

#[cfg(test)]
mod resource_lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn annotation(id: &str) -> Annotation {
        Annotation {
            id: id.to_string(),
            title: "highlighted".to_string(),
            reference: String::new(),
            text: vec!["highlighted".to_string()],
            occurrence: vec![0],
            color: "yellow".to_string(),
            content: String::new(),
            created_at: String::new(),
        }
    }

    fn start_writer(dir: &std::path::Path) -> LibraryWriter {
        let lock = crate::library::acquire_lock(dir).expect("test dir lock");
        LibraryWriter::start(dir.to_path_buf(), lock)
    }

    async fn seed_article(w: &LibraryWriter, id: &str) {
        w.save_article(
            Tier::Interactive,
            id.to_string(),
            format!("https://example.com/{id}"),
            "Title".to_string(),
            None,
            None,
            None,
            None,
            "body".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn reading_state_bookmark_and_annotation_mutations_share_the_entry_lane() {
        let dir = tempfile::tempdir().unwrap();
        let writer = start_writer(dir.path());
        seed_article(&writer, "saved").await;
        let (read, bookmark, annotation) = tokio::join!(
            writer.set_entry_unread(Tier::Interactive, "saved".into(), false),
            writer.set_entry_color(Tier::Interactive, "saved".into(), Some("blue".into())),
            writer.add_annotation(Tier::Interactive, "saved".into(), annotation("note")),
        );
        read.unwrap();
        bookmark.unwrap();
        annotation.unwrap();
        let saved = articles::read_article(dir.path(), "saved").await.unwrap();
        assert!(!saved.meta.is_unread && saved.meta.color.is_some());
        assert_eq!(saved.annotations[0].id, "note");
        let raw = std::fs::read_to_string(crate::library::entry_path(dir.path(), "saved")).unwrap();
        assert!(!crate::md::test_frontmatter_values(&raw).contains_key("is_unread"));
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn concurrent_pins_share_one_lane_and_survive_restart() {
        use crate::pins::{PinTarget, Pins};
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        let mut tasks = Vec::new();
        for i in 0..32 {
            let writer = w.clone();
            tasks.push(tokio::spawn(async move {
                writer
                    .set_pin(
                        PinTarget::Entry {
                            id: format!("entry{i}"),
                        },
                        true,
                    )
                    .await
                    .unwrap();
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        w.set_pin(
            PinTarget::Entry {
                id: "entry0".into(),
            },
            true,
        )
        .await
        .unwrap();
        assert_eq!(Pins::read(dir.path()).await.unwrap().pins.len(), 32);
        w.set_pin(
            PinTarget::Entry {
                id: "entry0".into(),
            },
            false,
        )
        .await
        .unwrap();
        w.shutdown().await;
        drop(w);
        assert_eq!(Pins::read(dir.path()).await.unwrap().pins.len(), 31);
        let other = tempfile::tempdir().unwrap();
        assert!(Pins::read(other.path()).await.unwrap().pins.is_empty());
    }

    #[tokio::test]
    async fn download_and_user_edits_preserve_persisted_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        let url = "https://example.com/preservation";
        let id = crate::util::url_hash12(url);
        w.write_feed_entry(
            Tier::Interactive,
            url.into(),
            "Feed title".into(),
            None,
            None,
            Some("Feed summary".into()),
        )
        .await
        .unwrap();
        let path = crate::library::entry_path(dir.path(), &id);
        let original = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            original.replacen("---\n", "---\ncustom: {nested: [one, two]}\n", 1),
        )
        .unwrap();
        let mut note = annotation("keep");
        note.content = "Irreplaceable note".into();
        w.add_annotation(Tier::Interactive, id.clone(), note)
            .await
            .unwrap();
        w.set_entry_color(Tier::Interactive, id.clone(), Some("blue".into()))
            .await
            .unwrap();
        // Exactly the regressed download input: new body, omitted feed metadata.
        w.save_article(
            Tier::Interactive,
            id.clone(),
            url.into(),
            "Downloaded title".into(),
            None,
            None,
            None,
            None,
            "Downloaded body".into(),
            Default::default(),
        )
        .await
        .unwrap();
        let saved = articles::read_article(dir.path(), &id).await.unwrap();
        assert_eq!(saved.meta.description.as_deref(), Some("Feed summary"));
        assert_eq!(saved.content, "Downloaded body");
        assert!(saved.meta.color.is_some());
        assert!(saved.meta.is_unread);
        assert_eq!(saved.annotations.len(), 1);
        assert_eq!(saved.annotations[0].content, "Irreplaceable note");
        let before_feed = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            crate::md::test_frontmatter_values(&before_feed)["custom"],
            noyalib::from_str::<noyalib::Value>("{nested: [one, two]}").unwrap()
        );
        // A repeated feed discovery must not replace the downloaded/user data.
        w.write_feed_entry(
            Tier::Interactive,
            url.into(),
            "Stale title".into(),
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before_feed);
        w.add_annotation(Tier::Interactive, id.clone(), annotation("temporary"))
            .await
            .unwrap();
        w.remove_annotation(Tier::Interactive, id.clone(), "temporary".into())
            .await
            .unwrap();
        let saved = articles::read_article(dir.path(), &id).await.unwrap();
        assert_eq!(saved.meta.description.as_deref(), Some("Feed summary"));
        assert_eq!(saved.annotations.len(), 1);
        assert_eq!(saved.annotations[0].content, "Irreplaceable note");
        assert_eq!(
            crate::md::test_frontmatter_values(&std::fs::read_to_string(&path).unwrap())["custom"],
            crate::md::test_frontmatter_values(&before_feed)["custom"]
        );
        w.shutdown().await;
    }

    #[tokio::test]
    async fn pdf_replacement_preserves_markdown_and_html_save_removes_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        let url = "https://example.com/document";
        let id = crate::util::url_hash12(&crate::util::normalize_url(url).unwrap());
        w.save_article(
            Tier::Interactive,
            id.clone(),
            url.into(),
            "Title".into(),
            None,
            None,
            None,
            None,
            "original body".into(),
            Default::default(),
        )
        .await
        .unwrap();
        w.add_annotation(Tier::Interactive, id.clone(), annotation("kept"))
            .await
            .unwrap();
        let before = articles::read_article(dir.path(), &id).await.unwrap();
        w.save_pdf(url.into(), Some(b"%PDF-valid".to_vec()))
            .await
            .unwrap();
        let pdf_stem = crate::util::url_hash12(&crate::util::normalize_url(url).unwrap());
        // The existing Entry is the one Markdown identity record; saving PDF
        // bytes neither changes it nor creates a competing companion.
        assert!(!crate::library::pdf_entry_path(dir.path(), &pdf_stem).exists());
        let after = articles::read_article(dir.path(), &id).await.unwrap();
        assert_eq!(before.content, after.content);
        assert_eq!(after.annotations.len(), 1);
        assert!(
            w.save_pdf(url.into(), Some(b"invalid".to_vec()))
                .await
                .is_err()
        );
        assert_eq!(
            crate::persistence::pdfs::read(dir.path(), url)
                .unwrap()
                .unwrap()
                .1,
            b"%PDF-valid"
        );
        w.save_article(
            Tier::Interactive,
            id.clone(),
            url.into(),
            "Title".into(),
            None,
            None,
            None,
            None,
            "new HTML body".into(),
            Default::default(),
        )
        .await
        .unwrap();
        assert!(
            crate::persistence::pdfs::read(dir.path(), url)
                .unwrap()
                .is_none()
        );
        assert!(!crate::library::pdf_entry_path(dir.path(), &pdf_stem).exists());
        assert_eq!(
            articles::read_article(dir.path(), &id)
                .await
                .unwrap()
                .annotations
                .len(),
            1
        );
        w.shutdown().await;
    }

    /// N concurrent annotation adds on one article lose nothing because the lane serializes them.
    #[tokio::test]
    async fn concurrent_same_file_annotations_all_survive() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        seed_article(&w, "abcdef123456").await;

        let n = 20;
        let mut handles = Vec::new();
        for i in 0..n {
            let w = w.clone();
            handles.push(tokio::spawn(async move {
                w.add_annotation(
                    Tier::Interactive,
                    "abcdef123456".to_string(),
                    annotation(&format!("ann-{i}")),
                )
                .await
            }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }

        let article = articles::read_article(dir.path(), "abcdef123456")
            .await
            .unwrap();
        assert_eq!(
            article.annotations.len(),
            n,
            "every concurrent annotation must survive — none clobbered"
        );
    }

    /// Regression: the same entry carried by two feeds (same URL → same entry file) must
    /// serialize on the ENTRY lane, not race on separate feed lanes. N concurrent
    /// `write_feed_entry` for one URL create the file exactly once (one `Some`, rest
    /// `None`); no unconditional double-write can clobber a user's state on that entry.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_feed_entry_creates_for_same_url_create_once() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());

        let url = "https://example.com/shared-across-feeds";
        let mut handles = Vec::new();
        for _ in 0..12 {
            let w = w.clone();
            handles.push(tokio::spawn(async move {
                w.write_feed_entry(
                    Tier::Regular,
                    url.to_string(),
                    "Title".to_string(),
                    None,
                    None,
                    None,
                )
                .await
            }));
        }

        let mut created = 0;
        for h in handles {
            if h.await.unwrap().unwrap().is_some() {
                created += 1;
            }
        }
        assert_eq!(
            created, 1,
            "exactly one concurrent create should win; rest are no-ops"
        );
    }

    /// Regression: bookmarking an entry that is created concurrently (a feed carrying the
    /// same URL) must still end with the bookmark color present — the existing-vs-new
    /// decision is inside the entry lane, so the marking is never dropped. Races
    /// `bookmark_entry` against `write_feed_entry` for the same URL; whichever lands first,
    /// the entry ends up bookmarked.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn bookmark_racing_feed_create_keeps_the_color() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        let url = "https://example.com/in-a-feed-and-bookmarked";

        let (wa, wb) = (w.clone(), w.clone());
        let feed = tokio::spawn(async move {
            wa.write_feed_entry(Tier::Regular, url.to_string(), "T".into(), None, None, None)
                .await
        });
        let bookmark = tokio::spawn(async move {
            wb.bookmark_entry(
                Tier::Interactive,
                url.to_string(),
                "T".into(),
                "".into(),
                None,
                None,
                None,
                None,
            )
            .await
        });
        feed.await.unwrap().unwrap();
        bookmark.await.unwrap().unwrap();

        // Regardless of ordering, the entry exists AND carries the bookmark color.
        let id = crate::util::url_hash12(&crate::util::normalize_url(url).unwrap());
        let tags = entries::read_entry_tags(dir.path(), &id).await.unwrap();
        assert!(
            entries::read_entry(dir.path(), &id)
                .await
                .unwrap()
                .color
                .is_some(),
            "bookmark color must survive a concurrent feed create; tags = {tags:?}"
        );
    }

    /// Writes to different files proceed independently (lanes, not a global queue).
    #[tokio::test]
    async fn different_files_write_in_parallel() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());

        let mut handles = Vec::new();
        for i in 0..8 {
            let w = w.clone();
            handles.push(tokio::spawn(async move {
                w.create_article(
                    Tier::Regular,
                    format!("https://example.com/item-{i}"),
                    format!("Title {i}"),
                    None,
                    None,
                    None,
                    None,
                    "body".to_string(),
                    std::collections::HashMap::new(),
                )
                .await
            }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }
    }

    /// Writes accepted before shutdown are durable after it — a graceful shutdown never
    /// loses a committed write.
    #[tokio::test]
    async fn shutdown_preserves_committed_interactive_writes() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        seed_article(&w, "abcdef123456").await;

        for i in 0..5 {
            w.add_annotation(
                Tier::Interactive,
                "abcdef123456".to_string(),
                annotation(&format!("keep-{i}")),
            )
            .await
            .unwrap();
        }

        w.shutdown().await;

        let article = articles::read_article(dir.path(), "abcdef123456")
            .await
            .unwrap();
        assert_eq!(
            article.annotations.len(),
            5,
            "committed writes survive shutdown"
        );
    }

    /// After shutdown begins, new submits are refused synchronously with a typed error —
    /// never accepted-then-abandoned (which would hang the caller). This is the exact
    /// failure mode that motivated the shutdown flag + channel drain.
    #[tokio::test]
    async fn shutdown_refuses_new_writes_without_hanging() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        seed_article(&w, "abcdef123456").await;

        w.shutdown().await;

        let r = w
            .add_annotation(
                Tier::Interactive,
                "abcdef123456".to_string(),
                annotation("late"),
            )
            .await;
        assert!(
            matches!(r, Err(WriterError::ShuttingDown)),
            "a post-shutdown write must be refused, not stranded; got {r:?}"
        );
    }

    #[tokio::test]
    async fn create_topic_with_member_persists_entry_and_annotation_members() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        seed_article(&w, "abcdef123456").await;

        let entry_topic = w
            .create_topic_with_member(
                Tier::Interactive,
                "Entry topic".into(),
                "Description".into(),
                Some(0.7),
                CollectionItem::Entry {
                    id: "abcdef123456".into(),
                },
            )
            .await
            .unwrap();
        assert!(matches!(
            entry_topic.members.as_slice(),
            [crate::types::Reference::Entry { id }] if id == "abcdef123456"
        ));
        assert_eq!(
            topics::read_topic_items(dir.path(), &entry_topic.id)
                .await
                .unwrap()
                .len(),
            1
        );

        w.add_annotation(
            Tier::Interactive,
            "abcdef123456".into(),
            annotation("annotation-1"),
        )
        .await
        .unwrap();
        let annotation_topic = w
            .create_topic_with_member(
                Tier::Interactive,
                "Annotation topic".into(),
                String::new(),
                None,
                CollectionItem::Annotation {
                    id: "annotation-1".into(),
                    reference: "abcdef123456".into(),
                },
            )
            .await
            .unwrap();
        assert!(matches!(
            annotation_topic.members.as_slice(),
            [crate::types::Reference::Annotation { id }] if id == "annotation-1"
        ));
        w.shutdown().await;
    }

    #[tokio::test]
    async fn create_topic_with_missing_member_leaves_no_topic() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());

        let result = w
            .create_topic_with_member(
                Tier::Interactive,
                "Orphan".into(),
                String::new(),
                None,
                CollectionItem::Entry {
                    id: "missing".into(),
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(WriterError::Entry(EntryError::NotFound))
        ));
        assert!(topics::list_topics(dir.path()).await.unwrap().is_empty());

        seed_article(&w, "abcdef123456").await;
        let result = w
            .create_topic_with_member(
                Tier::Interactive,
                "Orphan annotation".into(),
                String::new(),
                None,
                CollectionItem::Annotation {
                    id: "missing-annotation".into(),
                    reference: "abcdef123456".into(),
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(WriterError::Entry(EntryError::NotFound))
        ));
        assert!(topics::list_topics(dir.path()).await.unwrap().is_empty());
        w.shutdown().await;
    }

    /// Many writes submitted concurrently with shutdown must each resolve — Ok (drained) or ShuttingDown
    /// (refused) — with no hang and no clobber; the on-disk annotation count equals the
    /// number that reported Ok (accepted writes are consistent, none lost or torn).
    /// Runs on a multi-thread runtime so the submits and the shutdown genuinely race.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_submit_and_shutdown_never_hangs_and_stays_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let w = start_writer(dir.path());
        seed_article(&w, "abcdef123456").await;

        let mut interactive = Vec::new();
        for i in 0..10 {
            let w = w.clone();
            interactive.push(tokio::spawn(async move {
                w.add_annotation(
                    Tier::Interactive,
                    "abcdef123456".to_string(),
                    annotation(&format!("keep-{i}")),
                )
                .await
            }));
        }
        let mut regular = Vec::new();
        for i in 0..10 {
            let w = w.clone();
            regular.push(tokio::spawn(async move {
                w.create_article(
                    Tier::Regular,
                    format!("https://example.com/feed-{i}"),
                    "Feed entry".to_string(),
                    None,
                    None,
                    None,
                    None,
                    "body".to_string(),
                    std::collections::HashMap::new(),
                )
                .await
            }));
        }

        w.shutdown().await;

        // Every result is Ok or a clean ShuttingDown — never a hang, never another error.
        let mut ok_interactive = 0usize;
        for h in interactive {
            match h.await.unwrap() {
                Ok(_) => ok_interactive += 1,
                Err(WriterError::ShuttingDown) => {}
                Err(e) => panic!("unexpected interactive error: {e}"),
            }
        }
        for h in regular {
            match h.await.unwrap() {
                Ok(_) | Err(WriterError::ShuttingDown) => {}
                Err(e) => panic!("unexpected regular error: {e}"),
            }
        }

        // The file holds one annotation per write that returned Ok — no
        // accepted write was lost, no refused write landed.
        let article = articles::read_article(dir.path(), "abcdef123456")
            .await
            .unwrap();
        assert_eq!(
            article.annotations.len(),
            ok_interactive,
            "annotation count must match the number of accepted interactive writes"
        );
    }
}
