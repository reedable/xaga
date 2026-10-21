use std::collections::BTreeMap;

use crate::{
    embeddings::{EmbeddingsDb, EmbeddingsError},
    query::filter,
    site::normalize_site_hostname,
    types::{Entry, ItemFilter, SiteSummary},
};

/// Returns every Site represented by an Entry URL, grouped from one index read.
pub async fn list_sites(db: &EmbeddingsDb) -> Result<Vec<SiteSummary>, EmbeddingsError> {
    db.reload().await?;
    let mut summaries = BTreeMap::<String, SiteSummary>::new();

    for entry in db.list_entries().await? {
        let Some(hostname) = normalize_site_hostname(&entry.url) else {
            continue;
        };
        let hostname = hostname.to_string();
        let summary = summaries.entry(hostname.clone()).or_insert(SiteSummary {
            hostname,
            entry_count: 0,
            unread_count: 0,
        });
        summary.entry_count += 1;
        summary.unread_count += usize::from(entry.is_unread);
    }

    Ok(summaries.into_values().collect())
}

/// Lists Entries whose stored URL derives to `hostname`.
pub async fn list_site_entries(
    db: &EmbeddingsDb,
    hostname: &str,
    item_filter: ItemFilter,
) -> Result<Vec<Entry>, EmbeddingsError> {
    db.reload().await?;
    let entries = db
        .list_entries()
        .await?
        .into_iter()
        .filter(|entry| {
            normalize_site_hostname(&entry.url)
                .is_some_and(|candidate| candidate.as_str() == hostname)
        })
        .collect();
    let mut entries = filter::filter_items(entries, &item_filter)
        .await
        .expect("filtering Entries is infallible");
    filter::sort_items(&mut entries);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::EmbeddingEntry;
    use crate::types::Kind;

    fn row(id: &str, url: &str, is_unread: bool, is_bookmarked: bool) -> EmbeddingEntry {
        let mut tags = vec![];
        if is_unread {
            tags.push("unread".to_string());
        }
        if is_bookmarked {
            tags.push("bookmark".to_string());
        }
        EmbeddingEntry {
            id: id.to_string(),
            kind: Kind::Entry,
            source: url.to_string(),
            title: id.to_string(),
            created_at: id.to_string(),
            published_at: String::new(),
            updated_at: String::new(),
            reference: String::new(),
            color: String::new(),
            summary: String::new(),
            tags,
            is_unread,
            is_bookmarked,
            is_pinned: false,
            topics: vec![],
            items: vec![],
            checksum: String::new(),
            vector: None,
        }
    }

    #[tokio::test]
    async fn summaries_and_lists_derive_from_entry_urls() {
        let directory = tempfile::tempdir().unwrap();
        let db = EmbeddingsDb::open(directory.path()).await.unwrap();
        for entry in [
            row("a", "https://www.example.com/a", true, false),
            row("b", "https://example.com./b", false, true),
            row("c", "https://news.example.com/c", true, false),
            row("d", "https://127.0.0.1/d", true, false),
        ] {
            db.upsert(entry).await.unwrap();
        }

        assert_eq!(
            list_sites(&db).await.unwrap(),
            vec![
                SiteSummary {
                    hostname: "example.com".to_string(),
                    entry_count: 2,
                    unread_count: 1,
                },
                SiteSummary {
                    hostname: "news.example.com".to_string(),
                    entry_count: 1,
                    unread_count: 1,
                },
            ]
        );
        assert_eq!(
            list_site_entries(&db, "example.com", ItemFilter::Saved)
                .await
                .unwrap()
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            vec!["b"]
        );
        assert_eq!(
            list_site_entries(&db, "example.com", ItemFilter::Unread)
                .await
                .unwrap()
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            vec!["a"]
        );
        assert_eq!(
            list_site_entries(&db, "example.com", ItemFilter::All)
                .await
                .unwrap()
                .len(),
            2
        );

        db.upsert(row("b", "https://other.example/b", true, false))
            .await
            .unwrap();
        db.delete_by_id("c").await.unwrap();
        assert_eq!(
            list_sites(&db).await.unwrap(),
            vec![
                SiteSummary {
                    hostname: "example.com".to_string(),
                    entry_count: 1,
                    unread_count: 1,
                },
                SiteSummary {
                    hostname: "other.example".to_string(),
                    entry_count: 1,
                    unread_count: 1,
                },
            ]
        );
    }
}
