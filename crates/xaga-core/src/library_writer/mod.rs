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

/// Cap on concurrently executing lane writes. Writes are ms-scale file RMWs; the cap
/// bounds spawn pressure during bulk intake (feed refresh) without approaching the
/// design-scale corpus limits — lanes above the cap simply wait their turn.
const MAX_IN_FLIGHT: usize = 8;

/// Queue priority for a write request. Same names as the embeddings queue's tiers
/// (prevailing vocabulary — owner ruling); deliberately its own two-variant type, because
/// the writer's tiers carry a shutdown CONTRACT the embeddings tiers do not:
///
/// - `Interactive`: user-authored content that exists ONLY in this queue until written
///   (annotations, tag toggles, topic edits). MUST drain at shutdown — dropping it
///   destroys user intent permanently.
/// - `Regular`: content re-derivable from its source (feed-refresh entry creates — the
///   next refresh recreates them). Droppable at shutdown; the drop costs a re-fetch,
///   never user content. **Admission rule: enqueue as Regular ONLY what is re-derivable
///   from source** — the shutdown drop policy is keyed to this tier, so a
///   non-re-derivable write enqueued as Regular would be silently destroyed at shutdown.
///
/// Classification is by content re-derivability, not by trigger: a user-clicked
/// Refresh All still enqueues its entry creates as Regular (owner ruling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Interactive,
    Regular,
}

