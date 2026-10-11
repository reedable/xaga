//! Factory for the SSRF-proxied reqwest clients (feed XML).
//!
//! Clients are built per invocation — not held in `AppState` — so a
//! settings.json change applies on the next fetch without a relaunch.

use std::sync::Arc;

/// Redirect cap when `max_redirects` is absent from settings.json. 3 is a UX
/// exception to secure-by-default (0 would be the secure value) — see
/// docs/architecture/security/fetching_untrusted_content.md, exception granted 2026-07-02.
pub(crate) const DEFAULT_MAX_REDIRECTS: u32 = 3;

/// Hard ceiling on the redirect cap; always in force, the setting cannot
/// exceed it.
pub(crate) const MAX_REDIRECTS_CEILING: u32 = 10;

/// The redirect cap actually applied: the setting (or the default when
/// absent), never above [`MAX_REDIRECTS_CEILING`].
pub(crate) fn effective_max_redirects(setting: Option<u32>) -> u32 {
    setting
        .unwrap_or(DEFAULT_MAX_REDIRECTS)
        .min(MAX_REDIRECTS_CEILING)
}

/// Build a reqwest client that dials only through the local SSRF-filtering
/// proxy, following at most `max_redirects` redirects (0 = the initial
/// request only; reqwest's `Policy::limited(0)` refuses the first redirect).
/// `referer_allowed` controls whether redirect hops receive a `Referer`
/// header naming the previous URL — i.e. whether the header is permitted to
/// go out at all (reqwest defaults to sending it; `false` withholds it, so
/// redirect targets don't learn the original feed URL).
/// `gzip_allowed` controls whether requests carry `Accept-Encoding: gzip`
/// (with transparent decompression of compressed responses). `false` sends
/// no `Accept-Encoding` at all — the pre-existing request shape. One flag
/// configures clients from this factory; the reqwest `gzip`
/// cargo feature would otherwise default this to on, so it is always set
/// explicitly here.
///
/// Deliberately no `.https_only()`: the "HTTP allowed" setting is enforced in
/// [`xaga_core::network_proxy`] instead. See
/// docs/architecture/security/fetching_remote_content.md, "CONNECT port
/// policy," for why enforcement lives there and not here.
pub(crate) fn build(
    proxy_port: u16,
    max_redirects: u32,
    referer_allowed: bool,
    gzip_allowed: bool,
) -> reqwest::Client {
    try_build(proxy_port, max_redirects, referer_allowed, gzip_allowed)
        .expect("failed to build SSRF-proxied HTTP client")
}

/// Fallible construction for callers that collect per-step diagnostics.
pub(crate) fn try_build(
    proxy_port: u16,
    max_redirects: u32,
    referer_allowed: bool,
    gzip_allowed: bool,
) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(xaga_core::network_proxy::proxy_url(
            proxy_port,
        ))?)
        .redirect(reqwest::redirect::Policy::limited(max_redirects as usize))
        .referer(referer_allowed)
        .gzip(gzip_allowed)
        .build()
}

/// The `http_allowed` handle shared between the network proxy (enforcement) and
/// setup (seeding from settings.json). Starts `false`: the proxy is up before
/// settings are read, and the pre-settings window must be the secure state.
pub(crate) fn http_allowed_handle() -> Arc<std::sync::RwLock<bool>> {
    Arc::new(std::sync::RwLock::new(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn effective_max_redirects_defaults_and_clamps() {
        assert_eq!(effective_max_redirects(None), 3);
        assert_eq!(effective_max_redirects(Some(0)), 0);
        assert_eq!(effective_max_redirects(Some(7)), 7);
        assert_eq!(effective_max_redirects(Some(10)), 10);
        assert_eq!(effective_max_redirects(Some(50)), 10);
    }

    /// Pins the `Policy::limited(0)` semantics the factory relies on: the
    /// first redirect is refused with a redirect error — "hit the server
    /// once, honor no redirects". Uses a plain client with the same policy;
    /// the factory's client can't be exercised here because the network proxy
    /// blocks loopback targets.
    #[tokio::test]
    async fn limited_zero_refuses_first_redirect() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = server.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let _ = s.write_all(
                b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(0))
            .build()
            .unwrap();
        let err = client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .expect_err("redirect should be refused");
        assert!(err.is_redirect(), "expected a redirect error, got: {err}");
    }
}
