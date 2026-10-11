//! Local HTTP network proxy with configurable request policies.
//!
//! The SSRF policy validates destinations before forwarding requests for article
//! loaders and native fetch clients. The deny-all policy rejects UI requests
//! before DNS resolution or an upstream connection, with an exact loopback
//! development-server exception when configured.
//!
//! Policies apply to requests received by the proxy. WebKit's known loopback
//! bypass is documented in docs/architecture/security/fetching_remote_content.md.
//!
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Maximum number of header bytes read before the request line + headers must be
/// complete. Bounds memory and rejects slow/oversized header floods.
const MAX_HEAD_BYTES: usize = 16 * 1024;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProxyError {
    /// The request could not be parsed (bad request line / target).
    #[error("proxy request could not be parsed")]
    BadRequest,
    /// DNS resolution failed or returned no addresses.
    #[error("proxy DNS resolution failed")]
    Dns,
    /// The hostname is local-only or resolves to a disallowed address.
    #[error("proxy target hostname or address is disallowed")]
    Blocked,
}

impl ProxyError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    pub fn code(&self) -> &'static str {
        match self {
            ProxyError::BadRequest => "proxy:bad-request",
            ProxyError::Dns => "proxy:dns",
            ProxyError::Blocked => "proxy:blocked",
        }
    }

    /// User-safe message. Full detail stays in `Display`.
    pub fn user_message(&self) -> &'static str {
        match self {
            ProxyError::BadRequest => "The request couldn't be processed.",
            ProxyError::Dns => "Couldn't resolve the address.",
            ProxyError::Blocked => "This address is not allowed.",
        }
    }
}

// ── Address policy ──────────────────────────────────────────────────────────────

/// Returns true when `v4` is in a range the loader must never reach.
fn is_blocked_v4(v4: Ipv4Addr) -> bool {
    let o = v4.octets();
    v4.is_loopback()              // 127.0.0.0/8
        || v4.is_private()        // 10/8, 172.16/12, 192.168/16
        || v4.is_link_local()     // 169.254.0.0/16 (incl. 169.254.169.254 cloud metadata)
        || v4.is_unspecified()    // 0.0.0.0
        || v4.is_broadcast()      // 255.255.255.255
        || v4.is_documentation()  // 192.0.2/24, 198.51.100/24, 203.0.113/24
        || (o[0] == 100 && (o[1] & 0xc0) == 64)        // 100.64.0.0/10 CGNAT (RFC 6598)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24 IETF protocol assignments
}

/// Returns true when `ip` is in a range the loader must never reach. This is the
/// security-critical allow/deny core: only globally-routable addresses pass.
fn is_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => {
            // ::1 / :: — handle before any embedded-v4 extraction, since their
            // trailing hextets (0.0.0.1 / 0.0.0.0) don't both read as blocked v4.
            if v6.is_loopback() || v6.is_unspecified() {
                return true;
            }
            // IPv4-mapped (::ffff:a.b.c.d) — judge by the embedded v4 address.
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_blocked_v4(v4);
            }
            let seg = v6.segments();
            // Addresses that *carry* an IPv4 destination: the kernel routes them
            // to the embedded v4, so a public-looking IPv6 wrapper can otherwise
            // smuggle a private/loopback IPv4 past the filter. Judge by the
            // embedded address.
            //   NAT64        64:ff9b::/96  — well-known prefix, commonly routed
            //   IPv4-compat  ::/96         — deprecated ::a.b.c.d
            // Both carry the v4 in the final two hextets.
            let nat64 = seg[0] == 0x0064 && seg[1] == 0xff9b && seg[2..6].iter().all(|&s| s == 0);
            let v4_compat = seg[..6].iter().all(|&s| s == 0);
            if nat64 || v4_compat {
                let v4 = Ipv4Addr::new(
                    (seg[6] >> 8) as u8,
                    (seg[6] & 0xff) as u8,
                    (seg[7] >> 8) as u8,
                    (seg[7] & 0xff) as u8,
                );
                return is_blocked_v4(v4);
            }
            //   6to4         2002::/16     — v4 in the 2nd/3rd hextets
            if seg[0] == 0x2002 {
                let v4 = Ipv4Addr::new(
                    (seg[1] >> 8) as u8,
                    (seg[1] & 0xff) as u8,
                    (seg[2] >> 8) as u8,
                    (seg[2] & 0xff) as u8,
                );
                return is_blocked_v4(v4);
            }
            (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 unique-local (incl. fd00:ec2::254 IMDS)
                || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
        }
    }
}

// ── Resolution (injectable for tests) ───────────────────────────────────────────

/// Resolves a host to socket addresses. Abstracted so tests can simulate DNS
/// (including rebinding) deterministically and offline.
trait HostResolver: Send + Sync {
    fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>>;
}

