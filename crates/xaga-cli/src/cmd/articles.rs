use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::articles;
use xaga_core::embeddings::EmbeddingsDb;

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
}

pub async fn handle(cmd: ArticlesCommand, data_dir: &Path) {
    match cmd {
        ArticlesCommand::List => {
            let db = match EmbeddingsDb::open(data_dir).await {
                Ok(db) => db,
                Err(e) => die(e),
            };
            match db.all_entries().await {
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
    }
}
