use sha2::{Digest, Sha256};

/// Parses and normalises a URL: lowercases scheme + host (via the `url` crate),
/// strips the fragment, rewrites `http` to `https`, strips a leading `www.`
/// from the host (only when a dot remains, so hosts like `www.com` survive),
/// and strips any trailing slash.
///
/// The output feeds `url_hash12`, which names entry/feed/bookmark files on
/// disk — changing these rules re-keys persisted items (see the migration
/// item in TODO.md).
pub fn normalize_url(raw: &str) -> Result<String, url::ParseError> {
    let mut parsed = url::Url::parse(raw)?;
    parsed.set_fragment(None);
    if parsed.scheme() == "http" {
        let _ = parsed.set_scheme("https");
    }
    if let Some(stripped) = parsed
        .host_str()
        .and_then(|h| h.strip_prefix("www."))
        .filter(|h| h.contains('.'))
        .map(str::to_string)
    {
        let _ = parsed.set_host(Some(&stripped));
    }
    let mut s = parsed.to_string();
    if s.ends_with('/') {
        s.pop();
    }
    Ok(s)
}

/// Returns the first 12 hex characters of the SHA-256 of `normalized`.
/// Used as a compact, filesystem-safe identifier for both feeds and documents.
pub(crate) fn url_hash12(normalized: &str) -> String {
    let hash = Sha256::digest(normalized.as_bytes());
    hex::encode(hash)[..12].to_string()
}

/// Returns the stable `id` for a feed entry.
/// Uses the Atom `<entry><id>` value when non-empty; falls back to
/// `url_hash12(normalize_url(link_href))` when the entry id is absent or blank
/// (malformed Atom).
pub(crate) fn feed_entry_id(atom_id: &str, link_href: &str) -> String {
    if !atom_id.is_empty() {
        return atom_id.to_string();
    }
    normalize_url(link_href)
        .map(|n| url_hash12(&n))
        .unwrap_or_else(|_| url_hash12(link_href))
}

/// Returns the canonical ID for any library item: the file stem.
pub(crate) fn item_id(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_strips_fragment() {
        assert_eq!(
            normalize_url("https://example.com/a#section").unwrap(),
            "https://example.com/a"
        );
    }

    #[test]
    fn normalize_url_rewrites_http_to_https() {
        assert_eq!(
            normalize_url("http://example.com/a").unwrap(),
            "https://example.com/a"
        );
    }

    #[test]
    fn normalize_url_strips_leading_www() {
        assert_eq!(
            normalize_url("https://www.example.com/a").unwrap(),
            "https://example.com/a"
        );
    }

    #[test]
    fn normalize_url_keeps_bare_www_host() {
        assert_eq!(normalize_url("https://www.com/a").unwrap(), "https://www.com/a");
    }

    #[test]
    fn normalize_url_strips_trailing_slash() {
        assert_eq!(normalize_url("https://example.com/").unwrap(), "https://example.com");
    }

    #[test]
    fn normalize_url_preserves_query() {
        assert_eq!(
            normalize_url("https://example.com/a?page=2#x").unwrap(),
            "https://example.com/a?page=2"
        );
    }

    #[test]
    fn normalize_url_combined() {
        assert_eq!(
            normalize_url("HTTP://WWW.Example.COM/a/#sec").unwrap(),
            "https://example.com/a"
        );
    }
}
