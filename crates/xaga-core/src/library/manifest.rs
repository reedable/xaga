//! Authoritative library format metadata. Callers orchestrate conversion; core
//! owns the representation, validation, compatibility checks and durable writes.
use crate::fs::FileLock;
use noyalib::{ParserConfig, SerializerConfig, policy::DenyAnchors};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LibraryVersion {
    major: u64,
    minor: u64,
}
impl LibraryVersion {
    /// Construct a runtime version from the application's parsed major/minor.
    pub fn new(major: u64, minor: u64) -> Self {
        Self { major, minor }
    }
}
impl std::fmt::Display for LibraryVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
impl std::str::FromStr for LibraryVersion {
    type Err = String;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let invalid = || "library version must be a quoted major.minor string".to_owned();
        let (major, minor) = raw.split_once('.').ok_or_else(invalid)?;
        let component = |value: &str| -> Result<u64, String> {
            if value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
                || (value.len() > 1 && value.starts_with('0'))
            {
                return Err(invalid());
            }
            value.parse().map_err(|_| invalid())
        };
        Ok(Self {
            major: component(major)?,
            minor: component(minor)?,
        })
    }
}
impl TryFrom<String> for LibraryVersion {
    type Error = String;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        raw.parse()
    }
}
impl From<LibraryVersion> for String {
    fn from(version: LibraryVersion) -> Self {
        version.to_string()
    }
}

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryManifest {
    // deserialize_with makes the key required while accepting explicit null.
    #[serde(deserialize_with = "deserialize_version")]
    format_version: Option<LibraryVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    migration: Option<Migration>,
}
fn deserialize_version<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<LibraryVersion>, D::Error> {
    Option::<LibraryVersion>::deserialize(deserializer)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Migration {
    target_version: LibraryVersion,
}

#[derive(Debug, thiserror::Error)]
pub enum LibraryFormatError {
    #[error("library manifest IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid library manifest: {0}")]
    Invalid(String),
    #[error("library manifest changed during the upgrade")]
    Changed,
}

impl LibraryFormatError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Io(_) => "The library version metadata could not be read.",
            Self::Invalid(_) => "The library version metadata is malformed.",
            Self::Changed => "The library version metadata changed during the upgrade.",
        }
    }
}

/// Holds the library write right borrowed from the caller for the full conversion.
/// Dropping it without completing leaves the persisted migration marker intact.
pub struct LibraryUpgrade<'a> {
    dir: PathBuf,
    pending: LibraryManifest,
    _lock: &'a FileLock,
}
impl LibraryManifest {
    pub fn format_version(&self) -> Option<LibraryVersion> {
        self.format_version
    }
    pub fn migration_target(&self) -> Option<LibraryVersion> {
        self.migration
            .as_ref()
            .map(|migration| migration.target_version)
    }

