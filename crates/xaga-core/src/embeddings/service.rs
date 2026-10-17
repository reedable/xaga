use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::types::{Entry, Kind, ListItem};

/// Indicates whether the app should restart or exit after the service drains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Exit,
    Restart,
}

use crate::types::{Article, Feed, Topic};

use super::model::{Embedder, load_default_embedder};
use super::pipeline::{
    add_topic_member, apply_state, embed_entry_vector, embed_feed, embed_topic_vector,
    reconcile_topic_membership, remove_topic_member, sync_annotations, sync_entry_row,
    sync_topic_row,
};
use super::{EmbeddingsDb, EmbeddingsError};
use crate::fs::FileLock;

/// A reference to a library entity: its id plus what kind it is, so the consumer
/// never has to probe LanceDB to learn what it is handling.
#[derive(Debug, Clone)]
pub struct EntityRef {
    pub id: String,
    pub kind: Kind,
}

impl EntityRef {
    pub fn new(id: impl Into<String>, kind: Kind) -> Self {
        Self {
            id: id.into(),
            kind,
        }
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
/// (e.g. `EmbedEntry` on Regular from a live save, Maintenance from recovery).
pub enum Job {
    // ── Non-embedding CRUD & state sync (no ONNX) ──
    /// Write-through state update from a user action. Carries the new value(s);
    /// the consumer applies them and emits the UI change event — no file read.
    SyncState {
        entity: EntityRef,
        state: StatePatch,
    },
    /// Entry created/edited: sync the row from the carried `Article`, then defer an
    /// `EmbedEntry` follow-up (carrying the same object) if the checksum changed.
    /// The producer holds the domain object; the consumer never reads the file.
    UpsertEntry {
        article: Article,
    },
    /// Topic created/edited: sync the row from the carried `Topic`, then defer an `EmbedTopic`.
    UpsertTopic {
        topic: Topic,
    },
    /// Reconcile an entry's annotation rows (embedder-free) from the carried `Article`.
    SyncAnnotations {
        article: Article,
    },
    /// Precise reverse-index delta the producer already knows (single member).
    AddMember {
        topic_id: String,
        member_id: String,
    },
    RemoveMember {
        topic_id: String,
        member_id: String,
    },
    /// Remove a single cached row.
    Delete {
        entity: EntityRef,
    },

    // ── Embedding (ONNX vector compute) ──
    /// Compute/refresh an entry vector from the carried `Article` (checksum-gated).
    EmbedEntry {
        article: Article,
    },
    /// Compute/refresh a topic vector from the carried `Topic` (checksum-gated).
    EmbedTopic {
        topic: Topic,
    },
    /// Compute/refresh a feed row's vector from the carried domain `Feed` (feeds have no `.md`).
    EmbedFeed {
        feed: Feed,
    },

    // ── Maintenance ──
    /// Bulk-delete a batch of ids in one LanceDB operation (one version, not N).
    DeleteBatch {
        ids: Vec<String>,
    },
    /// Compact + prune the DB.
    Compact,
    /// Build scalar indexes once rows are populated.
    EnsureIndexes,

    // ── Control: loop control flow ──
    StartRecovery {
        library_dir: PathBuf,
    },
    Reset {
        library_dir: PathBuf,
    },
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
    /// Helper for entry embeds only: given the tier an `UpsertEntry` arrived on, the tier its
    /// deferred `EmbedEntry` follow-up rides — one step toward Maintenance, floored there
    /// (Control/Interactive → Regular; Regular/Maintenance → Maintenance). This is a convenience,
    /// not the routing authority, and topic embedding does not use it (topics are always
    /// Interactive — see `Job::UpsertTopic` in `handle_job`).
    fn embed_follow(self) -> Tier {
        match self {
            Tier::Control | Tier::Interactive => Tier::Regular,
            Tier::Regular | Tier::Maintenance => Tier::Maintenance,
        }
    }
}

/// The four tier senders, bundled so producers — and the loop's own follow-up
/// enqueues — route by `Tier` through one place. `pub(crate)` so the recovery bridge can
/// enqueue work onto the service tiers.
#[derive(Clone)]
pub(crate) struct Senders {
    control: flume::Sender<Job>,
    interactive: flume::Sender<Job>,
    regular: flume::Sender<Job>,
    maintenance: flume::Sender<Job>,
}

impl Senders {
    pub(crate) fn enqueue(&self, tier: Tier, job: Job) {
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
    annotation: flume::Sender<ListItem>,
}

/// Owns the `EmbeddingsDb` and `Embedder`; runs the embedding loop.
///
/// The Tauri layer creates one `EmbeddingService` per library and enqueues work
/// via `enqueue()` / `job_sender()`.  All LanceDB interactions live inside this
/// service — nothing outside needs to know LanceDB exists.
///
/// The service holds the library's `FileLock` for its entire lifetime.
/// Dropping the service (or calling `shutdown()`) releases the lock so the
/// next launch can acquire it.
pub struct EmbeddingService {
    senders: Senders,
    done_rx: flume::Receiver<()>,
    priority_drained_rx: flume::Receiver<()>,
    background_drained_rx: flume::Receiver<()>,
    unread_changed_rx: flume::Receiver<Entry>,
    bookmarked_changed_rx: flume::Receiver<Entry>,
    annotation_changed_rx: flume::Receiver<ListItem>,
    restart_requested: Arc<AtomicBool>,
    embedder: Arc<Embedder>,
    // A read handle sharing the loop's write-handle inner; handed to the app via `read_handle()`.
    read_db: EmbeddingsDb,
    // Held for the service's lifetime (shared with the app's LibraryHandle, which holds it
    // from startup); the flock releases when the last holder drops.
    _lock: std::sync::Arc<FileLock>,
}

impl EmbeddingService {
    /// Opens (or creates) the embeddings DB, loads the model, and starts the
    /// background loop.  Fails if the model files are missing.
    ///
    /// `lock` is the library lock, acquired by the caller at library-open (the app's
    /// setup, or the CLI's `require_write_lock`): only the lock-holding process may run
    /// this service. The service keeps its own `Arc` of it for its lifetime, so it never
    /// runs unlocked regardless of what the caller does with its copy.
    pub async fn start(
        library_dir: PathBuf,
        lock: std::sync::Arc<FileLock>,
    ) -> Result<Self, EmbeddingsError> {
        let embedder = load_default_embedder().await?;
        let service_embedder = Arc::clone(&embedder);

        let db = EmbeddingsDb::open(&library_dir).await?;
        // A clone shares the same underlying table state; handed to the app as its read handle so
        // reads see the service's writes (and the post-compaction reload) without a per-read reload.
        let read_db = db.clone();

        let (control_tx, control_rx) = flume::unbounded::<Job>();
        let (interactive_tx, interactive_rx) = flume::unbounded::<Job>();
        let (regular_tx, regular_rx) = flume::unbounded::<Job>();
        let (maintenance_tx, maintenance_rx) = flume::unbounded::<Job>();
        let (priority_drained_tx, priority_drained_rx) = flume::unbounded::<()>();
        let (background_drained_tx, background_drained_rx) = flume::unbounded::<()>();
        let (unread_changed_tx, unread_changed_rx) = flume::unbounded::<Entry>();
        let (bookmarked_changed_tx, bookmarked_changed_rx) = flume::unbounded::<Entry>();
        let (annotation_changed_tx, annotation_changed_rx) = flume::unbounded::<ListItem>();

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
            read_db,
            _lock: lock,
        })
    }

    /// Returns a read handle that shares the service's underlying table state. Reads through it see
    /// the service's writes and the synchronous post-compaction reload without any per-read reload.
    pub fn read_handle(&self) -> EmbeddingsDb {
        self.read_db.clone()
    }

    /// Enqueues a job onto the given priority tier. The tier *is* the priority;
    /// the same job may be enqueued on different tiers. This is the single
    /// producer surface used by the command layer.
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
    pub fn subscribe_to_annotation_changed(&self) -> flume::Receiver<ListItem> {
        self.annotation_changed_rx.clone()
    }

    /// Enqueues a full recovery pass (non-blocking).
    pub fn trigger_recovery(&self, library_dir: PathBuf) {
        self.senders
            .enqueue(Tier::Control, Job::StartRecovery { library_dir });
    }

    /// Returns a clone of the `Arc<Embedder>` for on-demand text embedding outside the service loop.
    pub fn embedder(&self) -> Arc<Embedder> {
        Arc::clone(&self.embedder)
    }

    /// Resets the embeddings DB and re-runs recovery (non-blocking).
    pub fn reset(&self, library_dir: PathBuf) {
        self.senders
            .enqueue(Tier::Control, Job::Reset { library_dir });
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

#[allow(clippy::too_many_arguments)]
async fn run_loop(
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

    // Compact every this many Interactive items processed consecutively. Kept separate from
    // BACKGROUND_COMPACT_INTERVAL: this cadence is paid on the user-facing path, so it may need
    // tuning independently of background compaction. The two happen to share the value 100 today.
    const INTERACTIVE_COMPACT_INTERVAL: usize = 100;

    // Cleared whenever the Interactive tier processes an item; the Maintenance arm
    // compacts the DB the first time it runs after Interactive drains so version files
    // never pile up for long.
    let mut compacted_after_drain = true; // true = no compact needed yet

    // Items processed since the last compact (Regular + Maintenance); resets to 0 after
    // every compact so the interval applies regardless of why the last compact fired.
    let mut since_compact: usize = 0;

    // Items processed on the Interactive tier since its last compact. A sustained stream of
    // Interactive work (bulk refresh, "mark all read") keeps the biased select from ever reaching
    // the Regular/Maintenance arms where compaction lives, so without this the DB version files
    // grow unbounded and every commit — and read — degrades. Resets to 0 after every interactive
    // compact.
    let mut since_interactive_compact: usize = 0;

    loop {
        // Queues are defined per *priority*, and the definition and examples of each level of
        // priority are provided below.

        // The word "priority" is used to derive precedence. It's the quality of what comes first.
        // It is driven by importance and urgency, not by the type, shape, size, and the cost of
        // work. Something expensive can be very important. Something big might be urgent. What the
        // activity is does not determine priority. Context and circumstances do, and the priority
        // is inherently relative.

        tokio::select! {
            biased;

            // T0 — Control: App-wide commands (and commands only)
            //      StartRecovery, Reset, Shutdown, etc
            result = control_rx.recv_async() => match result {
                Ok(Job::StartRecovery { library_dir: dir }) => {
                    log::info!("embedding service: starting recovery for {}", dir.display());
                    crate::recovery::dispatch::dispatch_recovery(&dir, &db, &embedder, &senders).await;
                    log::info!("embedding service: recovery dispatch complete");
                }
                Ok(Job::Reset { library_dir: dir }) => {
                    if let Err(e) = db.reset().await {
                        log::error!("embedding service: reset failed: {e}");
                    } else {
                        senders.enqueue(Tier::Control, Job::StartRecovery { library_dir: dir });
                    }
                }
                Ok(Job::Shutdown) => {
                    log::info!("embedding service: shutdown requested — breaking");
                    break;
                }
                Ok(_) => log::warn!("control tier: unexpected non-control job"),
                Err(_) => { log::warn!("control tier: disconnected"); break; }
            },

            // T1 — Interactive: User initiated activity
            //      All user initiated CRUD except embedding
            //      Topic embedding is on this channel

            result = interactive_rx.recv_async() => match result {
                Ok(job) => {
                    // Interactive has items — mark that a post-drain compact is needed.
                    compacted_after_drain = false;
                    handle_job(job, Tier::Interactive, &db, &embedder, &senders, &events).await;

                    // Compact on the Interactive path itself: a sustained Interactive stream never
                    // yields to the Regular/Maintenance arms (biased select), so their compaction
                    // never runs. Without this, version files pile up and commits/reads decay.
                    since_interactive_compact += 1;
                    if since_interactive_compact >= INTERACTIVE_COMPACT_INTERVAL {
                        since_interactive_compact = 0;
                        log::info!("interactive tier: {INTERACTIVE_COMPACT_INTERVAL}-item interval — compacting");

                        if let Err(e) = db.compact().await {
                            log::error!("interactive tier: interval compact failed: {e}");
                        }

                        // We just compacted, so the Maintenance arm's post-drain compact is redundant.
                        compacted_after_drain = true;
                    }
                }

                Err(_) => { log::warn!("interactive tier: disconnected"); break; }
            },

            // T2 — Regular: ONNX vector compute.
            //      All user initiated embedding except Topics embedding

            result = regular_rx.recv_async() => match result {
                Ok(job) => {
                    handle_job(job, Tier::Regular, &db, &embedder, &senders, &events).await;
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

            // T3 — Maintenance: System maintenance tasks, such as the side effect of recovery
            //      The Recovery command is dispatched via Control, but its side effects, such
            //      as resulting CRUD operations include embeddings are maintenance.
            //      Topic embeddings, however, is one exception, which goes though Interactive
            //      even if it is a side effect of system activity.

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
                    handle_job(job, Tier::Maintenance, &db, &embedder, &senders, &events).await;

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

/// Executes a single work `Job`. The control-flow variants (Shutdown / Reset /
/// StartRecovery) are handled by the loop's control arm, not here. `arrival` is the tier
/// the job was dequeued from; it only affects where a deferred `EmbedEntry` follow-up is
/// enqueued.
#[allow(clippy::too_many_arguments)]
async fn handle_job(
    job: Job,
    arrival: Tier,
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
        Job::UpsertEntry { article } => {
            if let Err(e) = sync_entry_row(
                &article,
                db,
                &events.unread,
                &events.bookmarked,
                &events.annotation,
            )
            .await
            {
                log::error!("upsert_entry failed id={}: {e}", article.meta.id);
            }
            // Defer the ONNX embed to the appropriate lower tier, carrying the object forward
            // so the embed pass never re-reads the file.
            senders.enqueue(arrival.embed_follow(), Job::EmbedEntry { article });
        }
        Job::UpsertTopic { topic } => {
            if let Err(e) = sync_topic_row(&topic, db).await {
                log::error!("upsert_topic failed id={}: {e}", topic.id);
            }
            // Reconcile the full member→topic reverse index from the carried membership — no read.
            let member_ids: Vec<String> =
                topic.members.iter().map(|m| m.id().to_string()).collect();
            if let Err(e) = reconcile_topic_membership(db, &topic.id, &member_ids).await {
                log::error!("reconcile topic {} failed: {e}", topic.id);
            }
            // All topic embeddings are dispatched at Interactive priority, regardless of the
            // arriving tier, per the priority definitions documented in `run_loop` (T1). This
            // does not use `embed_follow`: that helper only encodes the entry "step toward
            // Maintenance" rule and is not the authority for topic routing.
            senders.enqueue(Tier::Interactive, Job::EmbedTopic { topic });
        }
        Job::EmbedEntry { article } => {
            if let Err(e) = embed_entry_vector(&article, embedder, db).await {
                log::error!("embed_entry failed id={}: {e}", article.meta.id);
            }
        }
        Job::EmbedTopic { topic } => {
            if let Err(e) = embed_topic_vector(&topic, embedder, db).await {
                log::error!("embed_topic failed id={}: {e}", topic.id);
            }
        }
        Job::EmbedFeed { feed } => {
            if let Err(e) = embed_feed(&feed, embedder, db).await {
                log::error!("embed_feed failed id={}: {e}", feed.id);
            }
        }
        Job::SyncAnnotations { article } => {
            if let Err(e) = sync_annotations(&article, db, &events.annotation).await {
                log::error!("sync_annotations failed id={}: {e}", article.meta.id);
            }
        }
        Job::AddMember {
            topic_id,
            member_id,
        } => {
            if let Err(e) = add_topic_member(db, &topic_id, &member_id).await {
                log::error!("add_member topic={topic_id} member={member_id} failed: {e}");
            }
        }
        Job::RemoveMember {
            topic_id,
            member_id,
        } => {
            if let Err(e) = remove_topic_member(db, &topic_id, &member_id).await {
                log::error!("remove_member topic={topic_id} member={member_id} failed: {e}");
            }
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