/// Why a write did not execute. Persistence failures pass through wrapped; the
/// writer-level cases are the queue refusing or disappearing.
#[derive(Debug, thiserror::Error)]
pub enum WriterError {
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

/// A typed tag mutation, applied to the entry's current tags INSIDE its lane — the
/// read-modify-write that commands previously did across two calls (read_entry_tags →
/// set_entry_tags) happens as one serialized unit here. The variant carries the user's
/// intent end-to-end (docs/principles/carry_intent_end_to_end.md).
#[derive(Debug, Clone, Copy)]
pub enum TagPatch {
    MarkRead,
    MarkUnread,
    AddBookmark,
    RemoveBookmark,
}

impl TagPatch {
    fn apply(self, mut tags: Vec<String>) -> Vec<String> {
        match self {
            TagPatch::MarkRead => {
                tags.retain(|t| t != crate::tags::UNREAD);
                tags
            }
            TagPatch::MarkUnread => {
                if !crate::tags::is_unread(&tags) {
                    tags.push(crate::tags::UNREAD.to_string());
                }
                tags
            }
            TagPatch::AddBookmark => {
                if !crate::tags::is_bookmarked(&tags) {
                    tags.push(crate::tags::BOOKMARK.to_string());
                }
                tags
            }
            TagPatch::RemoveBookmark => {
                tags.retain(|t| t != crate::tags::BOOKMARK);
                tags
            }
        }
    }
}

/// One file-scoped write operation (owner ruling: the unit of work is the file; multi-file
/// commands submit N independent requests and cross-file intermediate states are
/// recovery's concern). Each variant mirrors a persistence write function and carries its
/// typed responder.
enum Op {
    SaveArticle {
        id: String,
        url: String,
        title: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        favicon: Option<String>,
        markdown: String,
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
    PatchEntryTags {
        id: String,
        patch: TagPatch,
        resp: oneshot::Sender<Result<Entry, WriterError>>,
    },
    /// Create a read (already-read) entry if one does not already exist at its id — the
    /// bookmark-import create path. Idempotent: an existing file is left untouched.
    WriteReadEntry {
        url: String,
        title: String,
        created_at: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        tags: Vec<String>,
        resp: oneshot::Sender<Result<Article, WriterError>>,
    },
    SaveTopic {
        id: String,
        title: String,
        description: String,
        threshold: Option<f32>,
        items: Vec<CollectionItem>,
        resp: oneshot::Sender<Result<Topic, WriterError>>,
    },
    /// Edit a topic's metadata while PRESERVING its current items — the read of the
    /// existing items and the write happen inside the lane, so a concurrent
    /// add/remove/replace item op can't be clobbered by a metadata edit.
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
    SetTopicPinned {
        id: String,
        pinned: bool,
        resp: oneshot::Sender<Result<bool, WriterError>>,
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
    SetFeedPinned {
        id: String,
        pinned: bool,
        resp: oneshot::Sender<Result<bool, WriterError>>,
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
    /// ordering invariant across the per-entry lanes.
    WriteFeedAtom {
        id: String,
        atom: Box<Atom>,
        resp: oneshot::Sender<Result<(), WriterError>>,
    },
}

impl Op {
    /// The serialization lane: one per target file. Kind-prefixed so an entry and a
    /// topic sharing an id string can never collide onto one lane.
    fn lane_key(&self) -> String {
        match self {
            Op::SaveArticle { id, .. }
            | Op::AddAnnotation { article_id: id, .. }
            | Op::ReplaceAnnotation { article_id: id, .. }
            | Op::RemoveAnnotation { article_id: id, .. }
            | Op::PatchEntryTags { id, .. } => format!("entry/{id}"),
            Op::WriteReadEntry { url, .. } | Op::WriteFeedEntry { url, .. } => {
                // Same id the persistence layer derives, so this shares the entry's lane.
                let id = crate::util::normalize_url(url)
                    .map(|u| crate::util::url_hash12(&u))
                    .unwrap_or_else(|_| url.clone());
                format!("entry/{id}")
            }
            Op::SaveTopic { id, .. }
            | Op::EditTopicMeta { id, .. }
            | Op::DeleteTopic { id, .. }
            | Op::SetTopicPinned { id, .. }
            | Op::AddTopicItem { topic_id: id, .. }
            | Op::ReplaceTopicItem { topic_id: id, .. }
            | Op::RemoveTopicItem { topic_id: id, .. } => format!("topic/{id}"),
            Op::SetFeedPinned { id, .. }
            | Op::RenameFeed { id, .. }
            | Op::RemoveFeed { id, .. }
            | Op::WriteFeedAtom { id, .. } => format!("feed/{id}"),
        }
    }

    /// Short name for shutdown-drop logging.
    fn label(&self) -> &'static str {
        match self {
            Op::SaveArticle { .. } => "save_article",
            Op::AddAnnotation { .. } => "add_annotation",
            Op::ReplaceAnnotation { .. } => "replace_annotation",
            Op::RemoveAnnotation { .. } => "remove_annotation",
            Op::PatchEntryTags { .. } => "patch_entry_tags",
            Op::WriteReadEntry { .. } => "write_read_entry",
            Op::SaveTopic { .. } => "save_topic",
            Op::EditTopicMeta { .. } => "edit_topic_meta",
            Op::DeleteTopic { .. } => "delete_topic",
            Op::SetTopicPinned { .. } => "set_topic_pinned",
            Op::AddTopicItem { .. } => "add_topic_item",
            Op::ReplaceTopicItem { .. } => "replace_topic_item",
            Op::RemoveTopicItem { .. } => "remove_topic_item",
            Op::SetFeedPinned { .. } => "set_feed_pinned",
            Op::RenameFeed { .. } => "rename_feed",
            Op::RemoveFeed { .. } => "remove_feed",
            Op::WriteFeedEntry { .. } => "write_feed_entry",
            Op::WriteFeedAtom { .. } => "write_feed_atom",
        }
    }

    /// Refuse without executing (Regular backlog at shutdown).
    fn refuse(self) {
        match self {
            Op::SaveArticle { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::AddAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ReplaceAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveAnnotation { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::PatchEntryTags { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteReadEntry { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SaveTopic { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::DeleteTopic { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::EditTopicMeta { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetTopicPinned { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::AddTopicItem { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::ReplaceTopicItem { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveTopicItem { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::SetFeedPinned { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RenameFeed { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::RemoveFeed { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteFeedEntry { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
            Op::WriteFeedAtom { resp, .. } => drop(resp.send(Err(WriterError::ShuttingDown))),
        }
    }

    /// Execute against `library_dir` and deliver the result through the responder.
    /// Runs inside the op's lane: nothing else touches this file concurrently.
    async fn execute(self, library_dir: &std::path::Path) {
        match self {
            Op::SaveArticle {
                id,
                url,
                title,
                published_at,
                updated_at,
                summary,
                favicon,
                markdown,
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
                )
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::AddAnnotation {
                article_id,
                annotation,
                resp,
            } => {
                let r = articles::add_annotation(library_dir, &article_id, annotation)
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
                let r = articles::replace_annotation(library_dir, &article_id, &old_id, annotation)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveAnnotation {
                article_id,
                annotation_id,
                resp,
            } => {
                let r = articles::remove_annotation(library_dir, &article_id, &annotation_id)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::PatchEntryTags { id, patch, resp } => {
                // The whole tag RMW inside the lane — the two-call read/set pattern the
                // command layer used to do, now uninterleavable.
                let r = async {
                    let tags = entries::read_entry_tags(library_dir, &id).await?;
                    entries::set_entry_tags(library_dir, &id, patch.apply(tags)).await
                }
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::WriteReadEntry {
                url,
                title,
                created_at,
                published_at,
                updated_at,
                summary,
                tags,
                resp,
            } => {
                let r = entries::write_read_entry(
                    library_dir,
                    &url,
                    &title,
                    &created_at,
                    published_at.as_deref(),
                    updated_at.as_deref(),
                    summary.as_deref(),
                    &tags,
                )
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
                let r = topics::save_topic(library_dir, &id, &title, &description, threshold, items)
                    .await
                    .map_err(WriterError::from);
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
                    topics::save_topic(library_dir, &id, &title, &description, threshold, items)
                        .await
                }
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::DeleteTopic { id, resp } => {
                let r = topics::delete_topic(library_dir, &id)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::SetTopicPinned { id, pinned, resp } => {
                let r = topics::set_topic_pinned(library_dir, &id, pinned)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::AddTopicItem {
                topic_id,
                item,
                resp,
            } => {
                let r = topics::add_topic_item(library_dir, &topic_id, item)
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
                let r = topics::replace_topic_item(library_dir, &topic_id, &old_id, item)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveTopicItem {
                topic_id,
                item_id,
                resp,
            } => {
                let r = topics::remove_topic_item(library_dir, &topic_id, &item_id)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::SetFeedPinned { id, pinned, resp } => {
                let r = feeds::set_feed_pinned(library_dir, &id, pinned)
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
                let r = feeds::rename_feed(library_dir, &id, &title, &unread, &bookmarked)
                    .await
                    .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::RemoveFeed { id, resp } => {
                let r = feeds::remove_feed(library_dir, &id)
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
                // cannot interleave. `write_entry` itself is an unconditional write; the
                // create-if-absent decision must live here, in the lane.
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
                        )
                        .await
                        .map(Some)
                    }
                }
                .await
                .map_err(WriterError::from);
                let _ = resp.send(r);
            }
            Op::WriteFeedAtom { id, atom, resp } => {
                let r = feeds::write_feed_atom(library_dir, &id, &atom)
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
                rt.block_on(dispatch_loop(dir, rx));
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

    pub async fn patch_entry_tags(
        &self,
        tier: Tier,
        id: String,
        patch: TagPatch,
    ) -> Result<Entry, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::PatchEntryTags { id, patch, resp };
        self.submit(tier, op, rx).await
    }

    /// Create an already-read entry if absent (bookmark-import create path). Idempotent.
    #[allow(clippy::too_many_arguments)]
    pub async fn write_read_entry(
        &self,
        tier: Tier,
        url: String,
        title: String,
        created_at: String,
        published_at: Option<String>,
        updated_at: Option<String>,
        summary: Option<String>,
        tags: Vec<String>,
    ) -> Result<Article, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::WriteReadEntry {
            url,
            title,
            created_at,
            published_at,
            updated_at,
            summary,
            tags,
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

    pub async fn delete_topic(&self, tier: Tier, id: String) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::DeleteTopic { id, resp };
        self.submit(tier, op, rx).await
    }

    pub async fn set_topic_pinned(
        &self,
        tier: Tier,
        id: String,
        pinned: bool,
    ) -> Result<bool, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::SetTopicPinned { id, pinned, resp };
        self.submit(tier, op, rx).await
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

    pub async fn set_feed_pinned(
        &self,
        tier: Tier,
        id: String,
        pinned: bool,
    ) -> Result<bool, WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::SetFeedPinned { id, pinned, resp };
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
    /// `Atom` is a crate-internal type, so this is unreachable from the command layer.
    pub(crate) async fn write_feed_atom(
        &self,
        tier: Tier,
        id: String,
        atom: Atom,
    ) -> Result<(), WriterError> {
        let (resp, rx) = oneshot::channel();
        let op = Op::WriteFeedAtom {
            id,
            atom: Box::new(atom),
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

async fn dispatch_loop(library_dir: PathBuf, rx: flume::Receiver<Msg>) {
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
                op.execute(&dir).await;
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
        )
        .await
        .unwrap();
    }

    /// The spike scenario: N concurrent annotation adds on ONE article lose nothing.
    /// Before the writer, each add was an unserialized whole-file RMW and concurrent
    /// adds clobbered each other; the lane serializes them.
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
        assert_eq!(created, 1, "exactly one concurrent create should win; rest are no-ops");
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
        assert_eq!(article.annotations.len(), 5, "committed writes survive shutdown");
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

    /// The racy case that originally deadlocked: many writes submitted CONCURRENTLY with
    /// shutdown. The contract is that every one resolves — Ok (drained) or ShuttingDown
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

        // On-disk state is consistent with what was accepted: exactly the Ok annotations,
        // none clobbered, none torn.
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
