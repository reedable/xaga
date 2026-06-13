mod cmd;
mod settings;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xaga-cli", about = "Headless Xaga PKM tool", version, arg_required_else_help = true)]
struct Cli {
    /// Override data directory (also: $XAGA_DATA_DIR, or set via `config set data-dir`)
    #[arg(long, env = "XAGA_DATA_DIR")]
    data_dir: Option<PathBuf>,

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
    /// Re-key library files after the normalize_url change (one-time; remove after use)
    Migrate(cmd::migrate::MigrateArgs),
    /// Move topic membership into each member's `.md` frontmatter (one-time; use --dry-run first)
    MigrateSot(cmd::migrate_sot::MigrateSotArgs),
    /// Move pins from settings.json into topic `.md` tags + feed `.atom` categories (one-time)
    MigratePins(cmd::migrate_pins::MigratePinsArgs),
    /// Read or write ~/.xagarc
    Config(cmd::config::ConfigArgs),
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let rc = settings::load();
    let data_dir = settings::resolve_data_dir(cli.data_dir, &rc);
    let feeds_dir = data_dir.join("feeds");

    let client = reqwest::Client::builder()
        .user_agent(concat!("xaga-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build HTTP client");

    match cli.command {
        Command::Feeds(args) => cmd::feeds::handle(args.subcommand, &client, &feeds_dir, &data_dir).await,
        Command::Articles(args) => cmd::articles::handle(args.subcommand, &data_dir).await,
        Command::Topics(args) => cmd::topics::handle(args.subcommand, &data_dir).await,
        Command::Bookmarks(args) => cmd::bookmarks::handle(args.subcommand, &data_dir).await,
        Command::Embeddings(args) => cmd::embeddings::handle(args.subcommand, &data_dir).await,
        Command::Migrate(args) => cmd::migrate::handle(args, &feeds_dir, &data_dir).await,
        Command::MigrateSot(args) => cmd::migrate_sot::handle(args, &data_dir).await,
        Command::MigratePins(args) => cmd::migrate_pins::handle(args, &data_dir, &feeds_dir).await,
        Command::Config(args) => cmd::config::handle(args.subcommand, &rc, &data_dir),
    }
}

pub(crate) fn die(e: impl std::fmt::Display) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1);
}
