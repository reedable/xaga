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
