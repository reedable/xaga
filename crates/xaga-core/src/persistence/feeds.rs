//! Feeds persistence: the `.atom` boundary for feeds and their entry membership.
//! Collapsed from the former `feeds/{mod,feed,feed_entry}.rs` split into one module.

use std::collections::HashSet;
use std::path::Path;

use thiserror::Error;

use crate::{
    atom::{
        Atom, AtomEntry, apply_incoming_xml, atom_to_entry, atom_to_feed, atom_to_xml, entry_ids,
        feed_to_atom, parse_atom, self_link_url, set_pinned, set_title,
    },
    library_writer::{LibraryWriter, Tier as WriteTier, WriteToken},
    types::{Article, Feed},
    util::{normalize_url, url_hash12},
};

use super::walk::for_each_atom;

// ── Shared error + result ────────────────────────────────────────────────────

/// Status code the app shell's network proxy answers with when it refuses a
/// plaintext `http://` request because the HTTP-allowed setting is off.
/// Non-standard on purpose — see
/// docs/architecture/security/fetching_remote_content.md, "Plaintext HTTP
/// enforcement." Produced in `network_proxy.rs`; consumed here by [`fetch_xml`].
pub const HTTP_BLOCKED_STATUS: u16 = 470;

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("HTTP error fetching {url}: {source}")]
    Http {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Atom parse error: {0}")]
    AtomParse(String),
    #[error("feed parse error: {0}")]
    FeedParse(String),
    #[error("feed not found: {0}")]
    NotFound(String),
    #[error("feed missing self link: {0}")]
    MissingSelfLink(String),
    #[error("response body for {url} exceeded {max_bytes} bytes")]
    BodyTooLarge { url: String, max_bytes: u64 },
    #[error("plaintext HTTP fetch blocked by the \"HTTP allowed\" setting: {url}")]
    HttpBlocked { url: String },
    #[error("library writer: {0}")]
    Writer(String),
}

impl FeedError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            FeedError::InvalidUrl(_) => "feed:invalid-url",
            FeedError::Http { .. } => "feed:http",
            FeedError::Io(_) => "feed:io",
            FeedError::AtomParse(_) => "feed:atom-parse",
            FeedError::FeedParse(_) => "feed:parse",
            FeedError::NotFound(_) => "feed:not-found",
            FeedError::MissingSelfLink(_) => "feed:missing-self-link",
            FeedError::BodyTooLarge { .. } => "feed:body-too-large",
            FeedError::HttpBlocked { .. } => "feed:http-blocked",
            FeedError::Writer(_) => "feed:writer",
        }
    }

    /// User-safe message (no URLs or parser fragments). Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            FeedError::InvalidUrl(_) => "This feed's address is invalid.",
            FeedError::Http { .. } => "Couldn't reach this feed.",
            FeedError::Io(_) => "Couldn't read or write feed data.",
            FeedError::AtomParse(_) | FeedError::FeedParse(_) => "This feed couldn't be read.",
            FeedError::NotFound(_) => "This feed could not be found.",
            FeedError::MissingSelfLink(_) => "This feed is missing required information.",
            FeedError::BodyTooLarge { .. } => "This feed's response was too large.",
            FeedError::HttpBlocked { .. } => "This feed uses insecure HTTP, which is turned off.",
            FeedError::Writer(_) => "Couldn't save feed changes.",
        }
    }
}

/// FIXME Untangle FeedError for least visibility
pub(crate) type Result<T> = std::result::Result<T, FeedError>;

// ── Public API ───────────────────────────────────────────────────────────────

/// Derive a deterministic ID from a feed URL.
/// Pure/sync — no I/O.
pub fn url_to_feed_id(url: &str) -> Result<String> {
    let normalized = normalize_url(url).map_err(FeedError::InvalidUrl)?;
    Ok(url_hash12(&normalized))
}