    /// Missing metadata is unversioned. No source scan or inference is performed.
    pub fn read(dir: &Path) -> Result<Option<Self>, LibraryFormatError> {
        let path = super::manifest_path(dir);
        match std::fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(LibraryFormatError::Invalid(
                    "expected a regular manifest file".into(),
                ));
            }
            Ok(_) => {}
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_MANIFEST_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(LibraryFormatError::Invalid(
                "manifest exceeds 64 KiB".into(),
            ));
        }
        let raw = std::str::from_utf8(&bytes)
            .map_err(|error| LibraryFormatError::Invalid(error.to_string()))?;
        let manifest: Self =
            noyalib::from_str_with_config(raw, &ParserConfig::strict().with_policy(DenyAnchors))
                .map_err(|error| LibraryFormatError::Invalid(error.to_string()))?;
        manifest.validate()?;
        Ok(Some(manifest))
    }
    fn validate(&self) -> Result<(), LibraryFormatError> {
        match (self.format_version(), self.migration_target()) {
            (None, None) => Err(LibraryFormatError::Invalid(
                "null format_version requires a migration".into(),
            )),
            (Some(from), Some(target)) if target <= from => Err(LibraryFormatError::Invalid(
                "migration target must exceed the completed format".into(),
            )),
            _ => Ok(()),
        }
    }
    /// Only completed metadata matching the supplied application version is writable.
    pub fn matches_version(&self, version: LibraryVersion) -> bool {
        self.migration.is_none() && self.format_version == Some(version)
    }
    fn write(&self, dir: &Path, _lock: &FileLock) -> Result<(), LibraryFormatError> {
        self.validate()?;
        let yaml =
            noyalib::to_string_with_config(self, &SerializerConfig::new().block_scalars(false))
                .map_err(|error| LibraryFormatError::Invalid(error.to_string()))?;
        crate::fs::write_atomic(&super::manifest_path(dir), yaml.as_bytes())?;
        Ok(())
    }
    /// Persist the pending state before callers mutate source files or caches.
    /// None means this library already completed the current storage upgrade.
    /// The supplied lock must be the write right for this library directory.
    pub fn begin_upgrade<'a>(
        dir: &Path,
        target_version: LibraryVersion,
        lock: &'a FileLock,
    ) -> Result<Option<LibraryUpgrade<'a>>, LibraryFormatError> {
        let previous = Self::read(dir)?;
        if let Some(manifest) = &previous {
            if let Some(target) = manifest.migration_target() {
                if target != target_version {
                    return Err(LibraryFormatError::Invalid(format!(
                        "pending migration targets {target}; cannot change it to {target_version}"
                    )));
                }
            } else if manifest.format_version() == Some(target_version) {
                return Ok(None);
            }
            if let Some(version) = manifest.format_version() {
                if version >= target_version {
                    return Err(LibraryFormatError::Invalid(format!(
                        "upgrade target {target_version} must be newer than library version {version}"
                    )));
                }
            }
        }
        let pending = Self {
            format_version: previous
                .as_ref()
                .and_then(|manifest| manifest.format_version),
            migration: Some(Migration { target_version }),
        };
        // Resuming does not rewrite a marker that is already durable.
        if previous.as_ref() != Some(&pending) {
            pending.write(dir, lock)?;
        }
        Ok(Some(LibraryUpgrade {
            dir: dir.into(),
            pending,
            _lock: lock,
        }))
    }
}
impl LibraryUpgrade<'_> {
    /// Call only after all source conversion and cache reconciliation succeeds.
    pub fn complete(self) -> Result<(), LibraryFormatError> {
        if LibraryManifest::read(&self.dir)?.as_ref() != Some(&self.pending) {
            return Err(LibraryFormatError::Changed);
        }
        LibraryManifest {
            format_version: Some(self.pending.migration.as_ref().unwrap().target_version),
            migration: None,
        }
        .write(&self.dir, self._lock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;

    fn version(value: &str) -> LibraryVersion {
        value.parse().unwrap()
    }
    fn seed(dir: &Path, value: &str) {
        let lock = library::acquire_lock(dir).unwrap();
        LibraryManifest::begin_upgrade(dir, version(value), &lock)
            .unwrap()
            .unwrap()
            .complete()
            .unwrap();
    }

    #[test]
    fn version_strings_use_major_minor_and_numeric_order() {
        assert_eq!(LibraryVersion::new(0, 10).to_string(), "0.10");
        assert!(version("0.9") < version("0.10"));
        for invalid in [
            "1",
            "0.10.0",
            "0.10-rc.1",
            "00.10",
            "0.01",
            " 0.10",
            "+0.10",
            "0.",
            ".10",
        ] {
            assert!(invalid.parse::<LibraryVersion>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn each_library_is_compared_with_the_runtime_app_version_independently() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        seed(first.path(), "0.9");
        seed(second.path(), "0.10");
        let before = std::fs::read(library::manifest_path(first.path())).unwrap();
        assert!(
            library::open(first.path(), version("0.10"))
                .unwrap()
                .is_none()
        );
        let first_writer = library::open(first.path(), version("0.9"))
            .unwrap()
            .unwrap();
        let second_writer = library::open(second.path(), version("0.10"))
            .unwrap()
            .unwrap();
        // Even matching versions are read-only while another process holds the right.
        assert!(
            library::open(second.path(), version("0.10"))
                .unwrap()
                .is_none()
        );
        assert!(
            library::open(second.path(), version("1.0"))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            std::fs::read(library::manifest_path(first.path())).unwrap(),
            before
        );
        assert!(!library::index_path(first.path()).exists());
        drop((first_writer, second_writer));
    }

    #[test]
    fn missing_and_pending_versions_are_readonly_without_initialization() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            library::open(dir.path(), version("0.10"))
                .unwrap()
                .is_none()
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        let missing = dir.path().join("absent");
        assert!(library::open(&missing, version("0.10")).unwrap().is_none());
        assert!(!missing.exists());
        let lock = library::acquire_lock(dir.path()).unwrap();
        let upgrade = LibraryManifest::begin_upgrade(dir.path(), version("0.10"), &lock)
            .unwrap()
            .unwrap();
        let pending = LibraryManifest::read(dir.path()).unwrap().unwrap();
        assert_eq!(pending.format_version(), None);
        assert_eq!(pending.migration_target(), Some(version("0.10")));
        assert!(
            library::open(dir.path(), version("0.10"))
                .unwrap()
                .is_none()
        );
        assert!(!library::index_path(dir.path()).exists());
        drop(upgrade);
        assert!(
            !LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .matches_version(version("0.10"))
        );
        LibraryManifest::begin_upgrade(dir.path(), version("0.10"), &lock)
            .unwrap()
            .unwrap()
            .complete()
            .unwrap();
        assert!(
            LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .matches_version(version("0.10"))
        );
        assert!(
            LibraryManifest::begin_upgrade(dir.path(), version("0.10"), &lock)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn upgrade_preserves_previous_version_until_success_and_does_not_change_targets_on_retry() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path(), "0.9");
        let lock = library::acquire_lock(dir.path()).unwrap();
        let upgrade = LibraryManifest::begin_upgrade(dir.path(), version("0.10"), &lock)
            .unwrap()
            .unwrap();
        assert_eq!(
            LibraryManifest::read(dir.path())
                .unwrap()
                .unwrap()
                .format_version(),
            Some(version("0.9"))
        );
        assert!(LibraryManifest::begin_upgrade(dir.path(), version("0.11"), &lock).is_err());
        drop(upgrade);
        LibraryManifest::begin_upgrade(dir.path(), version("0.10"), &lock)
            .unwrap()
            .unwrap()
            .complete()
            .unwrap();
        let raw = std::fs::read_to_string(library::manifest_path(dir.path())).unwrap();
        let encoded: serde_json::Value = noyalib::from_str(&raw).unwrap();
        assert_eq!(encoded["format_version"].as_str(), Some("0.10"));
        assert!(!raw.contains("migration"));
    }

    #[test]
    fn malformed_or_changed_metadata_is_reported_without_overwriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let path = library::manifest_path(dir.path());
        for raw in [
            "format_version: 1",
            "format_version: 0.10",
            "{}",
            "format_version: null",
            "format_version: '0.10.0'",
            "format_version: &v '0.10'\nextra: *v",
            "format_version: '0.10'\nformat_version: '0.9'",
            "format_version: '0.10'\nmigration: {target_version: '0.9'}",
        ] {
            std::fs::write(&path, raw).unwrap();
            assert!(LibraryManifest::read(dir.path()).is_err(), "{raw}");
            assert!(LibraryManifest::begin_upgrade(dir.path(), version("0.11"), &lock).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        }
        std::fs::remove_file(&path).unwrap();
        let upgrade = LibraryManifest::begin_upgrade(dir.path(), version("0.10"), &lock)
            .unwrap()
            .unwrap();
        std::fs::write(&path, "format_version: '999.1'\n").unwrap();
        assert!(matches!(
            upgrade.complete(),
            Err(LibraryFormatError::Changed)
        ));
    }
}
