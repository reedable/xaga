use super::{LibraryManifest, LibraryVersion};
use serde::Serialize;
use std::path::Path;

/// Compatibility and write ownership are distinct. Only an existing runtime
/// writer permits ordinary writes; this inspection never acquires one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryReadOnlyCause {
    Older,
    Newer,
    Unversioned,
    PendingUpgrade,
    PendingOtherVersion,
    InvalidMetadata,
    LockUnavailable,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub path: String,
    pub app_version: LibraryVersion,
    pub stored_version: Option<LibraryVersion>,
    pub pending_target: Option<LibraryVersion>,
    pub read_only_cause: Option<LibraryReadOnlyCause>,
    pub metadata_error: Option<String>,
    pub layout_upgrade_required: bool,
}
impl LibraryStatus {
    pub fn read(dir: &Path, app_version: LibraryVersion, has_writer: bool) -> Self {
        let mut status = Self {
            path: dir.to_string_lossy().into_owned(),
            app_version,
            stored_version: None,
            pending_target: None,
            read_only_cause: None,
            metadata_error: None,
            layout_upgrade_required: super::layout::upgrade_required(dir),
        };
        match LibraryManifest::read(dir) {
            Err(error) => {
                status.read_only_cause = Some(LibraryReadOnlyCause::InvalidMetadata);
                status.metadata_error = Some(error.user_message().into());
            }
            Ok(manifest) => {
                status.stored_version = manifest.as_ref().and_then(LibraryManifest::format_version);
                status.pending_target = manifest
                    .as_ref()
                    .and_then(LibraryManifest::migration_target);
                status.read_only_cause = if let Some(target) = status.pending_target {
                    Some(if target <= app_version {
                        LibraryReadOnlyCause::PendingUpgrade
                    } else {
                        LibraryReadOnlyCause::PendingOtherVersion
                    })
                } else {
                    match status.stored_version {
                        None => Some(LibraryReadOnlyCause::Unversioned),
                        Some(version) if version < app_version => Some(LibraryReadOnlyCause::Older),
                        Some(version) if version > app_version => Some(LibraryReadOnlyCause::Newer),
                        _ if !has_writer => Some(LibraryReadOnlyCause::LockUnavailable),
                        _ => None,
                    }
                };
            }
        }
        status
    }
    pub fn can_upgrade(&self) -> bool {
        matches!(
            self.read_only_cause,
            Some(
                LibraryReadOnlyCause::Older
                    | LibraryReadOnlyCause::Unversioned
                    | LibraryReadOnlyCause::PendingUpgrade
            )
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparisons_are_numeric_and_errors_preserve_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let app = LibraryVersion::new(0, 16);
        assert!(LibraryStatus::read(dir.path(), app, false).can_upgrade());
        for (raw, expected) in [
            ("format_version: '0.9'\n", LibraryReadOnlyCause::Older),
            (
                "format_version: '0.16'\n",
                LibraryReadOnlyCause::LockUnavailable,
            ),
            ("format_version: '0.100'\n", LibraryReadOnlyCause::Newer),
            (
                "format_version: '0.15'\nmigration:\n  target_version: '0.16'\n",
                LibraryReadOnlyCause::PendingUpgrade,
            ),
            (
                "format_version: '0.9'\nmigration:\n  target_version: '0.12'\n",
                LibraryReadOnlyCause::PendingUpgrade,
            ),
            (
                "format_version: '0.15'\nmigration:\n  target_version: '0.17'\n",
                LibraryReadOnlyCause::PendingOtherVersion,
            ),
            ("invalid: true\n", LibraryReadOnlyCause::InvalidMetadata),
        ] {
            std::fs::write(dir.path().join("library.yaml"), raw).unwrap();
            assert_eq!(
                LibraryStatus::read(dir.path(), app, false).read_only_cause,
                Some(expected)
            );
            assert_eq!(
                std::fs::read_to_string(dir.path().join("library.yaml")).unwrap(),
                raw
            );
        }
    }
}