/// Add a feed by URL. For an already-present feed, just refreshes it. For a new feed, builds the
/// entry-less domain [`Feed`], serializes it via `feed_to_atom`, fetches and merges in memory, then
/// commits once. If either the fetch/merge or the commit fails, the seeded (entry-less) `.atom` is
/// still written so the feed exists and can be refreshed later, and the original error is propagated.
/// Idempotent: re-adding an existing feed just refreshes it.
#[allow(clippy::too_many_arguments)]
pub async fn add_feed(
    client: &reqwest::Client,
    library_dir: &Path,
    writer: &LibraryWriter,
    url: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
    timeout_secs: u64,
    max_body_bytes: u64,
) -> Result<(Feed, Vec<Article>)> {
    let url = if url.trim_start().starts_with("http://") {
        format!("https://{}", url.trim_start()[7..].trim_start())
    } else {
        url.to_string()
    };

    let url = normalize_url(&url).map_err(FeedError::InvalidUrl)?;
    let id = url_to_feed_id(&url)?;

    tokio::fs::create_dir_all(crate::library::feeds_dir(library_dir)).await?;

    let path = crate::library::feed_path(library_dir, &id);

    if path.exists() {
        return refresh_feed(
            client,
            library_dir,
            writer,
            &id,
            unread,
            bookmarked,
            timeout_secs,
            max_body_bytes,
        )
        .await;
    }

    // New feed: serialize the entry-less domain Feed via atom.rs. Keep this seed so that, on any
    // later failure (fetch/merge or commit), a valid `.atom` is still persisted — the feed exists
    // and is refreshable, and the merge work just retries on the next refresh. The seed is the
    // entry-less atom, which serializes even when a merged entry would not. A seed commit is
    // `commit_via_writer` with no new entries (just the `.atom`), so even the fallback goes
    // through the feed lane.
    let empty_atom = seed_atom(id.clone(), url.clone());

    let mut atom = empty_atom.clone();

    let new_entries =
        match fetch_and_merge(client, &mut atom, &id, timeout_secs, max_body_bytes).await {
            Ok(entries) => entries,
            Err(e) => {
                commit_via_writer(writer, &id, &empty_atom, Vec::new(), false).await?;
                return Err(e);
            }
        };

    match commit_via_writer(writer, &id, &atom, new_entries, false).await {
        Ok(created) => Ok((atom_to_feed(&id, &atom, unread, bookmarked), created)),
        Err(e) => {
            commit_via_writer(writer, &id, &empty_atom, Vec::new(), false).await?;
            Err(e)
        }
    }
}

fn seed_atom(id: String, url: String) -> Atom {
    feed_to_atom(&Feed {
        id,
        url,
        title: String::new(),
        created_at: String::new(),
        updated_at: None,
        description: None,
        is_pinned: false,
        entry_count: 0,
        unread_count: 0,
        members: vec![],
    })
}

/// Re-fetch a feed by ID: read and parse the `.atom`, fetch + merge in memory, then commit.
/// Returns the summary and the Articles created, so the caller can enqueue them for indexing.
/// Plain ingestion — persistence only.
#[allow(clippy::too_many_arguments)]
async fn refresh_feed(
    client: &reqwest::Client,
    library_dir: &Path,
    writer: &LibraryWriter,
    id: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
    timeout_secs: u64,
    max_body_bytes: u64,
) -> Result<(Feed, Vec<Article>)> {
    let path = crate::library::feed_path(library_dir, id);

    if !path.exists() {
        return Err(FeedError::NotFound(id.to_string()));
    }

    let mut existing =
        parse_atom(&tokio::fs::read_to_string(&path).await?).map_err(FeedError::AtomParse)?;

    let new_entries =
        fetch_and_merge(client, &mut existing, id, timeout_secs, max_body_bytes).await?;
    let created = commit_via_writer(writer, id, &existing, new_entries, true).await?;

    Ok((atom_to_feed(id, &existing, unread, bookmarked), created))
}

/// Fetch a feed's live XML from its self-link and merge new entries into `existing` in memory.
/// No writes — the caller owns persistence (`commit_feed`). Returns the newly-added entries.
async fn fetch_and_merge(
    client: &reqwest::Client,
    atom: &mut Atom,
    id: &str,
    timeout_secs: u64,
    max_body_bytes: u64,
) -> Result<Vec<AtomEntry>> {
    let url = self_link_url(atom).ok_or_else(|| FeedError::MissingSelfLink(id.to_string()))?;
    let incoming_xml = fetch_xml(client, &url, timeout_secs, max_body_bytes).await?;
    apply_incoming_xml(atom, &incoming_xml, id).map_err(FeedError::FeedParse)
}

