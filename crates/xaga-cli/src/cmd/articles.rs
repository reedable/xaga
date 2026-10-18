use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::persistence::articles;

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
    /// Restore absent summary fields from local Atom copies (no network)
    RepairSummaries,
    /// Print an article
    Read { id: String },
}

pub async fn handle(cmd: ArticlesCommand, data_dir: &Path, dry_run: bool) {
    match cmd {
        ArticlesCommand::RepairSummaries => {
            if !data_dir.is_dir() {
                die("library directory does not exist");
            }
            let lock = xaga_core::library::acquire_lock(data_dir)
                .unwrap_or_else(|| die("another process holds the library lock"));
            let writer =
                xaga_core::library_writer::LibraryWriter::start(data_dir.to_path_buf(), lock);
            let report =
                xaga_core::persistence::summary_repair::repair_summaries(&writer, dry_run).await;
            writer.shutdown().await;
            println!(
                "{}: {}; skipped (summary present): {}; unavailable locally: {}; errors: {}",
                if dry_run { "Would repair" } else { "Repaired" },
                if dry_run {
                    report.would_repair
                } else {
                    report.repaired
                },
                report.skipped,
                report.unavailable,
                report.errors.len()
            );
            for error in &report.errors {
                eprintln!("{error}");
            }
            if !report.errors.is_empty() {
                die("summary repair completed with errors; see above");
            }
        }

        ArticlesCommand::List => {
            let db = match EmbeddingsDb::open_readonly(data_dir).await {
                Ok(db) => db,
                Err(e) => die(e),
            };
            match db.list_entries().await {
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

        ArticlesCommand::Read { id } => match articles::read_article(data_dir, &id).await {
            Ok(doc) => {
                println!("url:      {}", doc.meta.url);
                println!("title:    {}", doc.meta.title);
                println!("created_at: {}", doc.meta.created_at);
                println!();
                print!("{}", doc.content);
            }
            Err(e) => die(e),
        },
    }
}
