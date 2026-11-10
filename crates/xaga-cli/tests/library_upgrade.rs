//! Actual CLI process boundary: no personal library or model download.
use std::process::Command;
use xaga_core::library::{self, LibraryManifest, LibraryVersion};

#[tokio::test]
async fn cli_process_excludes_other_writers_and_matches_shared_converter() {
    let cli = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    let source = LibraryVersion::new(0, 15);
    let target = LibraryVersion::new(
        env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
    );
    drop(library::open(cli.path(), source).unwrap());
    drop(library::open(shared.path(), source).unwrap());
    let invoke = |operation: &str| {
        Command::new(env!("CARGO_BIN_EXE_xaga-cli"))
            .arg("--data-dir")
            .arg(cli.path())
            .args(["library", operation])
            .output()
            .unwrap()
    };
    let lock = library::acquire_lock(cli.path()).unwrap();
    let refused = invoke("upgrade");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("write lock"));
    assert_eq!(
        LibraryManifest::read(cli.path())
            .unwrap()
            .unwrap()
            .migration_target(),
        None
    );
    drop(lock);
    let result = invoke("upgrade");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let lock = library::acquire_lock(shared.path()).unwrap();
    assert!(
        library::upgrade_library(shared.path(), &lock, target, |_| {})
            .await
            .errors
            .is_empty()
    );
    assert_eq!(
        std::fs::read(cli.path().join("library.yaml")).unwrap(),
        std::fs::read(shared.path().join("library.yaml")).unwrap()
    );
    let info = invoke("info");
    assert!(info.status.success());
    let output = String::from_utf8(info.stdout).unwrap();
    assert!(output.contains(cli.path().to_str().unwrap()));
    assert!(output.contains(&format!("Library version: {target}")));
    assert!(
        invoke("upgrade").status.success(),
        "completed upgrades are safe to repeat"
    );
    assert!(library::open(cli.path(), target).unwrap().is_some());
}
