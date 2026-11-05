use std::collections::HashSet;
use std::path::Path;

use crate::{library, md::parse_document, persistence::topics::TopicsError, types::Reference};

/// Returns the union of embedding-matched entities and manually curated entities for a topic,
/// with the given filter applied and ordered by `sort`. Deduplicates by ID.
pub async fn list_topic_entities(
    library_dir: &Path,
    id: &str,
    entity_filter: crate::types::EntityFilter,
    sort: crate::types::EntitySort,
) -> Result<Vec<crate::types::ListEntity>, TopicsError> {
    let path = library::topic_path(library_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Err(TopicsError::NotFound);
    }
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, _body) = parse_document(&raw)?;
    let threshold = fm.threshold.unwrap_or(0.8);

    // Ungated similarity lookup: every embedded entity ranked by similarity to the topic.
    // The explicit low floor (-1.0) disables the default 0.5 cutoff so that even weakly
    // related manually-curated entities still receive a score for relevance ranking.
    let all_embedded = crate::query::entities::list_embedded_entities(
        library_dir,
        crate::query::entities::EmbeddedEntitySource::RelatedTo {
            id: id.to_string(),
            limit: None,
            min_similarity: Some(-1.0),
            max_similarity: None,
        },
        crate::types::EntityFilter::All,
    )
    .await
    .unwrap_or_default();

    // id -> similarity, used to score manually curated entities.
    let similarity_by_id: std::collections::HashMap<String, f32> = all_embedded
        .iter()
        .map(|e| (e.entry.id.clone(), e.similarity))
        .collect();

    let mut scored: Vec<(Option<f32>, crate::types::ListEntity)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1. Semantic matches at or above the topic threshold, with the requested filter applied.
    let semantic: Vec<_> = all_embedded
        .into_iter()
        .filter(|e| e.similarity >= threshold)
        .collect();
    for e in crate::query::filter::filter_entities(semantic, &entity_filter)
        .await
        .unwrap_or_default()
    {
        if seen.insert(e.entry.id.clone()) {
            scored.push((
                Some(e.similarity),
                crate::types::ListEntity {
                    id: e.entry.id.clone(),
                    title: e.entry.title.clone(),
                    created_at: e.entry.created_at.clone(),
                    url: e.entry.url.clone(),
                    is_unread: e.entry.is_unread,
                    is_pinned: e.entry.is_pinned,
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
    for entity in &fm.entities {
        if let Reference::Entry { id } = entity {
            if !seen.contains(id) {
                if let Ok(entry) = crate::persistence::entries::read_entry(library_dir, id).await {
                    manual_entries.push(entry);
                }
            }
        }
    }
    for entry in crate::query::filter::filter_entities(manual_entries, &entity_filter)
        .await
        .unwrap_or_default()
    {
        seen.insert(entry.id.clone());
        let similarity = similarity_by_id.get(&entry.id).copied();
        scored.push((
            similarity,
            crate::types::ListEntity {
                id: entry.id,
                title: entry.title,
                created_at: entry.created_at,
                url: entry.url,
                is_unread: entry.is_unread,
                is_pinned: entry.is_pinned,
                reference: None,
                color: entry.color.clone(),
            },
        ));
    }

    // 3. Manually curated annotations.
    // Annotations have no unread/bookmarked state; filter::filter_entities excludes them
    // from Unread and Bookmarked via is_unread()==false and color()==None,
    // and includes them in Saved via !reference().is_empty(). Replicated here since
    // Annotation does not implement Filterable.
    let include_annotations = matches!(
        entity_filter,
        crate::types::EntityFilter::All | crate::types::EntityFilter::Saved
    );
    for entity in &fm.entities {
        if let Reference::Annotation {
            id,
            article_id: Some(article_id),
        } = entity
        {
            if include_annotations && !seen.contains(id) && !article_id.is_empty() {
                if let Ok(article) =
                    crate::persistence::articles::read_article(library_dir, article_id).await
                {
                    if let Some(ann) = article.annotations.iter().find(|a| &a.id == id) {
                        seen.insert(ann.id.clone());
                        let similarity = similarity_by_id.get(&ann.id).copied();
                        scored.push((
                            similarity,
                            crate::types::ListEntity {
                                id: ann.id.clone(),
                                title: ann.text.join(" "),
                                created_at: ann.created_at.clone(),
                                url: article.meta.url.clone(),
                                is_unread: false,
                                is_pinned: false,
                                reference: Some(article_id.clone()),
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
        crate::types::EntitySort::Relevance => {
            // Highest similarity first; unscored (unembedded) entities last.
            scored.sort_by(|a, b| match (a.0, b.0) {
                (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
            Ok(scored.into_iter().map(|(_, entity)| entity).collect())
        }
        crate::types::EntitySort::Timestamp => {
            let mut entities: Vec<crate::types::ListEntity> =
                scored.into_iter().map(|(_, entity)| entity).collect();
            crate::query::filter::sort_entities(&mut entities);
            Ok(entities)
        }
    }
}
