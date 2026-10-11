//! Bounded, local article diagnostics. Remote pages may report only enumerated
//! milestones for their own pending loader; they cannot submit log text.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_ATTEMPTS: usize = 32;
const MAX_EVENTS: usize = 128;
static ATTEMPT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArticleDiagnosticOutcome {
    Started,
    Succeeded,
    Failed,
    Warning,
    Skipped,
    Cancelled,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArticleDiagnosticEvent {
    elapsed_ms: u128,
    source: String,
    step: String,
    outcome: ArticleDiagnosticOutcome,
    code: String,
    detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    download: Option<DownloadDiagnostic>,
}

/// Native observations only. URL paths/queries and filenames can be sensitive.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DownloadDiagnostic {
    pub loader_label: String,
    pub source_url: String,
    pub current_url: Option<String>,
    pub download_url: String,
    pub destination: Option<String>,
    pub success: Option<bool>,
}

impl DownloadDiagnostic {
    /// Collect callback evidence without requiring a committed document URL.
    pub(crate) fn from_download<R: tauri::Runtime>(
        webview: &tauri::Webview<R>,
        source_url: &str,
        download_url: &tauri::Url,
        destination: Option<String>,
        success: Option<bool>,
    ) -> Self {
        Self {
            loader_label: webview.label().to_owned(),
            source_url: source_url.to_owned(),
            // A download can precede the first document commit. Wry 0.57's
            // url() unwraps WebKit's absent URL; `.ok()` cannot catch that panic.
            current_url: None,
            download_url: download_url.to_string(),
            destination,
            success,
        }
    }
}

fn bounded_download_text(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).take(4096).collect()
}

fn diagnostic_url(value: &str) -> String {
    match tauri::Url::parse(value) {
        Ok(mut url) if matches!(url.scheme(), "http" | "https") => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_fragment(None);
            bounded_download_text(url.as_str())
        }
        _ => "[non-HTTP URL omitted]".into(),
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArticleDiagnostic {
    id: String,
    started_at_ms: u128,
    mode: String,
    outcome: Option<ArticleDiagnosticOutcome>,
    events: VecDeque<ArticleDiagnosticEvent>,
    dropped_events: usize,
}

struct ArticleLoadState {
    cancelled: bool,
    started: Instant,
    diagnostic: ArticleDiagnostic,
}
#[derive(Clone)]
pub(crate) struct ArticleLoadAttempt(Arc<Mutex<ArticleLoadState>>);
#[derive(Default)]
pub(crate) struct ArticleDiagnostics(Mutex<VecDeque<ArticleLoadAttempt>>);

/// Remove complete URL tokens rather than trying to enumerate secret query
/// parameter names. Diagnostic callers never pass content or request headers.
pub(crate) fn sanitize_detail(value: &str) -> String {
    value
        .split_whitespace()
        .take(100)
        .map(|word| {
            if word.contains("://") || word.starts_with("data:") {
                "[url omitted]".to_owned()
            } else {
                word.chars().filter(|c| !c.is_control()).take(160).collect()
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
        .chars()
        .take(1024)
        .collect()
}

impl ArticleDiagnostics {
    pub(crate) fn create_attempt(&self, mode: &str) -> ArticleLoadAttempt {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let attempt = ArticleLoadAttempt(Arc::new(Mutex::new(ArticleLoadState {
            cancelled: false,
            started: Instant::now(),
            diagnostic: ArticleDiagnostic {
                id: format!(
                    "{now:x}-{:x}",
                    ATTEMPT_COUNTER.fetch_add(1, Ordering::Relaxed)
                ),
                started_at_ms: now,
                mode: sanitize_detail(mode),
                outcome: None,
                events: VecDeque::new(),
                dropped_events: 0,
            },
        })));
        let mut attempts = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if attempts.len() == MAX_ATTEMPTS {
            attempts.pop_front();
        }
        attempts.push_back(attempt.clone());
        attempt
    }
    pub(crate) fn get_attempt(&self, id: &str) -> Option<ArticleLoadAttempt> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|t| t.get_id() == id)
            .cloned()
    }
}
impl ArticleLoadAttempt {
    pub(crate) fn cancel(&self) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).cancelled = true;
    }
    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).cancelled
    }
    pub(crate) fn get_id(&self) -> String {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .diagnostic
            .id
            .clone()
    }
    pub(crate) fn get_diagnostic(&self) -> ArticleDiagnostic {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .diagnostic
            .clone()
    }
    pub(crate) fn add_event(
        &self,
        source: &str,
        step: &str,
        outcome: ArticleDiagnosticOutcome,
        code: &str,
        detail: &str,
    ) {
        self.record_event(source, step, outcome, code, detail, None);
    }

    pub(crate) fn record_download(&self, requested: bool, accepted: bool, mut download: DownloadDiagnostic) {
        download.loader_label = bounded_download_text(&download.loader_label);
        download.source_url = diagnostic_url(&download.source_url);
        download.current_url = download.current_url.map(|url| diagnostic_url(&url));
        download.download_url = diagnostic_url(&download.download_url);
        download.destination = download.destination.map(|path| bounded_download_text(&path));
        self.record_event(
            "native",
            if requested { "download_requested" } else { "download_finished" },
            if requested && accepted {
                ArticleDiagnosticOutcome::Started
            } else if !requested && download.success == Some(true) {
                ArticleDiagnosticOutcome::Succeeded
            } else {
                ArticleDiagnosticOutcome::Warning
            },
            if requested { if accepted { "article:download-accepted" } else { "article:download-rejected" } } else { "article:download-finished" },
            if requested { if accepted { "Download accepted into private staging; byte validation required before persistence" } else { "Download rejected; no registered consumer accepted this resource" } }
            else { "Completion reported by runtime; validation and persistence are separate steps" },
            Some(download),
        );
    }

    fn record_event(
        &self,
        source: &str,
        step: &str,
        outcome: ArticleDiagnosticOutcome,
        code: &str,
        detail: &str,
        download: Option<DownloadDiagnostic>,
    ) {
        let mut attempt = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if source == "frontend"
            && step == "attempt"
            && !matches!(outcome, ArticleDiagnosticOutcome::Started)
        {
            attempt.diagnostic.outcome = Some(outcome.clone());
        }
        // Preserve the beginning and newest events; bound both memory and log
        // output from noisy pages. Terminal events are still retained.
        if attempt.diagnostic.events.len() >= MAX_EVENTS {
            attempt.diagnostic.events.remove(1);
            attempt.diagnostic.dropped_events += 1;
        }
        let event = ArticleDiagnosticEvent {
            elapsed_ms: attempt.started.elapsed().as_millis(),
            source: sanitize_detail(source),
            step: sanitize_detail(step),
            outcome,
            code: sanitize_detail(code),
            detail: sanitize_detail(detail),
            download,
        };
        if attempt.diagnostic.dropped_events == 0
            || matches!(
                event.outcome,
                ArticleDiagnosticOutcome::Failed | ArticleDiagnosticOutcome::Cancelled
            )
        {
            log::info!(target: "xaga_lib::article_diagnostics", "article_diagnostic {}", serde_json::json!({"attemptId": attempt.diagnostic.id, "mode": attempt.diagnostic.mode, "event": event}));
        }
        attempt.diagnostic.events.push_back(event);
    }
    pub(crate) fn add_started_event(&self, step: &str) {
        self.add_event("native", step, ArticleDiagnosticOutcome::Started, "", "");
    }
    pub(crate) fn add_succeeded_event(&self, step: &str) {
        self.add_event("native", step, ArticleDiagnosticOutcome::Succeeded, "", "");
    }
    pub(crate) fn add_failed_event(&self, step: &str, code: &str, detail: &str) {
        self.add_event(
            "native",
            step,
            ArticleDiagnosticOutcome::Failed,
            code,
            detail,
        );
    }
    pub(crate) fn is_failure_recorded(&self) -> bool {
        self.get_diagnostic()
            .events
            .iter()
            .any(|event| matches!(event.outcome, ArticleDiagnosticOutcome::Failed))
    }
    pub(crate) fn get_last_step(&self) -> String {
        self.get_diagnostic()
            .events
            .iter()
            .rev()
            .find(|e| e.source == "native" || e.source == "loader")
            .map(|e| e.step.clone())
            .unwrap_or_else(|| "navigation".into())
    }
}

