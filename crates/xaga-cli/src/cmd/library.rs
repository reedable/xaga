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
    /// Upgrade a 0.9 library to the 0.10 format and reconcile its cache
    Upgrade,
}

pub async fn handle(command: LibraryCommand, data_dir: &Path) {
    match command {
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
