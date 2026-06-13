//! One-time source-of-truth migration for the LanceDB-as-read-source roadmap.
//!
//! Moves topic membership out of each topic's `items` list and into the
//! `topics:` field of every member's own `.md` frontmatter — entries directly,
//! annotations within their parent article. Each member's `.md` becomes the
//! source of truth for "which topics am I in", which LanceDB then mirrors.
//!
//! Each affected file is loaded once and re-rendered once via `render_entry`, so
//! a file that is both a topic member and hosts member-annotations is written a
//! single time. Idempotent (re-adding an existing topic is a no-op).
//!
//! `--dry-run` writes nothing. Because the migration re-renders the whole file
//! (not a surgical insert), use `--diff` to see the exact before/after bytes;
//! scope it with `--id <ID>` (one file) or `--limit <N>` (a sample).
//!
//! Not yet covered (separate follow-up): migrating `settings.json` pins into
//! `.md` tags / `.atom` categories.

use std::collections::BTreeMap;
use std::path::Path;

use clap::Args;
use similar::TextDiff;
use xaga_core::entries::{find_entry_path, render_entry};
use xaga_core::types::CollectionItem;
use xaga_core::util::parse_document;

#[derive(Args)]
pub struct MigrateSotArgs {
    /// Write nothing; report what would change.
    #[arg(long)]
    pub dry_run: bool,
    /// Restrict to a single entry/article id (pair with --diff to inspect one file).
    #[arg(long)]
    pub id: Option<String>,
    /// Show a unified before/after diff for each changed file.
    #[arg(long)]
    pub diff: bool,
    /// Cap how many files' diffs are printed with --diff (0 = no cap).
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

/// Topic-membership additions destined for one entry/article file.
#[derive(Default)]
struct FileChanges {
    /// Topic ids to add to the entry's own `topics:` frontmatter.
    entry_topics: Vec<String>,
    /// annotation id -> topic ids to add to that annotation within this article.
    annotation_topics: BTreeMap<String, Vec<String>>,
}

pub async fn handle(args: MigrateSotArgs, data_dir: &Path) {
    let topics = match xaga_core::topics::list_topics(data_dir).await {
        Ok(t) => t,
        Err(e) => crate::die(e),
    };

    // Collect every membership keyed by the file it lands in, so each file is
    // loaded and rewritten exactly once.
    let mut plan: BTreeMap<String, FileChanges> = BTreeMap::new();
    for topic in &topics {
        for item in &topic.items {
            match item {
                CollectionItem::Entry { id } => {
                    plan.entry(id.clone())
                        .or_default()
                        .entry_topics
                        .push(topic.id.clone());
                }
                CollectionItem::Annotation { id, reference } => {
                    plan.entry(reference.clone())
                        .or_default()
                        .annotation_topics
                        .entry(id.clone())
                        .or_default()
                        .push(topic.id.clone());
                }
            }
        }
    }

    if let Some(only) = &args.id {
        plan.retain(|id, _| id == only);
        if plan.is_empty() {
            println!("no topic membership found for id {only}");
            return;
        }
    }

    let mut changed = 0usize;
    let mut diffs_shown = 0usize;
    for (file_id, changes) in &plan {
        let Some((path, old, new)) = plan_file(data_dir, file_id, changes).await else {
            continue;
        };
        changed += 1;

        if args.diff {
            if args.limit == 0 || diffs_shown < args.limit {
                println!("\n=== {file_id} ({}) ===", path.display());
                print!("{}", TextDiff::from_lines(old.as_str(), new.as_str()).unified_diff());
                diffs_shown += 1;
            }
        } else {
            summarize(file_id, changes);
        }

        if !args.dry_run {
            if let Err(e) = tokio::fs::write(&path, new.as_bytes()).await {
                eprintln!("error writing {}: {e}", path.display());
            }
        }
    }

    if args.diff && args.limit != 0 && changed > diffs_shown {
        println!(
            "\n… {} more changed file(s) not shown (raise --limit or use --id).",
            changed - diffs_shown
        );
    }
    if args.dry_run {
        println!("\n{changed} file(s) would change — dry run, nothing written.");
    } else {
        println!("\n{changed} file(s) updated.");
    }
}

/// Loads the file, applies the planned topic additions, and returns
/// `(path, old_content, new_content)` if anything actually changed.
async fn plan_file(
    data_dir: &Path,
    file_id: &str,
    changes: &FileChanges,
) -> Option<(std::path::PathBuf, String, String)> {
    let Some(path) = find_entry_path(data_dir, file_id).await else {
        eprintln!("skip: file not found for id {file_id}");
        return None;
    };
    let raw = match tokio::fs::read_to_string(&path).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skip: unreadable {}: {e}", path.display());
            return None;
        }
    };
    let Some((fm, body)) = parse_document(&raw) else {
        eprintln!("skip: bad frontmatter {}", path.display());
        return None;
    };

    let mut topics = fm.topics.clone();
    let mut changed = false;
    for t in &changes.entry_topics {
        if !topics.contains(t) {
            topics.push(t.clone());
            changed = true;
        }
    }

    let mut annotations = fm.annotations.clone();
    for ann in &mut annotations {
        if let Some(add) = changes.annotation_topics.get(&ann.id) {
            for t in add {
                if !ann.topics.contains(t) {
                    ann.topics.push(t.clone());
                    changed = true;
                }
            }
        }
    }

    if !changed {
        return None;
    }

    let new = render_entry(
        &fm.source,
        &fm.title,
        &fm.created_at,
        fm.published_at.as_deref(),
        fm.updated_at.as_deref(),
        fm.summary.as_deref(),
        &annotations,
        &fm.tags,
        &topics,
        fm.favicon.as_deref(),
        &body,
    );
    Some((path, raw, new))
}

fn summarize(file_id: &str, changes: &FileChanges) {
    if !changes.entry_topics.is_empty() {
        println!("entry {file_id}: +topics {:?}", changes.entry_topics);
    }
    for (ann_id, topics) in &changes.annotation_topics {
        println!("article {file_id} / annotation {ann_id}: +topics {topics:?}");
    }
}
