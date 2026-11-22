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
            let unread = read_unread_ids(data_dir).await;
            match feeds::list_feeds(data_dir, &unread).await {
                Ok(feeds) if feeds.is_empty() => {
                    println!("No feeds saved.");
                }
                Ok(feeds) => {
                    for feed in &feeds {
                        let updated = feed.updated_at.as_deref().unwrap_or("unknown");
                        println!(
                            "[{}] {} ({} entries)",
                            feed.id, feed.title, feed.entry_count
                        );
                        println!("  url:     {}", feed.url);
                        println!("  updated: {updated}");
                    }
                }
                Err(e) => die(e),
            }
        }
    }
}

/// Reads unread entry IDs from LanceDB. Dies on database open or read error.
async fn read_unread_ids(data_dir: &Path) -> HashSet<String> {
    let db = match EmbeddingsDb::open_readonly(data_dir).await {
        Ok(db) => db,
        Err(e) => die(e),
    };
    db.unread_ids().await.unwrap_or_else(|e| die(e))
}
