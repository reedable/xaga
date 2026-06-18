use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::types::{Entry, Kind, SavedItem};

/// Indicates whether the app should restart or exit after the service drains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Exit,
    Restart,
}

use super::lock::LockFile;
use super::model::{Embedder, load_default_embedder};
use super::pipeline::{
    add_topic_member, annotation_embedding_entry, apply_state, embed_feed, embed_feeds,
    embed_vector, reconcile_topic_membership, remove_topic_member, sync_annotations_from_file,
    sync_row,
};
use super::{EmbeddingEntry, EmbeddingsDb, EmbeddingsError};

/// A reference to a library entity: its id plus what kind it is, so the consumer
/// never has to probe LanceDB to learn what it is handling.
#[derive(Debug, Clone)]
pub struct EntityRef {
    pub id: String,
    pub kind: Kind,
}

impl EntityRef {
    pub fn new(id: impl Into<String>, kind: Kind) -> Self {
        Self { id: id.into(), kind }
    }
}

/// New cached state for a `SyncState` write-through. A `None` field leaves that
/// column unchanged.
#[derive(Debug, Clone, Default)]
pub struct StatePatch {
    pub is_unread: Option<bool>,
    pub is_bookmarked: Option<bool>,
    pub is_pinned: Option<bool>,
}

/// The unit of work for the embedding service: a semantic action plus (where
/// relevant) the entity it acts on. The tier a `Job` is enqueued on — not the
/// variant — sets its priority, so the same action may ride different tiers
/// (e.g. `EmbedVector` on Regular from a live save, Maintenance from recovery).
pub enum Job {
    // ── Interactive: cheap, must-reflect-now; no ONNX ──
    /// Write-through state update from a user action. Carries the new value(s);
    /// the consumer applies them and emits the UI change event — no file read.
    SyncState { entity: EntityRef, state: StatePatch },
    /// Content created/edited: sync the row from the file, then defer an
    /// `EmbedVector` follow-up if the checksum changed.
    UpsertContent { entity: EntityRef },
    /// Reconcile an entry's annotation rows (embedder-free).
    SyncAnnotations { id: String },
    /// Precise reverse-index delta the producer already knows (single member).
    AddMember { topic_id: String, member_id: String },
    RemoveMember { topic_id: String, member_id: String },
    /// Remove a single cached row.
    Delete { entity: EntityRef },

    // ── Embedding: ONNX (Regular for live, Maintenance for bulk) ──
    /// Compute/refresh the entity's vector (checksum-gated).
    EmbedVector { entity: EntityRef },

    // ── Maintenance ──
    /// Full diff + chunked bulk fan-out over a topic's whole member list.
    ReconcileMembership { topic_id: String },
    /// Bulk-delete a batch of ids in one LanceDB operation (one version, not N).
    DeleteBatch { ids: Vec<String> },
    /// Compact + prune the DB.
    Compact,
    /// Build scalar indexes once rows are populated.
    EnsureIndexes,

    // ── Control: loop control flow ──
    StartRecovery { data_dir: PathBuf },
    Reset { data_dir: PathBuf },
    Shutdown,
}

/// Priority tiers. A `Job` is enqueued onto exactly one tier; the loop drains
/// strictly Control > Interactive > Regular > Maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Control,
    Interactive,
    Regular,
    Maintenance,
}

impl Tier {
    /// The tier a deferred `EmbedVector` follow-up rides after an `UpsertContent`
    /// arrives on `self`: one step toward Maintenance, floored there. Live content
    /// (Interactive) embeds on Regular; bulk content (Maintenance) embeds on
    /// Maintenance.
    fn embed_follow(self) -> Tier {
        match self {
            Tier::Control | Tier::Interactive => Tier::Regular,
            Tier::Regular | Tier::Maintenance => Tier::Maintenance,
        }
    }
}

/// The four tier senders, bundled so producers — and the loop's own follow-up
/// enqueues — route by `Tier` through one place.
#[derive(Clone)]
struct Senders {
    control: flume::Sender<Job>,
    interactive: flume::Sender<Job>,
    regular: flume::Sender<Job>,
    maintenance: flume::Sender<Job>,
}