/// Commit a fetched+merged feed through the library writer, Regular-tier (re-derivable,
/// droppable at shutdown). Each new entry is created on ITS OWN lane (create-if-absent,
/// atomic against a concurrent create from another feed carrying the same entry, or a
/// user's write to that entry), and only once every entry has landed is the `.atom`
/// written on the feed lane — preserving the entries-before-atom ordering invariant.
/// Per-entry write failures are logged and skipped (best-effort, as before); an `.atom`
/// write failure fails the commit.
async fn commit_via_writer(
    writer: &LibraryWriter,
    id: &str,
    atom: &Atom,
    new_entries: Vec<AtomEntry>,
    require_exists: bool,
) -> Result<Vec<Article>> {
    let mut created = Vec::new();
    for atom_entry in &new_entries {
        let entry = atom_to_entry(atom_entry);
        let url = entry.url;
        if url.is_empty() {
            continue;
        }
        match writer
            .write_feed_entry(
                WriteTier::Regular,
                url.clone(),
                entry.title,
                entry.published_at,
                entry.updated_at,
                entry.description,
            )
            .await
        {
            Ok(Some(article)) => created.push(article),
            Ok(None) => {} // already existed — left untouched
            Err(e) => log::warn!("feed commit: entry {url}: {e}"),
        }
    }
    writer
        .write_feed_atom(
            WriteTier::Regular,
            id.to_string(),
            atom.clone(),
            require_exists,
        )
        .await
        .map_err(|e| FeedError::Writer(e.to_string()))?;
    Ok(created)
}

/// Write a feed's `.atom` file. Invoked by the writer's `WriteFeedAtom` op, on the feed
/// lane, after all of the feed's entries have been committed (the ordering invariant now
/// lives in `commit_via_writer`, which awaits the entry writes before this). When
/// `require_exists` is set (a refresh), skip the write if the feed file is gone — the user
/// removed it during the network fetch, and the refresh must not resurrect it. The check
/// and the write are atomic on the feed lane, serialized against `remove_feed`.
pub(crate) async fn write_feed_atom(
    library_dir: &Path,
    id: &str,
    atom: &Atom,
    require_exists: bool,
    _token: WriteToken,
) -> Result<()> {
    if require_exists && !tokio::fs::try_exists(crate::library::feed_path(library_dir, id)).await? {
        log::info!("feed refresh: {id} was removed during fetch — not resurrecting");
        return Ok(());
    }
    write_atom_atomic(library_dir, id, atom).await
}

/// Re-fetch all feeds. Per-feed errors are collected rather than propagated early.
pub async fn refresh_all_feeds(
    client: &reqwest::Client,
    library_dir: &Path,
    writer: &LibraryWriter,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
    timeout_secs: u64,
    max_body_bytes: u64,
) -> Vec<(String, Result<(Feed, Vec<Article>)>)> {
    // Walk + parse each `.atom` once; the parsed feed flows straight into the refresh, so we
    // never re-read the file it was just walked from.
    let mut feeds: Vec<(String, Atom)> = Vec::new();

    for_each_atom(
        &crate::library::feeds_dir(library_dir),
        &parse_atom,
        |path, feed| {
            feeds.push((crate::util::item_id(&path), feed));
        },
    )
    .await;

    let mut results = Vec::with_capacity(feeds.len());

    for (id, mut existing) in feeds {
        let result: Result<(Feed, Vec<Article>)> = async {
            let new_entries =
                fetch_and_merge(client, &mut existing, &id, timeout_secs, max_body_bytes).await?;
            let created = commit_via_writer(writer, &id, &existing, new_entries, true).await?;
            Ok((atom_to_feed(&id, &existing, unread, bookmarked), created))
        }
        .await;
        results.push((id, result));
    }

    results
}

/// List all feeds on disk, parsed from their Atom XML.
pub async fn list_feeds(
    library_dir: &Path,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
) -> Result<Vec<Feed>> {
    let mut feeds: Vec<Feed> = Vec::new();

    for_each_atom(
        &crate::library::feeds_dir(library_dir),
        &parse_atom,
        |path, atom_feed| {
            let id = crate::util::item_id(&path);
            feeds.push(atom_to_feed(&id, &atom_feed, unread, bookmarked));
        },
    )
    .await;

    // for_each_atom yields in directory order; sort by id for a stable listing.
    feeds.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(feeds)
}

/// Adds (or removes) the app-local `pinned` category on a feed's `.atom`, under
/// [`TAG_CATEGORY_SCHEME`]. Idempotent. Returns whether the file changed.
pub(crate) async fn set_feed_pinned(
    library_dir: &Path,
    id: &str,
    pinned: bool,
    _token: WriteToken,
) -> Result<bool> {
    let path = crate::library::feed_path(library_dir, id);
    let xml = tokio::fs::read_to_string(&path).await?;
    let mut atom = parse_atom(&xml).map_err(FeedError::AtomParse)?;

    if !set_pinned(&mut atom, pinned) {
        return Ok(false);
    }

    write_atom_atomic(library_dir, id, &atom).await?;
    Ok(true)
}

