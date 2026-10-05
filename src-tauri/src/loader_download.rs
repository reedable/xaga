//! Adapts native loader downloads to the shared staging/validation boundary.
//! The native pending capture phase selects the resource consumer. Future
//! consumers register their own validator and completion path here.
use crate::{
    article_diagnostics::{ArticleDiagnosticOutcome, ArticleLoadAttempt},
    article_loader::PendingRequests,
    resource_download::{ICON_MAX_BYTES, IconValidator, StagedDownload},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

struct ActiveDownload {
    url: String,
    staging: StagedDownload<IconValidator>,
}
#[derive(Debug)]
pub(crate) enum DownloadRejection {
    NoConsumer,
    AlreadyClaimed,
    StagingFailed,
}
#[derive(Clone)]
pub(crate) struct LoaderDownloads {
    active: Arc<Mutex<Option<ActiveDownload>>>,
    claimed: Arc<AtomicBool>,
    pending: PendingRequests,
    id: String,
    attempt: ArticleLoadAttempt,
}
impl LoaderDownloads {
    pub(crate) fn new(pending: PendingRequests, id: String, attempt: ArticleLoadAttempt) -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
            claimed: Arc::new(AtomicBool::new(false)),
            pending,
            id,
            attempt,
        }
    }
    pub(crate) fn request(&self, url: &str) -> Result<std::path::PathBuf, DownloadRejection> {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if active.is_some() {
            return Err(DownloadRejection::AlreadyClaimed);
        }
        let pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        // A native pending phase chooses the consumer, not the callback URL.
        // WKDownload may report the final URL after redirects. Content remains
        // untrusted: this phase can produce only a validated image, never an
        // arbitrary file or a PDF. The requested URL remains storage identity.
        if !pending
            .get(&self.id)
            .is_some_and(|request| request.partial.is_some())
        {
            return Err(DownloadRejection::NoConsumer);
        }
        drop(pending);
        if self.claimed.swap(true, Ordering::SeqCst) {
            return Err(DownloadRejection::AlreadyClaimed);
        }
        let staging = StagedDownload::new(IconValidator, ICON_MAX_BYTES)
            .map_err(|_| DownloadRejection::StagingFailed)?;
        let path = staging.path().to_owned();
        *active = Some(ActiveDownload {
            url: url.to_owned(),
            staging,
        });
        drop(active);
        let monitor = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let oversized = {
                    let active = monitor.active.lock().unwrap_or_else(|e| e.into_inner());
                    let Some(job) = active.as_ref() else {
                        break;
                    };
                    std::fs::metadata(job.staging.path()).is_ok_and(|m| m.len() > ICON_MAX_BYTES)
                };
                if oversized {
                    monitor.complete(Err("download exceeds favicon byte limit".into()));
                    monitor.cancel();
                    break;
                }
                if !monitor
                    .pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .contains_key(&monitor.id)
                {
                    monitor.cancel();
                    break;
                }
            }
        });
        Ok(path)
    }
    pub(crate) fn finish(&self, url: &str, success: bool) {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if !active.as_ref().is_some_and(|a| same_resource(&a.url, url)) {
            return;
        }
        let job = active.take().unwrap();
        drop(active);
        let this = self.clone();
        // Image decoding and disk reads must not block the native UI thread.
        tauri::async_runtime::spawn_blocking(move || {
            this.attempt.add_started_event("download_validation");
            let result = job.staging.finish(success);
            if let Err(reason) = &result {
                this.attempt.add_event(
                    "native",
                    "download_validation",
                    ArticleDiagnosticOutcome::Warning,
                    "article:resource-rejected",
                    reason,
                );
            }
            this.complete(result);
        });
    }
    pub(crate) fn rejected(&self, reason: DownloadRejection) {
        if matches!(reason, DownloadRejection::StagingFailed) {
            self.complete(Err("cannot create download staging directory".into()));
        }
    }
    fn complete(&self, result: Result<String, String>) {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if !pending.get(&self.id).is_some_and(|r| r.partial.is_some()) {
            return;
        }
        let mut request = pending.remove(&self.id).unwrap();
        drop(pending);
        let mut capture = request.partial.take().unwrap();
        match result {
            Ok(data) => {
                capture.favicon_data = Some(data);
                self.attempt.add_succeeded_event("download_validation");
                self.attempt.add_succeeded_event("favicon");
            }
            Err(reason) => {
                self.attempt.add_event(
                    "native",
                    "favicon",
                    ArticleDiagnosticOutcome::Warning,
                    "article:favicon-unavailable",
                    &reason,
                );
            }
        }
        // The normal load receiver closes the hidden loader and passes these
        // bytes to existing URL-hash image persistence under the library writer.
        let _ = request.tx.send(capture);
    }
    pub(crate) fn cancel(&self) {
        self.active.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
}
fn same_resource(left: &str, right: &str) -> bool {
    let parse = |value: &str| {
        tauri::Url::parse(value).ok().map(|mut u| {
            u.set_fragment(None);
            u
        })
    };
    matches!((parse(left), parse(right)), (Some(a), Some(b)) if a == b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        article_diagnostics::ArticleDiagnostics,
        article_loader::{LoaderCapture, PendingRequest},
    };
    fn fixture() -> (
        LoaderDownloads,
        tokio::sync::oneshot::Receiver<LoaderCapture>,
    ) {
        let attempt = ArticleDiagnostics::default().create_attempt("load");
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pending = Arc::new(Mutex::new(std::collections::HashMap::new()));
        pending.lock().unwrap().insert(
            "1".into(),
            PendingRequest {
                tx,
                keep_open: false,
                attempt: attempt.clone(),
                diagnostic_event_count: 0,
                reqwest_error: false,
                partial: Some(LoaderCapture {
                    html: "original article".into(),
                    mime_type: "text/html".into(),
                    favicon: Some("https://example.org/favicon.ico".into()),
                    favicon_data: None,
                }),
            },
        );
        (LoaderDownloads::new(pending, "1".into(), attempt), rx)
    }
    #[tokio::test]
    async fn download_completion_validates_and_resolves_without_waiting_for_loader_deadline() {
        for (valid, success) in [(true, true), (false, true), (true, false)] {
            let (downloads, rx) = fixture();
            let url = "https://example.org/favicon.ico";
            let path = downloads.request(url).unwrap();
            let bytes = if valid {
                let mut output = std::io::Cursor::new(Vec::new());
                image::DynamicImage::new_rgba8(16, 16)
                    .write_to(&mut output, image::ImageFormat::Ico)
                    .unwrap();
                output.into_inner()
            } else {
                b"not an image".to_vec()
            };
            std::fs::write(&path, bytes).unwrap();
            let rejection = downloads.request("https://example.org/second").unwrap_err();
            downloads.rejected(rejection);
            downloads.finish(url, success);
            let capture = tokio::time::timeout(std::time::Duration::from_secs(2), rx)
                .await
                .expect("must not wait for article deadline")
                .unwrap();
            assert_eq!(capture.html, "original article");
            assert_eq!(capture.favicon_data.is_some(), valid && success);
            assert!(!path.parent().unwrap().exists());
        }
    }

    #[test]
    fn native_phase_owns_redirected_downloads_and_rejects_without_a_consumer() {
        let (downloads, mut rx) = fixture();
        let path = downloads
            .request("https://cdn.example.org/icon-bytes")
            .unwrap();
        assert_eq!(path.file_name().unwrap(), "resource");
        assert!(
            downloads
                .request("https://cdn.example.org/icon-bytes")
                .is_err()
        );
        downloads.cancel();
        assert!(!path.parent().unwrap().exists());
        assert!(matches!(
            rx.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        let (downloads, _) = fixture();
        downloads
            .pending
            .lock()
            .unwrap()
            .get_mut("1")
            .unwrap()
            .partial = None;
        assert!(downloads.request("https://other.org/file").is_err());
    }
    #[test]
    fn terminal_failure_delivers_article_once_without_deadline() {
        let (downloads, mut rx) = fixture();
        downloads.complete(Err("invalid image".into()));
        let capture = rx.try_recv().unwrap();
        assert_eq!(capture.html, "original article");
        assert!(capture.favicon_data.is_none());
        downloads.complete(Ok("late image".into()));
        assert!(downloads.pending.lock().unwrap().is_empty());
    }
    #[test]
    fn accepted_output_joins_existing_capture_and_late_output_is_discarded() {
        let (downloads, mut rx) = fixture();
        downloads.complete(Ok("data:image/png;base64,validated".into()));
        assert_eq!(
            rx.try_recv().unwrap().favicon_data.as_deref(),
            Some("data:image/png;base64,validated")
        );
        let (downloads, mut rx) = fixture();
        downloads.pending.lock().unwrap().remove("1");
        downloads.complete(Ok("late".into()));
        assert!(rx.try_recv().is_err());
    }
}