impl Senders {
    fn enqueue(&self, tier: Tier, job: Job) {
        let tx = match tier {
            Tier::Control => &self.control,
            Tier::Interactive => &self.interactive,
            Tier::Regular => &self.regular,
            Tier::Maintenance => &self.maintenance,
        };
        let _ = tx.send(job);
    }
}

/// Outbound UI-notification senders, threaded into the job handlers.
#[derive(Clone)]
struct Events {
    unread: flume::Sender<Entry>,
    bookmarked: flume::Sender<Entry>,
    annotation: flume::Sender<SavedItem>,
}

/// Owns the `EmbeddingsDb` and `Embedder`; runs the embedding loop.
///
/// The Tauri layer creates one `EmbeddingService` per library and enqueues work
/// via `enqueue()` / `job_sender()`.  All LanceDB interactions live inside this
/// service — nothing outside needs to know LanceDB exists.
///
/// The service holds the library's `LockFile` for its entire lifetime.
/// Dropping the service (or calling `shutdown()`) releases the lock so the
/// next launch can acquire it.
pub struct EmbeddingService {
    senders: Senders,
    done_rx: flume::Receiver<()>,
    priority_drained_rx: flume::Receiver<()>,
    background_drained_rx: flume::Receiver<()>,
    unread_changed_rx: flume::Receiver<Entry>,
    bookmarked_changed_rx: flume::Receiver<Entry>,
    annotation_changed_rx: flume::Receiver<SavedItem>,
    restart_requested: Arc<AtomicBool>,
    embedder: Arc<Embedder>,
    // Held for the service's lifetime; released (file deleted) on Drop.
    _lock: LockFile,
}

impl EmbeddingService {
    /// Opens (or creates) the embeddings DB, loads the model, and starts the
    /// background loop.  Fails if the model files are missing.
    ///
    /// Returns `Err(EmbeddingsError::LockNotAcquired)` if another instance
    /// already holds the library lock.  In that case the caller should treat
    /// the embeddings DB as read-only and not start this service.
    pub async fn start(data_dir: PathBuf) -> Result<Self, EmbeddingsError> {
        // Acquire the lock before touching LanceDB.  If the file already exists
        // another instance is running against the same library; we must not
        // write to the DB from this instance.
        let lock = LockFile::try_acquire(LockFile::path_for(&data_dir))
            .ok_or(EmbeddingsError::LockNotAcquired)?;

        let embedder = load_default_embedder().await?;
        let service_embedder = Arc::clone(&embedder);

        let db = EmbeddingsDb::open(&data_dir).await?;

        let (control_tx, control_rx) = flume::unbounded::<Job>();
        let (interactive_tx, interactive_rx) = flume::unbounded::<Job>();
        let (regular_tx, regular_rx) = flume::unbounded::<Job>();
        let (maintenance_tx, maintenance_rx) = flume::unbounded::<Job>();
        let (priority_drained_tx, priority_drained_rx) = flume::unbounded::<()>();
        let (background_drained_tx, background_drained_rx) = flume::unbounded::<()>();
        let (unread_changed_tx, unread_changed_rx) = flume::unbounded::<Entry>();
        let (bookmarked_changed_tx, bookmarked_changed_rx) = flume::unbounded::<Entry>();
        let (annotation_changed_tx, annotation_changed_rx) = flume::unbounded::<SavedItem>();

        let senders = Senders {
            control: control_tx,
            interactive: interactive_tx,
            regular: regular_tx,
            maintenance: maintenance_tx,
        };
        let loop_senders = senders.clone();
        let events = Events {
            unread: unread_changed_tx,
            bookmarked: bookmarked_changed_tx,
            annotation: annotation_changed_tx,
        };

        let restart_requested = Arc::new(AtomicBool::new(false));
        let (done_tx, done_rx) = flume::bounded::<()>(1);

        std::thread::Builder::new()
            .name("embedding-service".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build embedding runtime");

                rt.block_on(async move {
                    log::info!("embedding service: run_loop starting");

                    run_loop(
                        data_dir,
                        control_rx,
                        interactive_rx,
                        regular_rx,
                        maintenance_rx,
                        loop_senders,
                        priority_drained_tx,
                        background_drained_tx,
                        events,
                        embedder,
                        db,
                    )
                    .await;

                    log::warn!("embedding service: run_loop exited");
                });

                let _ = done_tx.send(());
            })?;

