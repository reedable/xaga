use std::path::{Path, PathBuf};

use chrono::DateTime;
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

pub use crate::types::Entry;
use crate::util::{normalize_url, url_hash12};

#[derive(Debug, Error)]
pub enum BookmarksError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

// ── Path ──────────────────────────────────────────────────────────────────────

fn bookmarks_path(data_dir: &Path) -> PathBuf {
    data_dir.join("bookmarks.html")
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn rfc3339_to_unix(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.timestamp())
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
        if let Some(ref desc) = e.summary {
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
            summary,
            is_unread: false,
        });
    }
    entries
}

// ── Internal I/O ──────────────────────────────────────────────────────────────

async fn read_all(data_dir: &Path) -> Result<Vec<Entry>, BookmarksError> {
    let path = bookmarks_path(data_dir);
    if !tokio::fs::try_exists(&path).await? {
        return Ok(vec![]);
    }
    let raw = tokio::fs::read_to_string(&path).await?;
    Ok(parse(&raw))
}

async fn write_all(data_dir: &Path, entries: &[Entry]) -> Result<(), BookmarksError> {
    tokio::fs::create_dir_all(data_dir).await?;
    tokio::fs::write(bookmarks_path(data_dir), render(entries).as_bytes()).await?;
    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Adds or updates a bookmark.
///
/// Deduplicates by normalized URL: if a bookmark for the same URL already
/// exists it is replaced in-place (upsert); otherwise the entry is appended.
/// Returns the stored entry (with the normalized URL as `id` and `source`).
pub async fn add_bookmark(data_dir: &Path, entry: &Entry) -> Result<Entry, BookmarksError> {
    let normalized = normalize_url(&entry.url)?;
    let mut entries = read_all(data_dir).await?;

    let hash = url_hash12(&normalized);
    let new_entry = Entry {
        id: hash.clone(),
        url: normalized,
        title: entry.title.clone(),
        created_at: entry.created_at.clone(),
        published_at: entry.published_at.clone(),
        updated_at: entry.updated_at.clone(),
        summary: entry.summary.clone(),
        is_unread: false,
    };

    match entries.iter().position(|e| e.id == hash) {
        Some(pos) => entries[pos] = new_entry,
        None => entries.push(new_entry),
    }

    write_all(data_dir, &entries).await?;

    // Re-borrow the stored entry from the vec (position is stable after the write).
    Ok(entries.into_iter().find(|e| e.id == hash).unwrap())
}

/// Removes a bookmark by ID.  Returns the removed entry's ID, or `None` if not found.
pub async fn remove_bookmark(data_dir: &Path, id: &str) -> Result<Option<String>, BookmarksError> {
    let mut entries = read_all(data_dir).await?;
    let removed_id = entries.iter().find(|e| e.id == id).map(|e| e.id.clone());
    entries.retain(|e| e.id != id);
    write_all(data_dir, &entries).await?;
    Ok(removed_id)
}

/// Returns all bookmarks, in insertion order.
pub async fn list_bookmarks(data_dir: &Path) -> Result<Vec<Entry>, BookmarksError> {
    read_all(data_dir).await
}
