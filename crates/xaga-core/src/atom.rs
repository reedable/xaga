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

/// Namespace for Xaga-owned fields in locally persisted Atom documents.
pub(crate) const XAGA_NAMESPACE: &str = "urn:xaga:atom";

fn get_local_id(atom: &Atom) -> Result<Option<&str>, String> {
    let mut ids = atom
        .extensions
        .iter()
        .filter(|(prefix, _)| {
            atom.namespaces
                .get(*prefix)
                .is_some_and(|uri| uri == XAGA_NAMESPACE)
        })
        .flat_map(|(_, fields)| fields.get("id").into_iter().flatten());
    let Some(field) = ids.next() else {
        return Ok(None);
    };
    let id = field.value.as_deref().ok_or("empty local Feed identity")?;
    if ids.next().is_some()
        || !field.children.is_empty()
        || !crate::persistence::entries::is_safe_id(id)
    {
        return Err("invalid local Feed identity".into());
    }
    Ok(Some(id))
}

pub(crate) fn resolve_local_id(atom: &Atom, path: &std::path::Path) -> Result<String, String> {
    crate::persistence::identity::resolve(get_local_id(atom)?, path)
}

pub(crate) fn ensure_local_id(atom: &mut Atom, path: &std::path::Path) -> Result<bool, String> {
    let id = resolve_local_id(atom, path)?;
    if get_local_id(atom)?.is_some() {
        return Ok(false);
    }
    set_local_id(atom, &id);
    Ok(true)
}

fn set_local_id(atom: &mut Atom, id: &str) {
    let prefix = atom
        .namespaces
        .iter()
        .find(|(_, uri)| uri.as_str() == XAGA_NAMESPACE)
        .map(|(prefix, _)| prefix.clone())
        .unwrap_or_else(|| {
            let mut prefix = "xaga".to_owned();
            while atom.namespaces.contains_key(&prefix) {
                prefix.push('_');
            }
            atom.namespaces
                .insert(prefix.clone(), XAGA_NAMESPACE.into());
            prefix
        });
    let field = atom_persistence::extension::Extension {
        name: format!("{prefix}:id"),
        value: Some(id.into()),
        ..Default::default()
    };
    atom.extensions
        .entry(prefix)
        .or_default()
        .insert("id".into(), vec![field]);
}

// ── raw → Atom (parse) ─────────────────────────────────────────────────────────

/// Parse locally stored Atom XML file into an `Atom`. Returns the parse error message as a `String`
/// so this layer stays free of the persistence error type; the caller maps it into its own error.
pub fn parse_atom(xml: &str) -> Result<Atom, String> {
    let sanitized = fix_bare_ampersands(xml);
    let (normalized, saved_id) = normalize_local_namespaces(&sanitized)?;
    let mut atom = normalized.parse::<Atom>().map_err(|e| e.to_string())?;
    // The storage library trims extension text. Local IDs retain their exact
    // saved characters, including whitespace, just as Markdown IDs do.
    if let Some(saved_id) = saved_id {
        for (prefix, fields) in &mut atom.extensions {
            if atom
                .namespaces
                .get(prefix)
                .is_some_and(|uri| uri == XAGA_NAMESPACE)
            {
                if let Some(ids) = fields.get_mut("id") {
                    for id in ids {
                        id.value = Some(saved_id.clone());
                    }
                }
            }
        }
    }
    get_local_id(&atom)?;
    Ok(atom)
}

