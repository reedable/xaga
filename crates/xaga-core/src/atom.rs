//! Intersection of Atom and domain
//! File system concerns should be managed by `persistence` layer

use std::collections::HashSet;

use chrono::{DateTime, FixedOffset, Utc};

use crate::types::{Entry, Feed};
use crate::util::{feed_entry_id, normalize_url, url_hash12};

/// Use atom_syndication for serializing and persisting Feed data in .atom (XML) format.
/// Reads and write the local .atom files.
use atom_syndication as atom_persistence;
/// Use feed_rs for parsing incoming atom, rss, json.
/// Parses payload fetched from remote untrusted sites.
use feed_rs as atom_parser;

use atom_parser::model::Feed as IncomingAtom;
/// The `.atom` storage format. `Atom`/`AtomEntry` are xaga's names for the `atom_persistence`
/// feed/entry types; `atom.rs` is the only module that references `atom_persistence` directly — the
/// rest of the crate uses these names.
pub(crate) use atom_persistence::Entry as AtomEntry;
pub(crate) use atom_persistence::Feed as Atom;

/// Atom `<category>` scheme that namespaces xaga's own app-local categories (e.g. the
/// `pinned` flag) so they never collide with publisher-supplied categories.
const TAG_CATEGORY_SCHEME: &str = "https://xaga.app/ns/tag";

// ── raw → Atom (parse) ─────────────────────────────────────────────────────────

/// Parse locally stored Atom XML file into an `Atom`. Returns the parse error message as a `String`
/// so this layer stays free of the persistence error type; the caller maps it into its own error.
pub(crate) fn parse_atom(xml: &str) -> Result<Atom, String> {
    let sanitized = fix_bare_ampersands(xml);
    sanitized.parse::<Atom>().map_err(|e| e.to_string())
}

/// Parse a freshly-fetched feed (any format) via `atom_parser`.
/// `feed_rs` never leaves this module; callers hand `apply_incoming` the raw XML.
fn parse_incoming_atom(xml: &str) -> Result<IncomingAtom, String> {
    let sanitized = fix_bare_ampersands(xml);
    atom_parser::parser::parse(sanitized.as_bytes()).map_err(|e| e.to_string())
}

/// Escape bare `&` characters in XML that are not part of a valid entity or character reference.
/// Skips `<![CDATA[...]]>` sections so their content is never modified.
/// This makes lenient parsing of feeds from publishers who forget to encode `&` in URLs.
fn fix_bare_ampersands(xml: &str) -> String {
    let mut result = String::with_capacity(xml.len() + 64);
    let mut rest = xml;

    // Fast path: no CDATA sections — avoids rescanning the whole string for
    // "<![CDATA[" on every `&`, which is O(n×k) for files with many entities.
    if !xml.contains("<![CDATA[") {
        loop {
            match rest.find('&') {
                None => {
                    result.push_str(rest);
                    break;
                }
                Some(a) => {
                    result.push_str(&rest[..a]);
                    let after = &rest[a + 1..];
                    let is_valid_ref = after.starts_with("amp;")
                        || after.starts_with("lt;")
                        || after.starts_with("gt;")
                        || after.starts_with("apos;")
                        || after.starts_with("quot;")
                        || after.starts_with('#');
                    if is_valid_ref {
                        result.push('&');
                    } else {
                        result.push_str("&amp;");
                    }
                    rest = after;
                }
            }
        }
        return result;
    }

    // CDATA-aware path for feeds that embed content in CDATA sections.
    loop {
        let amp = rest.find('&');
        let cdata = rest.find("<![CDATA[");

        match (amp, cdata) {
            (None, _) => {
                result.push_str(rest);
                break;
            }
            (Some(a), Some(c)) if c < a => {
                // CDATA section starts before the next `&` — copy it through verbatim.
                result.push_str(&rest[..c]);
                rest = &rest[c..];
                if let Some(end) = rest.find("]]>") {
                    result.push_str(&rest[..end + 3]);
                    rest = &rest[end + 3..];
                } else {
                    // Unclosed CDATA; copy remainder as-is.
                    result.push_str(rest);
                    break;
                }
            }
            (Some(a), _) => {
                result.push_str(&rest[..a]);
                let after = &rest[a + 1..];
                let is_valid_ref = after.starts_with("amp;")
                    || after.starts_with("lt;")
                    || after.starts_with("gt;")
                    || after.starts_with("apos;")
                    || after.starts_with("quot;")
                    || after.starts_with('#');
                if is_valid_ref {
                    result.push('&');
                } else {
                    result.push_str("&amp;");
                }
                rest = after;
            }
        }
    }

    result
}

