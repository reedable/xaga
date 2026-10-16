//! Controlled remote-origin probes using production loader configuration/script.
//! The fixture proxy serves only publisher.invalid; all other requests pass to
//! the production SSRF proxy. No public destination is contacted.
#[cfg(target_os = "macos")]
#[path = "../src/webview_policy.rs"]
#[allow(dead_code)]
mod webview_policy;
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, Mutex, RwLock},
        time::Duration,
    };
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    use xaga_core::network_proxy::{self, ProxyConfig};
    fn head(s: &mut TcpStream) -> Vec<u8> {
        s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut h = Vec::new();
        let mut b = [0];
        while h.len() < 16384 && !h.ends_with(b"\r\n\r\n") {
            if s.read(&mut b).unwrap_or(0) == 0 {
                break;
            }
            h.push(b[0]);
        }
        h
    }
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = destination.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::<String>::new()));
    let dest_hits = hits.clone();
    std::thread::spawn(move || {
        for stream in destination.incoming() {
            let hits = dest_hits.clone();
            std::thread::spawn(move || {
                let mut s = stream.unwrap();
                let h = head(&mut s);
                let line = if h.first() == Some(&22) {
                    "TLS ClientHello (connection only; no TLS server)".to_owned()
                } else {
                    String::from_utf8_lossy(&h)
                        .lines()
                        .next()
                        .unwrap_or("empty")
                        .to_owned()
                };
                eprintln!("DESTINATION {line}");
                hits.lock().unwrap().push(line);
                let body = b"local-probe-response";
                let _=write!(s,"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Private-Network: true\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                let _ = s.write_all(body);
            });
        }
    });
    let fixture = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture_addr = fixture.local_addr().unwrap();
    let routes = Arc::new(Mutex::new(Vec::new()));
    let proxy_events = Arc::new(Mutex::new(Vec::new()));
    let events = proxy_events.clone();
    let proxy = tauri::async_runtime::block_on(network_proxy::start_scoped(
        ProxyConfig::new(Arc::new(RwLock::new(true)), Arc::new(RwLock::new(true))).with_observer(
            Arc::new(move |code, detail| {
                eprintln!("POLICY {code} {detail}");
                events.lock().unwrap().push(format!("{code} {detail}"));
            }),
        ),
    ))
    .unwrap();
    let policy_port = proxy.port;
    // Positive control: the production proxy must refuse the same local endpoint.
    let mut control = TcpStream::connect(("127.0.0.1", policy_port)).unwrap();
    write!(
        control,
        "GET http://127.0.0.1:{port}/policy-control HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
    )
    .unwrap();
    let response = head(&mut control);
    assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 403"));
    let server_routes = routes.clone();
    std::thread::spawn(move || {
        for stream in fixture.incoming() {
            let routes = server_routes.clone();
            std::thread::spawn(move || {
                let mut s = stream.unwrap();
                let mut h = head(&mut s);
                let line = String::from_utf8_lossy(&h)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                eprintln!("ROUTE {line}");
                routes.lock().unwrap().push(line.clone());
                if line.starts_with("CONNECT publisher.invalid:80 ") {
                    s.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                        .unwrap();
                    h = head(&mut s);
                } else if !line.starts_with("GET http://publisher.invalid/") {
                    let mut upstream = TcpStream::connect(("127.0.0.1", policy_port)).unwrap();
                    upstream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    upstream.write_all(&h).unwrap();
                    let _ = std::io::copy(&mut upstream, &mut s);
                    return;
                }
                let request = String::from_utf8_lossy(&h);
                let svg = request.contains("/svg");
                let mode = if svg { "svg" } else { "html" };
                let js = format!(
                    r#"
                window.probeResults=[];
                function record(x) {{ probeResults.push(x); }}
                for (const host of ['127.0.0.1','localhost']) {{
                    const base='http://'+host+':{port}/{mode}/'+host+'/';
                    fetch(base+'fetch').then(r=>r.text()).then(t=>record('fetch-readable:'+host+':'+t)).catch(e=>record('fetch-error:'+host));
                    fetch(base+'fetch-no-cors',{{mode:'no-cors'}}).then(()=>record('fetch-no-cors:'+host)).catch(()=>{{}});
                    const xhr=new XMLHttpRequest(); xhr.open('GET',base+'xhr'); xhr.onload=()=>record('xhr-readable:'+host); xhr.send();
                    const img=new Image(); img.src=base+'img'; document.documentElement.appendChild(img);
                    const script=document.createElementNS('http://www.w3.org/1999/xhtml','script'); script.src=base+'script'; document.documentElement.appendChild(script);
                    const frame=document.createElementNS('http://www.w3.org/1999/xhtml','iframe'); frame.src=base+'frame'; document.documentElement.appendChild(frame);
                    const style=document.createElementNS('http://www.w3.org/1999/xhtml','style'); style.textContent='@import url('+base+'style);'; document.documentElement.appendChild(style);
                    navigator.sendBeacon(base+'beacon','probe');
                    try {{ new WebSocket('ws://'+host+':{port}/{mode}/'+host+'/websocket'); }} catch(e) {{record('ws-error')}}
                    fetch('https://'+host+':{port}/{mode}/'+host+'/https').catch(()=>record('https-error:'+host));
                }}
                setTimeout(()=>{{document.title='PROBE:'+JSON.stringify(probeResults)}},4000);
            "#
                );
                let body = if svg {
                    format!(
                        r#"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="30"><script><![CDATA[{js}]]></script><image href="http://127.0.0.1:{port}/svg/declarative-image" width="20" height="20"/></svg>"#
                    )
                } else {
                    format!("<!doctype html><script>{js}</script><p>Publisher fixture</p>")
                };
                let mime = if svg { "image/svg+xml" } else { "text/html" };
                write!(s,"HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            });
        }
    });
    let reports = Arc::new(Mutex::new(Vec::new()));
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app=tauri::Builder::default().invoke_handler(|invoke| {invoke.resolver.resolve(()); true}).setup(move |app| {
        for (label,image_only) in [("html",false),("svg",true)] {
            let reports=reports.clone();
            WebviewWindowBuilder::new(app,label,WebviewUrl::External(format!("http://publisher.invalid/{label}").parse()?))
                .visible(false)
                .initialization_script(format!("window.__LOADER_ID__='probe';window.__LOADER_KEEP_OPEN__=true;window.__LOADER_IMAGE_ONLY__={image_only};window.__LOAD_REMOTE_IMAGE__=true;\n{}",include_str!("../src/article_loader.js")))
                .with_webview_configuration(webview_policy::loader_configuration(None).unwrap())
                .proxy_url(format!("http://{fixture_addr}").parse()?)
                .on_new_window(|_,_|tauri::webview::NewWindowResponse::Deny)
                .on_download(|_,_|false)
                .on_document_title_changed(move |_,title| { if title.starts_with("PROBE:") {eprintln!("REPORT {label} {title}"); reports.lock().unwrap().push(format!("{label} {title}"));} })
                .build()?;
        }
        let handle=app.handle().clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(8)).await;
            let destination = hits.lock().unwrap();
            let routed = routes.lock().unwrap();
            let complete = reports.lock().unwrap().len() == 2;
            let valid = complete && routed.iter().any(|r| r.contains("publisher.invalid"))
                && !destination.iter().any(|r| r.contains("policy-control"))
                && !proxy_events.lock().unwrap().is_empty();
            eprintln!("OBSERVED destination_connections={} policy_events={} fixture_complete={complete}", destination.len(), proxy_events.lock().unwrap().len());
            // Diagnostic success means the observations completed, NOT containment.
            drop(proxy);
            handle.exit(if valid {0} else {1});
        });
        Ok(())
    }).build(context).unwrap();
    app.run(|_, _| {});
}
