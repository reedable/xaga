//! Controlled local responses shared by native resource integration tests.
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Default)]
pub(super) struct Gate {
    open: AtomicBool,
    notify: tokio::sync::Notify,
}
impl Gate {
    pub(super) async fn wait(&self) {
        loop {
            let notified = self.notify.notified();
            if self.open.load(Ordering::SeqCst) {
                return;
            }
            notified.await;
        }
    }
    pub(super) fn release(&self) {
        self.open.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
}

pub(super) struct Response {
    body: Vec<u8>,
    mime: &'static str,
    before_headers: Option<Arc<Gate>>,
    after_first_byte: Option<Arc<Gate>>,
    pub(super) seen: AtomicBool,
    pub(super) requests: std::sync::atomic::AtomicUsize,
}
pub(super) struct Server {
    pub(super) address: std::net::SocketAddr,
    pub(super) responses: Arc<Mutex<HashMap<String, Arc<Response>>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    pub(super) async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let responses = Arc::new(Mutex::new(HashMap::<String, Arc<Response>>::new()));
        let routes = responses.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (mut stream, _) = accepted.unwrap();
                        let routes = routes.clone();
                        connections.spawn(async move {
                            let mut head = Vec::new();
                            while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
                                let Ok(byte) = stream.read_u8().await else { return; };
                                head.push(byte);
                            }
                            let head = String::from_utf8_lossy(&head);
                            let path = head.split_whitespace().nth(1).unwrap_or("");
                            eprintln!("fixture request: {path}");
                            let response = routes.lock().unwrap().get(path).cloned();
                            let Some(response) = response else {
                                let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                                return;
                            };
                            response.seen.store(true, Ordering::SeqCst);
                            response.requests.fetch_add(1, Ordering::SeqCst);
                            if let Some(gate) = &response.before_headers { gate.wait().await; }
                            let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", response.mime, response.body.len());
                            if stream.write_all(headers.as_bytes()).await.is_err() { return; }
                            if let Some(gate) = &response.after_first_byte {
                                if stream.write_all(&response.body[..1]).await.is_err() { return; }
                                gate.wait().await;
                                let _ = stream.write_all(&response.body[1..]).await;
                            } else {
                                let _ = stream.write_all(&response.body).await;
                            }
                        });
                    }
                    _ = connections.join_next(), if !connections.is_empty() => {}
                }
            }
        });
        Self {
            address,
            responses,
            task,
        }
    }
    pub(super) fn add_response(
        &self,
        path: &str,
        body: Vec<u8>,
        mime: &'static str,
        before_headers: Option<Arc<Gate>>,
        after_first_byte: Option<Arc<Gate>>,
    ) -> Arc<Response> {
        let response = Arc::new(Response {
            body,
            mime,
            before_headers,
            after_first_byte,
            seen: AtomicBool::new(false),
            requests: std::sync::atomic::AtomicUsize::new(0),
        });
        self.responses
            .lock()
            .unwrap()
            .insert(path.into(), response.clone());
        response
    }
    pub(super) fn add_article(
        &self,
        name: &str,
        article_gate: Option<Arc<Gate>>,
        icon_gate: Option<Arc<Gate>>,
        transfer_gate: Option<Arc<Gate>>,
        valid: bool,
    ) -> Arc<Response> {
        let html = format!(
            "<!doctype html><title>{name}</title><link rel=\"icon\" href=\"/{name}/icon\"><p>article-{name}</p>"
        );
        self.add_response(
            &format!("/{name}"),
            html.into_bytes(),
            "text/html",
            article_gate,
            None,
        );
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(8, 8)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        self.add_response(
            &format!("/{name}/icon"),
            if valid {
                png.into_inner()
            } else {
                b"not an image".to_vec()
            },
            "application/octet-stream",
            icon_gate,
            transfer_gate,
        )
    }
}

pub(super) fn list_staging_directories() -> Vec<std::path::PathBuf> {
    let mut paths: Vec<_> = std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("xaga-resource-")
        })
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    paths
}