        Ok(Self {
            senders,
            done_rx,
            priority_drained_rx,
            background_drained_rx,
            unread_changed_rx,
            bookmarked_changed_rx,
            annotation_changed_rx,
            restart_requested,
            embedder: service_embedder,
            _lock: lock,
        })
    }

    /// Enqueues a job onto the given priority tier. The tier *is* the priority;
    /// the same job may be enqueued on different tiers. This is the single
    /// producer surface used by the command layer (and, until it is retired in a
    /// later task, the file watcher).
    pub fn enqueue(&self, tier: Tier, job: Job) {
        self.senders.enqueue(tier, job);
    }

    /// Returns a receiver that fires once each time the priority queue drains to empty.
    pub fn subscribe_to_priority_drained(&self) -> flume::Receiver<()> {
        self.priority_drained_rx.clone()
    }

    /// Returns a receiver that fires once each time the background queue drains to empty.
    pub fn subscribe_to_background_drained(&self) -> flume::Receiver<()> {
        self.background_drained_rx.clone()
    }

    /// Returns a receiver that fires when an entry's is_unread state changes.
    pub fn subscribe_to_unread_changed(&self) -> flume::Receiver<Entry> {
        self.unread_changed_rx.clone()
    }

    /// Returns a receiver that fires when an entry's is_bookmarked state changes.
    pub fn subscribe_to_bookmarked_changed(&self) -> flume::Receiver<Entry> {
        self.bookmarked_changed_rx.clone()
    }

    /// Returns a receiver that fires when an annotation's color or content changes.
    pub fn subscribe_to_annotation_changed(&self) -> flume::Receiver<SavedItem> {
        self.annotation_changed_rx.clone()
    }

    /// Enqueues a full recovery pass (non-blocking).
    pub fn trigger_recovery(&self, data_dir: PathBuf) {
        self.senders.enqueue(Tier::Control, Job::StartRecovery { data_dir });
    }

    /// Returns a clone of the Arc<Embedder> for on-demand text embedding outside the service loop.
    pub fn embedder(&self) -> Arc<Embedder> {
        Arc::clone(&self.embedder)
    }

    /// Resets the embeddings DB and re-runs recovery (non-blocking).
    pub fn reset(&self, data_dir: PathBuf) {
        self.senders.enqueue(Tier::Control, Job::Reset { data_dir });
    }

    /// Marks the service as needing a restart after shutdown.  Must be called
    /// before `shutdown()` for `ShutdownReason::Restart` to be returned.
    pub fn request_restart(&self) {
        self.restart_requested.store(true, Ordering::SeqCst);
    }

    /// Sends `Shutdown` through the service channel, waits for the run loop to finish the
    /// current in-flight LanceDB write and exit, then returns the reason so the caller can
    /// restart or exit cleanly.
    pub async fn shutdown(self) -> ShutdownReason {
        self.senders.enqueue(Tier::Control, Job::Shutdown);
        let _ = self.done_rx.recv_async().await;
        if self.restart_requested.load(Ordering::SeqCst) {
            ShutdownReason::Restart
        } else {
            ShutdownReason::Exit
        }
    }
}

/// Finds the file for any item ID by checking entries directories first, then topics.
async fn find_item_path(data_dir: &Path, id: &str) -> Option<PathBuf> {
    if let Some(p) = crate::entries::find_entry_path(data_dir, id).await {
        return Some(p);
    }

    let tp = crate::topics::topic_path(data_dir, id);

    if tokio::fs::try_exists(&tp).await.unwrap_or(false) {
        return Some(tp);
    }

    None
}

