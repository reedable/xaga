use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::bookmarks;
use xaga_core::types::Entry;

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
    /// Add or update a bookmark
    Add {
        url: String,
        title: String,
        #[arg(long)]
        summary: Option<String>,
    },
    /// Remove a bookmark by ID
    Remove { id: String },
}

pub async fn handle(cmd: BookmarksCommand, data_dir: &Path) {
    match cmd {
        BookmarksCommand::List => {
            match bookmarks::list_bookmarks(data_dir).await {
                Ok(items) if items.is_empty() => println!("No bookmarks saved."),
                Ok(items) => {
                    for item in &items {
                        println!("{}", item.url);
                        println!("  {}", item.title);
                    }
                }
                Err(e) => die(e),
            }
        }

        BookmarksCommand::Add { url, title, summary } => {
            let now = chrono::Utc::now().to_rfc3339();
            let normalized = match xaga_core::util::normalize_url(&url) {
                Ok(n) => n,
                Err(e) => die(e),
            };
            let entry = Entry {
                id: xaga_core::util::url_hash12(&normalized),
                url: normalized,
                title,
                created_at: now,
                published_at: None,
                updated_at: None,
                summary,
                is_unread: false,
            };
            match bookmarks::add_bookmark(data_dir, &entry).await {
                Ok(stored) => println!("Bookmarked: {}", stored.url),
                Err(e) => die(e),
            }
        }

        BookmarksCommand::Remove { id } => {
            match bookmarks::remove_bookmark(data_dir, &id).await {
                Ok(_) => println!("Removed: {id}"),
                Err(e) => die(e),
            }
        }
    }
}
