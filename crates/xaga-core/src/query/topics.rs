use std::collections::HashSet;
use std::path::Path;

use crate::{
    library, md::parse_document, persistence::topics::TopicsError, types::CollectionItem,
};

/// Returns the union of embedding-matched items and manually curated items for a topic,
/// with the given filter applied and ordered by `sort`. Deduplicates by ID.
pub async fn list_topic_items(
    library_dir: &Path,
    id: &str,
    item_filter: crate::types::ItemFilter,
    sort: crate::types::ItemSort,
) -> Result<Vec<crate::types::ListItem>, TopicsError> {
    let path = library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    let (fm, _body) = parse_document(&raw).ok_or(TopicsError::NotFound)?;
    let threshold = fm.threshold.unwrap_or(0.8);

    // Ungated similarity lookup: every embedded item ranked by similarity to the topic.
    // The explicit low floor (-1.0) disables the default 0.5 cutoff so that even weakly
    // related manually-curated items still receive a score for relevance ranking.
    let all_embedded = crate::query::items::list_embedded_items(
        library_dir,
        crate::query::items::EmbeddedItemSource::RelatedTo {
            id: id.to_string(),
            limit: None,
            min_similarity: Some(-1.0),
            max_similarity: None,
        },
        crate::types::ItemFilter::All,
    )
    .await
    .unwrap_or_default();

    // id -> similarity, used to score manually curated items.
    let similarity_by_id: std::collections::HashMap<String, f32> = all_embedded
        .iter()
        .map(|e| (e.entry.id.clone(), e.similarity))
        .collect();

    let mut scored: Vec<(Option<f32>, crate::types::ListItem)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1. Semantic matches at or above the topic threshold, with the requested filter applied.
    let semantic: Vec<_> = all_embedded
        .into_iter()
        .filter(|e| e.similarity >= threshold)
        .collect();
    for e in crate::query::filter::filter_items(semantic, &item_filter)
        .await
        .unwrap_or_default()
    {
        if seen.insert(e.entry.id.clone()) {
            scored.push((
                Some(e.similarity),
                crate::types::ListItem {
                    id: e.entry.id.clone(),
                    title: e.entry.title.clone(),
                    created_at: e.entry.created_at.clone(),
                    url: e.entry.url.clone(),
                    is_unread: e.entry.is_unread,
                    is_bookmarked: e.entry.is_bookmarked,
                    reference: if e.reference.is_empty() {
                        None
                    } else {
                        Some(e.reference.clone())
                    },
                    color: e.entry.color.clone(),
                },
            ));
        }
    }

    // 2. Manually curated entries via the proven filter path.
    let mut manual_entries: Vec<crate::types::Entry> = Vec::new();
    for item in &fm.items {
        if let CollectionItem::Entry { id } = item {
            if !seen.contains(id) {
                if let Ok(entry) = crate::persistence::entries::read_entry(library_dir, id).await {
                    manual_entries.push(entry);
                }
            }
        }
    }
    for entry in crate::query::filter::filter_items(manual_entries, &item_filter)
        .await
        .unwrap_or_default()
    {
        seen.insert(entry.id.clone());
        let similarity = similarity_by_id.get(&entry.id).copied();
        scored.push((
            similarity,
            crate::types::ListItem {
                id: entry.id,
                title: entry.title,
                created_at: entry.created_at,
                url: entry.url,
                is_unread: entry.is_unread,
                is_bookmarked: entry.is_bookmarked,
                reference: None,
                color: None,
            },
        ));
    }

    // 3. Manually curated annotations.
    // Annotations have no unread/bookmarked state; filter::filter_items excludes them
    // from Unread and Bookmarked via is_unread()==false and is_bookmarked()==false,
    // and includes them in Saved via !reference().is_empty(). Replicated here since
    // Annotation does not implement Item.
    let include_annotations = matches!(
        item_filter,
        crate::types::ItemFilter::All | crate::types::ItemFilter::Saved
    );
    for item in &fm.items {
        if let CollectionItem::Annotation { id, reference } = item {
            if include_annotations && !seen.contains(id) && !reference.is_empty() {
                if let Ok(article) =
                    crate::persistence::articles::read_article(library_dir, reference).await
                {
                    if let Some(ann) = article.annotations.iter().find(|a| &a.id == id) {
                        seen.insert(ann.id.clone());
                        let similarity = similarity_by_id.get(&ann.id).copied();
                        scored.push((
                            similarity,
                            crate::types::ListItem {
                                id: ann.id.clone(),
                                title: ann.text.join(" "),
                                created_at: ann.created_at.clone(),
                                url: article.meta.url.clone(),
                                is_unread: false,
                                is_bookmarked: false,
                                reference: Some(reference.clone()),
                                color: Some(ann.color.clone()),
                            },
                        ));
                    }
                }
            }
        }
    }

    // Order the unified set.
    match sort {
        crate::types::ItemSort::Relevance => {
            // Highest similarity first; unscored (unembedded) items last.
            scored.sort_by(|a, b| match (a.0, b.0) {
                (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
            Ok(scored.into_iter().map(|(_, item)| item).collect())
        }
        crate::types::ItemSort::Timestamp => {
            let mut items: Vec<crate::types::ListItem> =
                scored.into_iter().map(|(_, item)| item).collect();
            crate::query::filter::sort_items(&mut items);
            Ok(items)
        }
    }
}