#[allow(clippy::too_many_arguments)]
async fn run_loop(
    data_dir: PathBuf,
    control_rx: flume::Receiver<Job>,
    interactive_rx: flume::Receiver<Job>,
    regular_rx: flume::Receiver<Job>,
    maintenance_rx: flume::Receiver<Job>,
    senders: Senders,
    priority_drained_tx: flume::Sender<()>,
    background_drained_tx: flume::Sender<()>,
    events: Events,
    embedder: Arc<Embedder>,
    mut db: EmbeddingsDb,
) {
    // Compact every this many Regular/Maintenance items processed consecutively.
    const BACKGROUND_COMPACT_INTERVAL: usize = 100;

    // Cleared whenever the Interactive tier processes an item; the Maintenance arm
    // compacts the DB the first time it runs after Interactive drains so version files
    // never pile up for long.
    let mut compacted_after_drain = true; // true = no compact needed yet

    // Items processed since the last compact (Regular + Maintenance); resets to 0 after
    // every compact so the interval applies regardless of why the last compact fired.
    let mut since_compact: usize = 0;

    loop {
        tokio::select! {
            biased;

            // T0 — Control: recovery / reset / shutdown (loop control flow).
            result = control_rx.recv_async() => match result {
                Ok(Job::StartRecovery { data_dir: dir }) => {
                    log::info!("embedding service: starting recovery for {}", dir.display());
                    dispatch_recovery(&dir, &db, &embedder, &senders).await;
                    log::info!("embedding service: recovery dispatch complete");
                }
                Ok(Job::Reset { data_dir: dir }) => {
                    if let Err(e) = db.reset().await {
                        log::error!("embedding service: reset failed: {e}");
                    } else {
                        senders.enqueue(Tier::Control, Job::StartRecovery { data_dir: dir });
                    }
                }
                Ok(Job::Shutdown) => {
                    log::info!("embedding service: shutdown requested — draining");
                    break;
                }
                Ok(_) => log::warn!("control tier: unexpected non-control job"),
                Err(_) => { log::warn!("control tier: disconnected"); break; }
            },

            // T1 — Interactive: cheap, must-reflect-now work (no ONNX).
            result = interactive_rx.recv_async() => match result {
                Ok(job) => {
                    // Interactive has items — mark that a post-drain compact is needed.
                    compacted_after_drain = false;
                    handle_job(job, Tier::Interactive, &data_dir, &db, &embedder, &senders, &events).await;
                }
                Err(_) => { log::warn!("interactive tier: disconnected"); break; }
            },

            // T2 — Regular: ONNX vector compute for single, live items.
            result = regular_rx.recv_async() => match result {
                Ok(job) => {
                    handle_job(job, Tier::Regular, &data_dir, &db, &embedder, &senders, &events).await;
                    since_compact += 1;
                    if since_compact >= BACKGROUND_COMPACT_INTERVAL {
                        since_compact = 0;
                        log::info!("regular tier: {BACKGROUND_COMPACT_INTERVAL}-item interval — compacting");
                        if let Err(e) = db.compact().await {
                            log::error!("regular tier: interval compact failed: {e}");
                        }
                    }
                }
                Err(_) => { log::warn!("regular tier: disconnected"); break; }
            },

            // T3 — Maintenance: bulk embeds, batch deletes, compaction. Because the
            // select is `biased`, this drains only when the higher tiers are empty.
            result = maintenance_rx.recv_async() => match result {
                Ok(job) => {
                    // First maintenance item after the Interactive tier drained: compact
                    // once so the version files written by interactive work are merged and
                    // pruned before the (potentially large) batch begins.
                    if !compacted_after_drain {
                        compacted_after_drain = true;
                        since_compact = 0;
                        log::info!("maintenance tier: interactive drained — compacting");
                        if let Err(e) = db.compact().await {
                            log::error!("maintenance tier: compact failed: {e}");
                        }
                        let _ = priority_drained_tx.send(());
                    }

                    let resets_interval = matches!(job, Job::Compact | Job::EnsureIndexes);
                    handle_job(job, Tier::Maintenance, &data_dir, &db, &embedder, &senders, &events).await;

                    if resets_interval {
                        since_compact = 0;
                    } else {
                        since_compact += 1;
                        if since_compact >= BACKGROUND_COMPACT_INTERVAL {
                            since_compact = 0;
                            log::info!("maintenance tier: {BACKGROUND_COMPACT_INTERVAL}-item interval — compacting");
                            if let Err(e) = db.compact().await {
                                log::error!("maintenance tier: interval compact failed: {e}");
                            }
                        }
                    }

                    if maintenance_rx.is_empty() {
                        let _ = background_drained_tx.send(());
                    }
                }
                Err(_) => { log::warn!("maintenance tier: disconnected"); break; }
            },
        }
    }
}

