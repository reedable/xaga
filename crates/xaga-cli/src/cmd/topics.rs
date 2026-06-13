use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::Path;

use clap::{Args, Subcommand};
use serde::Deserialize;
use uuid::Uuid;
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::topics;
use xaga_core::types::CollectionItem;
use xaga_core::util::parse_frontmatter;

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
    /// Migrate collections/*.yaml to topics/*.md (one-time; remove after use)
    Migrate {
        /// Print what would be done without writing any files
        #[arg(long)]
        dry_run: bool,
    },
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

#[derive(Deserialize)]
struct CollectionFile {
    id: String,
    title: String,
    created: String,
    updated: String,
    #[serde(default)]
    items: Vec<CollectionItem>,
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

        TopicsCommand::Migrate { dry_run } => {
            migrate(data_dir, dry_run).await;
        }
    }
}

async fn migrate(data_dir: &Path, dry_run: bool) {
    if dry_run {
        eprintln!("note: dry-run mode — no files will be written");
    }

    // ── Step 1: build annotation_id → article_id map from entries/*.md ────────

    let mut annotation_map: HashMap<String, String> = HashMap::new();
    let entries_dir = data_dir.join("entries");

    if let Ok(mut dir) = tokio::fs::read_dir(&entries_dir).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let article_id = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            if let Ok(raw) = tokio::fs::read_to_string(&path).await {
                if let Some(fm) = parse_frontmatter(&raw) {
                    for ann in &fm.annotations {
                        annotation_map.insert(ann.id.clone(), article_id.clone());
                    }
                }
            }
        }
    }

    // ── Step 2: open LanceDB for fallback ─────────────────────────────────────

    let db = EmbeddingsDb::open(data_dir).await.ok();

    // ── Step 3: migrate each collection ──────────────────────────────────────

    let collections_dir = data_dir.join("collections");
    let mut dir = match tokio::fs::read_dir(&collections_dir).await {
        Ok(d) => d,
        Err(_) => {
            println!("No collections directory found — nothing to migrate.");
            return;
        }
    };

    let mut n_collections = 0usize;
    let mut n_entries = 0usize;
    let mut n_annotations = 0usize;
    let mut n_dropped = 0usize;
    let mut n_lancedb_deleted = 0usize;

    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }

        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warn: could not read {}: {e}", path.display());
                continue;
            }
        };

        let collection: CollectionFile = match serde_yaml::from_str(&raw) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("warn: could not parse {}: {e}", path.display());
                continue;
            }
        };

        // Determine target topic ID.
        let topic_path = data_dir.join("topics").join(format!("{}.md", collection.id));
        let target_id = if tokio::fs::try_exists(&topic_path).await.unwrap_or(false) {
            let new_id = Uuid::new_v4().to_string();
            eprintln!(
                "warn: topics/{}.md already exists — using new id {} for '{}'",
                collection.id, new_id, collection.title
            );
            new_id
        } else {
            collection.id.clone()
        };

        // Resolve items.
        let mut resolved_items: Vec<CollectionItem> = Vec::new();

        for item in &collection.items {
            match item {
                CollectionItem::Entry { id } => {
                    n_entries += 1;
                    resolved_items.push(CollectionItem::Entry { id: id.clone() });
                }
                CollectionItem::Annotation { id, .. } => {
                    if let Some(reference) = annotation_map.get(id) {
                        n_annotations += 1;
                        resolved_items.push(CollectionItem::Annotation {
                            id: id.clone(),
                            reference: reference.clone(),
                        });
                    } else {
                        // Annotation no longer exists in any article — drop it.
                        eprintln!(
                            "warn: annotation {id} in '{}' — no longer exists, dropped",
                            collection.title
                        );
                        n_dropped += 1;

                        // Clean up any stale LanceDB row.
                        if let Some(ref db) = db {
                            if let Ok(Some(_)) = db.get_entry(id).await {
                                if dry_run {
                                    eprintln!("  (would delete stale LanceDB row for annotation {id})");
                                } else if db.delete_by_id(id).await.is_ok() {
                                    n_lancedb_deleted += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        if dry_run {
            println!(
                "would migrate '{}' ({}) → topics/{}.md  [{} entries, {} annotations]",
                collection.title,
                collection.id,
                target_id,
                resolved_items.iter().filter(|i| matches!(i, CollectionItem::Entry { .. })).count(),
                resolved_items.iter().filter(|i| matches!(i, CollectionItem::Annotation { .. })).count(),
            );
        } else {
            match topics::save_topic_with_timestamps(
                data_dir,
                &target_id,
                &collection.title,
                "",
                None,
                resolved_items,
                &[],
                &collection.created,
                &collection.updated,
            )
            .await
            {
                Ok(_) => println!("migrated '{}' → topics/{}.md", collection.title, target_id),
                Err(e) => eprintln!("error: failed to write topic for '{}': {e}", collection.title),
            }
        }

        n_collections += 1;
    }

    // ── Step 4: summary ───────────────────────────────────────────────────────

    let verb = if dry_run { "Would migrate" } else { "Migrated" };
    println!();
    println!("{verb} {n_collections} collection(s).");
    println!("  Entries carried over:    {n_entries}");
    println!("  Annotations migrated:    {n_annotations}");
    println!("  Annotations dropped:     {n_dropped}  (stale LanceDB rows deleted: {n_lancedb_deleted})");
}
