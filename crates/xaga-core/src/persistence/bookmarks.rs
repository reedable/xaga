use std::path::Path;

use chrono::DateTime;
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

use crate::{
    fm::parse_frontmatter,
    library,
    md::md_to_entry,
    persistence::scan::for_each_md,
    types::{Article, Entry},
    util::{normalize_url, url_hash12},
};

#[derive(Debug, Error)]
pub enum BookmarksError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn rfc3339_to_unix(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

fn unix_to_rfc3339(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

// ── Render ────────────────────────────────────────────────────────────────────

fn render(entries: &[Entry]) -> String {
    let mut out = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
         <!-- This is an automatically generated file.\n\
              It will be read and overwritten.\n\
              DO NOT EDIT! -->\n\
         <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
         <TITLE>Bookmarks</TITLE>\n\
         <H1>Bookmarks</H1>\n\
         <DL><p>\n",
    );

    for e in entries {
        let add_date = rfc3339_to_unix(&e.created_at).unwrap_or(0);
        let mut attrs = format!("HREF=\"{}\" ADD_DATE=\"{add_date}\"", html_escape(&e.url));

        if let Some(ts) = e.published_at.as_deref().and_then(rfc3339_to_unix) {
            attrs.push_str(&format!(" PUBLISHED_AT=\"{ts}\""));
        }

        if let Some(ts) = e.updated_at.as_deref().and_then(rfc3339_to_unix) {
            attrs.push_str(&format!(" LAST_MODIFIED=\"{ts}\""));
        }

        out.push_str(&format!(
            "    <DT><A {attrs}>{}</A>\n",
            html_escape(&e.title)
        ));

        if let Some(ref desc) = e.description {
            out.push_str(&format!("    <DD>{}\n", html_escape(desc)));
        }
    }

    out.push_str("</DL><p>\n");
    out
}

// ── Parse ─────────────────────────────────────────────────────────────────────

fn parse(html: &str) -> Vec<Entry> {
    let doc = Html::parse_document(html);
    let a_sel = Selector::parse("dl dt a").unwrap();
    let mut entries = Vec::new();

    for a in doc.select(&a_sel) {
        let href = match a.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        let normalized = match normalize_url(href) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let title = a.text().collect::<String>();

        let created_at = a
            .value()
            .attr("add_date")
            .and_then(|s| s.parse::<i64>().ok())
            .map(unix_to_rfc3339)
            .unwrap_or_default();

        let published_at = a
            .value()
            .attr("published_at")
            .and_then(|s| s.parse::<i64>().ok())
            .map(unix_to_rfc3339);

        let updated_at = a
            .value()
            .attr("last_modified")
            .and_then(|s| s.parse::<i64>().ok())
            .map(unix_to_rfc3339);

        // Description: the <DD> immediately following the parent <DT>
        let summary = (|| -> Option<String> {
            let dt_node = a.parent()?;
            let mut next = dt_node.next_sibling();

            while let Some(node) = next {
                if let Some(elem) = ElementRef::wrap(node) {
                    if elem.value().name() == "dd" {
                        let text = elem.text().collect::<String>().trim().to_string();
                        return if text.is_empty() { None } else { Some(text) };
                    }

                    return None; // Hit the next <DT> or other element — stop.
                }
                next = node.next_sibling(); // Skip text/comment nodes.
            }

            None
        })();

        entries.push(Entry {
            id: url_hash12(&normalized),
            url: normalized,
            title,
            created_at,
            published_at,
            updated_at,
            description: summary,
            is_unread: false,
            is_bookmarked: false,
            color: None,
        });
    }
    entries
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct BookmarkImportResult {
    /// [`Article`]s for Entries newly created by the import, so the caller can enqueue their
    /// indexing without re-reading the files.
    pub added: Vec<Article>,
    /// IDs of Entries that already existed and received the "bookmark" tag (a state change).
    pub updated: Vec<String>,
    /// URLs that could not be processed.
    pub failed: Vec<String>,
}

/// Parses `html` as a Netscape bookmark file and creates or updates Entries.
///
/// For each URL:
/// - If an Entry already exists (fetched or previously imported): adds the
///   "bookmark" tag via `set_entry_tags`.
/// - Otherwise: creates a new Entry in `read/` with `tags: ["bookmark"]`.
///
/// Duplicate URLs within the file are skipped after the first occurrence.
/// The caller is responsible for queuing embed messages for the returned ids.
pub async fn import_bookmarks(
    library_dir: &Path,
    html: &str,
) -> Result<BookmarkImportResult, BookmarksError> {
    let bookmark_tag = crate::tags::BOOKMARK.to_string();
    let parsed = parse(html);
    let mut seen = std::collections::HashSet::new();
    let mut added: Vec<Article> = Vec::new();
    let mut updated: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for entry in parsed {
        if !seen.insert(entry.url.clone()) {
            continue;
        }

        let id = url_hash12(&entry.url);

        if let Some(path) = crate::persistence::entries::find_entry_path(library_dir, &id).await {
            // Entry already exists — add "bookmark" tag if not already present. Tags are a
            // persistence detail (not on the domain Entry), so read them from the file.
            match tokio::fs::read_to_string(&path).await {
                Ok(raw) => {
                    let mut tags = parse_frontmatter(&raw)
                        .map(|fm| fm.tags)
                        .unwrap_or_default();
                    if !tags.contains(&bookmark_tag) {
                        tags.push(bookmark_tag.clone());
                        match crate::persistence::entries::set_entry_tags(library_dir, &id, tags).await
                        {
                            Ok(_) => updated.push(id),
                            Err(_) => failed.push(entry.url),
                        }
                    } else {
                        updated.push(id);
                    }
                }
                Err(_) => failed.push(entry.url),
            }
        } else {
            // No Entry exists — create one in read/ with the bookmark tag.
            match crate::persistence::entries::write_read_entry(
                library_dir,
                &entry.url,
                &entry.title,
                &entry.created_at,
                entry.published_at.as_deref(),
                entry.updated_at.as_deref(),
                entry.description.as_deref(),
                &[bookmark_tag.clone()],
            )
            .await
            {
                Ok(content) => added.push(content),
                Err(_) => failed.push(entry.url),
            }
        }
    }

    Ok(BookmarkImportResult {
        added,
        updated,
        failed,
    })
}

/// Exports all bookmarked entries as a Netscape bookmark file to `dest_path`.
pub async fn export_bookmarks(library_dir: &Path, dest_path: &Path) -> Result<(), BookmarksError> {
    let mut entries: Vec<Entry> = Vec::new();

    for_each_md(&library::entries_dir(library_dir), &mut entries, &md_to_entry).await;

    entries.retain(|e| e.is_bookmarked);

    tokio::fs::write(dest_path, render(&entries).as_bytes()).await?;
    Ok(())
}
