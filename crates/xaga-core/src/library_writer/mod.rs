//! The library writer: the single component through which every library `.md` write
//! flows in the one read-write instance (docs/architecture/single_writer_library_lock.md).
//!
//! Owns the library `FileLock` (no writer, no writes) and serializes writes per entity
//! file — the whole read-modify-write runs inside a lane, so two operations on the same
//! file can never interleave (the annotation-clobber class), while writes to different
//! files run in parallel. Peer to — not part of — the `EmbeddingService`: this component
//! owns `.md` writes; LanceDB writes stay with the service's queue. Files remain the
//! source of truth; LanceDB syncs downstream of the file write, exactly as before.

mod operations;
mod scheduler;
mod submission;

use operations::Op;
use scheduler::dispatch_loop;

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
use crate::types::{Annotation, Article, Entry, Feed, Reference, Topic};

/// Proof of the write right: a zero-sized capability that every `pub(crate)` persistence
/// write requires. Its field is private to this module, so it can be constructed ONLY here
/// (the writer mints one at `start`, having taken the library lock). A future in-crate
/// caller that tried to call a persistence write directly — bypassing the writer's per-file
/// serialization — could not, because it cannot obtain a `WriteToken`. C5 already made the
/// write functions `pub(crate)` (no src-tauri caller); this makes even an in-crate bypass a
/// compile error. Copy so the writer can hand it to each spawned lane task freely.
#[derive(Clone, Copy)]
pub(crate) struct WriteToken(());

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
    #[error(transparent)]
    Tags(#[from] crate::tags::TagsError),
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
            WriterError::Tags(e) => e.code(),
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
            WriterError::Tags(e) => e.user_message(),
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
pub(crate) enum BookmarkOutcome {
    Added(Box<Article>),
    Updated(Entry),
}

/// One file-scoped write operation (the unit of work is the file; multi-file
/// commands submit N independent requests and cross-file intermediate states are
/// recovery's concern). Each variant mirrors a persistence write function and carries its
/// typed responder.
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
    // A new rename may start while the previous worker is still returning from Finish.
    pub(crate) rename_generations: Arc<std::sync::Mutex<HashMap<String, u64>>>,
    pub(crate) rename_permits: Arc<tokio::sync::Semaphore>,
    pub(crate) rename_tasks: Arc<std::sync::Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    pub(crate) tag_updates: flume::Sender<Result<crate::tags::TagSnapshot, WriterError>>,
    tag_updates_rx: flume::Receiver<Result<crate::tags::TagSnapshot, WriterError>>,
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
                rt.block_on(dispatch_loop(dir, rx, WriteToken::new()));
            })
            .expect("failed to spawn library writer thread");

        let (tag_updates, tag_updates_rx) = flume::unbounded();
        Self {
            rename_generations: Default::default(),
            rename_permits: Arc::new(tokio::sync::Semaphore::new(4)),
            rename_tasks: Default::default(),
            tag_updates,
            tag_updates_rx,
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

    pub(crate) fn library_dir(&self) -> &std::path::Path {
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
        let tasks = std::mem::take(&mut *self.rename_tasks.lock().unwrap());
        for (_, task) in tasks {
            let _ = task.await;
        }
    }
}

#[cfg(test)]
mod resource_lifecycle_tests;

#[cfg(test)]
mod tests;