/// Production resolver backed by the system resolver.
struct SystemResolver;

impl HostResolver for SystemResolver {
    fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(vec![SocketAddr::new(ip, port)]);
        }
        // Absolute DNS names prevent resolver search domains from turning a
        // public hostname into a .local lookup.
        let absolute = format!("{}.", host.trim_end_matches('.'));
        Ok((absolute.as_str(), port).to_socket_addrs()?.collect())
    }
}

/// Resolves `host:port` and returns a single validated address to dial, or an
/// error if resolution failed or *any* resolved address is disallowed. See
/// docs/architecture/security/fetching_remote_content.md, "Containment: the network proxy’s
/// SSRF policy," point 2, for why rejecting on *any* blocked address matters.
fn validate_target(
    host: &str,
    port: u16,
    resolver: &dyn HostResolver,
    is_blocked_fn: fn(IpAddr) -> bool,
) -> Result<SocketAddr, ProxyError> {
    // Reject before invoking the system resolver: resolving .local itself can
    // issue mDNS traffic and trigger macOS's Local Network permission prompt.
    // Unqualified names are also forbidden; they can expand via search domains.
    if local_name(host) {
        return Err(ProxyError::Blocked);
    }
    let addrs = resolver.resolve(host, port).map_err(|_| ProxyError::Dns)?;
    if addrs.is_empty() {
        return Err(ProxyError::Dns);
    }
    if addrs.iter().any(|a| is_blocked_fn(a.ip())) {
        return Err(ProxyError::Blocked);
    }
    // Pin to a concrete, already-validated address — the WebView never resolves.
    Ok(addrs[0])
}

fn local_name(host: &str) -> bool {
    let name = host.trim_end_matches('.');
    name.eq_ignore_ascii_case("local")
        || name.to_ascii_lowercase().ends_with(".local")
        || (!name.contains('.') && name.parse::<IpAddr>().is_err())
}

/// Async wrapper around [`validate_target`]. `to_socket_addrs` performs blocking
/// DNS, so it is run on the blocking pool to avoid stalling a runtime worker
/// while a lookup is in flight.
async fn validate_target_async(
    host: String,
    port: u16,
    resolver: Arc<dyn HostResolver>,
    is_blocked_fn: fn(IpAddr) -> bool,
) -> Result<SocketAddr, ProxyError> {
    tokio::task::spawn_blocking(move || {
        validate_target(&host, port, resolver.as_ref(), is_blocked_fn)
    })
    .await
    .map_err(|_| ProxyError::Dns)?
}

// ── Request parsing ──────────────────────────────────────────────────────────────

/// Splits an authority (`example.com:443`, `127.0.0.1:443`, `[::1]:443`) into
/// `(host, port)`, stripping brackets from IPv6 literals.
fn split_authority(authority: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        // [ipv6]:port  or  [ipv6]
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None if after.is_empty() => default_port,
            None => return None,
        };
        return Some((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => Some((host.to_string(), port.parse().ok()?)),
        _ => Some((authority.to_string(), default_port)),
    }
}

/// The parsed first line of a proxied request.
enum Target {
    /// `CONNECT host:port` — establish a tunnel (used for HTTPS).
    Connect { host: String, port: u16 },
    /// Absolute-form `GET http://host/path` — plain HTTP through the proxy.
    Http { host: String, port: u16 },
}

/// Parses the request line (first line, without CRLF).
fn parse_request_line(line: &str) -> Result<Target, ProxyError> {
    let mut parts = line.split_whitespace();
    let method = parts.next().ok_or(ProxyError::BadRequest)?;
    let target = parts.next().ok_or(ProxyError::BadRequest)?;

    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = split_authority(target, 443).ok_or(ProxyError::BadRequest)?;
        return Ok(Target::Connect { host, port });
    }

    // Absolute-form: scheme://authority/path. We only proxy http here (https
    // arrives via CONNECT).
    let rest = target
        .strip_prefix("http://")
        .ok_or(ProxyError::BadRequest)?;
    let authority = rest.split('/').next().unwrap_or(rest);
    let (host, port) = split_authority(authority, 80).ok_or(ProxyError::BadRequest)?;
    Ok(Target::Http { host, port })
}

// ── Connection handling ──────────────────────────────────────────────────────────

