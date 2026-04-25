use std::io::{self, BufRead, Write};
use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::topics;

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct TopicsArgs {
    #[command(subcommand)]
    pub subcommand: TopicsCommand,
}

#[derive(Subcommand)]
pub enum TopicsCommand {
    /// Create a new topic interactively
    Add,
    /// List all topics (id + title)
    List,
    /// Print a topic
    Read { id: String },
}


fn prompt(label: &str) -> String {
    print!("{label}");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().lock().read_line(&mut buf).unwrap_or(0);
    buf.trim_end_matches('\n').trim_end_matches('\r').to_string()
}

/// Reads lines from stdin until EOF (Ctrl-D). Returns them joined with newlines.
fn read_until_eof(label: &str) -> String {
    eprintln!("{label}");
    io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn handle(cmd: TopicsCommand, data_dir: &Path) {
    match cmd {
        TopicsCommand::Add => {
            let title = prompt("Title: ");
            if title.is_empty() {
                die("title cannot be empty");
            }

            let content = read_until_eof("Content (end with Ctrl-D):");

            match topics::create_topic(data_dir, &title, &content, None).await {
                Ok(t) => println!("Saved topic '{}'", t.id),
                Err(e) => die(e),
            }
        }

        TopicsCommand::List => {
            match topics::list_topics(data_dir).await {
                Ok(ts) if ts.is_empty() => println!("No topics saved."),
                Ok(ts) => {
                    for t in &ts {
                        println!("{}", t.id);
                        println!("  {}", t.title);
                    }
                }
                Err(e) => die(e),
            }
        }

        TopicsCommand::Read { id } => {
            match topics::read_topic(data_dir, &id).await {
                Ok(t) => {
                    println!("id:      {}", t.id);
                    println!("title:   {}", t.title);
                    println!("created: {}", t.created_at);
                    println!();
                    print!("{}", t.content);
                }
                Err(e) => die(e),
            }
        }
    }
}
