use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, path::BaseDirectory};

const LIST: &str = "easyprivacy.txt";
const BUNDLED_LIST: &str = "resources/easyprivacy/easyprivacy.txt";

pub(super) fn installed(root: &Path) -> io::Result<bool> {
    match std::fs::metadata(root.join(LIST)) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The EasyPrivacy path is not a file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

const METADATA: &str = "easyprivacy.json";
const UPSTREAM: &str = "https://easylist.to/easylist/easyprivacy.txt";
// Advertised in the official list's Expires header.
pub(super) const MIN_REFRESH_DAYS: u64 = 4;
const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Deserialize, Serialize)]
struct Metadata {
    version: String,
    commit: String,
    installed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked_at: Option<u64>,
}

struct Revision<'a> {
    version: &'a str,
    commit: &'a str,
}

impl Metadata {
    fn new(revision: &Revision<'_>, installed_at: Option<u64>) -> Self {
        Self {
            version: revision.version.to_owned(),
            commit: revision.commit.to_owned(),
            installed_at,
            checked_at: None,
        }
    }

    fn matches(&self, revision: &Revision<'_>) -> bool {
        self.version == revision.version && self.commit == revision.commit
    }
}

fn unix_seconds() -> io::Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs())
}

fn read_metadata(root: &Path, revision: &Revision<'_>) -> Option<Metadata> {
    let bytes = std::fs::read(root.join(METADATA)).ok()?;
    let metadata: Metadata = serde_json::from_slice(&bytes).ok()?;
    metadata.matches(revision).then_some(metadata)
}

fn write_metadata(root: &Path, metadata: &Metadata) -> io::Result<()> {
    xaga_core::fs::write_atomic(&root.join(METADATA), &serde_json::to_vec_pretty(metadata)?)
}

fn header<'a>(list: &'a str, prefix: &str) -> io::Result<&'a str> {
    list.lines()
        .take(16)
        .find_map(|line| line.strip_prefix(prefix))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("Missing {prefix}")))
}

/// The caller holds the account lock across the existence check and both writes.
pub(super) fn install_if_missing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    root: &Path,
) -> Result<(), String> {
    let bundled = if tauri::is_dev() {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_LIST)
    } else {
        app.path()
            .resolve(BUNDLED_LIST, BaseDirectory::Resource)
            .map_err(|error| error.to_string())?
    };
    install(root, &bundled).map_err(|error| format!("Could not install EasyPrivacy: {error}"))
}

fn revision(bytes: &[u8]) -> io::Result<Revision<'_>> {
    let list = std::str::from_utf8(bytes).map_err(io::Error::other)?;
    let version = header(list, "! Version: ")?;
    let commit = header(list, "! Commit: ")?;
    if !list.starts_with("[Adblock Plus ")
        || header(list, "! Title: ")? != "EasyPrivacy"
        || version.len() != 12
        || !version.bytes().all(|byte| byte.is_ascii_digit())
        || commit.len() != 40
        || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !list.lines().any(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with(['!', '['])
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid EasyPrivacy list",
        ));
    }
    Ok(Revision { version, commit })
}

fn install(root: &Path, bundled: &Path) -> io::Result<()> {
    if installed(root)? {
        return Ok(());
    }

    let bytes = std::fs::read(bundled)?;
    let revision = revision(&bytes)?;
    xaga_core::fs::write_atomic(&root.join(LIST), &bytes)?;
    write_metadata(root, &Metadata::new(&revision, Some(unix_seconds()?)))
}

fn due(metadata: Option<&Metadata>, now: u64, days: u64) -> bool {
    let last_success = metadata.and_then(|metadata| metadata.checked_at.or(metadata.installed_at));
    last_success.is_none_or(|last| now.saturating_sub(last) >= days.saturating_mul(SECONDS_PER_DAY))
}

async fn fetch(proxy_port: u16, settings: &super::UserSettings) -> Result<Vec<u8>, String> {
    let client = super::fetch_client::try_build(
        proxy_port,
        super::fetch_client::effective_max_redirects(settings.max_redirects),
        false,
        false,
    )
    .map_err(|error| error.to_string())?;
    let mut response = client
        .get(UPSTREAM)
        .timeout(Duration::from_secs(
            settings
                .feed_timeout_secs
                .unwrap_or(super::DEFAULT_FEED_TIMEOUT_SECS),
        ))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?;
    let max_bytes = settings
        .feed_max_body_bytes
        .unwrap_or(super::DEFAULT_FEED_MAX_BODY_BYTES);
    let mut list = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if (list.len() as u64).saturating_add(chunk.len() as u64) > max_bytes {
            return Err(format!("EasyPrivacy list exceeded {max_bytes} bytes"));
        }
        list.extend_from_slice(&chunk);
    }
    Ok(list)
}

