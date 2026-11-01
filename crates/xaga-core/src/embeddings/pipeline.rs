use std::collections::HashSet;
use std::sync::Arc;

use sha2::Digest;

use super::db::EmbeddingEntry;
use super::model::Embedder;
use super::row::{article_embed_text, checksum, feed_embed_text, topic_embed_text};
use super::{EmbeddingsDb, EmbeddingsError};
use crate::types::{Annotation, Article, Entry, Feed, Kind, ListItem, Topic};

/// Compares old and new state; sends the entry to the appropriate channels
/// when reading state or color changes.
fn emit_entry_changes(
    entry: &Entry,
    old: Option<&EmbeddingEntry>,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) {
    let old_unread = old.is_some_and(|row| row.is_unread);
    let old_color = old.map(|row| row.color.as_str()).unwrap_or("");
    let new_unread = entry.is_unread;
    let new_color = entry.color.as_deref().unwrap_or("");
    if old_unread != new_unread || old_color != new_color {
        if old_unread != new_unread {
            let _ = unread_tx.send(entry.clone());
        }
        if old_color != new_color {
            let _ = bookmarked_tx.send(entry.clone());
        }
    }
}

async fn compute_vector(
    embedder: &Arc<Embedder>,
    title: String,
    body: String,
) -> Result<Vec<f32>, EmbeddingsError> {
    let embedder_ref = Arc::clone(embedder);
    match tokio::task::spawn_blocking(move || embedder_ref.embed(&title, &body)).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(EmbeddingsError::Model(e.to_string())),
    }
}

/// SHA-256 checksum of an annotation's embeddable content. `pub(crate)` so the recovery
/// diff can compare every annotation row's stored checksum against the `.md`.
pub(crate) fn annotation_checksum(annotation: &Annotation) -> String {
    let mut h = sha2::Sha256::new();
    h.update(annotation.text.join(" ").as_bytes());
    if !annotation.content.is_empty() {
        h.update(b"\n\n");
        h.update(annotation.content.as_bytes());
    }
    hex::encode(h.finalize())
}

/// Builds the LanceDB row for an annotation. `vector` is `None` while the embedding is
/// still pending (row inserted by [`sync_annotations`]); `Some` once `embed_annotations`
/// has computed it. `topics` is the reverse index, owned by the topic membership path — the
/// caller passes the row's *existing* topics forward so this write never clobbers it.
///
/// Each annotation row uses the annotation's own UUID as its DB ID. The title is the
/// highlighted text; the note (if present) is the embedding body. The `reference` column
/// holds the parent article ID; `color` holds the highlight color.
pub(crate) fn annotation_embedding_entry(
    article: &Article,
    annotation: &Annotation,
    topics: Vec<String>,
    vector: Option<Vec<f32>>,
) -> EmbeddingEntry {
    EmbeddingEntry {
        id: annotation.id.clone(),
        kind: Kind::Annotation,
        source: article.meta.url.clone(),
        title: annotation.text.join(" "),
        created_at: annotation.created_at.clone(),
        published_at: article.meta.published_at.clone().unwrap_or_default(),
        updated_at: article.meta.updated_at.clone().unwrap_or_default(),
        checksum: annotation_checksum(annotation),
        reference: article.meta.id.clone(),
        color: annotation.color.clone(),
        summary: String::new(),
        tags: vec![],
        topics,
        items: vec![],
        is_unread: false,
        is_pinned: false,
        vector,
    }
}

/// The live-update payload the UI consumes to add/refresh a saved annotation.
fn annotation_list_item(article: &Article, annotation: &Annotation) -> ListItem {
    ListItem {
        id: annotation.id.clone(),
        title: annotation.text.join(" "),
        created_at: annotation.created_at.clone(),
        url: article.meta.url.clone(),
        is_unread: false,
        is_pinned: false,
        reference: Some(article.meta.id.clone()),
        color: Some(annotation.color.clone()),
    }
}

