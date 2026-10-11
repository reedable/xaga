//! Only native loader construction can assign a session's source URL.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub struct LoaderSessions(Arc<Mutex<HashMap<String, String>>>);

impl LoaderSessions {
    pub fn register(&self, label: String, source_url: String) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(label, source_url);
    }

    pub fn remove(&self, label: &str) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(label);
    }

    pub fn source_url(&self, invoking_label: &str) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(invoking_label)
            .cloned()
    }
}

pub fn same_page(source: &str, current: &str) -> bool {
    match (
        xaga_core::util::normalize_url(source),
        xaga_core::util::normalize_url(current),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => source == current,
    }
}
