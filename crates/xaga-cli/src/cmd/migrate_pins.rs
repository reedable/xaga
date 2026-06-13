//! One-time pins migration: moves pinned state out of the app's `settings.json`
//! (`pinned_feed_ids` / `pinned_topic_ids`) into the content files themselves —
//! a `pinned` tag on each pinned topic's `.md`, and a `<category term="pinned">`
//! on each pinned feed's `.atom`. After this, the content files own pinned state
//! and LanceDB mirrors it; `settings.json` is no longer its source of truth.
//!
//! Idempotent. `--dry-run` writes nothing; `--diff` shows topic before/after.

use std::path::{Path, PathBuf};

use clap::Args;
use serde::Deserialize;
use similar::TextDiff;
use xaga_core::feeds::{set_feed_pinned, FeedId};
use xaga_core::topics::render_topic;
use xaga_core::util::parse_document;

#[derive(Args)]
pub struct MigratePinsArgs {
    /// Path to the app's settings.json (holds pinned_feed_ids / pinned_topic_ids).
    #[arg(long)]
    pub settings: PathBuf,
    /// Write nothing; report what would change.
    #[arg(long)]
    pub dry_run: bool,
    /// Show a unified before/after diff for each changed topic file.
    #[arg(long)]
    pub diff: bool,
}

#[derive(Deserialize, Default)]
struct Pins {
    #[serde(default)]
    pinned_feed_ids: Vec<String>,
    #[serde(default)]
    pinned_topic_ids: Vec<String>,
}

pub async fn handle(args: MigratePinsArgs, data_dir: &Path, feeds_dir: &Path) {
    let raw = match std::fs::read_to_string(&args.settings) {
        Ok(r) => r,
        Err(e) => crate::die(format!("cannot read {}: {e}", args.settings.display())),
    };
    let pins: Pins = match serde_json::from_str(&raw) {
        Ok(p) => p,
        Err(e) => crate::die(format!("cannot parse {}: {e}", args.settings.display())),
    };

    let mut changed = 0usize;

    for topic_id in &pins.pinned_topic_ids {
        if pin_topic(data_dir, topic_id, args.dry_run, args.diff).await {
            changed += 1;
        }
    }

    for feed_id in &pins.pinned_feed_ids {
        match set_feed_pinned(feeds_dir, &FeedId(feed_id.clone()), true, args.dry_run).await {
            Ok(true) => {
                println!("feed {feed_id}: +pinned category");
                changed += 1;
            }
            Ok(false) => {} // already pinned
            Err(e) => eprintln!("feed {feed_id}: {e}"),
        }
    }

    if args.dry_run {
        println!("\n{changed} file(s) would change — dry run, nothing written.");
    } else {
        println!("\n{changed} file(s) updated.");
    }
}

/// Adds the `pinned` tag to a topic's `.md` frontmatter. Returns true if it changed.
async fn pin_topic(data_dir: &Path, topic_id: &str, dry_run: bool, diff: bool) -> bool {
    let path = data_dir.join("topics").join(format!("{topic_id}.md"));
    let Ok(raw) = tokio::fs::read_to_string(&path).await else {
        eprintln!("topic {topic_id}: file not found");
        return false;
    };
    let Some((fm, body)) = parse_document(&raw) else {
        eprintln!("topic {topic_id}: bad frontmatter");
        return false;
    };
    if fm.tags.iter().any(|t| t == "pinned") {
        return false; // already pinned
    }

    let mut tags = fm.tags.clone();
    tags.push("pinned".to_string());
    let updated_at = fm.updated_at.clone().unwrap_or_else(|| fm.created_at.clone());
    let new = render_topic(
        topic_id,
        &fm.title,
        &fm.created_at,
        &updated_at,
        fm.threshold,
        &fm.items,
        &tags,
        &body,
    );

    if diff {
        println!("\n=== topic {topic_id} ({}) ===", path.display());
        print!("{}", TextDiff::from_lines(raw.as_str(), new.as_str()).unified_diff());
    } else {
        println!("topic {topic_id}: +pinned tag");
    }

    if !dry_run {
        if let Err(e) = tokio::fs::write(&path, new.as_bytes()).await {
            eprintln!("error writing {}: {e}", path.display());
        }
    }
    true
}