/// Fast path: reconciles annotation *rows* against `fm` **without** running the embedder, so a
/// newly added annotation is listable immediately (the vector is deferred to
/// `embed_annotations`). Row insert is decoupled from embedding, mirroring entries' [`sync_entry_row`].
///
/// - Deletes rows for annotations removed from the file.
/// - Inserts new / content-changed annotations with a `None` (pending) vector.
/// - Applies a color-only change in place, preserving the existing vector.
///
/// Emits the live `annotation_tx` event for inserted/updated rows.
pub(crate) async fn sync_annotations(
    article: &Article,
    db: &EmbeddingsDb,
    annotation_tx: &flume::Sender<ListItem>,
) -> Result<(), EmbeddingsError> {
    let article_id = &article.meta.id;
    let existing = db.get_annotation_meta_for_article(article_id).await?;

    let current_ids: HashSet<String> = article.annotations.iter().map(|a| a.id.clone()).collect();

    let to_delete: Vec<String> = existing
        .keys()
        .filter(|id| !current_ids.contains(*id))
        .cloned()
        .collect();

    // Re-keyed content: a row deleted in this pass whose checksum matches a current
    // annotation is the same embedder input under a new id (a color change is delete/create).
    // The vector is a pure function of the checksummed content, so it moves to the new row —
    // the embed pass then checksum-skips it and no embedder run happens. `topics` moves with
    // it (the later AddMember writes are idempotent). The pairing is inferred from content
    // identity, not carried symbolically; at design scale the SHA-256 collision rate makes
    // the inference practically guaranteed.
    let current_checksums: HashSet<String> = article
        .annotations
        .iter()
        .map(annotation_checksum)
        .collect();
    let mut carried: std::collections::HashMap<String, (Option<Vec<f32>>, Vec<String>)> =
        std::collections::HashMap::new();
    for id in &to_delete {
        if let Some((stored_checksum, _)) = existing.get(id) {
            if current_checksums.contains(stored_checksum) && !carried.contains_key(stored_checksum)
            {
                if let Some(e) = db.get_entry(id).await? {
                    carried.insert(e.checksum.clone(), (e.vector, e.topics));
                }
            }
        }
    }

    if !to_delete.is_empty() {
        log::debug!(
            "sync_annotations: removing {} stale annotation(s) for {article_id}",
            to_delete.len()
        );
        db.delete_by_ids(&to_delete).await?;
    }

    for annotation in &article.annotations {
        let checksum = annotation_checksum(annotation);

        match existing.get(&annotation.id) {
            Some((stored_checksum, stored_color))
                if stored_checksum == &checksum && stored_color == &annotation.color =>
            {
                // Unchanged — row and vector already current.
                continue;
            }
            Some((stored_checksum, _)) if stored_checksum == &checksum => {
                // Color-only change: update in place, preserving the existing vector — no embed.
                if let Some(mut existing_entry) = db.get_entry(&annotation.id).await? {
                    existing_entry.color = annotation.color.clone();
                    db.upsert(existing_entry).await?;
                    let _ = annotation_tx.send(annotation_list_item(article, annotation));
                }
                continue;
            }
            _ => {}
        }

        // New or content-changed: insert/refresh the row now so it is immediately listable.
        // The vector comes from a same-checksum row deleted above when one exists (re-keyed
        // content — no embed needed); otherwise it is pending (`None`) and embed_vector fills
        // it later. Carry the existing row's topics forward (the topic path owns that column)
        // so this write never clobbers the index.
        let (vector, carried_topics) = match carried.get(&checksum) {
            Some((v, t)) => (v.clone(), Some(t.clone())),
            None => (None, None),
        };
        let topics = if existing.contains_key(&annotation.id) {
            db.find_topic_ids_by_item_id(&annotation.id)
                .await?
                .unwrap_or_default()
        } else {
            carried_topics.unwrap_or_default()
        };
        db.upsert(annotation_embedding_entry(
            article, annotation, topics, vector,
        ))
        .await?;
        let _ = annotation_tx.send(annotation_list_item(article, annotation));
    }

    // Record the entry's annotation membership (in the entry row's `items`) as the *final* step,
    // after the annotation rows above are written. A crash between the row writes and this leaves the
    // rows present and the membership stale, so the next recovery diff flags this entry and
    // re-reconciles it (idempotent — the rows already match the `.md`). No-op if the entry row
    // doesn't exist yet (recovery will create it).
    let member_ids: Vec<String> = article.annotations.iter().map(|a| a.id.clone()).collect();
    if let Some(mut entry_row) = db.get_entry(article_id).await? {
        if entry_row.items != member_ids {
            entry_row.items = member_ids;
            db.upsert(entry_row).await?;
        }
    }

    Ok(())
}

/// Completes pending vectors only for annotation rows whose content still matches
/// this queued snapshot. Row sync/recovery owns creation and metadata; an older
/// embedding job must not recreate deleted rows or overwrite newer content.
async fn embed_annotations<F, Fut>(
    article: &Article,
    db: &EmbeddingsDb,
    mut embed: F,
) -> Result<(), EmbeddingsError>
where
    F: FnMut(String, String) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<f32>, EmbeddingsError>>,
{
    for annotation in &article.annotations {
        let Some(mut row) = db.get_entry(&annotation.id).await? else {
            continue;
        };
        if row.checksum != annotation_checksum(annotation) || row.vector.is_some() {
            continue;
        }

        log::debug!("embed_annotations: embedding annotation {}", annotation.id);
        let vector = embed(annotation.text.join(" "), annotation.content.clone()).await?;
        row.vector = Some(vector);
        db.upsert(row).await?;
    }

    Ok(())
}

/// Fast path (entry): brings the row's metadata, tags, and denormalized state in sync with the
/// carried `Article` *without* running the embedder. The vector is preserved when the content is
/// unchanged, or left `None` (pending) when the row is new or its content changed. Annotation rows
/// are reconciled here too (also embedder-free); their vectors are filled by [`embed_entry_vector`].
pub async fn sync_entry_row(
    article: &Article,
    db: &EmbeddingsDb,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
    annotation_tx: &flume::Sender<ListItem>,
) -> Result<(), EmbeddingsError> {
    sync_entry_row_with_tag_changes(article, db, unread_tx, bookmarked_tx, annotation_tx, None)
        .await
}

