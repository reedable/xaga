use super::library_upgrade::upgrade_library;
use crate::die;
use clap::{Args, Subcommand};
use std::path::Path;
use xaga_core::library;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct LibraryArgs {
    #[command(subcommand)]
    pub subcommand: LibraryCommand,
}

#[derive(Subcommand)]
pub enum LibraryCommand {
    /// Show the stored format and incomplete upgrade state without modifying the library
    Info,
    /// Upgrade legacy reading state and pins and reconcile the library cache
    Upgrade,
}

/// The CLI supplies its application version at runtime; core owns no global
/// version for the user's libraries.
pub(super) fn application_version() -> library::LibraryVersion {
    library::LibraryVersion::new(
        env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
    )
}

pub async fn handle(command: LibraryCommand, data_dir: &Path) {
    if !data_dir.is_dir() {
        die("library directory does not exist");
    }
    match command {
        LibraryCommand::Info => {
            let manifest =
                library::LibraryManifest::read(data_dir).unwrap_or_else(|error| die(error));
            println!("Library: {}", data_dir.display());
            match manifest {
                None => println!("Library version: unversioned; explicit upgrade required"),
                Some(manifest) => {
                    let version = manifest
                        .format_version()
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "unversioned".into());
                    println!("Library version: {version}");
                    if let Some(target) = manifest.migration_target() {
                        println!("Upgrade: incomplete; target version {target}");
                    }
                }
            }
            println!("Application major.minor: {}", application_version());
        }
        LibraryCommand::Upgrade => {
            if !data_dir.is_dir() {
                die("library directory does not exist");
            }
            let lock = library::acquire_lock(data_dir).unwrap_or_else(|| die("Could not acquire the library write lock. Close other Xaga instances and retry."));
            let report = upgrade_library(data_dir, &lock).await;
            println!(
                "library upgrade: {} converted, {} unchanged, {} cache entries reconciled, {} errors",
                report.converted,
                report.unchanged,
                report.reconciled,
                report.errors.len()
            );
            for error in &report.errors {
                eprintln!("  {error}");
            }
            if !report.errors.is_empty() {
                die("Library upgrade incomplete. Resolve the errors and rerun the same command.");
            }
            println!("Library upgrade complete.");
        }
    }
}
