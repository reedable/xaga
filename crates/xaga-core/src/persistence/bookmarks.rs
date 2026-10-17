use std::path::Path;

use chrono::DateTime;
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

use crate::{
    library,
    library_writer::{BookmarkOutcome, LibraryWriter, Tier as WriteTier},
    md::md_to_entry,
    persistence::walk::for_each_md,
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

impl BookmarksError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            BookmarksError::Io(_) => "bookmark:io",
            BookmarksError::InvalidUrl(_) => "bookmark:invalid-url",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            BookmarksError::Io(_) => "Couldn't read or write bookmark data.",
            BookmarksError::InvalidUrl(_) => "This bookmark's address is invalid.",
        }
    }
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn escape_html(s: &str) -> String {
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
        let mut attrs = format!("HREF=\"{}\" ADD_DATE=\"{add_date}\"", escape_html(&e.url));

        if let Some(ts) = e.published_at.as_deref().and_then(rfc3339_to_unix) {
            attrs.push_str(&format!(" PUBLISHED_AT=\"{ts}\""));
        }

        if let Some(ts) = e.updated_at.as_deref().and_then(rfc3339_to_unix) {
            attrs.push_str(&format!(" LAST_MODIFIED=\"{ts}\""));
        }

        out.push_str(&format!(
            "    <DT><A {attrs}>{}</A>\n",
            escape_html(&e.title)
        ));

        if let Some(ref desc) = e.description {
            out.push_str(&format!("    <DD>{}\n", escape_html(desc)));
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
/// - If an Entry already exists: adds the
///   "bookmark" tag via `set_entry_tags`.
/// - Otherwise: creates a new Entry in `read/` with `tags: ["bookmark"]`.
///
/// Duplicate URLs within the file are skipped after the first occurrence.
/// The caller is responsible for queuing embed messages for the returned ids.
pub async fn import_bookmarks(
    writer: &LibraryWriter,
    html: &str,
) -> Result<BookmarkImportResult, BookmarksError> {
    let parsed = parse(html);
    let mut seen = std::collections::HashSet::new();
    let mut added: Vec<Article> = Vec::new();
    let mut updated: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    // Bookmarks are user-authored intent (not re-derivable), so imports write on the
    // Interactive tier — the shutdown drain must not drop them. The existing-vs-new
    // decision lives INSIDE the entry lane (one `bookmark_entry` op), so a concurrent
    // create of the same URL can never cause the bookmark tag to be silently dropped.
    for entry in parsed {
        if !seen.insert(entry.url.clone()) {
            continue;
        }

        match writer
            .bookmark_entry(
                WriteTier::Interactive,
                entry.url.clone(),
                entry.title,
                entry.created_at,
                entry.published_at,
                entry.updated_at,
                entry.description,
            )
            .await
        {
            Ok(BookmarkOutcome::Added(article)) => added.push(*article),
            Ok(BookmarkOutcome::Updated(id)) => updated.push(id),
            Err(error) => {
                log::warn!("import_bookmarks: failed entry {:?}: {error}", entry.url);
                failed.push(entry.url);
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

    for_each_md(&library::entries_dir(library_dir), &md_to_entry, |e| entries.push(e)).await;

    entries.retain(|e| e.is_bookmarked);

    tokio::fs::write(dest_path, render(&entries).as_bytes()).await?;
    Ok(())
}