/// Rename a feed by updating its `<title>` in the stored Atom file.
pub(crate) async fn rename_feed(
    library_dir: &Path,
    id: &str,
    new_title: &str,
    unread: &HashSet<String>,
    bookmarked: &HashSet<String>,
    _token: WriteToken,
) -> Result<Feed> {
    let path = crate::library::feed_path(library_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.to_string()));
    }
    let xml = tokio::fs::read_to_string(&path).await?;
    let mut feed = parse_atom(&xml).map_err(FeedError::AtomParse)?;
    set_title(&mut feed, new_title);
    write_atom_atomic(library_dir, id, &feed).await?;
    Ok(atom_to_feed(id, &feed, unread, bookmarked))
}

/// Remove a feed (deletes `{id}.atom`).
pub(crate) async fn remove_feed(library_dir: &Path, id: &str, _token: WriteToken) -> Result<()> {
    let path = crate::library::feed_path(library_dir, id);
    if !path.exists() {
        return Err(FeedError::NotFound(id.to_string()));
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

/// Returns the membership id-set for `url` (one feed) or all feeds (`None`),
/// deduplicated. The `.atom` is the source of truth for feed membership; entry data
/// and state are fetched from LanceDB by the caller (see the `list_entries` command).
pub async fn list_entry_ids(library_dir: &Path, url: Option<&str>) -> Result<Vec<String>> {
    let feeds_dir = crate::library::feeds_dir(library_dir);
    let mut ids = Vec::new();
    let mut seen = HashSet::new();

    match url {
        Some(url) => {
            let normalized = normalize_url(url)?;
            let id = url_to_feed_id(&normalized)?;
            let path = crate::library::feed_path(library_dir, &id);

            if !tokio::fs::try_exists(&path).await? {
                return Err(FeedError::NotFound(id));
            }

            let xml = tokio::fs::read_to_string(&path).await?;
            let feed = parse_atom(&xml).map_err(FeedError::AtomParse)?;
            for id in entry_ids(&feed) {
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
        None => {
            for_each_atom(&feeds_dir, &parse_atom, |_path, feed| {
                for id in entry_ids(&feed) {
                    if seen.insert(id.clone()) {
                        ids.push(id);
                    }
                }
            })
            .await;
        }
    }

    Ok(ids)
}

// ── Shared infrastructure (private) ──────────────────────────────────────────

async fn fetch_xml(
    client: &reqwest::Client,
    url: &str,
    timeout_secs: u64,
    max_body_bytes: u64,
) -> Result<String> {
    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(|e| FeedError::Http {
            url: url.to_string(),
            source: e,
        })?;

    // The network proxy's "plaintext HTTP is disabled" refusal. Mapped before
    // error_for_status so it doesn't surface as a generic server 4xx.
    if response.status().as_u16() == HTTP_BLOCKED_STATUS {
        return Err(FeedError::HttpBlocked {
            url: url.to_string(),
        });
    }

    let mut response = response.error_for_status().map_err(|e| FeedError::Http {
        url: url.to_string(),
        source: e,
    })?;

    // Streamed read with a byte cap: bounds the memory a single response can
    // occupy. The `.timeout(...)` above spans this loop too — reqwest's total
    // request timeout runs until the body is fully read.
    let mut body: Vec<u8> = Vec::new();

    while let Some(chunk) = response.chunk().await.map_err(|e| FeedError::Http {
        url: url.to_string(),
        source: e,
    })? {
        if body.len() as u64 + chunk.len() as u64 > max_body_bytes {
            return Err(FeedError::BodyTooLarge {
                url: url.to_string(),
                max_bytes: max_body_bytes,
            });
        }
        body.extend_from_slice(&chunk);
    }

    // reqwest is built without the `charset` feature (default-features = false),
    // so the `.text()` this replaces decoded as lossy UTF-8; this preserves that.
    Ok(String::from_utf8_lossy(&body).into_owned())
}

async fn write_atom_atomic(library_dir: &Path, id: &str, atom: &Atom) -> Result<()> {
    let buf = atom_to_xml(atom).map_err(FeedError::AtomParse)?;
    crate::fs::write_atomic_async(crate::library::feed_path(library_dir, id), buf).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    /// Serves one canned HTTP response on a random loopback port and hands
    /// back the request head it received. Plain client, no network proxy — the
    /// proxied client can't be exercised here because the proxy blocks
    /// loopback targets (same constraint as the fetch_client tests).
    fn serve_once(response: Vec<u8>) -> (u16, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut head = Vec::new();
            let mut buf = [0u8; 1024];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => head.extend_from_slice(&buf[..n]),
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&head).to_lowercase());
            let _ = stream.write_all(&response);
        });
        (port, rx)
    }

    fn xml_response(body: &[u8], gzipped: bool) -> Vec<u8> {
        let encoding = if gzipped {
            "Content-Encoding: gzip\r\n"
        } else {
            ""
        };
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n",
            encoding,
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(body);
        response
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    const TINY_XML: &[u8] = b"<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>";

    /// `gzip(false)` sends no `Accept-Encoding` header at all — the request
    /// shape must stay byte-identical to the pre-gzip-feature client, because
    /// the header's absence is the documented default (see
    /// docs/architecture/security/fetching_untrusted_content.md).
    #[tokio::test]
    async fn gzip_disabled_sends_no_accept_encoding() {
        let (port, rx) = serve_once(xml_response(TINY_XML, false));
        let client = reqwest::Client::builder().gzip(false).build().unwrap();
        fetch_xml(&client, &format!("http://127.0.0.1:{port}/"), 5, 1 << 20)
            .await
            .unwrap();
        let head = rx.recv().unwrap();
        assert!(
            !head.contains("accept-encoding"),
            "expected no Accept-Encoding header, got:\n{head}"
        );
    }

    /// `gzip(true)` negotiates gzip and transparently decompresses.
    #[tokio::test]
    async fn gzip_enabled_sends_accept_encoding_and_decompresses() {
        let (port, rx) = serve_once(xml_response(&gzip(TINY_XML), true));
        let client = reqwest::Client::builder().gzip(true).build().unwrap();
        let body = fetch_xml(&client, &format!("http://127.0.0.1:{port}/"), 5, 1 << 20)
            .await
            .unwrap();
        assert_eq!(body.as_bytes(), TINY_XML);
        let head = rx.recv().unwrap();
        assert!(
            head.contains("accept-encoding: gzip"),
            "expected Accept-Encoding: gzip, got:\n{head}"
        );
    }

    /// The `max_body_bytes` cap counts *decompressed* bytes: reqwest inflates
    /// before `chunk()` yields, so a small compressed response cannot smuggle
    /// an over-limit body past the cap (decompression-bomb bound).
    #[tokio::test]
    async fn gzip_cap_counts_decompressed_bytes() {
        let inflated = vec![b'a'; 64 * 1024];
        let compressed = gzip(&inflated);
        let cap: u64 = 1024;
        assert!(
            (compressed.len() as u64) < cap,
            "test premise: compressed body must fit under the cap"
        );
        let (port, _rx) = serve_once(xml_response(&compressed, true));
        let client = reqwest::Client::builder().gzip(true).build().unwrap();
        let err = fetch_xml(&client, &format!("http://127.0.0.1:{port}/"), 5, cap)
            .await
            .expect_err("over-cap decompressed body must be refused");
        assert!(
            matches!(err, FeedError::BodyTooLarge { max_bytes, .. } if max_bytes == cap),
            "expected BodyTooLarge, got: {err:?}"
        );
    }

    /// A refresh commit (`require_exists = true`) must NOT recreate a feed the user removed
    /// during the network fetch; an add commit (`require_exists = false`) creates it. This
    /// is the existence gate that closes the remove-during-refresh resurrection.
    #[tokio::test]
    async fn refresh_commit_does_not_resurrect_a_removed_feed() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path();
        let id = "feedid123456";
        tokio::fs::create_dir_all(crate::library::feeds_dir(lib))
            .await
            .unwrap();
        let atom = feed_to_atom(&Feed {
            id: id.into(),
            url: "https://example.com/feed".into(),
            title: "T".into(),
            created_at: String::new(),
            updated_at: None,
            description: None,
            is_pinned: false,
            entry_count: 0,
            unread_count: 0,
            members: vec![],
        });
        let path = crate::library::feed_path(lib, id);

        // Add creates it.
        write_feed_atom(lib, id, &atom, false, WriteToken::for_test())
            .await
            .unwrap();
        assert!(tokio::fs::try_exists(&path).await.unwrap());

        // User removes it (as if during a refresh's fetch).
        tokio::fs::remove_file(&path).await.unwrap();

        // Refresh commit must skip — no resurrection.
        write_feed_atom(lib, id, &atom, true, WriteToken::for_test())
            .await
            .unwrap();
        assert!(
            !tokio::fs::try_exists(&path).await.unwrap(),
            "refresh must not resurrect a removed feed"
        );

        // Sanity: an add commit still would.
        write_feed_atom(lib, id, &atom, false, WriteToken::for_test())
            .await
            .unwrap();
        assert!(tokio::fs::try_exists(&path).await.unwrap());
    }
}
