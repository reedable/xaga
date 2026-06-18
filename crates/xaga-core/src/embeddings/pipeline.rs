use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use sha2::Digest;

use crate::types::{Annotation, Entry, Kind, SavedItem};
use crate::util::Frontmatter;
use super::db::EmbeddingEntry;
use super::model::Embedder;
use super::{EmbeddingsDb, EmbeddingsError};

/// Constructs an `Entry` from a frontmatter reference without consuming it.
fn entry_from_fm_ref(id: &str, fm: &Frontmatter) -> Entry {
    Entry {
        id: id.to_string(),
        url: fm.source.clone(),
        title: fm.title.clone(),
        created_at: fm.created_at.clone(),
        published_at: fm.published_at.clone(),
        updated_at: fm.updated_at.clone(),
        summary: fm.summary.clone(),
        is_unread: fm.tags.contains(&"unread".to_string()),
        is_bookmarked: fm.tags.contains(&"bookmark".to_string()),
        tags: fm.tags.clone(),
    }
}

/// Compares old and new tag state; sends the entry to the appropriate channels
/// when is_unread or is_bookmarked actually changed.
fn emit_entry_changes(
    id: &str,
    fm: &Frontmatter,
    old_tags: Option<&[String]>,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
) {
    let old_unread = old_tags.map_or(false, |t| t.contains(&"unread".to_string()));
    let old_bookmarked = old_tags.map_or(false, |t| t.contains(&"bookmark".to_string()));
    let new_unread = fm.tags.contains(&"unread".to_string());
    let new_bookmarked = fm.tags.contains(&"bookmark".to_string());
    if old_unread != new_unread || old_bookmarked != new_bookmarked {
        let entry = entry_from_fm_ref(id, fm);
        if old_unread != new_unread {
            let _ = unread_tx.send(entry.clone());
        }
        if old_bookmarked != new_bookmarked {
            let _ = bookmarked_tx.send(entry);
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

/// SHA-256 checksum of an annotation's embeddable content.
fn annotation_checksum(annotation: &Annotation) -> String {
    let mut h = sha2::Sha256::new();
    h.update(annotation.text.join(" ").as_bytes());
    if let Some(note) = &annotation.note {
        h.update(b"\n\n");
        h.update(note.as_bytes());
    }
    hex::encode(h.finalize())
}

/// Builds the LanceDB row for an annotation. `vector` is `None` while the embedding is
/// still pending (row inserted by [`sync_annotations`]); `Some` once [`embed_annotations`]
/// has computed it. `topics` is the reverse index, owned by the topic membership path — the
/// caller passes the row's *existing* topics forward so this write never clobbers it.
///
/// Each annotation row uses the annotation's own UUID as its DB ID. The title is the
/// highlighted text; the note (if present) is the embedding body. The `reference` column
/// holds the parent article ID; `color` holds the highlight color.
pub(crate) fn annotation_embedding_entry(
    article_id: &str,
    fm: &Frontmatter,
    annotation: &Annotation,
    topics: Vec<String>,
    vector: Option<Vec<f32>>,
) -> EmbeddingEntry {
    EmbeddingEntry {
        id: annotation.id.clone(),
        kind: Kind::Annotation,
        source: fm.source.clone(),
        title: annotation.text.join(" "),
        created_at: annotation.created_at.clone(),
        published_at: fm.published_at.clone().unwrap_or_default(),
        updated_at: fm.updated_at.clone().unwrap_or_default(),
        checksum: annotation_checksum(annotation),
        reference: article_id.to_string(),
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
fn annotation_saved_item(article_id: &str, fm: &Frontmatter, annotation: &Annotation) -> SavedItem {
    SavedItem {
        id: annotation.id.clone(),
        title: annotation.text.join(" "),
        created_at: annotation.created_at.clone(),
        url: fm.source.clone(),
        is_unread: false,
        is_bookmarked: false,
        reference: Some(article_id.to_string()),
        color: Some(annotation.color.clone()),
    }
}

/// Fast path: reconciles annotation *rows* against `fm` **without** running the embedder, so a
/// newly added annotation is listable immediately (the vector is deferred to
/// [`embed_annotations`]). Row insert is decoupled from embedding, mirroring entries' [`sync_row`].
///
/// - Deletes rows for annotations removed from the file.
/// - Inserts new / content-changed annotations with a `None` (pending) vector.
/// - Applies a color-only change in place, preserving the existing vector.
///
/// Emits the live `annotation_tx` event for inserted/updated rows.
async fn sync_annotations(
    article_id: &str,
    fm: &Frontmatter,
    db: &EmbeddingsDb,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    let existing = db.annotation_meta_for_article(article_id).await?;

    let current_ids: HashSet<String> = fm.annotations.iter().map(|a| a.id.clone()).collect();

    let to_delete: Vec<String> = existing
        .keys()
        .filter(|id| !current_ids.contains(*id))
        .cloned()
        .collect();
    if !to_delete.is_empty() {
        log::debug!(
            "sync_annotations: removing {} stale annotation(s) for {article_id}",
            to_delete.len()
        );
        db.delete_by_ids(&to_delete).await?;
    }

    for annotation in &fm.annotations {
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
                    let _ = annotation_tx.send(annotation_saved_item(article_id, fm, annotation));
                }
                continue;
            }
            _ => {}
        }

        // New or content-changed: insert/refresh the row now with a pending (`None`) vector so it
        // is immediately listable; embed_vector fills the vector later. Carry the existing row's
        // topics forward (the topic path owns that column) so this write never clobbers the index.
        let topics = if existing.contains_key(&annotation.id) {
            db.get_entry(&annotation.id).await?.map(|e| e.topics).unwrap_or_default()
        } else {
            Vec::new()
        };
        db.upsert(annotation_embedding_entry(article_id, fm, annotation, topics, None))
            .await?;
        let _ = annotation_tx.send(annotation_saved_item(article_id, fm, annotation));
    }

    Ok(())
}

/// Slow path: computes and stores embedding vectors for this article's annotation rows whose
/// vector is missing or stale. Also creates any annotation row that does not yet exist as a
/// safety net — annotation rows are normally created earlier (fresh recovery via `bulk_insert`,
/// live edits via `sync_annotations`). Runs the embedder.
async fn embed_annotations(
    article_id: &str,
    fm: &Frontmatter,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    for annotation in &fm.annotations {
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
        let body = annotation.note.clone().unwrap_or_default();
        let vector = compute_vector(embedder, title, body).await?;
        // Carry the existing row's topics forward — the topic path owns that column.
        let topics = existing.map(|e| e.topics).unwrap_or_default();
        db.upsert(annotation_embedding_entry(article_id, fm, annotation, topics, Some(vector)))
            .await?;
    }

    Ok(())
}

/// Fast path: brings the row's metadata, tags, and denormalized state in sync with
/// the `.md` file *without* running the embedder. The vector is preserved when the
/// content is unchanged, or left `None` (pending) when the row is new or its content
/// changed — `None` ⟺ still needs embedding. Annotation rows are reconciled here too
/// (also embedder-free) by [`sync_annotations`]; their vectors are filled by [`embed_vector`].
pub async fn sync_row(
    path: &Path,
    db: &EmbeddingsDb,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(EmbeddingsError::Io)?;
    let (fm, body) = crate::util::parse_document(&raw)
        .ok_or_else(|| EmbeddingsError::Parse(path.display().to_string()))?;
    let id = crate::util::item_id(path);

    let existing = db.get_entry(&id).await?;
    let checksum = fm.content_checksum(&body);
    let content_unchanged =
        existing.as_ref().map(|e| e.checksum.as_str()) == Some(checksum.as_str());

    // Preserve the existing vector only when content is unchanged; otherwise leave it
    // pending so the embed pass recomputes it.
    let vector = if content_unchanged {
        existing.as_ref().and_then(|e| e.vector.clone())
    } else {
        None
    };

    // Write only when something the row stores actually changed (state derives from tags).
    // `topics` is excluded: it's the reverse index owned by the topic membership path, not the
    // entry frontmatter, so it neither triggers a write nor is sourced from `fm` here.
    let changed = match &existing {
        None => true,
        Some(e) => {
            !content_unchanged
                || e.tags != fm.tags
                || e.summary != fm.summary.clone().unwrap_or_default()
        }
    };

    if changed {
        let old_tags = existing.as_ref().map(|e| e.tags.clone());
        // Carry the existing row's topics + items forward so this write never clobbers the
        // reverse index — both columns are owned by the topic-membership reconcile path.
        let mut row = fm.to_embedding_entry(id.clone(), &body, vector);
        row.topics = existing.as_ref().map(|e| e.topics.clone()).unwrap_or_default();
        row.items = existing.as_ref().map(|e| e.items.clone()).unwrap_or_default();
        db.upsert(row).await?;
        emit_entry_changes(&id, &fm, old_tags.as_deref(), unread_tx, bookmarked_tx);
    }

    // Always reconcile annotation rows (embedder-free) so a new/removed annotation is
    // listable immediately, even when the entry body itself is unchanged.
    sync_annotations(&id, &fm, db, annotation_tx).await?;

    Ok(())
}

/// Slow path: computes and stores the embedding vector for `id` when it is missing or
/// stale, then fills any pending annotation vectors. Runs the embedder.
pub async fn embed_vector(
    path: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(EmbeddingsError::Io)?;
    let (fm, body) = crate::util::parse_document(&raw)
        .ok_or_else(|| EmbeddingsError::Parse(path.display().to_string()))?;
    let id = crate::util::item_id(path);

    let existing = db.get_entry(&id).await?;
    let checksum = fm.content_checksum(&body);
    let needs_vector = match &existing {
        None => true,
        Some(e) => e.vector.is_none() || e.checksum != checksum,
    };

    if needs_vector {
        log::debug!("embed_vector: embedding id={id} title={:?}", fm.title);
        let vector =
            compute_vector(embedder, fm.title.clone(), fm.embed_text(&body).to_string()).await?;
        // Carry the existing row's topics + items forward — owned by the topic reconcile path.
        let mut row = fm.to_embedding_entry(id.clone(), &body, Some(vector));
        row.topics = existing.as_ref().map(|e| e.topics.clone()).unwrap_or_default();
        row.items = existing.as_ref().map(|e| e.items.clone()).unwrap_or_default();
        db.upsert(row).await?;
    }

    // Fill any pending annotation vectors. Rows (incl. stale-row deletes) were already
    // reconciled by sync_annotations; this also creates rows the recovery path skipped.
    embed_annotations(&id, &fm, embedder, db).await?;

    Ok(())
}

/// Synchronous full sync: metadata/state (`sync_row`) then vector + annotations
/// (`embed_vector`) in one pass. Used by `xaga-cli embeddings recover`, which wants a
/// finished index rather than the service's deferred two-phase behaviour.
pub async fn embed_file(
    path: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
    unread_tx: &flume::Sender<Entry>,
    bookmarked_tx: &flume::Sender<Entry>,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    sync_row(path, db, unread_tx, bookmarked_tx, annotation_tx).await?;
    embed_vector(path, embedder, db).await?;
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
/// fragments or block the loop. Crash window: a hard kill between a chunk's delete and its add loses
/// that chunk's member rows until a later sync/recovery (accepted tradeoff of delete-then-add).
///
/// This is the sole writer of the `topics` (member) and `items` (topic) columns — entry/annotation
/// sync carry them forward untouched. Members whose rows don't exist yet are skipped (reconciled on a
/// later run once they exist). Member-type-agnostic — operates on whatever ids the caller passes
/// (entries **and** annotations); annotation rows exist by this point (created by `bulk_insert` on
/// fresh recovery, `sync_annotations` on live edits).
pub async fn reconcile_topic_membership(
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
        summary: opt(&row.summary),
        is_unread: row.is_unread,
        is_bookmarked: row.is_bookmarked,
        tags: row.tags.clone(),
    }
}

/// Write-through state update for a `SyncState` job: applies the carried value(s) directly
/// to the cached row's `tags` + denormalized boolean columns and emits the UI change event,
/// without reading the `.md`. `get_entry` is still required because `upsert` replaces the
/// whole row (preserving vector/topics/items/content). A `None` field is left unchanged; a
/// missing row is a no-op (recovery will create it).
pub async fn apply_state(
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
        set_tag(&mut row.tags, "unread", v);
        row.is_unread = v;
    }
    if let Some(v) = is_bookmarked {
        set_tag(&mut row.tags, "bookmark", v);
        row.is_bookmarked = v;
    }
    if let Some(v) = is_pinned {
        set_tag(&mut row.tags, "pinned", v);
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

/// Standalone annotation reconcile for a `SyncAnnotations` job: reads the entry's `.md`,
/// parses frontmatter, and reconciles its annotation rows (embedder-free). Used when only
/// annotations changed; content edits reconcile annotations via `sync_row`.
pub async fn sync_annotations_from_file(
    path: &Path,
    db: &EmbeddingsDb,
    annotation_tx: &flume::Sender<SavedItem>,
) -> Result<(), EmbeddingsError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(EmbeddingsError::Io)?;
    let (fm, _body) = crate::util::parse_document(&raw)
        .ok_or_else(|| EmbeddingsError::Parse(path.display().to_string()))?;
    let id = crate::util::item_id(path);
    sync_annotations(&id, &fm, db, annotation_tx).await
}

/// Precise single-member reverse-index add: sets `topic_id` on `member_id`'s `topics`
/// column and advances the topic row's `items` base so a later full reconcile sees no
/// spurious delta. The O(1) counterpart to [`reconcile_topic_membership`] for a live add.
pub async fn add_topic_member(
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
pub async fn remove_topic_member(
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

/// SHA-256 of a feed's embeddable content (its title).
fn feed_checksum(title: &str) -> String {
    let mut h = sha2::Sha256::new();
    h.update(title.as_bytes());
    hex::encode(h.finalize())
}

/// Embeds feed rows (`kind = 'feed'`) from the `.atom` files so feeds are searchable and
/// carry their pinned state. Feeds are few (sidebar-scale), so this runs in one pass.
/// Re-embeds a feed only when its title/subtitle or pinned state changed, or its vector is missing.
/// Embeds (or refreshes) one feed's row from its metadata. Checksum-gated: a no-op when the
/// row already has a vector and its title/subtitle checksum and `is_pinned` are unchanged.
/// The single-feed unit that both `embed_feeds` (the `.atom` sweep) and the live
/// `EmbedVector` path for `kind=Feed` call.
pub async fn embed_feed(
    meta: &crate::feeds::FeedMeta,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let checksum = feed_checksum(&format!("{}\n{}", meta.title, meta.subtitle));
    let existing = db.get_entry(&meta.id).await?;
    let up_to_date = matches!(
        &existing,
        Some(e) if e.vector.is_some() && e.checksum == checksum && e.is_pinned == meta.is_pinned
    );
    if up_to_date {
        return Ok(());
    }

    log::debug!("embed_feed: embedding feed id={} title={:?}", meta.id, meta.title);
    let vector = compute_vector(embedder, meta.title.clone(), meta.subtitle.clone()).await?;
    db.upsert(EmbeddingEntry {
        id: meta.id.clone(),
        kind: Kind::Feed,
        source: meta.url.clone(),
        title: meta.title.clone(),
        created_at: String::new(),
        published_at: String::new(),
        updated_at: String::new(),
        checksum,
        reference: String::new(),
        color: String::new(),
        summary: meta.subtitle.clone(),
        tags: vec![],
        topics: vec![],
        items: vec![],
        is_unread: false,
        is_bookmarked: false,
        is_pinned: meta.is_pinned,
        vector: Some(vector),
    })
    .await?;
    Ok(())
}

pub async fn embed_feeds(
    feeds_dir: &Path,
    embedder: &Arc<Embedder>,
    db: &EmbeddingsDb,
) -> Result<(), EmbeddingsError> {
    let metas = crate::feeds::feed_embedding_meta(feeds_dir).await;
    let current: HashSet<String> = metas.iter().map(|m| m.id.clone()).collect();

    for meta in &metas {
        embed_feed(meta, embedder, db).await?;
    }

    // Delete feed rows whose `.atom` no longer exists.
    let stale: Vec<String> = db
        .all_entries_meta()
        .await?
        .into_iter()
        .filter(|(id, (_, _, _, kind))| kind == "feed" && !current.contains(id))
        .map(|(id, _)| id)
        .collect();
    if !stale.is_empty() {
        db.delete_by_ids(&stale).await?;
    }
    Ok(())
}