fn save_refresh(
    root: &Path,
    installed: &Revision<'_>,
    metadata: Option<Metadata>,
    bytes: &[u8],
    now: u64,
) -> io::Result<()> {
    let incoming = revision(bytes)?;
    let metadata = if incoming.version > installed.version {
        xaga_core::fs::write_atomic(&root.join(LIST), bytes)?;
        Metadata::new(&incoming, Some(now))
    } else {
        let mut metadata = metadata.unwrap_or_else(|| Metadata::new(installed, None));
        metadata.checked_at = Some(now);
        metadata
    };
    write_metadata(root, &metadata)
}

/// Called once during startup, after the network proxy preferences are loaded.
pub(super) async fn refresh_at_launch<R: tauri::Runtime>(
    app: AppHandle<R>,
    account_lock: std::sync::Arc<std::sync::Mutex<super::AccountLock>>,
    settings_path: std::path::PathBuf,
    proxy_port: u16,
) -> Result<(), String> {
    if !super::load_settings(&settings_path).easyprivacy_enabled() {
        return Ok(());
    }
    let Some(_lease) = super::account_lease(&account_lock, &settings_path) else {
        return Ok(());
    };
    let root = settings_path.parent().expect("account directory");
    let (settings, bytes) = {
        let _guard = account_lock.lock().unwrap();
        let settings = super::load_settings(&settings_path);
        if !settings.easyprivacy_enabled() {
            return Ok(());
        }
        if !installed(root).map_err(|error| error.to_string())? {
            return install_if_missing(&app, root);
        }
        let bytes = std::fs::read(root.join(LIST)).map_err(|error| error.to_string())?;
        (settings, bytes)
    };
    let installed = revision(&bytes).map_err(|error| error.to_string())?;
    let metadata = read_metadata(root, &installed);
    let now = unix_seconds().map_err(|error| error.to_string())?;
    if !due(metadata.as_ref(), now, settings.easyprivacy_refresh_days()) {
        return Ok(());
    }

    let response = fetch(proxy_port, &settings).await?;
    let _guard = account_lock.lock().unwrap();
    if !super::load_settings(&settings_path).easyprivacy_enabled() {
        return Ok(());
    }
    save_refresh(
        root,
        &installed,
        metadata,
        &response,
        unix_seconds().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(version: &str) -> Vec<u8> {
        format!(
            "[Adblock Plus 1.1]\n! Version: {version}\n! Title: EasyPrivacy\n! Commit: {}\n||tracking.example^\n",
            "a".repeat(40),
        )
        .into_bytes()
    }

    #[test]
    fn refresh_cadence_uses_current_revision_and_latest_success() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let bytes = snapshot("202611270000");
        let installed = revision(&bytes).unwrap();
        let initial = Metadata::new(&installed, Some(100));
        assert!(!due(Some(&initial), 100 + 4 * SECONDS_PER_DAY - 1, 4));
        assert!(due(Some(&initial), 100 + 4 * SECONDS_PER_DAY, 4));
        assert!(!due(Some(&initial), 100 + 4 * SECONDS_PER_DAY, 7));

        let mismatched = snapshot("202611260000");
        write_metadata(
            root,
            &Metadata::new(&revision(&mismatched).unwrap(), Some(100)),
        )
        .unwrap();
        let metadata = read_metadata(root, &installed);
        assert!(due(metadata.as_ref(), 100, 4));

        let check_at = 500;
        save_refresh(root, &installed, metadata, &bytes, check_at).unwrap();
        let metadata = read_metadata(root, &installed).unwrap();
        assert!(metadata.installed_at.is_none());
        assert!(!due(Some(&metadata), check_at + 4 * SECONDS_PER_DAY - 1, 4));
        assert!(due(Some(&metadata), check_at + 4 * SECONDS_PER_DAY, 4));
    }

    #[test]
    fn refresh_replaces_only_on_valid_newer_revision() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let bytes = snapshot("202611270000");
        let installed = revision(&bytes).unwrap();
        let initial = Metadata::new(&installed, Some(100));
        std::fs::write(root.join(LIST), &bytes).unwrap();
        write_metadata(root, &initial).unwrap();

        let older = snapshot("202611260000");
        save_refresh(root, &installed, Some(initial), &older, 200).unwrap();
        assert_eq!(std::fs::read(root.join(LIST)).unwrap(), bytes);
        let checked = read_metadata(root, &installed).unwrap();
        assert_eq!(checked.checked_at, Some(200));
        assert_eq!(checked.installed_at, Some(100));

        assert!(save_refresh(root, &installed, Some(checked), b"invalid response", 300).is_err());
        assert_eq!(std::fs::read(root.join(LIST)).unwrap(), bytes);
        assert_eq!(
            read_metadata(root, &installed).unwrap().checked_at,
            Some(200)
        );

        let later = snapshot("202611280000");
        save_refresh(
            root,
            &installed,
            read_metadata(root, &installed),
            &later,
            400,
        )
        .unwrap();
        assert_eq!(std::fs::read(root.join(LIST)).unwrap(), later);
        let metadata = read_metadata(root, &revision(&later).unwrap()).unwrap();
        assert_eq!(metadata.installed_at, Some(400));
    }

    #[tokio::test]
    async fn launch_restores_missing_list_despite_recent_metadata_only_when_enabled() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let settings_path = root.join("settings.json");
        std::fs::write(&settings_path, br#"{"easyprivacy_enabled":false}"#).unwrap();
        let bytes = snapshot("202611270000");
        let revision = revision(&bytes).unwrap();
        let now = unix_seconds().unwrap();
        write_metadata(root, &Metadata::new(&revision, Some(now))).unwrap();

        let app = tauri::test::mock_app();
        let registry =
            std::sync::Arc::new(std::sync::Mutex::new(super::super::AccountLock::default()));
        refresh_at_launch(
            app.handle().clone(),
            registry.clone(),
            settings_path.clone(),
            0,
        )
        .await
        .unwrap();
        assert!(!installed(root).unwrap());

        std::fs::write(&settings_path, br#"{"easyprivacy_enabled":true}"#).unwrap();
        refresh_at_launch(app.handle().clone(), registry, settings_path, 0)
            .await
            .unwrap();
        assert!(installed(root).unwrap());
        let expected =
            std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_LIST)).unwrap();
        assert_eq!(std::fs::read(root.join(LIST)).unwrap(), expected);
    }

    #[test]
    fn failed_refresh_writes_leave_the_check_due() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let bytes = snapshot("202611270000");
        let installed = revision(&bytes).unwrap();
        let newer = snapshot("202611280000");
        let later = 400_000;

        write_metadata(root, &Metadata::new(&installed, Some(100))).unwrap();
        let original_metadata = std::fs::read(root.join(METADATA)).unwrap();
        std::fs::create_dir(root.join(LIST)).unwrap();
        let metadata = read_metadata(root, &installed);
        assert!(save_refresh(root, &installed, metadata, &newer, later).is_err());
        assert_eq!(
            std::fs::read(root.join(METADATA)).unwrap(),
            original_metadata
        );
        assert!(due(read_metadata(root, &installed).as_ref(), later, 4));

        std::fs::remove_dir(root.join(LIST)).unwrap();
        std::fs::write(root.join(LIST), &bytes).unwrap();
        std::fs::remove_file(root.join(METADATA)).unwrap();
        std::fs::create_dir(root.join(METADATA)).unwrap();
        assert!(save_refresh(root, &installed, None, &newer, later).is_err());
        assert_eq!(std::fs::read(root.join(LIST)).unwrap(), newer);
        let new_revision = revision(&newer).unwrap();
        assert!(due(read_metadata(root, &new_revision).as_ref(), later, 4));
    }

    #[test]
    fn installs_exact_snapshot_and_preserves_existing_file_and_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_LIST);
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        install(root, &bundled).unwrap();

        let list = std::fs::read_to_string(bundled).unwrap();
        let installed = root.join(LIST);
        assert_eq!(std::fs::read(&installed).unwrap(), list.as_bytes());
        let metadata_path = root.join("easyprivacy.json");
        let recorded = std::fs::read(&metadata_path).unwrap();
        let metadata: serde_json::Value = serde_json::from_slice(&recorded).unwrap();
        assert_eq!(metadata["version"], header(&list, "! Version: ").unwrap());
        assert_eq!(metadata["commit"], header(&list, "! Commit: ").unwrap());
        assert!(metadata["installed_at"].as_u64().unwrap() >= before);

        // A subsequent launch or enable reuses the file even if the bundled
        // resource has changed or is unavailable.
        std::fs::write(&installed, b"existing revision").unwrap();
        install(root, &root.join("missing.txt")).unwrap();
        assert_eq!(std::fs::read(installed).unwrap(), b"existing revision");
        assert_eq!(std::fs::read(metadata_path).unwrap(), recorded);
    }
}