/// Reads a topic's current member list from its `.md` and reconciles the full
/// member→topic reverse index. Shared by the topic `UpsertContent` path and the
/// explicit `ReconcileMembership` job.
async fn reconcile_topic_full(db: &EmbeddingsDb, data_dir: &Path, topic_id: &str) {
    match crate::topics::read_topic(data_dir, topic_id).await {
        Ok(topic) => {
            let member_ids: Vec<String> =
                topic.items.iter().map(|i| i.id().to_string()).collect();
            if let Err(e) = reconcile_topic_membership(db, topic_id, &member_ids).await {
                log::error!("reconcile topic {topic_id} failed: {e}");
            }
        }
        Err(e) => log::error!("read_topic {topic_id} for reconcile failed: {e}"),
    }
}

/// Executes a single work `Job`. The control-flow variants (Shutdown / Reset /
/// StartRecovery) are handled by the loop's control arm, not here. `arrival` is the tier
/// the job was dequeued from; it only affects where a deferred `EmbedVector` follow-up is
/// enqueued.
#[allow(clippy::too_many_arguments)]
async fn handle_job(
    job: Job,
    arrival: Tier,
    data_dir: &Path,
    db: &EmbeddingsDb,
    embedder: &Arc<Embedder>,
    senders: &Senders,
    events: &Events,
) {
    match job {
        Job::SyncState { entity, state } => {
            if let Err(e) = apply_state(
                db,
                &entity.id,
                state.is_unread,
                state.is_bookmarked,
                state.is_pinned,
                &events.unread,
                &events.bookmarked,
            )
            .await
            {
                log::error!("sync_state failed id={}: {e}", entity.id);
            }
        }
        Job::UpsertContent { entity } => match find_item_path(data_dir, &entity.id).await {
            Some(path) => {
                if let Err(e) =
                    sync_row(&path, db, &events.unread, &events.bookmarked, &events.annotation).await
                {
                    log::error!("upsert_content sync_row failed id={}: {e}", entity.id);
                }
                // A topic `.md` change reconciles its full member→topic reverse index
                // (preserves the pre-refactor watcher behavior; precise per-member deltas
                // arrive with the command-layer wiring in a later task).
                if entity.kind == Kind::Topic {
                    reconcile_topic_full(db, data_dir, &entity.id).await;
                }
                // Defer the ONNX embed to the appropriate lower tier.
                senders.enqueue(arrival.embed_follow(), Job::EmbedVector { entity });
            }
            None => log::warn!("upsert_content: file not found id={}", entity.id),
        },
        Job::EmbedVector { entity } if entity.kind == Kind::Feed => {
            // Feeds embed from their `.atom`, not a `.md`, so they take the single-feed path
            // rather than `find_item_path` + `embed_vector`. (Format still leaks into this
            // branch — the generic-Item refactor is the planned circle-back.)
            let feeds_dir = data_dir.join("feeds");
            match crate::feeds::feed_embedding_meta_one(&feeds_dir, &crate::feeds::FeedId(entity.id.clone())).await {
                Some(meta) => {
                    if let Err(e) = embed_feed(&meta, embedder, db).await {
                        log::error!("embed_feed failed id={}: {e}", entity.id);
                    }
                }
                None => log::warn!("embed_vector: feed .atom not found id={}", entity.id),
            }
        }
        Job::EmbedVector { entity } => match find_item_path(data_dir, &entity.id).await {
            Some(path) => {
                if let Err(e) = embed_vector(&path, embedder, db).await {
                    log::error!("embed_vector failed id={}: {e}", entity.id);
                }
            }
            None => log::warn!("embed_vector: file not found id={}", entity.id),
        },
        Job::SyncAnnotations { id } => match find_item_path(data_dir, &id).await {
            Some(path) => {
                if let Err(e) = sync_annotations_from_file(&path, db, &events.annotation).await {
                    log::error!("sync_annotations failed id={id}: {e}");
                }
            }
            None => log::warn!("sync_annotations: file not found id={id}"),
        },
        Job::AddMember { topic_id, member_id } => {
            if let Err(e) = add_topic_member(db, &topic_id, &member_id).await {
                log::error!("add_member topic={topic_id} member={member_id} failed: {e}");
            }
        }
        Job::RemoveMember { topic_id, member_id } => {
            if let Err(e) = remove_topic_member(db, &topic_id, &member_id).await {
                log::error!("remove_member topic={topic_id} member={member_id} failed: {e}");
            }
        }
        Job::ReconcileMembership { topic_id } => {
            reconcile_topic_full(db, data_dir, &topic_id).await;
        }
        Job::Delete { entity } => {
            if let Err(e) = db.delete_by_id(&entity.id).await {
                log::error!("delete failed id={}: {e}", entity.id);
            }
        }
        Job::DeleteBatch { ids } => {
            log::debug!("delete batch n={}", ids.len());
            if let Err(e) = db.delete_by_ids(&ids).await {
                log::error!("delete_batch failed: {e}");
            }
        }
        Job::Compact => {
            log::info!("maintenance tier: compaction starting");
            if let Err(e) = db.compact().await {
                log::error!("compact failed: {e}");
            }
        }
        Job::EnsureIndexes => {
            if let Err(e) = db.ensure_indexes().await {
                log::error!("ensure_indexes failed: {e}");
            }
        }
        // Control variants never reach the work dispatcher (handled by the control arm).
        Job::Shutdown | Job::Reset { .. } | Job::StartRecovery { .. } => {
            log::warn!("handle_job: control job reached work dispatcher (ignored)");
        }
    }
}

