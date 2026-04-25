use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::articles;
use xaga_core::entries;

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct ArticlesArgs {
    #[command(subcommand)]
    pub subcommand: ArticlesCommand,
}

#[derive(Subcommand)]
pub enum ArticlesCommand {
    /// List all saved articles (title + URL)
    List,
    /// Print an article
    Read { id: String },
    /// Mark an item as read (move from unread/ to read/)
    MarkRead { id: String },
    /// Mark an item as unread (move from read/ to unread/)
    MarkUnread { id: String },
}

pub async fn handle(cmd: ArticlesCommand, data_dir: &Path) {
    match cmd {
        ArticlesCommand::List => {
            match entries::list_entries(data_dir, &xaga_core::filter::unread_ids(data_dir).await).await {
                Ok(docs) if docs.is_empty() => {
                    println!("No articles saved.");
                }
                Ok(docs) => {
                    for doc in &docs {
                        println!("{}", doc.url);
                        println!("  {}", doc.title);
                    }
                }
                Err(e) => die(e),
            }
        }

        ArticlesCommand::Read { id } => {
            match articles::read_article(data_dir, &id).await {
                Ok(doc) => {
                    println!("url:      {}", doc.meta.url);
                    println!("title:    {}", doc.meta.title);
                    println!("created_at: {}", doc.meta.created_at);
                    println!();
                    print!("{}", doc.content);
                }
                Err(e) => die(e),
            }
        }

        ArticlesCommand::MarkRead { id } => {
            match entries::mark_entry_read(data_dir, &id).await {
                Ok(_) => println!("Marked as read."),
                Err(e) => die(e),
            }
        }

        ArticlesCommand::MarkUnread { id } => {
            match entries::mark_entry_unread(data_dir, &id).await {
                Ok(_) => println!("Marked as unread."),
                Err(e) => die(e),
            }
        }
    }
}