pub(crate) async fn sync_entry_row_with_tag_changes(
    article: &Article,
    db: &EmbeddingsDb,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
    annotation_tx: &flume::Sender<ListItem>,
    tag_changes: Option<&std::collections::HashMap<String, bool>>,
) -> Result<(), EmbeddingsError> {
    let id = article.meta.id.clone();

    let existing = db.get_entry(&id).await?;
    let new_checksum = checksum(&article.meta.title, article_embed_text(article));
    let content_unchanged =
        existing.as_ref().map(|e| e.checksum.as_str()) == Some(new_checksum.as_str());

    // Preserve the existing vector only when content is unchanged; otherwise leave it
    // pending so the embed pass recomputes it.
    let vector = if content_unchanged {
        existing.as_ref().and_then(|e| e.vector.clone())
    } else {
        None
    };

    let tags = match &article.meta.tags {
        Some(names) => db.resolve_tag_names(names).await?,
        None => existing
            .as_ref()
            .map(|row| row.tags.clone())
            .unwrap_or_default(),
    };
    let mut row = EmbeddingEntry::from_article(article, vector, tags);
    if let Some(changes) = tag_changes {
        for (id, assigned) in changes {
            row.tags.retain(|existing| existing != id);
            if *assigned {
                row.tags.push(id.clone());
            }
        }
    }

    // Write only when something the row stores actually changed.
    let changed = match &existing {
        None => true,
        Some(e) => {
            !content_unchanged
                || e.tags != row.tags
                || e.color != row.color
                || e.is_unread != row.is_unread
                || e.summary != row.summary
                || e.is_pinned != row.is_pinned
        }
    };

    if changed {
        // Carry the existing row's topics + items forward so this write never clobbers the reverse
        // index or the recorded annotation membership — for entry rows `items` holds that membership,
        // owned by the annotation reconcile path, not this one.
        row.topics = existing
            .as_ref()
            .map(|e| e.topics.clone())
            .unwrap_or_default();
        row.items = existing
            .as_ref()
            .map(|e| e.items.clone())
            .unwrap_or_default();
        db.upsert(row).await?;
        emit_entry_changes(&article.meta, existing.as_ref(), unread_tx, bookmarked_tx);
    }

    // Matching membership alone does not prove matching annotation content (e.g.
    // a note edited outside the app). Reconcile rows before queuing vector work;
    // vector completion must never own creation or restore an older snapshot.
    if !article.annotations.is_empty() || existing.as_ref().is_some_and(|row| !row.items.is_empty())
    {
        sync_annotations(article, db, annotation_tx).await?;
    }

    Ok(())
}

/// Fast path (topic): syncs the topic's row from the carried `Topic` (embedder-free).
pub async fn sync_topic_row(topic: &Topic, db: &EmbeddingsDb) -> Result<(), EmbeddingsError> {
    let existing = db.get_entry(&topic.id).await?;
    let new_checksum = checksum(&topic.title, topic_embed_text(topic));
    let content_unchanged =
        existing.as_ref().map(|e| e.checksum.as_str()) == Some(new_checksum.as_str());

    let vector = if content_unchanged {
        existing.as_ref().and_then(|e| e.vector.clone())
    } else {
        None
    };

    let mut row = EmbeddingEntry::from_topic(topic, vector);

    let changed = match &existing {
        None => true,
        Some(e) => {
            !content_unchanged
                || e.tags != row.tags
                || e.summary != row.summary
                || e.is_pinned != row.is_pinned
        }
    };

    if changed {
        row.topics = existing
            .as_ref()
            .map(|e| e.topics.clone())
            .unwrap_or_default();
        row.items = existing
            .as_ref()
            .map(|e| e.items.clone())
            .unwrap_or_default();
        db.upsert(row).await?;
    }

    Ok(())
}

/// Slow path (entry): computes and stores the entry's embedding vector when missing or stale,
/// then fills any pending annotation vectors. Runs the embedder.
pub async fn embed_entry_vector(
    article: &Article,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let id = article.meta.id.clone();

    let existing = db.get_entry(&id).await?;
    let new_checksum = checksum(&article.meta.title, article_embed_text(article));
    let needs_vector = match &existing {
        None => true,
        Some(e) => e.vector.is_none() || e.checksum != new_checksum,
    };

    if needs_vector {
        log::debug!(
            "embed_entry_vector: embedding id={id} title={:?}",
            article.meta.title
        );
        let vector = compute_vector(
            embedder,
            article.meta.title.clone(),
            article_embed_text(article).to_string(),
        )
        .await?;
        store_entry_vector(article, vector, existing.as_ref(), db).await?;
    }

    // Fill pending annotation vectors only where the row still matches this snapshot.
    // Reconciliation owns row creation and deletion, including during recovery.
    embed_annotations(article, db, |title, body| {
        compute_vector(embedder, title, body)
    })
    .await?;

    Ok(())
}

