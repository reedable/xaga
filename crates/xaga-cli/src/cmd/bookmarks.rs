use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::entries;

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
            match entries::list_entries(data_dir).await {
                Ok(items) => {
                    let bookmarks: Vec<_> = items.into_iter().filter(|e| e.is_bookmarked).collect();
                    if bookmarks.is_empty() {
                        println!("No bookmarks saved.");
                    } else {
                        for item in &bookmarks {
                            println!("{}", item.url);
                            println!("  {}", item.title);
                        }
                    }
                }
                Err(e) => die(e),
            }
        }

        BookmarksCommand::Add { url, title, summary } => {
            let now = chrono::Utc::now().to_rfc3339();
            match entries::write_read_entry(
                data_dir,
                &url,
                &title,
                &now,
                None,
                None,
                summary.as_deref(),
                &["bookmark".to_string()],
            )
            .await
            {
                Ok(id) => {
                    // If the Entry already existed, write_read_entry was a no-op.
                    // Ensure the bookmark tag is present regardless.
                    match entries::read_entry(data_dir, &id).await {
                        Ok(existing) if !existing.is_bookmarked => {
                            let mut tags = existing.tags;
                            tags.push("bookmark".to_string());
                            match entries::set_entry_tags(data_dir, &id, tags).await {
                                Ok(e) => println!("Bookmarked: {}", e.url),
                                Err(e) => die(e),
                            }
                        }
                        Ok(existing) => println!("Bookmarked: {}", existing.url),
                        Err(e) => die(e),
                    }
                }
                Err(e) => die(e),
            }
        }

        BookmarksCommand::Remove { id } => {
            match entries::read_entry(data_dir, &id).await {
                Ok(existing) => {
                    let tags: Vec<String> =
                        existing.tags.into_iter().filter(|t| t != "bookmark").collect();
                    match entries::set_entry_tags(data_dir, &id, tags).await {
                        Ok(_) => println!("Removed: {id}"),
                        Err(e) => die(e),
                    }
                }
                Err(e) => die(e),
            }
        }
    }
}
