use std::collections::HashSet;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::persistence::feeds::{self, RepairOutcome};

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct FeedsArgs {
    #[command(subcommand)]
    pub subcommand: FeedsCommand,
}

#[derive(Subcommand)]
pub enum FeedsCommand {
    /// List all saved feeds
    List,
    /// Repair feed files: add missing <link rel="self"> and rename files whose
    /// hash no longer matches their feed URL
    Repair,
}

pub async fn handle(cmd: FeedsCommand, feeds_dir: &PathBuf, data_dir: &PathBuf) {
    match cmd {
        FeedsCommand::List => {
            let (unread, bookmarked) = read_state_sets(data_dir).await;
            match feeds::list_feeds(feeds_dir, &unread, &bookmarked).await {
                Ok(feeds) if feeds.is_empty() => {
                    println!("No feeds saved.");
                }
                Ok(feeds) => {
                    for feed in &feeds {
                        let updated = feed.updated_at.as_deref().unwrap_or("unknown");
                        println!("[{}] {} ({} entries)", feed.id, feed.title, feed.entry_count);
                        println!("  url:     {}", feed.url);
                        println!("  updated: {updated}");
                    }
                }
                Err(e) => die(e),
            }
        }

        FeedsCommand::Repair => {
            let results = match feeds::repair_feeds(feeds_dir).await {
                Ok(r) => r,
                Err(e) => die(e),
            };
            if results.is_empty() {
                println!("No feed files found.");
                return;
            }
            print_repair_results(&results);
        }
    }
}

/// Reads the unread and bookmarked entry-id sets from LanceDB — the same source the UI uses
/// (`db.unread_ids()` / `db.bookmarked_ids()` via the read handle). Dies on db open/read error.
async fn read_state_sets(data_dir: &Path) -> (HashSet<String>, HashSet<String>) {
    let db = match EmbeddingsDb::open(data_dir).await {
        Ok(db) => db,
        Err(e) => die(e),
    };
    let unread = db.unread_ids().await.unwrap_or_else(|e| die(e));
    let bookmarked = db.bookmarked_ids().await.unwrap_or_else(|e| die(e));
    (unread, bookmarked)
}

/// Prints one line per repair result and a final count summary.
/// Shared by `feeds repair` and `migrate`.
pub(crate) fn print_repair_results(results: &[feeds::RepairResult]) {
    let mut n_ok = 0usize;
    let mut n_repaired = 0usize;
    let mut n_skipped = 0usize;
    for r in results {
        match &r.outcome {
            RepairOutcome::Ok => {
                n_ok += 1;
            }
            RepairOutcome::ParseError(msg) => {
                eprintln!("  err  [{}] parse error: {}", r.id, msg);
                n_skipped += 1;
            }
            RepairOutcome::NoId => {
                eprintln!("  skip [{}] no <id> element", r.id);
                n_skipped += 1;
            }
            RepairOutcome::NoRepairableUrl => {
                eprintln!(
                    "  skip [{}] no <link rel=\"self\"> and <id> is not an HTTP(S) URL",
                    r.id
                );
                n_skipped += 1;
            }
            RepairOutcome::RenameConflict { target } => {
                eprintln!(
                    "  skip [{}] rename conflict: target [{}] already exists",
                    r.id, target
                );
                n_skipped += 1;
            }
            RepairOutcome::Repaired { feed_id_synthesized, self_link_added, renamed_to, entry_links_normalized, entry_ids_synthesized } => {
                let mut parts = Vec::new();
                if *feed_id_synthesized {
                    parts.push("synthesised feed <id>".to_string());
                }
                if *self_link_added {
                    parts.push("added <link rel=\"self\">".to_string());
                }
                if *entry_links_normalized {
                    parts.push("normalised entry links".to_string());
                }
                if *entry_ids_synthesized {
                    parts.push("synthesised missing entry <id>s".to_string());
                }
                if let Some(new_id) = renamed_to {
                    parts.push(format!("renamed → [{}]", new_id));
                }
                println!("  fix  [{}] {}", r.id, parts.join(", "));
                n_repaired += 1;
            }
        }
    }
    println!();
    println!(
        "Done: {} ok, {} repaired, {} skipped.",
        n_ok, n_repaired, n_skipped
    );
}