/// Reads the request line + headers (up to the blank `\r\n\r\n`). Returns the
/// head bytes (including the terminator) and any bytes already read past it.
async fn read_head(stream: &mut TcpStream) -> std::io::Result<Option<(Vec<u8>, Vec<u8>)>> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(None); // client closed before sending a full head
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            let head = buf[..pos + 4].to_vec();
            let leftover = buf[pos + 4..].to_vec();
            return Ok(Some((head, leftover)));
        }
        if buf.len() > MAX_HEAD_BYTES {
            return Ok(None);
        }
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[derive(Clone)]
pub struct ProxyConfig {
    deny_all: bool,
    // Development-only exception: exact configured host/port, pinned to loopback.
    local_endpoint: Option<(String, std::net::SocketAddr)>,
    observer: Option<Arc<dyn Fn(&str, &str) + Send + Sync>>,
    resolver: Arc<dyn HostResolver>,
    is_blocked_fn: fn(IpAddr) -> bool,
    /// Whether plaintext `http://` requests (absolute-form) are forwarded.
    /// Shared handle to the "HTTP allowed" setting, read per request; the proxy
    /// starts before settings load, so it holds the handle, not the value.
    /// `false` refuses with [`HTTP_BLOCKED_STATUS`]. See
    /// docs/architecture/security/fetching_remote_content.md, "CONNECT port
    /// policy," for why this is enforced here rather than client-side.
    http_allowed: Arc<std::sync::RwLock<bool>>,
    /// Whether `CONNECT` may target a port other than 443. Read per request,
    /// same reason as `http_allowed`: the proxy starts before settings load.
    /// `false` refuses non-443 `CONNECT` targets with 403 Forbidden.
    connect_nonstandard_ports_allowed: Arc<std::sync::RwLock<bool>>,
}

impl ProxyConfig {
    /// Reject requests before DNS resolution or opening an upstream connection.
    pub fn deny_all() -> Self {
        let mut config = Self::new(
            Arc::new(std::sync::RwLock::new(false)),
            Arc::new(std::sync::RwLock::new(false)),
        );
        config.deny_all = true;
        config
    }
    /// Permit only the application's local development server, never a remote host.
    pub fn with_local_endpoint(
        mut self,
        host: String,
        address: std::net::SocketAddr,
    ) -> std::io::Result<Self> {
        if !address.ip().is_loopback()
            || !(host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback()))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "development endpoint must be loopback",
            ));
        }
        self.local_endpoint = Some((host, address));
        Ok(self)
    }

    /// Receives bounded, content-free failure codes. The app associates this
    /// observer with a dedicated listener, never by hostname/time heuristics.
    pub fn with_observer(mut self, observer: Arc<dyn Fn(&str, &str) + Send + Sync>) -> Self {
        self.observer = Some(observer);
        self
    }
    fn report(&self, code: &str, detail: &str) {
        if let Some(observer) = &self.observer {
            observer(code, detail);
        }
    }
    /// Production configuration: system DNS + the real address policy.
    pub fn new(
        http_allowed: Arc<std::sync::RwLock<bool>>,
        connect_nonstandard_ports_allowed: Arc<std::sync::RwLock<bool>>,
    ) -> Self {
        Self {
            deny_all: false,
            local_endpoint: None,
            observer: None,
            resolver: Arc::new(SystemResolver),
            is_blocked_fn: is_blocked,
            http_allowed,
            connect_nonstandard_ports_allowed,
        }
    }
}

/// Binds the proxy to `127.0.0.1:0` and spawns its accept loop. Returns the
/// chosen local port to hand to `proxy_url`.
pub async fn start(config: ProxyConfig) -> std::io::Result<u16> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let cfg = config.clone();
                    tokio::spawn(async move {
                        let _ = handle_conn(stream, cfg).await;
                    });
                }
                Err(_) => break,
            }
        }
    });
    Ok(port)
}

/// A per-load proxy with exactly the same enforcement as the shared proxy.
/// Dropping it closes the listener and all tunnels, including hung requests.
pub struct ScopedProxy {
    pub port: u16,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for ScopedProxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn start_scoped(config: ProxyConfig) -> std::io::Result<ScopedProxy> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break };
                    let cfg = config.clone();
                    connections.spawn(async move {
                        if let Err(error) = handle_conn(stream, cfg.clone()).await {
                            cfg.report("proxy:io", &format!("{:?}", error.kind()));
                        }
                    });
                }
                _ = connections.join_next(), if !connections.is_empty() => {}
            }
        }
    });
    Ok(ScopedProxy { port, task })
}

/// The URL a client uses to dial the proxy listening on `port` — the single
/// construction of the proxy's address; every consumer (reqwest clients, the
/// loader webview's `proxy_url`) derives it here.
pub fn proxy_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// A reqwest client whose every request — including each redirect hop — dials
/// through the proxy at `port`, so it inherits the proxy's address validation
/// and plaintext-HTTP refusal.
pub fn proxied_client(port: u16) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy_url(port))?)
        .build()
}

async fn write_status(stream: &mut TcpStream, status: &str) -> std::io::Result<()> {
    stream
        .write_all(format!("HTTP/1.1 {status}\r\nConnection: close\r\n\r\n").as_bytes())
        .await
}