/// Builds the recovery plan and enqueues the work as Maintenance-tier jobs.
///
/// Topics are enqueued before articles so they index first; both ride the
/// Maintenance tier — recovery is bulk/maintenance work (see SPAM.md), kept off
/// the interactive tier so live user actions are never blocked by a backfill.
async fn dispatch_recovery(
    data_dir: &Path,
    db: &EmbeddingsDb,
    embedder: &Arc<Embedder>,
    senders: &Senders,
) {
    // Compact the DB before queuing any new work to clear version files
    // accumulated during previous sessions.  Without this, Lance's auto-cleanup
    // fires mid-upsert and blocks the run_loop for minutes on a cluttered DB.
    log::info!("embedding service: pre-recovery compact starting");
    if let Err(e) = db.compact().await {
        log::error!("embedding service: pre-recovery compact failed: {e}");
    }

    // Capture emptiness *before* embed_feeds inserts feed rows, so a fresh post-reset
    // recovery can use the fast bulk-insert path below instead of per-row upserts.
    let fresh = db.count().await.unwrap_or(1) == 0;

    // The entries that are already in atom files that are not yet in the file system are
    // presumably recent entries. These should get indexed ahead of those *.md files that are
    // already in the file system but have not yet been indexed. We reconcile the feeds
    // synchronously to ensure this ordering; however, this comes with one drawback, which is that
    // the run_loop thread is block while we reconcile the atom feed entries.
    crate::recovery::reconcile_feed_entries(data_dir).await;

    // Embed feed rows (kind = 'feed') from the .atom files. Feeds are few, so this runs
    // synchronously here alongside reconcile.
    if let Err(e) = embed_feeds(&data_dir.join("feeds"), embedder, db).await {
        log::error!("embedding service: embed_feeds failed: {e}");
    }

    match crate::recovery::build_recovery_plan(data_dir, db).await {
        Ok(plan) => {
            log::info!(
                "embedding service: recovery — {} topics + {} articles to embed, {} to delete",
                plan.to_embed_topics.len(),
                plan.to_embed_articles.len(),
                plan.to_delete.len(),
            );

            // All deletes as a single batch (one LanceDB version instead of N).
            senders.enqueue(Tier::Maintenance, Job::DeleteBatch { ids: plan.to_delete });

            if fresh {
                // Empty table: bulk-insert rows (no per-row get_entry/delete), then queue
                // each id for the vector pass. Topics first so they embed before articles.
                bulk_insert(data_dir, db, &plan.to_embed_topics, Kind::Topic, senders).await;
                bulk_insert(data_dir, db, &plan.to_embed_articles, Kind::Entry, senders).await;
            } else {
                for id in plan.to_embed_topics {
                    senders.enqueue(
                        Tier::Maintenance,
                        Job::UpsertContent { entity: EntityRef::new(id, Kind::Topic) },
                    );
                }
                for id in plan.to_embed_articles {
                    senders.enqueue(
                        Tier::Maintenance,
                        Job::UpsertContent { entity: EntityRef::new(id, Kind::Entry) },
                    );
                }
            }

            // Rebuild the member→topic reverse index from the topic files. Unconditional: a
            // membership-only change doesn't alter a member's checksum, so build_recovery_plan
            // would skip it. Covers entries + annotations (annotation rows exist post bulk_insert).
            // Bounded by drift via the topic-row `items` base.
            match crate::topics::list_topics(data_dir).await {
                Ok(topics) => {
                    for topic in &topics {
                        let member_ids: Vec<String> =
                            topic.items.iter().map(|i| i.id().to_string()).collect();
                        if let Err(e) =
                            reconcile_topic_membership(db, &topic.id, &member_ids).await
                        {
                            log::error!(
                                "recovery: reverse-index reconcile for topic {} failed: {e}",
                                topic.id
                            );
                        }
                    }
                }
                Err(e) => log::error!("recovery: list_topics for reverse-index rebuild failed: {e}"),
            }

            // After all background work drains, compact + prune old versions, then
            // (re)build scalar indexes on the populated, compacted table.
            senders.enqueue(Tier::Maintenance, Job::Compact);
            senders.enqueue(Tier::Maintenance, Job::EnsureIndexes);
        }

        Err(e) => log::error!("embedding service: recovery plan failed: {e}"),
    }
}

