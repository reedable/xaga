//! Recovery's feed-entry reconciliation. The `.atom` write and the entry `.md` writes aren't in
//! one transaction, so a crash between them (in add/refresh) can leave atom entries with no `.md`.
//! This compares each feed's atom membership against the entries on disk and **returns** the ones
//! that are missing — it creates nothing. The caller saves them, keeping the diff (here) and the
//! side effect (the save) separate.

use std::path::Path;

use crate::types::Entry;

/// Compare every feed's `.atom` membership against the entries on disk and return the entries that
/// have no `.md` yet (the atom lists them, the file is absent). Reads only — the caller saves them.
pub(crate) async fn reconcile_feed_entries(library_dir: &Path) -> Vec<Entry> {
    let feeds_dir = crate::library::feeds_dir(library_dir);
    let mut missing = Vec::new();

    let mut dir = match tokio::fs::read_dir(&feeds_dir).await {
        Ok(d) => d,
        Err(_) => return missing,
    };

    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let name = path.to_string_lossy();
        if !name.ends_with(".atom") || name.ends_with(".atom.tmp") {
            continue;
        }

        let entries = match crate::persistence::feeds::entries_from_path(&path).await {
            Ok(e) => e,
            Err(e) => {
                log::warn!("reconcile_feed_entries: {e}");
                continue;
            }
        };

        for entry in entries {
            if entry.url.is_empty() {
                continue;
            }
            if crate::persistence::entries::find_entry_path(library_dir, &entry.id)
                .await
                .is_none()
            {
                missing.push(entry);
            }
        }
    }

    missing
}
