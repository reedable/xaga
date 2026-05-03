use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Indicates whether the app should restart or exit after the service drains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Exit,
    Restart,
}

use super::lock::LockFile;
use super::model::{Embedder, load_default_embedder};
use super::pipeline::embed_file;
use super::{EmbeddingsDb, EmbeddingsError};

/// Application-level control messages (low volume, high significance).
enum ServiceMsg {
    StartRecovery {
        data_dir: PathBuf,
    },
    ResetDatabase {
        data_dir: PathBuf,
    },
    RecoverCache {
        cache: crate::entry_cache::EntryStateCache,
        data_dir: PathBuf,
    },
    Shutdown,
}

/// High-priority embedding work items (live file-watcher events).
pub enum PriorityMsg {
    Embed(String),
    Delete(String),
}

/// Background embedding work items dispatched by recovery or the file watcher.
pub enum BackgroundMsg {
    Embed(String),
    Delete(String),
    /// Bulk-delete a batch of IDs in a single LanceDB operation.
    /// Used by recovery to avoid creating one version per deleted row.
    DeleteBatch(Vec<String>),
    /// Compact and prune the database after a bulk-ingestion pass.
    /// Sent by recovery as the last item in the queue so it fires once all
    /// other background work has been processed.
    Compact,
}

/// Owns the `EmbeddingsDb` and `Embedder`; runs the embedding loop.
///
/// The Tauri layer creates one `EmbeddingService` per library and forwards
/// file-watcher events via `priority_sender()`.  All LanceDB interactions live
/// inside this service — nothing outside needs to know LanceDB exists.
///
/// The service holds the library's `LockFile` for its entire lifetime.
/// Dropping the service (or calling `shutdown()`) releases the lock so the
/// next launch can acquire it.
pub struct EmbeddingService {
    priority_tx: flume::Sender<PriorityMsg>,
    background_tx: flume::Sender<BackgroundMsg>,
    service_tx: flume::Sender<ServiceMsg>,
    done_rx: flume::Receiver<()>,
    priority_drained_rx: flume::Receiver<()>,
    background_drained_rx: flume::Receiver<()>,
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

        let (priority_tx, priority_rx) = flume::unbounded::<PriorityMsg>();
        let (service_tx, service_rx) = flume::unbounded::<ServiceMsg>();
        let (background_tx, background_rx) = flume::unbounded::<BackgroundMsg>();
        let (priority_drained_tx, priority_drained_rx) = flume::unbounded::<()>();
        let (background_drained_tx, background_drained_rx) = flume::unbounded::<()>();

