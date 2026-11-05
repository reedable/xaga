use std::{collections::HashSet, path::Path};

use thiserror::Error;

use crate::{
    embeddings::EmbeddingsDb,
    query::filter,
    types::{
        EmbeddedEntity, EmbeddedEntry, EmbeddedFeed, EmbeddedTopic, EntityFilter, Kind, ListEntity,
    },
};

// ── EmbeddedEntitySource ──────────────────────────────────────────────────────────

/// Describes the source for an embedding similarity query.
pub enum EmbeddedEntitySource {
    /// Entities related to a given entity, ranked by embedding similarity.
    RelatedTo {
        id: String,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    },
    /// Entities related to an arbitrary pre-computed query vector (e.g. from embedded search text).
    RelatedToVector {
        vector: Vec<f32>,
        limit: Option<usize>,
        min_similarity: Option<f32>,
        max_similarity: Option<f32>,
    },
    /// Topics ranked by similarity to a given entity.
    TopicsRelatedTo {
        id: String,
        limit: Option<usize>,
        min_similarity: Option<f32>,
    },
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum EntityError {
    #[error(transparent)]
    Embeddings(#[from] crate::embeddings::EmbeddingsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl EntityError {
    /// Stable code the UI matches on. Embeddings failures delegate to the inner
    /// code. Mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            EntityError::Embeddings(e) => e.code(),
            EntityError::Io(_) => "entity:io",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            EntityError::Embeddings(e) => e.user_message(),
            EntityError::Io(_) => "Couldn't read this item.",
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, EntityError>;

// ── Public API ────────────────────────────────────────────────────────────────

/// Fetch embedded entities from `source`, preserving similarity rank, then filter.
/// Reading state and color are projected directly from LanceDB.
pub async fn list_embedded_entities(
    library_dir: &Path,
    source: EmbeddedEntitySource,
    entity_filter: EntityFilter,
) -> Result<Vec<EmbeddedEntry>> {
    match source {
        EmbeddedEntitySource::RelatedTo {
            id,
            limit,
            min_similarity,
            max_similarity,
        } => {
            let db = EmbeddingsDb::open_readonly(library_dir).await?;
            let entries = db
                .search_related(&id, limit, min_similarity, max_similarity, None)
                .await?;
            filter::filter_entities(entries, &entity_filter)
                .await
                .map_err(EntityError::Io)
        }
        EmbeddedEntitySource::RelatedToVector {
            vector,
            limit,
            min_similarity,
            max_similarity,
        } => {
            let db = EmbeddingsDb::open_readonly(library_dir).await?;
            let entries = db
                .search_by_vector(vector, limit, min_similarity, max_similarity)
                .await?;
            filter::filter_entities(entries, &entity_filter)
                .await
                .map_err(EntityError::Io)
        }
        EmbeddedEntitySource::TopicsRelatedTo {
            id,
            limit,
            min_similarity,
        } => {
            let db = EmbeddingsDb::open_readonly(library_dir).await?;
            db.search_related(&id, limit, min_similarity, None, Some(Kind::Topic))
                .await
                .map_err(EntityError::Embeddings)
        }
    }
}

/// Lists every entity in the library — entry rows and annotation rows — read entirely from
/// LanceDB, filtered by `entity_filter` and sorted by `created_at` descending. Returns the flat
/// `ListEntity` the entities list renders (annotations carry `reference`/`color`).
pub async fn list_all_entities(
    library_dir: &Path,
    entity_filter: EntityFilter,
) -> Result<Vec<ListEntity>> {
    let db = EmbeddingsDb::open_readonly(library_dir).await?;

    let mut entities = db.filter_entities(entity_filter).await?;

    entities.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(entities)
}

/// Enriches raw LanceDB results from their authoritative files while preserving rank.
/// Feed rows whose source `.atom` file cannot be read are omitted. Rows with no URL are
/// resolved as topics and fall back to entries when they cannot be resolved.
pub async fn enrich_embedded(
    library_dir: &Path,
    entries: Vec<EmbeddedEntry>,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Vec<EmbeddedEntity> {
    let mut result = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.kind == Kind::Feed {
            if let Ok(feed) = crate::persistence::feeds::read_feed(
                library_dir,
                &entry.entry.id,
                unread,
                bookmarked,
            )
            .await
            {
                result.push(EmbeddedEntity::Feed(EmbeddedFeed {
                    feed,
                    kind: entry.kind,
                    similarity: entry.similarity,
                }));
            }
            continue;
        }
        if entry.entry.url.is_empty() {
            if let Ok(topic) =
                crate::persistence::topics::read_topic(library_dir, &entry.entry.id).await
            {
                result.push(EmbeddedEntity::Topic(EmbeddedTopic {
                    topic,
                    kind: Kind::Topic,
                    similarity: entry.similarity,
                }));
                continue;
            }
        }
        result.push(EmbeddedEntity::Entry(entry));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entry;

    fn embedded_entry(id: &str, kind: Kind, similarity: f32) -> EmbeddedEntry {
        EmbeddedEntry {
            entry: Entry {
                tags: None,
                id: id.to_string(),
                url: format!("https://index.example/{id}"),
                title: format!("Indexed {id}"),
                created_at: "2025-01-01T00:00:00+00:00".to_string(),
                published_at: None,
                updated_at: None,
                description: None,
                is_unread: false,
                is_pinned: false,
                color: None,
            },
            kind,
            similarity,
            reference: String::new(),
        }
    }

    #[tokio::test]
    async fn enriches_feeds_from_atom_and_omits_missing_feeds_without_reordering() {
        let directory = tempfile::tempdir().unwrap();
        let library_dir = directory.path();
        tokio::fs::create_dir_all(crate::library::feeds_dir(library_dir))
            .await
            .unwrap();

        let feed_xml = r#"<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Authoritative feed</title>
  <id>stored-feed-id</id>
  <updated>2026-10-23T12:00:00Z</updated>
  <link href="https://example.com/feed.xml" rel="self" />
  <category term="pinned" scheme="https://xaga.app/ns/tag" />
  <entry>
    <title>One</title>
    <id>one</id>
    <updated>2026-10-23T11:00:00Z</updated>
    <link href="https://example.com/one" rel="alternate" />
  </entry>
  <entry>
    <title>Two</title>
    <id>two</id>
    <updated>2026-10-23T10:00:00Z</updated>
    <link href="https://example.com/two" rel="alternate" />
  </entry>
</feed>"#;
        tokio::fs::write(crate::library::feed_path(library_dir, "feed"), feed_xml)
            .await
            .unwrap();

        crate::pins::Pins::set(
            library_dir,
            crate::pins::PinTarget::Feed { id: "feed".into() },
            true,
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        let unread = HashSet::from([crate::util::url_hash12("https://example.com/one")]);
        let bookmarked = HashSet::from([crate::util::url_hash12("https://example.com/two")]);
        let entries = vec![
            embedded_entry("first", Kind::Entry, 0.9),
            embedded_entry("feed", Kind::Feed, 0.8),
            embedded_entry("missing", Kind::Feed, 0.7),
            embedded_entry("last", Kind::Entry, 0.6),
        ];

        let enriched = enrich_embedded(library_dir, entries, &unread, &bookmarked).await;

        assert_eq!(enriched.len(), 3);
        assert!(matches!(
            &enriched[0],
            EmbeddedEntity::Entry(entry) if entry.entry.id == "first" && entry.similarity == 0.9
        ));
        match &enriched[1] {
            EmbeddedEntity::Feed(feed) => {
                assert_eq!(feed.feed.id, "feed");
                assert_eq!(feed.feed.title, "Authoritative feed");
                assert_eq!(feed.feed.url, "https://example.com/feed.xml");
                assert_eq!(feed.feed.entry_count, 2);
                assert_eq!(feed.feed.unread_count, 1);
                assert!(feed.feed.is_pinned);
                assert_eq!(feed.similarity, 0.8);
            }
            _ => panic!("second result should be the authoritative feed"),
        }
        assert!(matches!(
            &enriched[2],
            EmbeddedEntity::Entry(entry) if entry.entry.id == "last" && entry.similarity == 0.6
        ));
    }
}
