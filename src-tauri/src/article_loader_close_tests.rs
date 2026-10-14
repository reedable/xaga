use super::*;
use crate::article_diagnostics::ArticleDiagnostics;

fn request(download_active: bool) -> PendingRequest {
    let (tx, _rx) = oneshot::channel();
    PendingRequest {
        tx,
        keep_open: true,
        purpose: LoaderPurpose::Article,
        attempt: ArticleDiagnostics::default().create_attempt("load"),
        diagnostic_event_count: 0,
        download_active,
        partial: None,
    }
}

#[test]
fn closing_visible_loader_after_initial_capture_requests_update_capture_once() {
    let mut pending = HashMap::from([("article", request(false))]);
    // receive_html consumes the initial request, leaving the visible window open.
    pending.remove("article");
    let closing = AtomicBool::new(false);
    assert!(capture_on_close(pending.get("article"), &closing));
    // capture_and_close_loader closes again after delivering loader:closed.
    assert!(!capture_on_close(pending.get("article"), &closing));
}

#[test]
fn closing_before_initial_capture_still_captures() {
    assert!(capture_on_close(
        Some(&request(false)),
        &AtomicBool::new(false)
    ));
}

#[test]
fn active_native_download_closes_without_html_capture() {
    assert!(!capture_on_close(
        Some(&request(true)),
        &AtomicBool::new(false)
    ));
}

#[test]
fn programmatic_download_cleanup_closes_after_request_removal() {
    assert!(!capture_on_close(None, &AtomicBool::new(true)));
}