/// Vector completion does not own state: a higher-priority state mutation may have
/// overtaken the Article snapshot carried by this queued embedding job.
async fn store_entry_vector(
    article: &Article,
    vector: Vec<f32>,
    existing: Option<&EmbeddingEntry>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let tags = match existing {
        Some(row) => row.tags.clone(),
        None => {
            db.resolve_tag_names(article.meta.tags.as_deref().unwrap_or_default())
                .await?
        }
    };
    let mut row = EmbeddingEntry::from_article(article, Some(vector), tags);
    if let Some(existing) = existing {
        row.color = existing.color.clone();
        row.is_unread = existing.is_unread;
        row.is_pinned = existing.is_pinned;
        row.topics = existing.topics.clone();
        row.items = existing.items.clone();
    }
    db.upsert(row).await
}

/// Slow path (topic): computes and stores the topic's embedding vector when missing or stale.
pub async fn embed_topic_vector(
    topic: &Topic,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let existing = db.get_entry(&topic.id).await?;
    let new_checksum = checksum(&topic.title, topic_embed_text(topic));
    let needs_vector = match &existing {
        None => true,
        Some(e) => e.vector.is_none() || e.checksum != new_checksum,
    };

    if needs_vector {
        log::debug!(
            "embed_topic_vector: embedding id={} title={:?}",
            topic.id,
            topic.title
        );
        let vector = compute_vector(
            embedder,
            topic.title.clone(),
            topic_embed_text(topic).to_string(),
        )
        .await?;
        let mut row = EmbeddingEntry::from_topic(topic, Some(vector));
        row.topics = existing
            .as_ref()
            .map(|e| e.topics.clone())
            .unwrap_or_default();
        row.items = existing
            .as_ref()
            .map(|e| e.items.clone())
            .unwrap_or_default();
        db.upsert(row).await?;
    }

    Ok(())
}

/// Number of member rows written per batch in the reverse-index fan-out. A bulk membership
/// change (topic delete, bulk import, fresh-recovery rebuild of a large topic) is processed
/// `MEMBERSHIP_CHUNK` members at a time rather than one upsert per member — bounding fragment
/// churn and the work done per loop turn. Matches `bulk_insert`'s batch size.
const MEMBERSHIP_CHUNK: usize = 1024;

/// Reconciles one topic's member→topic reverse index. Diffs `member_ids` (the topic's members from
/// its `.md`) against the topic row's recorded `items` (the last-propagated base), then adds/removes
/// `topic_id` on the **delta** members' `topics` columns and updates `items` to the propagated set so
/// the next diff is correct. Work is bounded by the delta, not the table size.
///
/// The delta is written in chunks of `MEMBERSHIP_CHUNK` — each chunk is one `delete_by_ids` +
/// `add_batch` (not a per-row upsert), and the topic-row `items` base is **checkpointed after each
/// chunk**. So a crash mid-fan-out resumes from the remaining members (the next diff sees the
/// advanced base) instead of re-scanning the whole membership, and large changes don't explode
/// fragments or block the loop. `delete_by_ids`+`add_batch` was chosen over `merge_insert` because
/// it preserves `upsert`'s duplicate self-healing (a duplicated row is deleted with its id and
/// re-added once). Crash window: a hard kill between a chunk's delete and its add loses that
/// chunk's member rows until a later sync/recovery heals them (consequence of delete-then-add).
///
/// This is the sole writer of the `topics` (member) and `items` (topic) columns — entry/annotation
/// sync carry them forward untouched. Members whose rows don't exist yet are skipped (reconciled on a
/// later run once they exist). Member-type-agnostic — operates on whatever ids the caller passes
/// (entries **and** annotations); annotation rows exist by this point (created by `bulk_insert` on
/// fresh recovery, `sync_annotations` on live edits).
pub(crate) async fn reconcile_topic_membership(
    db: &EmbeddingsDb,
    topic_id: &str,
    member_ids: &[String],
) -> Result<(), EmbeddingsError> {
    reconcile_topic_membership_chunked(db, topic_id, member_ids, MEMBERSHIP_CHUNK).await
}

