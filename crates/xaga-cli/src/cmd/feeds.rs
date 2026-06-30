use std::collections::HashSet;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::persistence::feeds;

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
}

pub async fn handle(cmd: FeedsCommand, data_dir: &PathBuf) {
    match cmd {
        FeedsCommand::List => {
            let (unread, bookmarked) = read_state_sets(data_dir).await;
            match feeds::list_feeds(data_dir, &unread, &bookmarked).await {
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
