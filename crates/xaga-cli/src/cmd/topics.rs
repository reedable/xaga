use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::persistence::topics;

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct TopicsArgs {
    #[command(subcommand)]
    pub subcommand: TopicsCommand,
}

#[derive(Subcommand)]
pub enum TopicsCommand {
    /// List all topics (id + title)
    List,
    /// Print a topic
    Read { id: String },
}

pub async fn handle(cmd: TopicsCommand, data_dir: &Path) {
    match cmd {
        TopicsCommand::List => match topics::list_topics(data_dir).await {
            Ok(ts) if ts.is_empty() => println!("No topics saved."),
            Ok(ts) => {
                for t in &ts {
                    println!("{}", t.id);
                    println!("  {}", t.title);
                }
            }
            Err(e) => die(e),
        },

        TopicsCommand::Read { id } => match topics::read_topic(data_dir, &id).await {
            Ok(t) => {
                println!("id:      {}", t.id);
                println!("title:   {}", t.title);
                println!("created: {}", t.created_at);
                println!();
                print!("{}", t.description.as_deref().unwrap_or(""));
            }
            Err(e) => die(e),
        },
    }
}
