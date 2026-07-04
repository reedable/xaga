use std::collections::HashSet;
use std::sync::Arc;

use sha2::Digest;

use crate::types::{Annotation, Article, Entry, Feed, Kind, ListItem, Topic};
use super::db::EmbeddingEntry;
use super::model::Embedder;
use super::row::{article_embed_text, checksum, feed_embed_text, topic_embed_text};
use super::{EmbeddingsDb, EmbeddingsError};

/// Compares old and new tag state; sends the entry to the appropriate channels
/// when is_unread or is_bookmarked actually changed.
fn emit_entry_changes(
    entry: &Entry,
    old_tags: Option<&[String]>,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) {
    let old_unread = old_tags.map_or(false, crate::tags::is_unread);
    let old_bookmarked = old_tags.map_or(false, crate::tags::is_bookmarked);
    let new_unread = entry.is_unread;
    let new_bookmarked = entry.is_bookmarked;
    if old_unread != new_unread || old_bookmarked != new_bookmarked {
        if old_unread != new_unread {
            let _ = unread_tx.send(entry.clone());
        }
        if old_bookmarked != new_bookmarked {
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
        is_bookmarked: false,
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
        is_bookmarked: false,
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
            if current_checksums.contains(stored_checksum)
                && !carried.contains_key(stored_checksum)
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
            db.get_entry(&annotation.id).await?.map(|e| e.topics).unwrap_or_default()
        } else {
            carried_topics.unwrap_or_default()
        };
        db.upsert(annotation_embedding_entry(article, annotation, topics, vector))
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

/// Slow path: computes and stores embedding vectors for this article's annotation rows whose
/// vector is missing or stale. Also creates any annotation row that does not yet exist as a
/// safety net — annotation rows are normally created earlier (fresh recovery via `bulk_insert`,
/// live edits via `sync_annotations`). Runs the embedder.
async fn embed_annotations(
    article: &Article,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    for annotation in &article.annotations {
        let checksum = annotation_checksum(annotation);
        let existing = db.get_entry(&annotation.id).await?;
        let needs_vector = match &existing {
            None => true,
            Some(e) => e.vector.is_none() || e.checksum != checksum,
        };
        if !needs_vector {
            continue;
        }

        log::debug!("embed_annotations: embedding annotation {}", annotation.id);
        let title = annotation.text.join(" ");
        let body = annotation.content.clone();
        let vector = compute_vector(embedder, title, body).await?;
        // Carry the existing row's topics forward — the topic path owns that column.
        let topics = existing.map(|e| e.topics).unwrap_or_default();
        db.upsert(annotation_embedding_entry(article, annotation, topics, Some(vector)))
            .await?;
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

    let mut row = EmbeddingEntry::from_article(article, vector);

    // Write only when something the row stores actually changed.
    let changed = match &existing {
        None => true,
        Some(e) => !content_unchanged || e.tags != row.tags || e.summary != row.summary,
    };

    if changed {
        let old_tags = existing.as_ref().map(|e| e.tags.clone());
        // Carry the existing row's topics + items forward so this write never clobbers the reverse
        // index or the recorded annotation membership — for entry rows `items` holds that membership,
        // owned by the annotation reconcile path, not this one.
        row.topics = existing.as_ref().map(|e| e.topics.clone()).unwrap_or_default();
        row.items = existing.as_ref().map(|e| e.items.clone()).unwrap_or_default();
        db.upsert(row).await?;
        emit_entry_changes(&article.meta, old_tags.as_deref(), unread_tx, bookmarked_tx);
    }

    // Reconcile annotation rows (embedder-free) only when the recorded membership (the entry row's
    // `items`) differs from the `.md` — the entry row we just read carries it, so this diff costs no
    // query. Unchanged membership means the annotation rows already match; skip the reconcile.
    let recorded: HashSet<&str> = existing
        .as_ref()
        .map(|e| e.items.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();
    let current: HashSet<&str> = article.annotations.iter().map(|a| a.id.as_str()).collect();
    if recorded != current {
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
        Some(e) => !content_unchanged || e.tags != row.tags || e.summary != row.summary,
    };

    if changed {
        row.topics = existing.as_ref().map(|e| e.topics.clone()).unwrap_or_default();
        row.items = existing.as_ref().map(|e| e.items.clone()).unwrap_or_default();
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
        log::debug!("embed_entry_vector: embedding id={id} title={:?}", article.meta.title);
        let vector = compute_vector(
            embedder,
            article.meta.title.clone(),
            article_embed_text(article).to_string(),
        )
        .await?;
        let mut row = EmbeddingEntry::from_article(article, Some(vector));
        row.topics = existing.as_ref().map(|e| e.topics.clone()).unwrap_or_default();
        row.items = existing.as_ref().map(|e| e.items.clone()).unwrap_or_default();
        db.upsert(row).await?;
    }

    // Fill any pending annotation vectors. Rows (incl. stale-row deletes) were already
    // reconciled by sync_annotations; this also creates rows the recovery path skipped.
    embed_annotations(article, embedder, db).await?;

    Ok(())
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
        log::debug!("embed_topic_vector: embedding id={} title={:?}", topic.id, topic.title);
        let vector = compute_vector(
            embedder,
            topic.title.clone(),
            topic_embed_text(topic).to_string(),
        )
        .await?;
        let mut row = EmbeddingEntry::from_topic(topic, Some(vector));
        row.topics = existing.as_ref().map(|e| e.topics.clone()).unwrap_or_default();
        row.items = existing.as_ref().map(|e| e.items.clone()).unwrap_or_default();
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
    let prior: Vec<String> = db.get_entry(topic_id).await?.map(|t| t.items).unwrap_or_default();
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
        let Some(mut m) = db.get_entry(member_id).await? else { continue };
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
        id: row.id.clone(),
        url: row.source.clone(),
        title: row.title.clone(),
        created_at: row.created_at.clone(),
        published_at: opt(&row.published_at),
        updated_at: opt(&row.updated_at),
        description: opt(&row.summary),
        is_unread: row.is_unread,
        is_bookmarked: row.is_bookmarked,
        color: None,
    }
}

/// Write-through state update for a `SyncState` job: applies the carried value(s) directly
/// to the cached row's `tags` + denormalized boolean columns and emits the UI change event,
/// without reading the `.md`. `get_entry` is still required because `upsert` replaces the
/// whole row (preserving vector/topics/items/content). A `None` field is left unchanged; a
/// missing row is a no-op (recovery will create it).
pub(crate) async fn apply_state(
    db: &EmbeddingsDb,
    id: &str,
    is_unread: Option<bool>,
    is_bookmarked: Option<bool>,
    is_pinned: Option<bool>,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) -> Result<(), EmbeddingsError> {
    let Some(mut row) = db.get_entry(id).await? else {
        log::warn!("apply_state: row not found id={id} (recovery will create it)");
        return Ok(());
    };

    let old_unread = row.is_unread;
    let old_bookmarked = row.is_bookmarked;

    fn set_tag(tags: &mut Vec<String>, tag: &str, on: bool) {
        let has = tags.iter().any(|t| t == tag);
        if on && !has {
            tags.push(tag.to_string());
        } else if !on && has {
            tags.retain(|t| t != tag);
        }
    }
    if let Some(v) = is_unread {
        set_tag(&mut row.tags, crate::tags::UNREAD, v);
        row.is_unread = v;
    }
    if let Some(v) = is_bookmarked {
        set_tag(&mut row.tags, crate::tags::BOOKMARK, v);
        row.is_bookmarked = v;
    }
    if let Some(v) = is_pinned {
        set_tag(&mut row.tags, crate::tags::PINNED, v);
        row.is_pinned = v;
    }

    let new_unread = row.is_unread;
    let new_bookmarked = row.is_bookmarked;
    let entry = entry_from_row(&row);
    db.upsert(row).await?;

    if new_unread != old_unread {
        let _ = unread_tx.send(entry.clone());
    }
    if new_bookmarked != old_bookmarked {
        let _ = bookmarked_tx.send(entry);
    }
    Ok(())
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
    let mut base = db.get_entry(topic_id).await?.map(|t| t.items).unwrap_or_default();
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
    let base = db.get_entry(topic_id).await?.map(|t| t.items).unwrap_or_default();
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

    log::debug!("embed_feed: embedding feed id={} title={:?}", feed.id, feed.title);
    let vector =
        compute_vector(embedder, feed.title.clone(), feed_embed_text(feed).to_string()).await?;
    db.upsert(EmbeddingEntry::from_feed(feed, Some(vector))).await?;
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
                id: "article-1".to_string(),
                url: "https://example.com/a".to_string(),
                title: "Title".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                published_at: None,
                updated_at: None,
                description: None,
                is_unread: false,
                is_bookmarked: false,
                color: None,
            },
            content: "body".to_string(),
            annotations,
            favicon: None,
        }
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
        sync_annotations(&article(vec![new]), &db, &tx).await.unwrap();

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
        sync_annotations(&article(vec![new]), &db, &tx).await.unwrap();

        let row = db.get_entry("new-id").await.unwrap().unwrap();
        assert_eq!(row.vector, None);
    }
}
