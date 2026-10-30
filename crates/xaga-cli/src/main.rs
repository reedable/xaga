mod cmd;
mod settings;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "xaga-cli",
    about = "Headless Xaga PKM tool",
    version,
    arg_required_else_help = true
)]
struct Cli {
    /// Override data directory (also: $XAGA_DATA_DIR, or set via `config set data-dir`)
    #[arg(long, env = "XAGA_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Report summary repairs without writing changes
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage RSS/Atom feeds
    Feeds(cmd::feeds::FeedsArgs),
    /// Read saved articles
    Articles(cmd::articles::ArticlesArgs),
    /// Manage topics
    Topics(cmd::topics::TopicsArgs),
    /// Manage bookmarks
    Bookmarks(cmd::bookmarks::BookmarksArgs),
    /// Query the embeddings index
    Embeddings(cmd::embeddings::EmbeddingsArgs),
    /// Garbage-collect stored entry images
    Images(cmd::images::ImagesArgs),
    /// Manage the library storage format
    Library(cmd::library::LibraryArgs),
    /// Read or write ~/.xagarc
    Config(cmd::config::ConfigArgs),
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.dry_run
        && !matches!(
            &cli.command,
            Command::Articles(cmd::articles::ArticlesArgs {
                subcommand: cmd::articles::ArticlesCommand::RepairSummaries,
            }) | Command::Images(_)
        )
    {
        die("--dry-run is supported for articles repair-summaries and images gc");
    }
    let rc = settings::load();
    let data_dir = settings::resolve_data_dir(cli.data_dir, &rc);

    match cli.command {
        Command::Feeds(args) => cmd::feeds::handle(args.subcommand, &data_dir).await,
        Command::Articles(args) => {
            cmd::articles::handle(args.subcommand, &data_dir, cli.dry_run).await
        }
        Command::Topics(args) => cmd::topics::handle(args.subcommand, &data_dir).await,
        Command::Bookmarks(args) => cmd::bookmarks::handle(args.subcommand, &data_dir).await,
        Command::Embeddings(args) => cmd::embeddings::handle(args.subcommand, &data_dir).await,
        Command::Images(mut args) => {
            let cmd::images::ImagesCommand::Gc { dry_run } = &mut args.subcommand;
            *dry_run |= cli.dry_run;
            cmd::images::handle(args.subcommand, &data_dir).await;
        }
        Command::Library(args) => cmd::library::handle(args.subcommand, &data_dir).await,
        Command::Config(args) => cmd::config::handle(args.subcommand, &rc, &data_dir),
    }
}

pub(crate) fn die(e: impl std::fmt::Display) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1);
}