/// Accept fixed vocabulary and codes, never arbitrary frontend exception text.
pub(crate) fn validate_event(step: &str, code: &str) -> Result<(), ()> {
    if ![
        "attempt",
        "lookup",
        "read",
        "fetch",
        "parse",
        "readability",
        "images",
        "sanitize",
        "markdown",
        "create",
        "save",
        "readback",
        "pdf_decode",
        "render",
        "extract",
    ]
    .contains(&step)
        || code.len() > 80
        || !code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_evidence_is_bounded_and_preserves_request_identity() {
        let diagnostics = ArticleDiagnostics::default();
        let attempt = diagnostics.create_attempt("load");
        for _ in 0..140 {
            attempt.record_download(true, false, DownloadDiagnostic {
                loader_label: "loader-7".into(),
                source_url: "https://example.org/article".into(),
                current_url: Some("data:text/html,private-content".into()),
                download_url: "https://user:password@example.org/file?token=investigate#secret".into(),
                destination: Some(format!("/Downloads/{}\n", "x".repeat(5000))),
                success: None,
            });
        }
        let record = attempt.get_diagnostic();
        assert_eq!(record.events.len(), MAX_EVENTS);
        assert_eq!(record.dropped_events, 12);
        let event = record.events.back().unwrap();
        assert_eq!(event.code, "article:download-rejected");
        let evidence = event.download.as_ref().unwrap();
        assert_eq!(evidence.download_url, "https://example.org/file?token=investigate");
        assert_eq!(evidence.current_url.as_deref(), Some("[non-HTTP URL omitted]"));
        assert_eq!(evidence.destination.as_ref().unwrap().len(), 4096);
        assert!(evidence.success.is_none());
        assert!(diagnostics.create_attempt("other").get_diagnostic().events.is_empty());
    }
    #[test]
    fn bounds_and_isolates_attempts() {
        let diagnostics = ArticleDiagnostics::default();
        let a = diagnostics.create_attempt("load");
        let b = diagnostics.create_attempt("load");
        for _ in 0..200 {
            a.add_started_event("capture");
        }
        a.add_failed_event("capture", "timeout", "no acknowledgement");
        assert_eq!(a.get_diagnostic().events.len(), MAX_EVENTS);
        assert_eq!(a.get_diagnostic().dropped_events, 73);
        assert!(b.get_diagnostic().events.is_empty());
        assert_eq!(a.get_diagnostic().events.back().unwrap().code, "timeout");
        for _ in 0..MAX_ATTEMPTS {
            diagnostics.create_attempt("load");
        }
        assert!(diagnostics.get_attempt(&a.get_id()).is_none());
    }
    #[test]
    fn removes_urls_controls_and_bounds_detail() {
        let result = sanitize_detail(
            "request (https://user:password@host/path?token=secret#fragment) failed\nnow",
        );
        assert_eq!(result, "request [url omitted] failed now");
        assert!(sanitize_detail(&"x".repeat(5000)).len() <= 1024);
    }
}
