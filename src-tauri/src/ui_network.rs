//! UI-only proxy policy and delivery through the existing error/diagnostic channels.
use crate::{
    article_diagnostics::{ArticleDiagnostics, ArticleLoadAttempt},
    error::SerializedError,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State};
use xaga_core::network_proxy::{ProxyConfig, ScopedProxy};

/// Retain bounded notifications so listener installation cannot lose early errors.
#[derive(Default)]
pub(crate) struct UiErrors(Mutex<HashMap<String, VecDeque<SerializedError>>>);

impl UiErrors {
    fn take(&self, label: &str) -> Vec<SerializedError> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(label)
            .unwrap_or_default()
            .into_iter()
            .collect()
    }
    fn retain(&self, label: &str, error: SerializedError) {
        let mut pending = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let queue = pending.entry(label.into()).or_default();
        if queue.len() == 8 {
            queue.pop_front();
        }
        queue.push_back(error);
    }
}

#[tauri::command]
pub(crate) fn take_ui_errors(
    window: tauri::WebviewWindow,
    errors: State<'_, UiErrors>,
) -> Vec<SerializedError> {
    errors.take(window.label())
}

pub(crate) fn start(
    app: &AppHandle,
    label: &'static str,
) -> Result<ScopedProxy, Box<dyn std::error::Error>> {
    // A newly created window must not inherit notifications from an earlier one.
    app.state::<UiErrors>().take(label);
    let mut config = ProxyConfig::deny_all();
    // Tauri dev serves assets and HMR from a local server. Release builds have
    // no exception; development never permits an arbitrary remote dev URL.
    if tauri::is_dev() {
        if let Some(url) = &app.config().build.dev_url {
            use std::net::ToSocketAddrs;
            let host = url.host_str().ok_or("development URL has no host")?;
            if host != "localhost"
                && !host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            {
                return Err("UI development server must be loopback".into());
            }
            let port = url
                .port_or_known_default()
                .ok_or("development URL has no port")?;
            let address = (host, port)
                .to_socket_addrs()?
                .find(|a| a.ip().is_loopback())
                .ok_or("no loopback development endpoint")?;
            config = config.with_local_endpoint(host.to_owned(), address)?;
        }
    }
    let handle = app.clone();
    let burst: Arc<Mutex<Option<(Instant, ArticleLoadAttempt)>>> = Arc::new(Mutex::new(None));
    config = config.with_observer(Arc::new(move |code, detail| {
        let mut current = burst.lock().unwrap_or_else(|e| e.into_inner());
        let notify = current
            .as_ref()
            .is_none_or(|(last, _)| last.elapsed() >= Duration::from_secs(1));
        if notify {
            let attempt = handle
                .state::<ArticleDiagnostics>()
                .create_attempt(&format!("{label}-network"));
            *current = Some((Instant::now(), attempt));
        }
        let (last, attempt) = current.as_mut().unwrap();
        *last = Instant::now();
        attempt.add_failed_event("network", code, detail);
        if !notify {
            return;
        }
        let error = SerializedError {
            diagnostic_id: Some(attempt.get_id()),
            code: "proxy:blocked",
            message: "Remote content was blocked.",
            detail: format!("{label}: {code}; {detail}"),
        };
        drop(current);
        handle.state::<UiErrors>().retain(label, error.clone());
        crate::events::emit_error(&handle, label, error);
    }));
    Ok(tauri::async_runtime::block_on(
        xaga_core::network_proxy::start_scoped(config),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notifications_are_bounded_and_drained_only_for_the_calling_window() {
        let errors = UiErrors::default();
        for n in 0..10 {
            errors.retain(
                "main",
                SerializedError {
                    code: "proxy:blocked",
                    message: "Remote content was blocked.",
                    detail: n.to_string(),
                    diagnostic_id: None,
                },
            );
        }
        errors.retain(
            "settings",
            SerializedError {
                code: "proxy:blocked",
                message: "Remote content was blocked.",
                detail: "settings".into(),
                diagnostic_id: None,
            },
        );
        assert!(errors.take("loader-1").is_empty());
        let main = errors.take("main");
        assert_eq!(main.len(), 8);
        assert_eq!(main[0].detail, "2");
        assert!(errors.take("main").is_empty());
        assert_eq!(errors.take("settings")[0].detail, "settings");
    }
}

/// Tie the listener and its connections to the native window lifetime.
pub(crate) fn retain_for_window(window: &tauri::WebviewWindow, proxy: ScopedProxy) {
    let proxy = Mutex::new(Some(proxy));
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            proxy.lock().unwrap_or_else(|e| e.into_inner()).take();
        }
    });
}
