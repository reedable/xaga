use std::path::PathBuf;

use clap::{Args, Subcommand};
use xaga_core::feeds::{self, FeedId, RepairOutcome};

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct FeedsArgs {
    #[command(subcommand)]
    pub subcommand: FeedsCommand,
}

#[derive(Subcommand)]
pub enum FeedsCommand {
    /// Fetch and save a new feed
    Add { url: String },
    /// Remove a saved feed by ID
    Remove { id: FeedId },
    /// List all saved feeds
    List,
    /// Re-fetch one feed (or all if no ID given)
    Refresh { id: Option<FeedId> },
    /// Repair feed files: add missing <link rel="self"> and rename files whose
    /// hash no longer matches their feed URL
    Repair,
}

pub async fn handle(cmd: FeedsCommand, client: &reqwest::Client, feeds_dir: &PathBuf, data_dir: &PathBuf) {
    match cmd {
        FeedsCommand::Add { url } => {
            let (unread, bookmarked) = tokio::join!(
                xaga_core::filter::unread_ids(data_dir),
                xaga_core::filter::bookmarked_ids(data_dir),
            );
            eprint!("Fetching {url} ... ");
            match feeds::add_feed(client, feeds_dir, data_dir, &url, &unread, &bookmarked).await {
                Ok(feed) => {
                    eprintln!("done");
                    println!("[{}] {}", feed.id, feed.title);
                }
                Err(e) => die(e),
            }
        }

        FeedsCommand::Remove { id } => match feeds::remove_feed(feeds_dir, &id).await {
            Ok(()) => println!("Removed: {id}"),
            Err(e) => die(e),
        },

        FeedsCommand::List => {
            let (unread, bookmarked) = tokio::join!(
                xaga_core::filter::unread_ids(data_dir),
                xaga_core::filter::bookmarked_ids(data_dir),
            );
            match feeds::list_feeds(feeds_dir, &unread, &bookmarked).await {
                Ok(feeds) if feeds.is_empty() => {
                    println!("No feeds saved. Use `feeds add <url>` to add one.");
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

        FeedsCommand::Refresh { id: Some(id) } => {
            let (unread, bookmarked) = tokio::join!(
                xaga_core::filter::unread_ids(data_dir),
                xaga_core::filter::bookmarked_ids(data_dir),
            );
            eprint!("Refreshing {id} ... ");
            match feeds::refresh_feed(client, feeds_dir, data_dir, &id, &unread, &bookmarked).await {
                Ok(feed) => {
                    eprintln!("done");
                    println!("[{}] {}", feed.id, feed.title);
                }
                Err(e) => die(e),
            }
        }

        FeedsCommand::Refresh { id: None } => {
            let (unread, bookmarked) = tokio::join!(
                xaga_core::filter::unread_ids(data_dir),
                xaga_core::filter::bookmarked_ids(data_dir),
            );
            let results = feeds::refresh_all_feeds(client, feeds_dir, data_dir, &unread, &bookmarked).await;
            if results.is_empty() {
                println!("No feeds saved. Use `feeds add <url>` to add one.");
                return;
            }
            let mut had_error = false;
            for (id, result) in &results {
                match result {
                    Ok(feed) => println!("  ok  [{}] {}", id, feed.title),
                    Err(e) => {
                        eprintln!("  err [{id}] {e}");
                        had_error = true;
                    }
                }
            }
            if had_error {
                std::process::exit(1);
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
