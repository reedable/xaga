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

    let Host::Domain(domain) = url.host()? else {
        return None;
    };
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    let domain = domain.strip_prefix("www.").unwrap_or(domain);
    if !is_valid_dns_hostname(domain) {
        return None;
    }

    Some(SiteHostname(domain.to_string()))
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