/// Core of [`reconcile_topic_membership`] with an injectable `chunk_size`, so tests can exercise the
/// multi-chunk and resume paths with a handful of rows instead of thousands. Production always calls
/// it with `MEMBERSHIP_CHUNK`.
pub(crate) async fn reconcile_topic_membership_chunked(
    db: &EmbeddingsDb,
    topic_id: &str,
    member_ids: &[String],
    chunk_size: usize,
) -> Result<(), EmbeddingsError> {
    let prior: Vec<String> = db
        .find_item_ids_by_topic_id(topic_id)
        .await?
        .unwrap_or_default();
    let new_set: HashSet<&str> = member_ids.iter().map(|s| s.as_str()).collect();
    let prior_set: HashSet<&str> = prior.iter().map(|s| s.as_str()).collect();

    let added: Vec<&str> = member_ids
        .iter()
        .map(|s| s.as_str())
        .filter(|id| !prior_set.contains(id))
        .collect();
    let removed: Vec<&str> = prior
        .iter()
        .map(|s| s.as_str())
        .filter(|id| !new_set.contains(id))
        .collect();
    if added.is_empty() && removed.is_empty() {
        return Ok(());
    }

    // Running base, persisted after each chunk so a crash resumes from the remainder. Starts at the
    // prior membership and ends at the file membership; intermediate values are valid because the
    // base is only ever diffed as a set on the next reconcile (order/content of `items` is not relied
    // on for anything else).
    let mut base: Vec<String> = prior.clone();

    for chunk in added.chunks(chunk_size) {
        write_topic_delta(db, topic_id, chunk, true).await?;
        base.extend(chunk.iter().map(|id| id.to_string()));
        set_topic_base(db, topic_id, &base).await?;
    }
    for chunk in removed.chunks(chunk_size) {
        write_topic_delta(db, topic_id, chunk, false).await?;
        let drop: HashSet<&str> = chunk.iter().copied().collect();
        base.retain(|id| !drop.contains(id.as_str()));
        set_topic_base(db, topic_id, &base).await?;
    }

    Ok(())
}

/// Adds (`add = true`) or removes (`add = false`) `topic_id` on a chunk of member rows in a single
/// `delete_by_ids` + `add_batch`. Only rows that actually change are rewritten; members already in
/// the desired state are skipped (covers a resume after a crash between a chunk's add and its base
/// checkpoint). Members whose rows don't exist are skipped.
async fn write_topic_delta(
    db: &EmbeddingsDb,
    topic_id: &str,
    member_ids: &[&str],
    add: bool,
) -> Result<(), EmbeddingsError> {
    let mut ids: Vec<String> = Vec::with_capacity(member_ids.len());
    let mut rows: Vec<EmbeddingEntry> = Vec::with_capacity(member_ids.len());
    for member_id in member_ids {
        let Some(mut m) = db.get_entry(member_id).await? else {
            continue;
        };
        let has = m.topics.iter().any(|t| t == topic_id);
        if add && !has {
            m.topics.push(topic_id.to_string());
        } else if !add && has {
            m.topics.retain(|t| t != topic_id);
        } else {
            continue; // already in the desired state
        }
        ids.push((*member_id).to_string());
        rows.push(m);
    }
    if rows.is_empty() {
        return Ok(());
    }
    db.delete_by_ids(&ids).await?;
    db.add_batch(rows).await?;
    Ok(())
}

/// Persists the topic row's `items` base (the prior-membership base for the next diff). No-op if the
/// topic row doesn't exist (nothing to anchor the base on).
async fn set_topic_base(
    db: &EmbeddingsDb,
    topic_id: &str,
    base: &[String],
) -> Result<(), EmbeddingsError> {
    if let Some(mut t) = db.get_entry(topic_id).await? {
        t.items = base.to_vec();
        db.upsert(t).await?;
    }
    Ok(())
}

/// Builds an `Entry` (the UI-facing shape) from a cached row, for change-event emission.
fn entry_from_row(row: &EmbeddingEntry) -> Entry {
    let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
    Entry {
        tags: None,
        id: row.id.clone(),
        url: row.source.clone(),
        title: row.title.clone(),
        created_at: row.created_at.clone(),
        published_at: opt(&row.published_at),
        updated_at: opt(&row.updated_at),
        description: opt(&row.summary),
        is_unread: row.is_unread,

        is_pinned: row.is_pinned,
        color: opt(&row.color),
    }
}

/// Write-through state update for a `SyncState` job: applies the carried value(s) directly
/// to the cached row's reading state, color, and derived pin columns and emits the UI change event,
/// without reading the `.md`. `get_entry` is still required because `upsert` replaces the
/// whole row (preserving vector/topics/items/content). A `None` field is left unchanged; a
/// missing row is a no-op (recovery will create it).
pub(crate) async fn apply_state(
    db: &EmbeddingsDb,
    id: &str,
    is_unread: Option<bool>,
    color: Option<Option<String>>,
    is_pinned: Option<bool>,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) -> Result<(), EmbeddingsError> {
    let Some(mut row) = db.get_entry(id).await? else {
        log::warn!("apply_state: row not found id={id} (recovery will create it)");
        return Ok(());
    };

    let old_unread = row.is_unread;
    let old_color = row.color.clone();

    if let Some(v) = is_unread {
        row.is_unread = v;
    }
    if let Some(value) = color {
        row.color = value.unwrap_or_default();
    }
    if let Some(v) = is_pinned {
        row.is_pinned = v;
    }

    let new_unread = row.is_unread;
    let new_color = row.color.clone();
    let entry = entry_from_row(&row);
    db.upsert(row).await?;

    if new_unread != old_unread {
        let _ = unread_tx.send(entry.clone());
    }
    if new_color != old_color {
        let _ = bookmarked_tx.send(entry);
    }
    Ok(())
}