        let loop_service_tx = service_tx.clone();
        let loop_priority_tx = priority_tx.clone();
        let loop_background_tx = background_tx.clone();

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
                        loop_priority_tx,
                        priority_rx,
                        service_rx,
                        loop_service_tx,
                        background_rx,
                        loop_background_tx,
                        priority_drained_tx,
                        background_drained_tx,
                        embedder,
                        db,
                    )
                    .await;

                    log::warn!("embedding service: run_loop exited");
                });

                let _ = done_tx.send(());
            })?;

        Ok(Self {
            priority_tx,
            background_tx,
            service_tx,
            done_rx,
            priority_drained_rx,
            background_drained_rx,
            restart_requested,
            embedder: service_embedder,
            _lock: lock,
        })
    }

    /// Returns the sender for the file watcher to deliver high-priority embedding events.
    pub fn priority_sender(&self) -> flume::Sender<PriorityMsg> {
        self.priority_tx.clone()
    }

    /// Returns the sender for the file watcher to deliver background embedding events.
    pub fn background_sender(&self) -> flume::Sender<BackgroundMsg> {
        self.background_tx.clone()
    }

    /// Returns a receiver that fires once each time the priority queue drains to empty.
    pub fn subscribe_to_priority_drained(&self) -> flume::Receiver<()> {
        self.priority_drained_rx.clone()
    }

    /// Returns a receiver that fires once each time the background queue drains to empty.
    pub fn subscribe_to_background_drained(&self) -> flume::Receiver<()> {
        self.background_drained_rx.clone()
    }

    /// Enqueues a full recovery pass (non-blocking).
    pub fn trigger_recovery(&self, data_dir: PathBuf) {
        let _ = self.service_tx.send(ServiceMsg::StartRecovery { data_dir });
    }

    /// Enqueues a non-blocking cache recovery pass that updates and broadcasts
    /// any entries that changed since the snapshot was written.
    pub fn trigger_cache_recovery(
        &self,
        cache: crate::entry_cache::EntryStateCache,
        data_dir: PathBuf,
    ) {
        let _ = self
            .service_tx
            .send(ServiceMsg::RecoverCache { cache, data_dir });
    }

    /// Returns a clone of the Arc<Embedder> for on-demand text embedding outside the service loop.
    pub fn embedder(&self) -> Arc<Embedder> {
        Arc::clone(&self.embedder)
    }

    /// Resets the embeddings DB and re-runs recovery (non-blocking).
    pub fn reset(&self, data_dir: PathBuf) {
        let _ = self.service_tx.send(ServiceMsg::ResetDatabase { data_dir });
    }

    /// Marks the service as needing a restart after shutdown.  Must be called
    /// before `shutdown()` for `ShutdownReason::Restart` to be returned.
    pub fn request_restart(&self) {
        self.restart_requested.store(true, Ordering::SeqCst);
    }

    /// Persists the cache, sends `Shutdown` through the service channel, waits
    /// for the run loop to finish the current in-flight LanceDB write and exit,
    /// then returns the reason so the caller can restart or exit cleanly.
    pub async fn shutdown(
        self,
        cache: Option<&crate::entry_cache::EntryStateCache>,
        data_dir: &Path,
    ) -> ShutdownReason {
        if let Some(cache) = cache {
            cache.persist(data_dir).await;
        }
        let _ = self.service_tx.send(ServiceMsg::Shutdown);
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

async fn run_loop(
    data_dir: PathBuf,
    priority_tx: flume::Sender<PriorityMsg>,
    priority_rx: flume::Receiver<PriorityMsg>,
    service_rx: flume::Receiver<ServiceMsg>,
    service_tx: flume::Sender<ServiceMsg>,
    background_rx: flume::Receiver<BackgroundMsg>,
    background_tx: flume::Sender<BackgroundMsg>,
    priority_drained_tx: flume::Sender<()>,
    background_drained_tx: flume::Sender<()>,
    embedder: Arc<Embedder>,
    mut db: EmbeddingsDb,
) {
    // Compact every this many background items processed consecutively.
    const BACKGROUND_COMPACT_INTERVAL: usize = 100;

    // Set to true once priority drains to empty; cleared whenever priority
    // processes an item.  The background arm compacts the DB the first time it
    // runs after a priority drain so version files never pile up for long.
    let mut compacted_after_priority_drain = true; // true = no compact needed yet

    // Counts background items processed since the last compact.  Resets to 0
    // after every compact so the interval applies regardless of why the last
    // compact fired.
    let mut background_since_compact: usize = 0;

    loop {
        tokio::select! {
            biased;

            // Highest priority: application-level control (recovery, reset).
            result = service_rx.recv_async() => match result {
                Ok(ServiceMsg::StartRecovery { data_dir: dir }) => {
                    log::info!("embedding service: starting recovery for {}", dir.display());
                    dispatch_recovery(&dir, &db, &priority_tx, &background_tx).await;
                    log::info!("embedding service: recovery dispatch complete");
                }
                Ok(ServiceMsg::RecoverCache { cache, data_dir: dir }) => {
                    log::info!("embedding service: recovering cache for {}", dir.display());
                    cache.recover(&dir).await;
                    log::info!("embedding service: cache recovery complete");
                }
                Ok(ServiceMsg::ResetDatabase { data_dir: dir }) => {
                    if let Err(e) = db.reset().await {
                        log::error!("embedding service: reset failed: {e}");
                    } else {
                        let _ = service_tx.send(ServiceMsg::StartRecovery { data_dir: dir });
                    }
                }
                Ok(ServiceMsg::Shutdown) => {
                    log::info!("embedding service: shutdown requested — draining");
                    break;
                }
                Err(_) => { log::warn!("service_rx: disconnected"); break; }
            },

            // Medium priority: live .md file events (entries and topics).
            result = priority_rx.recv_async() => {
                // Priority has items — mark that a post-drain compact is needed.
                compacted_after_priority_drain = false;

                match result {
                    Ok(PriorityMsg::Embed(id)) => {
                        log::debug!("priority_rx: embed id={id}");

                        match find_item_path(&data_dir, &id).await {
                            Some(path) => {
                                log::debug!("priority_rx: path={}", path.display());

                                if let Err(e) = embed_file(&path, &embedder, &db).await {
                                    log::error!("priority_rx: embed_file failed id={id}: {e}");
                                }
                            }
                            None => log::warn!("priority_rx: file not found id={id}"),
                        }
                    }
                    Ok(PriorityMsg::Delete(id)) => {
                        if let Err(e) = db.delete_by_id(&id).await {
                            log::error!("priority_rx: delete failed {id}: {e}");
                        }
                    }
                    Err(_) => { log::warn!("priority_rx: disconnected"); break; }
                }
            },

            // Low priority: background embedding work queued by recovery.
            result = background_rx.recv_async() => {
                // background_rx winning the biased select means priority_rx is
                // currently empty.  Compact once at this transition so that the
                // version files written by priority work are merged and pruned
                // before the (potentially large) background batch begins.
                if !compacted_after_priority_drain {
                    compacted_after_priority_drain = true;
                    background_since_compact = 0;
                    log::info!("background_rx: priority queue drained — compacting");
                    if let Err(e) = db.compact().await {
                        log::error!("background_rx: compact failed: {e}");
                    }
                    let _ = priority_drained_tx.send(());
                }

                match result {
                Ok(BackgroundMsg::Embed(id)) => {
                    log::debug!("background_rx: embed id={id}");

                    match find_item_path(&data_dir, &id).await {
                        Some(path) => {
                            log::debug!("background_rx: path={}", path.display());

                            if let Err(e) = embed_file(&path, &embedder, &db).await {
                                log::error!("background_rx: embed_file failed id={id}: {e}");
                            }
                        }
                        None => log::warn!("background_rx: file not found id={id}"),
                    }

                    background_since_compact += 1;

                    if background_since_compact >= BACKGROUND_COMPACT_INTERVAL {
                        background_since_compact = 0;
                        log::info!("background_rx: {BACKGROUND_COMPACT_INTERVAL}-item interval — compacting");
                        if let Err(e) = db.compact().await {
                            log::error!("background_rx: interval compact failed: {e}");
                        }
                    }
                }
                Ok(BackgroundMsg::Delete(id)) => {
                    if let Err(e) = db.delete_by_id(&id).await {
                        log::error!("background_rx: delete failed {id}: {e}");
                    }
                }
                Ok(BackgroundMsg::DeleteBatch(ids)) => {
                    log::debug!("background_rx: delete batch n={}", ids.len());
                    if let Err(e) = db.delete_by_ids(&ids).await {
                        log::error!("background_rx: delete_batch failed: {e}");
                    }
                }
                Ok(BackgroundMsg::Compact) => {
                    background_since_compact = 0;
                    log::info!("background_rx: post-recovery compaction starting");
                    if let Err(e) = db.compact().await {
                        log::error!("background_rx: compact failed: {e}");
                    }
                }
                Err(_) => { log::warn!("background_rx: disconnected"); break; }
                }

                if background_rx.is_empty() {
                    let _ = background_drained_tx.send(());
                }
            },
        }
    }
}

/// Builds the recovery plan and enqueues individual work items.
///
/// Topics are sent via `priority_tx` (medium-priority channel) so they are
/// indexed before any article, even if article embeds are already queued.
/// Articles and deletes go via `background_tx` (lowest-priority channel).
async fn dispatch_recovery(
    data_dir: &Path,
    db: &EmbeddingsDb,
    priority_tx: &flume::Sender<PriorityMsg>,
    background_tx: &flume::Sender<BackgroundMsg>,
) {
    // Compact the DB before queuing any new work to clear version files
    // accumulated during previous sessions.  Without this, Lance's auto-cleanup
    // fires mid-upsert and blocks the run_loop for minutes on a cluttered DB.
    log::info!("embedding service: pre-recovery compact starting");
    if let Err(e) = db.compact().await {
        log::error!("embedding service: pre-recovery compact failed: {e}");
    }

    // The entries that are already in atom files that are not yet in the file system are
    // presumably recent entries. These should get indexed ahead of those *.md files that are
    // already in the file system but have not yet been indexed. We reconcile the feeds
    // synchronously to ensure this ordering; however, this comes with one drawback, which is that
    // the run_loop thread is block while we reconcile the atom feed entries.
    crate::recovery::reconcile_feed_entries(data_dir).await;

    match crate::recovery::build_recovery_plan(data_dir, db).await {
        Ok(plan) => {
            log::info!(
                "embedding service: recovery — {} topics + {} articles to embed, {} to delete",
                plan.to_embed_topics.len(),
                plan.to_embed_articles.len(),
                plan.to_delete.len(),
            );

            // Send all deletes as a single batch (one LanceDB version instead of N).
            let _ = background_tx.send(BackgroundMsg::DeleteBatch(plan.to_delete));

            for id in plan.to_embed_topics {
                let _ = priority_tx.send(PriorityMsg::Embed(id));
            }

            for id in plan.to_embed_articles {
                let _ = background_tx.send(BackgroundMsg::Embed(id));
            }

            // After all background work drains, compact fragments and prune old
            // versions so the on-disk layout stays tidy for future commits.
            let _ = background_tx.send(BackgroundMsg::Compact);
        }

        Err(e) => log::error!("embedding service: recovery plan failed: {e}"),
    }
}