/// The storage library groups extensions by prefix and does not resolve local
/// namespace declarations. Normalize Xaga element names at the XML boundary so
/// prefix aliases and shadowing cannot change ownership or overwrite foreign fields.
fn normalize_local_namespaces(xml: &str) -> Result<(String, Option<String>), String> {
    use quick_xml::{Writer, events::Event, name::ResolveResult, reader::NsReader};
    use std::collections::BTreeMap;
    const ATOM_NAMESPACE: &[u8] = b"http://www.w3.org/2005/Atom";
    // NsReader exposes raw namespace attribute bytes, including character references.
    let decode_namespace = |namespace: &[u8]| -> Result<String, String> {
        let namespace = std::str::from_utf8(namespace).map_err(|e| e.to_string())?;
        Ok(quick_xml::escape::unescape(namespace)
            .map_err(|e| e.to_string())?
            .into_owned())
    };
    let mut declarations = BTreeMap::<String, HashSet<String>>::new();
    let mut used = BTreeMap::<String, String>::new();
    let mut scan = NsReader::from_str(xml);
    loop {
        let (namespace, event) = scan.read_resolved_event().map_err(|e| e.to_string())?;
        if matches!(namespace, ResolveResult::Unknown(_)) {
            return Err("unbound XML namespace prefix".into());
        }
        if let Event::Start(element) | Event::Empty(element) = &event {
            if let ResolveResult::Bound(namespace) = namespace {
                let namespace = decode_namespace(namespace.as_ref())?;
                if namespace.as_bytes() != ATOM_NAMESPACE {
                    let name = element.name();
                    let prefix = name
                        .prefix()
                        .map(|p| String::from_utf8_lossy(p.as_ref()).into_owned())
                        .unwrap_or_else(|| "ext".into());
                    used.entry(namespace).or_insert(prefix);
                }
            }
            for attr in element.attributes() {
                let attr = attr.map_err(|e| e.to_string())?;
                if let Some(prefix) = attr.key.as_ref().strip_prefix(b"xmlns:") {
                    declarations
                        .entry(String::from_utf8_lossy(prefix).into_owned())
                        .or_default()
                        .insert(
                            attr.decode_and_unescape_value(scan.decoder())
                                .map_err(|e| e.to_string())?
                                .into_owned(),
                        );
                }
            }
        }
        if matches!(event, Event::Eof) {
            break;
        }
    }
    used.insert(XAGA_NAMESPACE.into(), "xaga".into());
    let mut prefixes = BTreeMap::<String, String>::new();
    for (uri, mut prefix) in used {
        while declarations
            .get(&prefix)
            .is_some_and(|uris| uris.iter().any(|other| other != &uri))
            || prefixes.values().any(|existing| existing == &prefix)
        {
            prefix.push('_');
        }
        prefixes.insert(uri, prefix);
    }
    let mut reader = NsReader::from_str(xml);
    let mut writer = Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut extension = false;
    let mut ids = 0usize;
    let mut reading_id = false;
    let mut saved_id: Option<String> = None;
    loop {
        let (ns, event) = reader.read_resolved_event().map_err(|e| e.to_string())?;
        let uri = match ns {
            ResolveResult::Bound(n) => Some(decode_namespace(n.as_ref())?),
            _ => None,
        };
        let owned = uri.as_deref() == Some(XAGA_NAMESPACE);
        let empty = matches!(event, Event::Empty(_));
        let event = match event {
            Event::Start(element) | Event::Empty(element) => {
                if reading_id {
                    return Err("local Feed identity must contain only text".into());
                }
                if depth == 1 {
                    extension = uri
                        .as_deref()
                        .is_some_and(|uri| uri.as_bytes() != ATOM_NAMESPACE);
                }
                if owned && depth == 1 && element.local_name().as_ref() == b"id" {
                    ids += 1;
                    if ids > 1 || empty {
                        return Err("invalid or duplicate local Feed identity".into());
                    }
                    reading_id = true;
                    saved_id = Some(String::new());
                }
                let mut element = element.into_owned();
                if extension {
                    // Resolved element names now use root declarations. The storage
                    // parser otherwise turns xmlns declarations into ordinary attributes.
                    let attributes = element
                        .attributes()
                        .map(|attribute| {
                            let attribute = attribute.map_err(|e| e.to_string())?;
                            Ok((
                                attribute.key.as_ref().to_vec(),
                                attribute.value.into_owned(),
                            ))
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    element.clear_attributes();
                    for (key, value) in &attributes {
                        if key != b"xmlns" && !key.starts_with(b"xmlns:") {
                            element.push_attribute((key.as_slice(), value.as_slice()));
                        }
                    }
                    if let Some(prefix) = uri.as_ref().and_then(|uri| prefixes.get(uri)) {
                        let name = format!(
                            "{prefix}:{}",
                            String::from_utf8_lossy(element.local_name().as_ref())
                        );
                        element.set_name(name.as_bytes());
                    }
                }
                if depth == 0 {
                    for (uri, prefix) in &prefixes {
                        let attr = format!("xmlns:{prefix}");
                        if !element
                            .attributes()
                            .any(|a| a.is_ok_and(|a| a.key.as_ref() == attr.as_bytes()))
                        {
                            // The string attribute API escapes values; the byte API does not.
                            element.push_attribute((attr.as_str(), uri.as_str()));
                        }
                    }
                }
                if empty {
                    Event::Empty(element)
                } else {
                    depth += 1;
                    Event::Start(element)
                }
            }
            Event::End(element) => {
                depth = depth.saturating_sub(1);
                let event = if extension {
                    if let Some(prefix) = uri.as_ref().and_then(|uri| prefixes.get(uri)) {
                        Event::End(quick_xml::events::BytesEnd::new(format!(
                            "{prefix}:{}",
                            String::from_utf8_lossy(element.local_name().as_ref())
                        )))
                    } else {
                        Event::End(element.into_owned())
                    }
                } else {
                    Event::End(element.into_owned())
                };
                if depth == 1 {
                    extension = false;
                    reading_id = false;
                }
                event
            }
            Event::Text(text) if reading_id => {
                saved_id
                    .as_mut()
                    .expect("identity element")
                    .push_str(&text.unescape().map_err(|e| e.to_string())?);
                Event::Text(text.into_owned())
            }
            Event::CData(text) if reading_id => {
                saved_id
                    .as_mut()
                    .expect("identity element")
                    .push_str(std::str::from_utf8(text.as_ref()).map_err(|e| e.to_string())?);
                Event::CData(text.into_owned())
            }
            Event::Eof => break,
            event => event.into_owned(),
        };
        writer.write_event(event).map_err(|e| e.to_string())?;
    }
    Ok((
        String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())?,
        saved_id,
    ))
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
pub fn atom_to_xml(feed: &Atom) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    // atom_syndication writes namespace attribute values verbatim. Keep decoded
    // URIs in memory for ownership checks, encoding only the serialization copy.
    if feed
        .namespaces
        .values()
        .any(|uri| uri.contains(['&', '<', '>', '\"', '\'']))
    {
        let mut encoded = feed.clone();
        for uri in encoded.namespaces.values_mut() {
            *uri = quick_xml::escape::escape(uri.as_str()).into_owned();
        }
        encoded.write_to(&mut buf).map_err(|e| e.to_string())?;
    } else {
        feed.write_to(&mut buf).map_err(|e| e.to_string())?;
    }
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
    set_local_id(&mut atom, &feed.id);
    atom.set_updated(now_fixed());
    atom.set_links(vec![self_link]);
    if !feed.title.is_empty() {
        set_title(&mut atom, &feed.title);
    }

    atom
}

/// Set the feed's `<title>`.
pub(crate) fn set_title(feed: &mut Atom, title: &str) {
    feed.set_title(atom_persistence::Text::plain(title));
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

/// Replace the fetch address when re-adding an existing feed under the same ID.
pub(crate) fn set_self_link_url(feed: &mut Atom, url: &str) {
    let mut links = feed.links().to_vec();
    links.retain(|link| link.rel() != "self");
    let mut link = atom_persistence::Link::default();
    link.set_rel("self");
    link.set_href(xml_attr_encode(url));
    links.push(link);
    feed.set_links(links);
}

/// The membership entry-ids of a feed (one id per `<entry>`), in document order.
pub(crate) fn entry_ids(feed: &Atom) -> Vec<String> {
    feed.entries().iter().map(|e| atom_to_entry(e).id).collect()
}

// ── Atom → domain (convert) ────────────────────────────────────────────────────

pub(crate) fn count_unread_feed_entries(
    entries: &[AtomEntry],
    unread_ids: &HashSet<String>,
) -> usize {
    entries
        .iter()
        .filter(|entry| {
            let url = entry
                .links()
                .iter()
                .find(|link| link.rel() == "alternate")
                .or_else(|| entry.links().first())
                .map(|link| link.href())
                .unwrap_or("");
            let id = normalize_url(url)
                .map(|normalized| url_hash12(&normalized))
                .unwrap_or_else(|_| url_hash12(url));
            unread_ids.contains(&id)
        })
        .count()
}

pub(crate) fn atom_to_feed(id: &str, feed: &Atom, unread_count: usize) -> Feed {
    Feed {
        id: get_local_id(feed)
            .expect("validated local Feed identity")
            .unwrap_or(id)
            .to_owned(),
        url: self_link_url(feed).unwrap_or_default(),
        title: feed.title().value.clone(),
        description: feed.subtitle().map(|s| s.value.clone()),
        entry_count: feed.entries().len(),
        unread_count,
        // Atom has no feed-level creation time (`atom:feed` has `<updated>`, no `<published>`), so
        // created_at reuses `<updated>` for now — tech debt.
        created_at: feed.updated().to_rfc3339(),
        updated_at: Some(feed.updated().to_rfc3339()),
        is_pinned: false,
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
        tags: None,
        id: url_hash12(&normalized_url),
        url,
        title,
        created_at: updated_at.clone(),
        published_at,
        updated_at: Some(updated_at),
        description: summary,
        is_unread: false,
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