/// Apply the carried assignment delta without replacing content or other state.
pub(crate) async fn apply_entry_tag(
    db: &EmbeddingsDb,
    change: &crate::tags::EntryTagChange,
) -> Result<(), EmbeddingsError> {
    let Some(mut row) = db.get_entry(&change.entry_id).await? else {
        return Ok(()); // Recovery reconstructs missing rows from source files.
    };
    row.tags.retain(|id| id != &change.tag_id);
    if change.assigned {
        row.tags.push(change.tag_id.clone());
    }
    db.upsert(row).await
}

/// Precise single-member reverse-index add: sets `topic_id` on `member_id`'s `topics`
/// column and advances the topic row's `items` base so a later full reconcile sees no
/// spurious delta. The O(1) counterpart to [`reconcile_topic_membership`] for a live add.
pub(crate) async fn add_topic_member(
    db: &EmbeddingsDb,
    topic_id: &str,
    member_id: &str,
) -> Result<(), EmbeddingsError> {
    write_topic_delta(db, topic_id, &[member_id], true).await?;
    let mut base = db
        .find_item_ids_by_topic_id(topic_id)
        .await?
        .unwrap_or_default();
    if !base.iter().any(|m| m == member_id) {
        base.push(member_id.to_string());
        set_topic_base(db, topic_id, &base).await?;
    }
    Ok(())
}

/// Precise single-member reverse-index remove: clears `topic_id` from `member_id`'s
/// `topics` column and drops it from the topic row's `items` base.
pub(crate) async fn remove_topic_member(
    db: &EmbeddingsDb,
    topic_id: &str,
    member_id: &str,
) -> Result<(), EmbeddingsError> {
    write_topic_delta(db, topic_id, &[member_id], false).await?;
    let base = db
        .find_item_ids_by_topic_id(topic_id)
        .await?
        .unwrap_or_default();
    if base.iter().any(|m| m == member_id) {
        let new_base: Vec<String> = base.into_iter().filter(|m| m != member_id).collect();
        set_topic_base(db, topic_id, &new_base).await?;
    }
    Ok(())
}