/// The refusal for plaintext HTTP when the "HTTP allowed" setting is off.
/// Status is [`HTTP_BLOCKED_STATUS`] so fetch paths can tell this apart from a
/// real server 4xx; the body is for the loader webview, whose capture would
/// otherwise show an empty page with no explanation.
async fn write_http_blocked(stream: &mut TcpStream) -> std::io::Result<()> {
    let body = "Plaintext HTTP is disabled (Xaga \"HTTP allowed\" setting).";
    stream
        .write_all(
            format!(
                "HTTP/1.1 {} HTTP Blocked\r\nConnection: close\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
                crate::persistence::feeds::HTTP_BLOCKED_STATUS,
                body.len(),
                body
            )
            .as_bytes(),
        )
        .await
}

async fn handle_conn(mut client: TcpStream, cfg: ProxyConfig) -> std::io::Result<()> {
    let Some((head, leftover)) = read_head(&mut client).await? else {
        return Ok(());
    };
    let head_str = String::from_utf8_lossy(&head);
    let first_line = head_str.lines().next().unwrap_or("");

    let target = match parse_request_line(first_line) {
        Ok(t) => t,
        Err(_) => {
            cfg.report("proxy:bad-request", "invalid request head");
            return write_status(&mut client, "400 Bad Request").await;
        }
    };

    let local_address = match (&cfg.local_endpoint, &target) {
        (Some((allowed, address)), Target::Http { host, port })
        | (Some((allowed, address)), Target::Connect { host, port })
            if host == allowed && *port == address.port() =>
        {
            Some(*address)
        }
        _ => None,
    };
    if cfg.deny_all && local_address.is_none() {
        let (host, port) = match &target {
            Target::Http { host, port } | Target::Connect { host, port } => (host, port),
        };
        cfg.report("proxy:blocked", &format!("UI remote access disabled; destination={host}:{port}; refused before DNS or connection"));
        return write_status(&mut client, "403 Forbidden").await;
    }
    match target {
        Target::Connect { host, port } => {
            if local_address.is_none()
                && port != 443
                && !*cfg.connect_nonstandard_ports_allowed.read().unwrap()
            {
                cfg.report("proxy:port-blocked", "nonstandard CONNECT port disabled");
                return write_status(&mut client, "403 Forbidden").await;
            }
            let addr = if let Some(address) = local_address {
                address
            } else {
                match validate_target_async(
                    host.clone(),
                    port,
                    cfg.resolver.clone(),
                    cfg.is_blocked_fn,
                )
                .await
                {
                    Ok(addr) => addr,
                    Err(ProxyError::Blocked) => {
                        let code = if local_name(&host) {
                            "proxy:hostname-blocked"
                        } else {
                            "proxy:address-blocked"
                        };
                        cfg.report(code, "target refused before connection");
                        return write_status(&mut client, "403 Forbidden").await;
                    }
                    Err(_) => {
                        cfg.report("proxy:dns", "resolution failed or returned no addresses");
                        return write_status(&mut client, "502 Bad Gateway").await;
                    }
                }
            };
            let mut upstream = match TcpStream::connect(addr).await {
                Ok(s) => s,
                Err(error) => {
                    cfg.report(
                        "proxy:connect",
                        &format!("{:?}; os_code={:?}", error.kind(), error.raw_os_error()),
                    );
                    return write_status(&mut client, "502 Bad Gateway").await;
                }
            };
            client
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .await?;
            // Any bytes already read past the CONNECT head belong to the tunnel.
            if !leftover.is_empty() {
                upstream.write_all(&leftover).await?;
            }
            let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
            Ok(())
        }
        Target::Http { host, port } => {
            if local_address.is_none() && !*cfg.http_allowed.read().unwrap() {
                cfg.report("proxy:http-blocked", "plaintext HTTP disabled");
                return write_http_blocked(&mut client).await;
            }
            let addr = if let Some(address) = local_address {
                address
            } else {
                match validate_target_async(
                    host.clone(),
                    port,
                    cfg.resolver.clone(),
                    cfg.is_blocked_fn,
                )
                .await
                {
                    Ok(addr) => addr,
                    Err(ProxyError::Blocked) => {
                        let code = if local_name(&host) {
                            "proxy:hostname-blocked"
                        } else {
                            "proxy:address-blocked"
                        };
                        cfg.report(code, "target refused before connection");
                        return write_status(&mut client, "403 Forbidden").await;
                    }
                    Err(_) => {
                        cfg.report("proxy:dns", "resolution failed or returned no addresses");
                        return write_status(&mut client, "502 Bad Gateway").await;
                    }
                }
            };
            let mut upstream = match TcpStream::connect(addr).await {
                Ok(s) => s,
                Err(error) => {
                    cfg.report(
                        "proxy:connect",
                        &format!("{:?}; os_code={:?}", error.kind(), error.raw_os_error()),
                    );
                    return write_status(&mut client, "502 Bad Gateway").await;
                }
            };
            // Rewrite the absolute-form request line to origin-form and forward.
            // NOTE: this handles one request per proxy connection (sufficient for
            // the WebView, which opens a fresh connection per origin); pipelined
            // requests for a *different* host on the same connection are not
            // re-validated — acceptable because the connection is pinned to the
            // already-validated upstream IP.
            let rewritten = rewrite_http_head(&head, &host, port);
            upstream.write_all(&rewritten).await?;
            if !leftover.is_empty() {
                upstream.write_all(&leftover).await?;
            }
            let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
            Ok(())
        }
    }
}

