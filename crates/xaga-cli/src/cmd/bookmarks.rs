use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::embeddings::EmbeddingsDb;

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct BookmarksArgs {
    #[command(subcommand)]
    pub subcommand: BookmarksCommand,
}

#[derive(Subcommand)]
pub enum BookmarksCommand {
    /// List all bookmarks (title + URL)
    List,
}

pub async fn handle(cmd: BookmarksCommand, data_dir: &Path) {
    match cmd {
        BookmarksCommand::List => {
            let db = match EmbeddingsDb::open(data_dir).await {
                Ok(db) => db,
                Err(e) => die(e),
            };
            match db.filter_bookmarked().await {
                Ok(bookmarks) if bookmarks.is_empty() => {
                    println!("No bookmarks saved.");
                }
                Ok(bookmarks) => {
                    for item in &bookmarks {
                        println!("{}", item.url);
                        println!("  {}", item.title);
                    }
                }
                Err(e) => die(e),
            }
        }
    }
}
