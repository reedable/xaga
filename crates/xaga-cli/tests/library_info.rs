use std::{
    path::Path,
    process::{Command, Output},
};
use xaga_core::library::{self, LibraryManifest};

fn cli(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xaga-cli"))
        .arg("--data-dir")
        .arg(dir)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn info_reads_each_library_version_without_modifying_it_or_enforcing_compatibility() {
    let dir = tempfile::tempdir().unwrap();
    let result = cli(dir.path(), &["library", "info"]);
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("Library version: unversioned")
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let lock = library::acquire_lock(dir.path()).unwrap();
    LibraryManifest::begin_upgrade(dir.path(), "0.9".parse().unwrap(), &lock)
        .unwrap()
        .unwrap()
        .complete()
        .unwrap();
    let second = tempfile::tempdir().unwrap();
    let second_lock = library::acquire_lock(second.path()).unwrap();
    LibraryManifest::begin_upgrade(second.path(), "999.1".parse().unwrap(), &second_lock)
        .unwrap()
        .unwrap()
        .complete()
        .unwrap();
    for (path, expected) in [(dir.path(), "0.9"), (second.path(), "999.1")] {
        let before = std::fs::read(path.join("library.yaml")).unwrap();
        let result = cli(path, &["library", "info"]);
        assert!(result.status.success());
        assert!(
            String::from_utf8(result.stdout)
                .unwrap()
                .contains(&format!("Library version: {expected}"))
        );
        assert_eq!(std::fs::read(path.join("library.yaml")).unwrap(), before);
        assert!(!library::index_path(path).exists());
    }
    let upgrade = LibraryManifest::begin_upgrade(dir.path(), "0.10".parse().unwrap(), &lock)
        .unwrap()
        .unwrap();
    let result = cli(dir.path(), &["library", "info"]);
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("Library version: 0.9"));
    assert!(output.contains("Upgrade: incomplete; target version 0.10"));
    drop(upgrade);
}

#[test]
fn normal_cli_reads_are_not_blocked_by_the_app_startup_policy() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(library::entries_dir(dir.path())).unwrap();
    std::fs::write(library::entry_path(dir.path(), "article"), "---\nsource: https://example.org\ntitle: Kept title\ncreated: yesterday\n---\nUser content\n").unwrap();
    for metadata in [
        None,
        Some("format_version: '999.1'\n"),
        Some("format_version: null\nmigration: {target_version: '0.10'}\n"),
    ] {
        if let Some(raw) = metadata {
            std::fs::write(dir.path().join("library.yaml"), raw).unwrap();
        }
        let result = cli(dir.path(), &["articles", "read", "article"]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            String::from_utf8(result.stdout)
                .unwrap()
                .contains("User content")
        );
        assert!(!library::index_path(dir.path()).exists());
    }
}