/// Rewrites an absolute-form request head (`GET http://host/path HTTP/1.1`) to
/// origin-form (`GET /path HTTP/1.1`) so the upstream origin server accepts it.
fn rewrite_http_head(head: &[u8], host: &str, port: u16) -> Vec<u8> {
    let text = String::from_utf8_lossy(head);
    let mut lines = text.split("\r\n");
    let first = lines.next().unwrap_or("");
    let mut parts = first.splitn(3, ' ');
    let method = parts.next().unwrap_or("GET");
    let target = parts.next().unwrap_or("/");
    let version = parts.next().unwrap_or("HTTP/1.1");

    // Derive the origin-form path from the absolute URL.
    let path = target
        .strip_prefix("http://")
        .and_then(|rest| rest.find('/').map(|i| rest[i..].to_string()))
        .unwrap_or_else(|| "/".to_string());

    let authority = if port == 80 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };

    let mut out = format!("{method} {path} {version}\r\n");
    let mut has_host = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if line.to_ascii_lowercase().starts_with("host:") {
            has_host = true;
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    if !has_host {
        out.push_str(&format!("Host: {authority}\r\n"));
    }
    out.push_str("\r\n");
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::net::Ipv6Addr;

    // ── is_blocked: address policy ──────────────────────────────────────────────

    fn v4(s: &str) -> IpAddr {
        IpAddr::V4(s.parse::<Ipv4Addr>().unwrap())
    }
    fn v6(s: &str) -> IpAddr {
        IpAddr::V6(s.parse::<Ipv6Addr>().unwrap())
    }

    #[test]
    fn blocks_private_and_local_ipv4() {
        for s in [
            "127.0.0.1",       // loopback
            "10.0.0.5",        // RFC1918
            "172.16.3.4",      // RFC1918
            "192.168.1.1",     // RFC1918
            "169.254.169.254", // cloud metadata (link-local)
            "0.0.0.0",         // unspecified
            "255.255.255.255", // broadcast
            "100.64.1.1",      // CGNAT
            "192.0.0.1",       // IETF protocol assignments
        ] {
            assert!(is_blocked(v4(s)), "{s} should be blocked");
        }
    }

    #[test]
    fn allows_public_ipv4() {
        for s in ["8.8.8.8", "1.1.1.1", "93.184.216.34"] {
            assert!(!is_blocked(v4(s)), "{s} should be allowed");
        }
    }

    #[test]
    fn blocks_private_and_local_ipv6() {
        for s in [
            "::1",                    // loopback
            "::",                     // unspecified
            "fc00::1",                // ULA
            "fd00:ec2::254",          // AWS IMDS over IPv6 (within fc00::/7)
            "fe80::1",                // link-local
            "::ffff:127.0.0.1",       // IPv4-mapped loopback
            "::ffff:169.254.169.254", // IPv4-mapped metadata
        ] {
            assert!(is_blocked(v6(s)), "{s} should be blocked");
        }
    }

    #[test]
    fn allows_public_ipv6() {
        for s in ["2606:4700:4700::1111", "2001:4860:4860::8888"] {
            assert!(!is_blocked(v6(s)), "{s} should be allowed");
        }
    }

    #[test]
    fn blocks_ipv6_embedding_private_ipv4() {
        // NAT64 / 6to4 / IPv4-compatible wrappers around a private or loopback
        // IPv4 must be judged by the embedded address, not allowed as "public".
        for s in [
            "64:ff9b::7f00:1",    // NAT64    → 127.0.0.1
            "64:ff9b::a9fe:a9fe", // NAT64    → 169.254.169.254 (metadata)
            "64:ff9b::a00:1",     // NAT64    → 10.0.0.1
            "2002:7f00:1::",      // 6to4     → 127.0.0.1
            "2002:a00:1::",       // 6to4     → 10.0.0.1
            "::7f00:1",           // ::a.b.c.d → 127.0.0.1
            "::a00:1",            // ::a.b.c.d → 10.0.0.1
        ] {
            assert!(is_blocked(v6(s)), "{s} should be blocked");
        }
    }

    #[test]
    fn allows_ipv6_embedding_public_ipv4() {
        // The same wrappers around a genuinely public IPv4 stay allowed.
        for s in ["64:ff9b::808:808", "2002:808:808::"] {
            assert!(!is_blocked(v6(s)), "{s} should be allowed");
        }
    }

    // ── validate_target: resolution + rebinding ─────────────────────────────────

    struct FakeResolver(HashMap<String, Vec<SocketAddr>>);
    impl HostResolver for FakeResolver {
        fn resolve(&self, host: &str, _port: u16) -> std::io::Result<Vec<SocketAddr>> {
            Ok(self.0.get(host).cloned().unwrap_or_default())
        }
    }
    fn sa(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    #[test]
    fn rejects_local_names_without_resolving() {
        struct MustNotResolve;
        impl HostResolver for MustNotResolve {
            fn resolve(&self, host: &str, _: u16) -> std::io::Result<Vec<SocketAddr>> {
                panic!("forbidden DNS lookup: {host}");
            }
        }
        for host in [
            "printer.local",
            "PRINTER.LoCaL",
            "printer.local.",
            "a.b.local",
            "local",
            "LOCAL.",
            "printer",
            "printer.",
        ] {
            assert_eq!(
                validate_target(host, 443, &MustNotResolve, is_blocked),
                Err(ProxyError::Blocked),
                "{host}"
            );
        }
    }

    #[test]
    fn local_substring_does_not_block_public_domains() {
        for host in ["local.example.com", "notlocal.example", "example.local.com"] {
            let r = FakeResolver(HashMap::from([(
                host.into(),
                vec![sa("93.184.216.34:443")],
            )]));
            assert_eq!(
                validate_target(host, 443, &r, is_blocked),
                Ok(sa("93.184.216.34:443"))
            );
        }
    }

    #[test]
    fn rejects_host_that_resolves_to_loopback() {
        // The anti-rebinding case: hostname looks fine, resolves to 127.0.0.1.
        let r = FakeResolver(HashMap::from([(
            "rebind.test".into(),
            vec![sa("127.0.0.1:443")],
        )]));
        assert_eq!(
            validate_target("rebind.test", 443, &r, is_blocked),
            Err(ProxyError::Blocked)
        );
    }

    #[test]
    fn rejects_host_with_any_private_record() {
        // Resolves to both a public and a private address → refused entirely.
        let r = FakeResolver(HashMap::from([(
            "mixed.test".into(),
            vec![sa("8.8.8.8:443"), sa("10.0.0.1:443")],
        )]));
        assert_eq!(
            validate_target("mixed.test", 443, &r, is_blocked),
            Err(ProxyError::Blocked)
        );
    }

    #[test]
    fn allows_public_host_and_pins_address() {
        let r = FakeResolver(HashMap::from([(
            "good.test".into(),
            vec![sa("93.184.216.34:443")],
        )]));
        assert_eq!(
            validate_target("good.test", 443, &r, is_blocked),
            Ok(sa("93.184.216.34:443"))
        );
    }

    #[test]
    fn errors_when_no_addresses() {
        let r = FakeResolver(HashMap::new());
        assert_eq!(
            validate_target("nx.test", 443, &r, is_blocked),
            Err(ProxyError::Dns)
        );
    }

    // ── request parsing ─────────────────────────────────────────────────────────

    #[test]
    fn parses_connect_with_ipv6_authority() {
        match parse_request_line("CONNECT [::1]:8443 HTTP/1.1").unwrap() {
            Target::Connect { host, port } => {
                assert_eq!(host, "::1");
                assert_eq!(port, 8443);
            }
            _ => panic!("expected CONNECT"),
        }
    }

    #[test]
    fn parses_connect_default_port_and_http_absolute() {
        match parse_request_line("CONNECT example.com:443 HTTP/1.1").unwrap() {
            Target::Connect { host, port } => {
                assert_eq!(host, "example.com");
                assert_eq!(port, 443);
            }
            _ => panic!("expected CONNECT"),
        }
        match parse_request_line("GET http://example.com/feed HTTP/1.1").unwrap() {
            Target::Http { host, port } => {
                assert_eq!(host, "example.com");
                assert_eq!(port, 80);
            }
            _ => panic!("expected HTTP"),
        }
    }

    #[tokio::test]
    async fn scoped_proxy_diagnostics_are_isolated_and_follow_policy() {
        let (a_tx, mut a_rx) = tokio::sync::mpsc::unbounded_channel();
        let (b_tx, mut b_rx) = tokio::sync::mpsc::unbounded_channel();
        let config = ProxyConfig::new(
            Arc::new(std::sync::RwLock::new(false)),
            Arc::new(std::sync::RwLock::new(false)),
        );
        let a = start_scoped(config.clone().with_observer(Arc::new(move |code, _| {
            let _ = a_tx.send(code.to_owned());
        })))
        .await
        .unwrap();
        let b = start_scoped(config.with_observer(Arc::new(move |code, _| {
            let _ = b_tx.send(code.to_owned());
        })))
        .await
        .unwrap();
        for (port, request, expected_status) in [
            (a.port, "CONNECT localhost:443 HTTP/1.1\r\n\r\n", "403"),
            (b.port, "CONNECT example.com:80 HTTP/1.1\r\n\r\n", "403"),
            (a.port, "GET http://example.com/ HTTP/1.1\r\n\r\n", "470"),
            (b.port, "CONNECT 127.0.0.1:443 HTTP/1.1\r\n\r\n", "403"),
        ] {
            let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
                .await
                .unwrap();
            client.write_all(request.as_bytes()).await.unwrap();
            assert!(read_http_head(&mut client).await.contains(expected_status));
        }
        assert_eq!(a_rx.recv().await.unwrap(), "proxy:hostname-blocked");
        assert_eq!(a_rx.recv().await.unwrap(), "proxy:http-blocked");
        assert_eq!(b_rx.recv().await.unwrap(), "proxy:port-blocked");
        assert_eq!(b_rx.recv().await.unwrap(), "proxy:address-blocked");
        assert!(a_rx.try_recv().is_err());
        assert!(b_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dropping_scoped_proxy_closes_active_tunnels_and_listener() {
        let server = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let proxy = start_scoped(permissive()).await.unwrap();
        let port = proxy.port;
        let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        client
            .write_all(
                format!(
                    "CONNECT 127.0.0.1:{} HTTP/1.1\r\n\r\n",
                    server.local_addr().unwrap().port()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let (_upstream, _) = server.accept().await.unwrap();
        assert!(read_http_head(&mut client).await.contains("200"));
        drop(proxy);
        let mut byte = [0];
        let n = tokio::time::timeout(std::time::Duration::from_secs(2), client.read(&mut byte))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(n, 0);
        assert!(TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn deny_all_refuses_http_and_connect_without_dns_or_upstream() {
        struct MustNotResolve;
        impl HostResolver for MustNotResolve {
            fn resolve(&self, host: &str, _: u16) -> std::io::Result<Vec<SocketAddr>> {
                panic!("deny-all attempted DNS for {host}");
            }
        }
        let upstream = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = upstream.local_addr().unwrap();
        let reports = Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = reports.clone();
        let mut config = ProxyConfig::deny_all().with_observer(Arc::new(move |code, _| {
            observed.lock().unwrap().push(code.to_owned())
        }));
        config.resolver = Arc::new(MustNotResolve);
        let proxy = start_scoped(config).await.unwrap();
        for target in ["blocked.example:443".to_owned(), address.to_string()] {
            for request in [
                format!("CONNECT {target} HTTP/1.1\r\n\r\n"),
                format!("GET http://{target}/image HTTP/1.1\r\nHost: {target}\r\n\r\n"),
            ] {
                let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, proxy.port))
                    .await
                    .unwrap();
                client.write_all(request.as_bytes()).await.unwrap();
                assert!(read_http_head(&mut client).await.contains("403"));
            }
        }
        assert_eq!(*reports.lock().unwrap(), vec!["proxy:blocked"; 4]);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), upstream.accept())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn deny_all_local_exception_is_exact_and_loopback_only() {
        assert!(ProxyConfig::deny_all()
            .with_local_endpoint("example.org".into(), "127.0.0.1:1420".parse().unwrap())
            .is_err());
        assert!(ProxyConfig::deny_all()
            .with_local_endpoint("localhost".into(), "192.0.2.1:1420".parse().unwrap())
            .is_err());
        let server = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = server.local_addr().unwrap();
        let proxy = start_scoped(
            ProxyConfig::deny_all()
                .with_local_endpoint("localhost".into(), address)
                .unwrap(),
        )
        .await
        .unwrap();
        let served = tokio::spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();
            assert!(read_http_head(&mut stream).await.starts_with("GET /local "));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        });
        for (host, port, status) in [
            ("localhost", address.port(), "200"),
            ("127.0.0.1", address.port(), "403"),
            ("localhost", 1, "403"),
            ("example.org", address.port(), "403"),
        ] {
            let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, proxy.port))
                .await
                .unwrap();
            client
                .write_all(
                    format!(
                        "GET http://{host}:{port}/local HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            assert!(read_http_head(&mut client).await.contains(status));
        }
        served.await.unwrap();
    }

    // ── end-to-end tunnel ───────────────────────────────────────────────────────

    fn permissive() -> ProxyConfig {
        // Real resolver, but nothing blocked — lets us tunnel to a loopback test
        // server (which the real policy would correctly refuse).
        ProxyConfig {
            deny_all: false,
            local_endpoint: None,
            observer: None,
            resolver: Arc::new(SystemResolver),
            is_blocked_fn: |_| false,
            http_allowed: Arc::new(std::sync::RwLock::new(true)),
            connect_nonstandard_ports_allowed: Arc::new(std::sync::RwLock::new(true)),
        }
    }

    #[tokio::test]
    async fn connect_tunnels_to_allowed_upstream() {
        // Echo server on loopback.
        let echo = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let echo_port = echo.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = echo.accept().await.unwrap();
            let mut b = [0u8; 5];
            s.read_exact(&mut b).await.unwrap();
            s.write_all(&b).await.unwrap();
        });

        let port = start(permissive()).await.unwrap();
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        c.write_all(format!("CONNECT 127.0.0.1:{echo_port} HTTP/1.1\r\n\r\n").as_bytes())
            .await
            .unwrap();

        // Consume the full CONNECT response head before tunneling, otherwise its
        // trailing bytes would be mistaken for tunnel data.
        let status = read_http_head(&mut c).await;
        assert!(status.starts_with("HTTP/1.1 200"), "got: {status:?}");

        c.write_all(b"hello").await.unwrap();
        let mut echoed = [0u8; 5];
        c.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"hello");
    }

    /// Reads a response up to and including the blank `\r\n\r\n`.
    async fn read_http_head(stream: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut one = [0u8; 1];
        while stream.read_exact(&mut one).await.is_ok() {
            buf.push(one[0]);
            if buf.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[tokio::test]
    async fn connect_to_blocked_target_is_refused() {
        // Real policy: a CONNECT to loopback must be rejected with 403.
        // Port 443 keeps this test isolated to the address check, not the
        // separate non-443-port check.
        let port = start(ProxyConfig::new(
            Arc::new(std::sync::RwLock::new(false)),
            Arc::new(std::sync::RwLock::new(false)),
        ))
        .await
        .unwrap();
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        c.write_all(b"CONNECT 127.0.0.1:443 HTTP/1.1\r\n\r\n")
            .await
            .unwrap();

        let status = read_http_head(&mut c).await;
        assert!(status.starts_with("HTTP/1.1 403"), "got: {status:?}");
    }

    #[tokio::test]
    async fn connect_to_nonstandard_port_refused_by_default() {
        // connect_nonstandard_ports_allowed=false: CONNECT to a non-443 port
        // is refused before address validation runs.
        let port = start(ProxyConfig::new(
            Arc::new(std::sync::RwLock::new(false)),
            Arc::new(std::sync::RwLock::new(false)),
        ))
        .await
        .unwrap();
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        c.write_all(b"CONNECT example.com:8443 HTTP/1.1\r\n\r\n")
            .await
            .unwrap();

        let status = read_http_head(&mut c).await;
        assert!(status.starts_with("HTTP/1.1 403"), "got: {status:?}");
    }

    #[tokio::test]
    async fn plain_http_refused_when_http_not_allowed() {
        // http_allowed=false: absolute-form HTTP is answered with the
        // HTTP_BLOCKED_STATUS signature before any DNS/dial happens.
        let cfg = ProxyConfig {
            deny_all: false,
            local_endpoint: None,
            observer: None,
            resolver: Arc::new(SystemResolver),
            is_blocked_fn: |_| false,
            http_allowed: Arc::new(std::sync::RwLock::new(false)),
            connect_nonstandard_ports_allowed: Arc::new(std::sync::RwLock::new(false)),
        };
        let port = start(cfg).await.unwrap();
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        c.write_all(b"GET http://example.com/feed HTTP/1.1\r\nHost: example.com\r\n\r\n")
            .await
            .unwrap();

        let status = read_http_head(&mut c).await;
        let expected = format!(
            "HTTP/1.1 {}",
            crate::persistence::feeds::HTTP_BLOCKED_STATUS
        );
        assert!(status.starts_with(&expected), "got: {status:?}");
    }

    #[tokio::test]
    async fn plain_http_forwarded_when_http_allowed() {
        // http_allowed=true: absolute-form HTTP reaches the upstream server.
        let upstream = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = upstream.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await.unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .await
                .unwrap();
        });

        let port = start(permissive()).await.unwrap();
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        c.write_all(
            format!("GET http://127.0.0.1:{upstream_port}/x HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();

        let status = read_http_head(&mut c).await;
        assert!(status.starts_with("HTTP/1.1 200"), "got: {status:?}");
    }
}
