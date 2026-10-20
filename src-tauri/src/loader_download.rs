//! Adapts native loader downloads to the shared staging/validation boundary.
//! The native pending capture phase selects the resource consumer. Future
//! consumers register their own validator and completion path here.
use crate::{
    article_diagnostics::{ArticleDiagnosticOutcome, ArticleLoadAttempt},
    article_loader::PendingRequests,
    resource_download::{
        IMAGE_MAX_BYTES, ImageValidator, PdfValidator, StagedDownload, prepare_favicon,
    },
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

enum DownloadJob {
    Icon(StagedDownload<ImageValidator>),
    Image(StagedDownload<ImageValidator>),
    Pdf(StagedDownload<PdfValidator>),
}
enum Accepted {
    Icon(String),
    Image(String),
    Pdf(Vec<u8>),
}
impl DownloadJob {
    fn path(&self) -> &std::path::Path {
        match self {
            Self::Icon(s) => s.path(),
            Self::Image(s) => s.path(),
            Self::Pdf(s) => s.path(),
        }
    }
    fn limit(&self) -> u64 {
        match self {
            Self::Icon(_) => IMAGE_MAX_BYTES,
            Self::Image(_) => IMAGE_MAX_BYTES,
            Self::Pdf(_) => xaga_core::persistence::pdfs::MAX_PDF_BYTES,
        }
    }
    fn finish(self, success: bool) -> Result<Accepted, String> {
        match self {
            Self::Icon(s) => s
                .finish(success)
                .and_then(prepare_favicon)
                .map(Accepted::Icon),
            Self::Image(s) => s.finish(success).map(Accepted::Image),
            Self::Pdf(s) => s.finish(success).map(Accepted::Pdf),
        }
    }
}
struct ActiveDownload {
    url: String,
    staging: DownloadJob,
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
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let Some(request) = pending.get_mut(&self.id) else {
            return Err(DownloadRejection::NoConsumer);
        };
        if self.claimed.swap(true, Ordering::SeqCst) {
            return Err(DownloadRejection::AlreadyClaimed);
        }
        // Native phase chooses the validator, never the callback URL or server filename.
        let staging = if request.purpose == crate::article_loader::LoaderPurpose::Image {
            StagedDownload::new(ImageValidator, IMAGE_MAX_BYTES).map(DownloadJob::Image)
        } else if request.partial.is_some() {
            StagedDownload::new(ImageValidator, IMAGE_MAX_BYTES).map(DownloadJob::Icon)
        } else {
            StagedDownload::new(PdfValidator, xaga_core::persistence::pdfs::MAX_PDF_BYTES)
                .map(DownloadJob::Pdf)
        }
        .map_err(|_| DownloadRejection::StagingFailed)?;
        request.download_active = true;
        drop(pending);
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
                    std::fs::metadata(job.staging.path())
                        .is_ok_and(|m| m.len() > job.staging.limit())
                };
                if oversized {
                    monitor.complete(Err("download exceeds byte limit".into()));
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
    fn complete(&self, result: Result<Accepted, String>) {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let Some(mut request) = pending.remove(&self.id) else {
            return;
        };
        drop(pending);
        let is_image = request.purpose == crate::article_loader::LoaderPurpose::Image;
        let is_icon = !is_image && request.partial.is_some();
        let mut capture = request.partial.take().unwrap_or_default();
        match result {
            Ok(Accepted::Image(data)) if is_image => {
                capture.image_data = Some(data);
                self.attempt.add_succeeded_event("download_validation");
            }
            Ok(Accepted::Icon(data)) if is_icon => {
                capture.favicon_data = Some(data);
                self.attempt.add_succeeded_event("download_validation");
                self.attempt.add_succeeded_event("favicon");
            }
            Ok(Accepted::Pdf(bytes)) if !is_icon && !is_image => {
                capture.mime_type = "application/pdf".into();
                capture.pdf_bytes = Some(bytes);
                self.attempt.add_succeeded_event("download_validation");
            }
            Err(reason) if is_icon => {
                self.attempt.add_event(
                    "native",
                    "favicon",
                    ArticleDiagnosticOutcome::Warning,
                    "article:favicon-unavailable",
                    &reason,
                );
            }
            Err(reason) => {
                capture.download_error = Some(reason);
            }
            _ => {
                capture.download_error = Some("resource consumer changed during download".into());
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
                purpose: crate::article_loader::LoaderPurpose::Article,
                attempt: attempt.clone(),
                diagnostic_event_count: 0,
                download_active: false,
                partial: Some(LoaderCapture {
                    html: "original article".into(),
                    mime_type: "text/html".into(),
                    favicon: Some("https://example.org/favicon.ico".into()),
                    favicon_data: None,
                    ..Default::default()
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

    #[tokio::test]
    async fn favicon_pdf_response_stays_with_image_consumer() {
        let (downloads, rx) = fixture();
        let url = "https://example.org/favicon.ico";
        let path = downloads.request(url).unwrap();
        std::fs::write(&path, b"%PDF-test").unwrap();

        downloads.finish(url, true);
        let capture = tokio::time::timeout(std::time::Duration::from_secs(2), rx)
            .await
            .expect("favicon rejection must not wait for the article deadline")
            .unwrap();

        assert_eq!(capture.html, "original article");
        assert_eq!(capture.mime_type, "text/html");
        assert!(capture.favicon_data.is_none());
        assert!(capture.pdf_bytes.is_none());
        assert!(capture.download_error.is_none());
        assert!(!path.parent().unwrap().exists());

        let report = serde_json::to_value(downloads.attempt.get_diagnostic()).unwrap();
        assert!(report["events"].as_array().unwrap().iter().any(|event| {
            event["step"] == "download_validation"
                && event["outcome"] == "warning"
                && event["code"] == "article:resource-rejected"
        }));
        assert!(report["events"].as_array().unwrap().iter().any(|event| {
            event["step"] == "favicon"
                && event["outcome"] == "warning"
                && event["code"] == "article:favicon-unavailable"
        }));
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
        downloads.pending.lock().unwrap().clear();
        assert!(downloads.request("https://other.org/file").is_err());
    }
    #[tokio::test]
    async fn pdf_consumer_validates_bytes_and_finishes_without_html() {
        for valid in [true, false] {
            let (downloads, rx) = fixture();
            downloads
                .pending
                .lock()
                .unwrap()
                .get_mut("1")
                .unwrap()
                .partial = None;
            let url = "https://example.org/redirected-paper";
            let path = downloads.request(url).unwrap();
            std::fs::write(
                &path,
                if valid {
                    b"%PDF-test".as_slice()
                } else {
                    b"PK-zip"
                },
            )
            .unwrap();
            downloads.finish(url, true);
            let capture = tokio::time::timeout(std::time::Duration::from_secs(2), rx)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(capture.pdf_bytes.is_some(), valid);
            assert_eq!(capture.download_error.is_some(), !valid);
            assert!(capture.favicon_data.is_none());
            assert!(!path.parent().unwrap().exists());
        }
    }

    #[tokio::test]
    async fn direct_image_consumer_does_not_require_article_capture() {
        let (downloads, rx) = fixture();
        {
            let mut pending = downloads.pending.lock().unwrap();
            let request = pending.get_mut("1").unwrap();
            request.purpose = crate::article_loader::LoaderPurpose::Image;
            request.partial = None;
        }
        let url = "https://example.org/comic.png";
        let path = downloads.request(url).unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(900, 300)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(&path, bytes.into_inner()).unwrap();
        downloads.finish(url, true);
        let capture = tokio::time::timeout(std::time::Duration::from_secs(2), rx)
            .await
            .unwrap()
            .unwrap();
        assert!(capture.image_data.is_some());
        assert!(capture.pdf_bytes.is_none());
        assert!(capture.favicon_data.is_none());
        assert!(!path.exists());
    }

    #[test]
    fn terminal_failure_delivers_article_once_without_deadline() {
        let (downloads, mut rx) = fixture();
        downloads.complete(Err("invalid image".into()));
        let capture = rx.try_recv().unwrap();
        assert_eq!(capture.html, "original article");
        assert!(capture.favicon_data.is_none());
        downloads.complete(Ok(Accepted::Icon("late image".into())));
        assert!(downloads.pending.lock().unwrap().is_empty());
    }
    #[test]
    fn accepted_output_joins_existing_capture_and_late_output_is_discarded() {
        let (downloads, mut rx) = fixture();
        downloads.complete(Ok(Accepted::Icon("data:image/png;base64,validated".into())));
        assert_eq!(
            rx.try_recv().unwrap().favicon_data.as_deref(),
            Some("data:image/png;base64,validated")
        );
        let (downloads, mut rx) = fixture();
        downloads.pending.lock().unwrap().remove("1");
        downloads.complete(Ok(Accepted::Icon("late".into())));
        assert!(rx.try_recv().is_err());
    }
}