/// Bulk path for fresh recovery: reads each id's `.md`, builds a null-vector row (plus a
/// null-vector row for each of the entry's annotations), and inserts in batches with a single
/// `add` per batch — no per-row `get_entry`/`delete`. Each id is queued onto the vector channel
/// for the (lowest-priority) embed pass, which fills the entry's and its annotations' vectors.
/// Safe only when the table has no pre-existing rows for these ids.
async fn bulk_insert(
    data_dir: &Path,
    db: &EmbeddingsDb,
    ids: &[String],
    kind: Kind,
    senders: &Senders,
) {
    const BULK_BATCH: usize = 1024;
    let mut buf: Vec<EmbeddingEntry> = Vec::with_capacity(BULK_BATCH);

    for id in ids {
        let Some(path) = find_item_path(data_dir, id).await else {
            log::warn!("bulk_insert: file not found id={id}");
            continue;
        };
        let Ok(raw) = tokio::fs::read_to_string(&path).await else {
            log::warn!("bulk_insert: unreadable {}", path.display());
            continue;
        };
        let Some((fm, body)) = crate::util::parse_document(&raw) else {
            log::warn!("bulk_insert: bad frontmatter {}", path.display());
            continue;
        };

        buf.push(fm.to_embedding_entry(id.clone(), &body, None));
        // Create annotation rows (null-vector) in the bulk phase too, so they're listable
        // immediately and exist before the reverse-index reconcile (W2). Topics empty for now;
        // the entry's embed pass fills annotation vectors via embed_annotations.
        for annotation in &fm.annotations {
            buf.push(annotation_embedding_entry(id, &fm, annotation, Vec::new(), None));
        }
        senders.enqueue(
            Tier::Maintenance,
            Job::EmbedVector { entity: EntityRef::new(id.clone(), kind) },
        );

        if buf.len() >= BULK_BATCH {
            if let Err(e) = db.add_batch(std::mem::take(&mut buf)).await {
                log::error!("bulk_insert: add_batch failed: {e}");
            }
        }
    }
    if let Err(e) = db.add_batch(buf).await {
        log::error!("bulk_insert: add_batch failed: {e}");
    }
}