fn to_fixed(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

fn now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

// ── Atom → raw (serialize) ─────────────────────────────────────────────────────

/// Serialize an `Atom` to its `.atom` XML bytes in memory. `atom_synidcation::feed::Feed.write_to`
/// does *not* write anything to the disc. It merely "writes" them in memory.
pub(crate) fn atom_to_xml(feed: &Atom) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    feed.write_to(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

// ── construction & mutation ────────────────────────────────────────────────────

/// Creates an empty `.atom` file object for `feed`. `feed.url` is stored as
/// `<link rel="self">`.
///
/// Use this for creating a new feed (`add_feed`).
///
/// WARNING: Writing an existing feed through here would erase its entries.
///
/// To update an existing feed (refresh/pin/rename): load the stored `.atom`
/// with `parse_atom`, change it, write it back. `refresh_feed` gets new
/// entries by fetching the self-link URL.
pub(crate) fn feed_to_atom(feed: &Feed) -> Atom {
    let mut self_link = atom_persistence::Link::default();
    self_link.set_href(xml_attr_encode(&feed.url));
    self_link.set_rel("self");

    let mut atom = Atom::default();
    atom.set_id(feed.id.clone());
    atom.set_updated(now_fixed());
    atom.set_links(vec![self_link]);
    if !feed.title.is_empty() {
        set_title(&mut atom, &feed.title);
    }
    set_pinned(&mut atom, feed.is_pinned);
    atom
}

/// Set the feed's `<title>`.
pub(crate) fn set_title(feed: &mut Atom, title: &str) {
    feed.set_title(atom_persistence::Text::plain(title));
}

/// Add or remove the app-local pinned `<category>` (under [`TAG_CATEGORY_SCHEME`]). Idempotent.
/// Returns whether the feed changed.
pub(crate) fn set_pinned(feed: &mut Atom, pinned: bool) -> bool {
    if is_feed_pinned(feed) == pinned {
        return false;
    }

    let mut categories = feed.categories().to_vec();
    if pinned {
        let mut cat = atom_persistence::Category::default();
        cat.set_term(crate::tags::PINNED);
        cat.set_scheme(Some(TAG_CATEGORY_SCHEME.to_string()));
        categories.push(cat);
    } else {
        categories.retain(|c| {
            !(c.term() == crate::tags::PINNED && c.scheme() == Some(TAG_CATEGORY_SCHEME))
        });
    }
    feed.set_categories(categories);
    true
}

/// Apply a freshly-fetched raw feed onto `existing`: parse it (`atom_parser `), take its `<id>` (or
/// `fallback_id` when absent), merge in new entries (duplicates skipped, all sorted newest-first),
/// and refresh `<updated>`. Returns the newly-added entries as `AtomEntry`s (storage form — the
/// caller converts to domain `Entry` only where it needs to). The parse error is returned as a `String`.
pub(crate) fn apply_incoming_xml(
    atom: &mut Atom,
    incoming_raw: &str,
    fallback_id: &str,
) -> Result<Vec<AtomEntry>, String> {
    let incoming_atom: IncomingAtom = parse_incoming_atom(incoming_raw)?;

    let feed_id = if incoming_atom.id.is_empty() {
        fallback_id.to_string()
    } else {
        incoming_atom.id.clone()
    };

    atom.set_id(feed_id);

    if atom.title().value.is_empty() {
        let fallback = self_link_url(atom)
            .map(|u| normalize_url(&u).unwrap_or(u))
            .unwrap_or_default();

        atom.set_title(atom_persistence::Text::plain(incoming_title(
            &incoming_atom.title,
            &fallback,
        )));
    }

    let new_entries: Vec<AtomEntry> = incoming_atom.entries.iter().map(to_atom_entry).collect();
    let added = merge_entries(atom, new_entries);
    atom.set_updated(now_fixed());

    Ok(added)
}

// ── reads ──────────────────────────────────────────────────────────────────────

pub(crate) fn self_link_url(feed: &Atom) -> Option<String> {
    feed.links()
        .iter()
        .find(|l| l.rel() == "self")
        .map(|l| l.href().to_string())
}

/// The membership entry-ids of a feed (one id per `<entry>`), in document order.
pub(crate) fn entry_ids(feed: &Atom) -> Vec<String> {
    feed.entries().iter().map(|e| atom_to_entry(e).id).collect()
}

/// Whether `feed` carries the app-local [`crate::tags::PINNED`] flag, stored as an Atom
/// `<category term="…">` under [`TAG_CATEGORY_SCHEME`].
fn is_feed_pinned(feed: &Atom) -> bool {
    feed.categories()
        .iter()
        .any(|c| c.term() == crate::tags::PINNED && c.scheme() == Some(TAG_CATEGORY_SCHEME))
}

// ── Atom → domain (convert) ────────────────────────────────────────────────────

pub(crate) fn atom_to_feed(
    id: &str,
    feed: &Atom,
    unread_ids: &HashSet<String>,
    bookmarked_ids: &HashSet<String>,
) -> Feed {
    let entry_ids: Vec<String> = feed
        .entries()
        .iter()
        .map(|e| {
            let url = e
                .links()
                .iter()
                .find(|l| l.rel() == "alternate")
                .or_else(|| e.links().first())
                .map(|l| l.href())
                .unwrap_or("");
            normalize_url(url)
                .map(|n| url_hash12(&n))
                .unwrap_or_else(|_| url_hash12(url))
        })
        .collect();

    let unread_count = entry_ids
        .iter()
        .filter(|id| unread_ids.contains(*id))
        .count();
    let _bookmarked_count = entry_ids
        .iter()
        .filter(|id| bookmarked_ids.contains(*id))
        .count();

    Feed {
        id: id.to_string(),
        url: self_link_url(feed).unwrap_or_default(),
        title: feed.title().value.clone(),
        description: feed.subtitle().map(|s| s.value.clone()),
        entry_count: feed.entries().len(),
        unread_count,
        // Atom has no feed-level creation time (`atom:feed` has `<updated>`, no `<published>`), so
        // created_at reuses `<updated>` for now — tech debt.
        created_at: feed.updated().to_rfc3339(),
        updated_at: Some(feed.updated().to_rfc3339()),
        is_pinned: is_feed_pinned(feed),
        members: vec![],
    }
}

pub(crate) fn atom_to_entry(e: &AtomEntry) -> Entry {
    let title = e.title().value.clone();
    let updated_at = e.updated().to_rfc3339();

    let url = e
        .links()
        .iter()
        .find(|l| l.rel() == "alternate")
        .or_else(|| e.links().first())
        .map(|l| l.href().to_string())
        .unwrap_or_default();

    let summary = e.summary().map(|s| s.value.clone());
    let published_at = e.published().map(|d| d.to_rfc3339());
    let normalized_url = normalize_url(&url).unwrap_or_else(|_| url.clone());

    Entry {
        id: url_hash12(&normalized_url),
        url,
        title,
        created_at: updated_at.clone(),
        published_at,
        updated_at: Some(updated_at),
        description: summary,
        is_unread: false,
        is_bookmarked: false,
        is_pinned: false,
        color: None,
    }
}

// ── atom_parser  → Atom (internal merge helpers) ────────────────────────────────────

/// Returns the title content from a feed-rs `Text`, or `fallback` if absent or empty.
fn incoming_title<'a>(title: &'a Option<atom_parser::model::Text>, fallback: &'a str) -> &'a str {
    title
        .as_ref()
        .filter(|t| !t.content.is_empty())
        .map(|t| t.content.as_str())
        .unwrap_or(fallback)
}

/// Encode a URL string for use as an XML attribute value.
/// `atom_persistence`/`quick_xml` writes href values verbatim, so bare `&` must be
/// pre-encoded as `&amp;` here; the XML parser decodes them back when reading.
fn xml_attr_encode(s: &str) -> String {
    if s.contains('&') {
        s.replace('&', "&amp;")
    } else {
        s.to_string()
    }
}

/// Normalise every HTTP(S) link `href` on an entry in-place.
/// Falls back to the raw href when `normalize_url` rejects the URL.
/// Used as a shared helper by `to_atom_entry`.
fn normalize_entry_links(entry: &mut AtomEntry) {
    let links: Vec<atom_persistence::Link> = entry
        .links()
        .iter()
        .map(|l| {
            let mut link = l.clone();
            let raw = if let Ok(norm) = normalize_url(l.href()) {
                norm
            } else {
                l.href().to_string()
            };
            link.set_href(xml_attr_encode(&raw));
            link
        })
        .collect();
    entry.set_links(links);
}

fn to_atom_person(src: &atom_parser::model::Person) -> atom_persistence::Person {
    let mut person = atom_persistence::Person::default();
    person.set_name(src.name.clone());
    if let Some(email) = &src.email {
        person.set_email(email.clone());
    }
    if let Some(uri) = &src.uri {
        person.set_uri(uri.clone());
    }
    person
}

fn to_atom_entry(src: &atom_parser::model::Entry) -> AtomEntry {
    let title = src
        .title
        .as_ref()
        .map(|t| t.content.as_str())
        .unwrap_or(src.id.as_str());

    // Derive a stable id: use the upstream id when present, otherwise hash the
    // first link href so the id is persisted and stays consistent across refreshes.
    let first_link_href = src.links.first().map(|l| l.href.as_str()).unwrap_or("");
    let entry_id = feed_entry_id(&src.id, first_link_href);

    let mut entry = AtomEntry::default();
    entry.set_id(entry_id);
    entry.set_title(atom_persistence::Text::plain(title));
    entry.set_updated(src.updated.map(to_fixed).unwrap_or_else(now_fixed));
    if let Some(pub_date) = src.published {
        entry.set_published(to_fixed(pub_date));
    }
    entry.set_authors(src.authors.iter().map(to_atom_person).collect::<Vec<_>>());
    entry.set_links(
        src.links
            .iter()
            .map(|l| {
                let mut link = atom_persistence::Link::default();
                link.set_href(l.href.clone());
                link.set_rel(l.rel.clone().unwrap_or_else(|| "alternate".to_string()));
                link
            })
            .collect::<Vec<_>>(),
    );
    if let Some(summary) = &src.summary {
        entry.set_summary(atom_persistence::Text::plain(summary.content.clone()));
    }
    if let Some(content) = &src.content {
        if let Some(body) = &content.body {
            let mut c = atom_persistence::Content::default();
            c.set_value(body.clone());
            c.set_content_type("html".to_string());
            entry.set_content(c);
        }
    }
    normalize_entry_links(&mut entry);
    entry
}

/// Append entries from `new_entries` that are not already in `feed` (matched by id).
/// Sorts all entries newest-first after merging.
/// Returns the entries that were actually added.
///
/// Membership is append-only: the `.atom` file is allowed to grow indefinitely.
/// The growth rate is capped by the payload size (`feed_max_body_bytes`,
/// default 10 MiB per feed per fetch) and by the frequency of user-initiated
/// refresh — no timer or scheduler refreshes feeds, so a hostile feed emitting
/// endless unique entries grows only as fast as the user chooses to refresh
/// it, and deleting the feed stops the growth (assessed 2026-07-02, not a
/// security gap). That assessment rests on refresh being user-initiated,
/// which is not incidental: Xaga is a PKM (see README.md), not a
/// latest-news engagement product, and automatic/background refresh is
/// contrary to that identity — there is no plan to add it. Should that
/// premise somehow change, this assessment is void and unbounded `.atom`
/// growth becomes a security risk.
fn merge_entries(feed: &mut Atom, new_entries: Vec<AtomEntry>) -> Vec<AtomEntry> {
    let existing_ids: HashSet<String> = feed.entries().iter().map(|e| e.id().to_string()).collect();

    let to_add: Vec<_> = new_entries
        .into_iter()
        .filter(|e| !existing_ids.contains(e.id()))
        .collect();

    if !to_add.is_empty() {
        let mut all = feed.entries().to_vec();
        all.extend(to_add.clone());
        all.sort_by(|a: &AtomEntry, b: &AtomEntry| b.updated().cmp(a.updated()));
        feed.set_entries(all);
    }

    to_add
}
