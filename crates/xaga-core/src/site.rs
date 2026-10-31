use std::fmt;

use url::{Host, Url};

/// The normalized DNS hostname that identifies a derived Site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SiteHostname(String);

impl SiteHostname {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SiteHostname {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Derives a Site identity from an HTTP(S) URL with a valid DNS hostname.
pub(crate) fn normalize_site_hostname(value: &str) -> Option<SiteHostname> {
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    let domain = dns_hostname(&url)?;
    let domain = domain.strip_prefix("www.").unwrap_or(domain);
    Some(SiteHostname(domain.to_string()))
}

fn dns_hostname(url: &Url) -> Option<&str> {
    let Host::Domain(domain) = url.host()? else {
        return None;
    };
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    is_valid_dns_hostname(domain).then_some(domain)
}

/// Normalize a bare hostname supplied to the pin store, never a URL or path.
pub(crate) fn normalize_pin_hostname(value: &str) -> Option<String> {
    if value.is_empty()
        || value.chars().any(|c| {
            c.is_whitespace() || matches!(c, '/' | '\\' | ':' | '?' | '#' | '@' | '%' | '\0')
        })
    {
        return None;
    }
    // Pin payloads carry an existing Site identity, not an article URL.
    // Reapplying the one-www alias rule would change www.www.example.com's
    // derived identity (www.example.com) on every persistence round trip.
    let url = Url::parse(&format!("https://{value}/")).ok()?;
    dns_hostname(&url).map(str::to_owned)
}

fn is_valid_dns_hostname(hostname: &str) -> bool {
    !hostname.is_empty()
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_http_hostnames() {
        for value in [
            "https://Example.COM/article",
            "http://example.com./article",
            "https://www.example.com/article",
            "https://WWW.EXAMPLE.COM./article",
        ] {
            assert_eq!(
                normalize_site_hostname(value)
                    .as_ref()
                    .map(SiteHostname::as_str),
                Some("example.com"),
                "{value}"
            );
        }
    }

    #[test]
    fn preserves_non_www_subdomains() {
        assert_eq!(
            normalize_site_hostname("https://news.example.com/article")
                .as_ref()
                .map(SiteHostname::as_str),
            Some("news.example.com")
        );
    }

    #[test]
    fn normalizes_pin_identity_without_reapplying_url_aliases() {
        for (input, expected) in [
            ("Example.COM.", "example.com"),
            ("www.example.com", "www.example.com"),
            ("BÜCHER.example", "xn--bcher-kva.example"),
        ] {
            assert_eq!(normalize_pin_hostname(input).as_deref(), Some(expected));
            assert_eq!(normalize_pin_hostname(expected).as_deref(), Some(expected));
        }
    }

    #[test]
    fn excludes_non_http_invalid_and_non_dns_hosts() {
        for value in [
            "not a url",
            "mailto:reader@example.com",
            "file:///tmp/article",
            "https://127.0.0.1/article",
            "https://[::1]/article",
            "https://-invalid.example/article",
        ] {
            assert!(normalize_site_hostname(value).is_none(), "{value}");
        }
    }
}