/// Embeds (or refreshes) one feed's row from the domain [`Feed`] carried to it — no file read.
/// Checksum-gated: a no-op when the row already has a vector and its title/description checksum and
/// `is_pinned` are unchanged. The `.atom` read and stale-row cleanup live in recovery (the bridge).
pub(crate) async fn embed_feed(
    feed: &Feed,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let new_checksum = checksum(&feed.title, feed_embed_text(feed));
    let existing = db.get_entry(&feed.id).await?;
    let up_to_date = matches!(
        &existing,
        Some(e) if e.vector.is_some() && e.checksum == new_checksum && e.is_pinned == feed.is_pinned
    );
    if up_to_date {
        return Ok(());
    }

    log::debug!(
        "embed_feed: embedding feed id={} title={:?}",
        feed.id,
        feed.title
    );
    let vector = compute_vector(
        embedder,
        feed.title.clone(),
        feed_embed_text(feed).to_string(),
    )
    .await?;
    db.upsert(EmbeddingEntry::from_feed(feed, Some(vector)))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn annotation(id: &str, color: &str) -> Annotation {
        Annotation {
            id: id.to_string(),
            title: "highlighted".to_string(),
            reference: "article-1".to_string(),
            text: vec!["highlighted".to_string()],
            occurrence: vec![0],
            color: color.to_string(),
            content: "a note".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn article(annotations: Vec<Annotation>) -> Article {
        Article {
            meta: Entry {
                tags: None,
                id: "article-1".to_string(),
                url: "https://example.com/a".to_string(),
                title: "Title".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                published_at: None,
                updated_at: None,
                description: None,
                is_unread: false,
                is_pinned: false,
                color: None,
            },
            content: "body".to_string(),
            annotations,
            favicon: None,
        }
    }

    #[tokio::test]
    async fn tag_assignment_delta_preserves_vectors_state_and_rebuilds_from_source_labels() {
        let dir = tempfile::tempdir().unwrap();
        let writer = crate::library_writer::LibraryWriter::start(
            dir.path().into(),
            crate::library::acquire_lock(dir.path()).unwrap(),
        );
        let registry = writer
            .discover_tags(vec!["Research".into(), "Work".into()])
            .await
            .unwrap();
        let db = EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let mut source = article(vec![]);
        source.meta.tags = Some(vec!["Research".into()]);
        source.meta.is_unread = true;
        source.meta.color = Some("blue".into());
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        sync_entry_row(&source, &db, &events, &events, &annotations)
            .await
            .unwrap();
        let mut row = db.get_entry(&source.meta.id).await.unwrap().unwrap();
        row.vector = Some(vec![0.5; 384]);
        db.upsert(row).await.unwrap();
        let row = db.get_entry(&source.meta.id).await.unwrap().unwrap();
        let change = crate::tags::EntryTagChange {
            entry_id: source.meta.id.clone(),
            tag_id: registry.tags[1].id.clone(),
            assigned: true,
        };
        apply_entry_tag(&db, &change).await.unwrap();
        apply_entry_tag(&db, &change).await.unwrap();
        let after = db.get_entry(&source.meta.id).await.unwrap().unwrap();
        assert_eq!(
            after.tags,
            registry
                .tags
                .iter()
                .map(|t| t.id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(after.vector, row.vector);
        assert_eq!(after.checksum, row.checksum);
        assert_eq!(after.is_unread, row.is_unread);
        assert_eq!(after.color, row.color);
        let changes = std::collections::HashMap::from([(change.tag_id.clone(), true)]);
        sync_entry_row_with_tag_changes(
            &source,
            &db,
            &events,
            &events,
            &annotations,
            Some(&changes),
        )
        .await
        .unwrap();
        assert!(
            db.get_entry(&source.meta.id)
                .await
                .unwrap()
                .unwrap()
                .tags
                .contains(&change.tag_id)
        );

        apply_entry_tag(
            &db,
            &crate::tags::EntryTagChange {
                assigned: false,
                ..change
            },
        )
        .await
        .unwrap();
        assert_eq!(
            db.get_entry(&source.meta.id).await.unwrap().unwrap().tags,
            row.tags
        );
        source.meta.tags = Some(vec!["Work".into()]);
        sync_entry_row(&source, &db, &events, &events, &annotations)
            .await
            .unwrap();
        assert_eq!(
            db.get_entry(&source.meta.id).await.unwrap().unwrap().tags,
            vec![registry.tags[1].id.clone()]
        );
        source.meta.tags = Some(vec![]);
        sync_entry_row(&source, &db, &events, &events, &annotations)
            .await
            .unwrap();
        assert!(
            db.get_entry(&source.meta.id)
                .await
                .unwrap()
                .unwrap()
                .tags
                .is_empty()
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_pending_vector_completion_preserves_newer_assignments() {
        let dir = tempfile::tempdir().unwrap();
        let writer = crate::library_writer::LibraryWriter::start(
            dir.path().into(),
            crate::library::acquire_lock(dir.path()).unwrap(),
        );
        let registry = writer
            .discover_tags(vec!["Old".into(), "New".into()])
            .await
            .unwrap();
        let db = EmbeddingsDb::open_writer(dir.path(), &writer.lock())
            .await
            .unwrap();
        let mut queued = article(vec![]);
        queued.meta.tags = Some(vec!["Old".into()]);
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        sync_entry_row(&queued, &db, &events, &events, &annotations)
            .await
            .unwrap();
        let mut latest = db.get_entry(&queued.meta.id).await.unwrap().unwrap();
        latest.tags = registry.resolver().resolve(&["New".into()]).unwrap();
        db.upsert(latest).await.unwrap();
        let current = db.get_entry(&queued.meta.id).await.unwrap().unwrap();
        store_entry_vector(&queued, vec![0.5; 384], Some(&current), &db)
            .await
            .unwrap();
        let completed = db.get_entry(&queued.meta.id).await.unwrap().unwrap();
        assert_eq!(
            completed.tags,
            registry.resolver().resolve(&["New".into()]).unwrap()
        );
        assert_eq!(completed.vector, Some(vec![0.5; 384]));
        let mut summary = queued.clone();
        summary.meta.tags = None;
        sync_entry_row(&summary, &db, &events, &events, &annotations)
            .await
            .unwrap();
        assert_eq!(
            db.get_entry(&queued.meta.id).await.unwrap().unwrap().tags,
            completed.tags
        );
        summary.meta.tags = Some(Vec::new());
        sync_entry_row(&summary, &db, &events, &events, &annotations)
            .await
            .unwrap();
        assert!(
            db.get_entry(&queued.meta.id)
                .await
                .unwrap()
                .unwrap()
                .tags
                .is_empty()
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn pending_vector_completion_preserves_newer_color_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let queued = article(vec![]);
        let (events, _) = flume::unbounded();
        let (annotations, _) = flume::unbounded();
        sync_entry_row(&queued, &db, &events, &events, &annotations)
            .await
            .unwrap();
        for color in [Some("blue"), Some("green"), None] {
            apply_state(
                &db,
                &queued.meta.id,
                Some(true),
                Some(color.map(str::to_owned)),
                Some(true),
                &events,
                &events,
            )
            .await
            .unwrap();
            let current = db.get_entry(&queued.meta.id).await.unwrap().unwrap();
            store_entry_vector(&queued, vec![0.1; 384], Some(&current), &db)
                .await
                .unwrap();
            let row = db.get_entry(&queued.meta.id).await.unwrap().unwrap();
            assert_eq!(row.color, color.unwrap_or(""));
            assert!(row.is_unread && row.is_pinned);
            assert!(row.vector.is_some());
        }
    }

    #[tokio::test]
    async fn queued_annotation_embedding_does_not_recreate_deleted_annotation() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let queued = article(vec![annotation("deleted", "yellow")]);
        let (tx, _) = flume::unbounded();
        let (state_tx, _) = flume::unbounded();
        sync_entry_row(&queued, &db, &state_tx, &state_tx, &tx)
            .await
            .unwrap();
        sync_annotations(&article(vec![]), &db, &tx).await.unwrap();

        embed_annotations(&queued, &db, |_, _| async {
            panic!("a deleted annotation must not be embedded")
        })
        .await
        .unwrap();

        assert!(db.get_entry("deleted").await.unwrap().is_none());
        assert!(
            db.get_entry("article-1")
                .await
                .unwrap()
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn queued_annotation_embedding_does_not_overwrite_newer_content() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let queued = article(vec![annotation("edited", "yellow")]);
        let mut current = queued.clone();
        current.annotations[0].content = "newer note".into();
        let (tx, _) = flume::unbounded();
        sync_annotations(&current, &db, &tx).await.unwrap();

        embed_annotations(&queued, &db, |_, _| async {
            panic!("a superseded annotation snapshot must not be embedded")
        })
        .await
        .unwrap();

        let row = db.get_entry("edited").await.unwrap().unwrap();
        assert_eq!(row.checksum, annotation_checksum(&current.annotations[0]));
        assert!(row.vector.is_none());
    }

    #[tokio::test]
    async fn queued_annotation_embedding_preserves_current_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let queued = article(vec![annotation("pending", "yellow")]);
        let mut current = queued.clone();
        current.annotations[0].color = "blue".into();
        db.upsert(annotation_embedding_entry(
            &current,
            &current.annotations[0],
            vec!["topic".into()],
            None,
        ))
        .await
        .unwrap();

        embed_annotations(&queued, &db, |_, _| async { Ok(vec![0.1; 384]) })
            .await
            .unwrap();

        let row = db.get_entry("pending").await.unwrap().unwrap();
        assert_eq!(row.vector, Some(vec![0.1; 384]));
        assert_eq!(row.color, "blue");
        assert_eq!(row.topics, vec!["topic"]);
    }

    #[tokio::test]
    async fn row_sync_reconciles_note_changes_when_membership_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();
        let mut current = article(vec![annotation("edited", "yellow")]);
        let (state_tx, _) = flume::unbounded();
        let (annotation_tx, _) = flume::unbounded();
        sync_entry_row(&current, &db, &state_tx, &state_tx, &annotation_tx)
            .await
            .unwrap();
        current.annotations[0].content = "a note edited outside the app".into();
        sync_entry_row(&current, &db, &state_tx, &state_tx, &annotation_tx)
            .await
            .unwrap();
        embed_annotations(&current, &db, |_, body| async move {
            assert_eq!(body, "a note edited outside the app");
            Ok(vec![0.1; 384])
        })
        .await
        .unwrap();
        let row = db.get_entry("edited").await.unwrap().unwrap();
        assert_eq!(row.checksum, annotation_checksum(&current.annotations[0]));
        assert_eq!(row.vector, Some(vec![0.1; 384]));
    }

    /// A recolor is delete/create under a new id with unchanged text — the deleted row's
    /// vector and `topics` move to the new row (checksum match), so the embed pass has
    /// nothing to do.
    #[tokio::test]
    async fn sync_annotations_carries_vector_and_topics_to_rekeyed_row() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let old = annotation("old-id", "yellow");
        let vector = vec![0.5f32; 384];
        db.upsert(annotation_embedding_entry(
            &article(vec![old.clone()]),
            &old,
            vec!["topic-1".to_string()],
            Some(vector.clone()),
        ))
        .await
        .unwrap();

        // The `.md` now holds the same content under a new id with a new color.
        let new = annotation("new-id", "red");
        let (tx, _rx) = flume::unbounded::<ListItem>();
        sync_annotations(&article(vec![new]), &db, &tx)
            .await
            .unwrap();

        assert!(db.get_entry("old-id").await.unwrap().is_none());
        let row = db.get_entry("new-id").await.unwrap().unwrap();
        assert_eq!(row.vector, Some(vector), "vector moved, not re-embedded");
        assert_eq!(row.topics, vec!["topic-1".to_string()], "topics moved");
        assert_eq!(row.color, "red");
    }

    /// Genuinely new content gets no carried vector — it stays pending for the embed pass.
    #[tokio::test]
    async fn sync_annotations_leaves_new_content_pending() {
        let dir = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(dir.path()).await.unwrap();

        let mut new = annotation("new-id", "red");
        new.text = vec!["different text".to_string()];
        new.title = "different text".to_string();
        let (tx, _rx) = flume::unbounded::<ListItem>();
        sync_annotations(&article(vec![new]), &db, &tx)
            .await
            .unwrap();

        let row = db.get_entry("new-id").await.unwrap().unwrap();
        assert_eq!(row.vector, None);
    }
}
