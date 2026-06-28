//! The diff stage: walk the filesystem, compare against the index, and drive the repair. Each item
//! that needs embedding is streamed to the caller's `RepairSink`; the post-walk residue (feed embeds
//! + deletes) is returned. Pure computation — the reads are delegated to persistence
//! (`for_each_md`/`parse_document`) and embeddings (`all_entries_meta`); the *apply* is the sink's.

use std::collections::HashSet;
use std::path::Path;

use crate::embeddings::row::{article_embed_text, checksum, feed_embed_text, topic_embed_text};
use crate::embeddings::{EmbeddingsDb, EmbeddingsError};
use crate::library;
use crate::md::{md_to_article, md_to_topic};
use crate::persistence::scan::for_each_md;
use crate::types::{Feed, Kind};

use super::{RepairResidue, RepairSink};

/// Walks `{library_dir}/topics/` and `{library_dir}/entries/`, comparing each against LanceDB and
/// streaming the items that need embedding to `sink` (topics before articles, so they index first).
/// Returns the residue that can only be computed after the full walk: feed embeds and deletes.
pub(crate) async fn repair_item_embeddings<S: RepairSink>(
    library_dir: &Path,
    db: &EmbeddingsDb,
    sink: &mut S,
) -> Result<RepairResidue, EmbeddingsError> {
    let db_meta = db.all_entries_meta().await?; // id → (checksum, reference, has_vector, kind)

    // Embedding is needed when the row is absent, its content changed, or its vector is missing.
    let needs_embedding = |id: &str, cksum: &str| -> bool {
        match db_meta.get(id) {
            None => true,
            Some((stored, _, has_vector, _)) => stored != cksum || !has_vector,
        }
    };

    // Reconcile as we walk: record every on-disk id (for `to_delete` below) and stream only the
    // items that need embedding straight to the sink. Nothing is accumulated here — an object that
    // doesn't need work is dropped the moment the walk moves past it, freeing its content.
    let mut fs_ids: HashSet<String> = HashSet::new();

    // Topics first so they index before articles.
    for_each_md(&library::topics_dir(library_dir), &md_to_topic, |topic| {
        fs_ids.insert(topic.id.clone());
        if needs_embedding(&topic.id, &checksum(&topic.title, topic_embed_text(&topic))) {
            sink.topic(topic);
        }
    })
    .await;

    for_each_md(&library::entries_dir(library_dir), &md_to_article, |article| {
        fs_ids.insert(article.meta.id.clone());
        if needs_embedding(
            &article.meta.id,
            &checksum(&article.meta.title, article_embed_text(&article)),
        ) {
            sink.article(article);
        }
    })
    .await;

    // Feeds: diff like items — re-embed a feed only when its row is absent, its checksum changed,
    // or its vector is pending. A feed row is stale when its feed is gone from disk.

    let empty = HashSet::new();

    let all_feeds =
        crate::persistence::feeds::list_feeds(&library::feeds_dir(library_dir), &empty, &empty)
            .await
            .unwrap_or_default();

    let feed_ids: HashSet<String> = all_feeds.iter().map(|f| f.id.clone()).collect();

    let to_embed_feeds: Vec<Feed> = all_feeds
        .into_iter()
        .filter(|f| needs_embedding(&f.id, &checksum(&f.title, feed_embed_text(f))))
        .collect();

    // Build to_delete: in LanceDB but no longer on disk. Item rows: the `.md` is gone (annotation
    // rows carry a non-empty `reference` to their parent article — check that). Feed rows: the feed
    // is gone from disk.
    let to_delete: Vec<String> = db_meta
        .into_iter()
        .filter(|(id, (_, reference, _, kind))| {
            if kind == Kind::Feed.as_str() {
                return !feed_ids.contains(id);
            }

            let file_id = if reference.is_empty() {
                id.as_str()
            } else {
                reference.as_str()
            };

            !fs_ids.contains(file_id)
        })
        .map(|(id, _)| id)
        .collect();

    Ok(RepairResidue {
        to_embed_feeds,
        to_delete,
    })
}
