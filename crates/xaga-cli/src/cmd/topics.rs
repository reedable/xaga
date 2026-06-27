use std::collections::HashMap;
use std::path::Path;

use clap::{Args, Subcommand};
use serde::Deserialize;
use uuid::Uuid;
use xaga_core::embeddings::EmbeddingsDb;
use xaga_core::fm::parse_frontmatter;
use xaga_core::md::{Markdown, md_to_topic, parse_document};
use xaga_core::persistence::topics;
use xaga_core::types::CollectionItem;
use xaga_core::util::item_id;

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
    /// Migrate collections/*.yaml to topics/*.md (one-time; remove after use)
    Migrate {
        /// Print what would be done without writing any files
        #[arg(long)]
        dry_run: bool,
    },
    /// Repair topic memberships where an annotation was mis-recorded as an entry.
    /// Rewrites `{type: entry, id: <annotationId>}` -> `{type: annotation, id, reference}`.
    RepairAnnotations {
        /// Print what would be done without writing any files
        #[arg(long)]
        dry_run: bool,
    },
    /// Move each topic's blurb from the `.md` body into the `summary:` frontmatter key
    /// (one-time; remove after use). Timestamps and the embedding checksum are preserved,
    /// so migrated topics are not re-embedded.
    MigrateSummary {
        /// Print what would be done without writing any files
        #[arg(long)]
        dry_run: bool,
    },
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

        TopicsCommand::Migrate { dry_run } => {
            migrate(data_dir, dry_run).await;
        }

        TopicsCommand::RepairAnnotations { dry_run } => {
            repair_annotations(data_dir, dry_run).await;
        }

        TopicsCommand::MigrateSummary { dry_run } => {
            migrate_summary(data_dir, dry_run).await;
        }
    }
}

/// Rewrites every `topics/*.md` whose blurb still lives in the body, moving it into the
/// `summary:` frontmatter key. `embed_text` (= `summary` or the body) is byte-identical before
/// and after, so `content_checksum` is unchanged and the watcher will not re-embed the topic.
async fn migrate_summary(data_dir: &Path, dry_run: bool) {
    let topics_dir = xaga_core::library::topics_dir(data_dir);
    let mut dir_entries = match tokio::fs::read_dir(&topics_dir).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("warn: could not read {}: {e}", topics_dir.display());
            return;
        }
    };

    let mut n_scanned = 0usize;
    let mut n_migrated = 0usize;

    loop {
        let dir_entry = match dir_entries.next_entry().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(e) => {
                eprintln!("warn: error reading topics dir: {e}");
                break;
            }
        };
        let path = dir_entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        n_scanned += 1;

        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warn: could not read {}: {e}", path.display());
                continue;
            }
        };
        let (fm, body) = match parse_document(&raw) {
            Some(parsed) => parsed,
            None => {
                eprintln!("warn: could not parse {}", path.display());
                continue;
            }
        };

        // Already migrated (summary present) or nothing to move (no body blurb).
        if fm.summary.is_some() || body.is_empty() {
            continue;
        }

        let id = item_id(&path);
        if dry_run {
            println!("would migrate topics/{id}.md  (move body blurb -> summary frontmatter)");
            n_migrated += 1;
            continue;
        }

        // Preserve timestamps — moving storage location is not a content edit.
        let updated_at = fm
            .updated_at
            .clone()
            .unwrap_or_else(|| fm.created_at.clone());

        let items = fm.items.clone();

        let mut topic = md_to_topic(Markdown {
            id: id.clone(),
            fm,
            body,
        });
        topic.updated_at = Some(updated_at);

        let rendered = topics::render_topic(&topic, &items);

        match tokio::fs::write(&path, rendered.as_bytes()).await {
            Ok(_) => {
                println!("migrated topics/{id}.md");
                n_migrated += 1;
            }
            Err(e) => eprintln!("error: failed to write {}: {e}", path.display()),
        }
    }

    if dry_run {
        println!("\n{n_migrated} of {n_scanned} topic file(s) would be migrated.");
    } else {
        println!("\nMigrated {n_migrated} of {n_scanned} topic file(s).");
    }
}

/// One-time repair. An annotation added to a topic at creation-time — before the new-annotation
/// object carried its `__kind` brand — was mis-recorded in the topic `.md` as
/// `{type: entry, id: <annotationId>}` with no `reference`. `list_topic_items` then silently
/// drops it (it tries to read an *entry* file by the annotation id, which doesn't exist), so the
/// annotation never renders in the topic even though it still counts toward membership.
///
/// This rewrites each such item to `{type: annotation, id, reference}`, recovering `reference`
/// (the parent article id) from the article frontmatter. The discriminator is reliable: only an
/// `Entry` item whose id is a known annotation id is rewritten — real entry ids never appear in
/// the annotation map. LanceDB is untouched (its reverse index is already keyed by the correct
/// id; only the on-disk `type` was wrong). Idempotent.
async fn repair_annotations(data_dir: &Path, dry_run: bool) {
    if dry_run {
        eprintln!("note: dry-run mode — no files will be written");
    }

    // Build annotation id -> parent article id from entries/*.md frontmatter.
    let mut annotation_map: HashMap<String, String> = HashMap::new();
    let entries_dir = xaga_core::library::entries_dir(data_dir);
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

    let topics_list = match topics::list_topics(data_dir).await {
        Ok(ts) => ts,
        Err(e) => die(e),
    };

    let mut n_topics_scanned = 0usize;
    let mut n_topics_changed = 0usize;
    let mut n_repaired = 0usize;

    for topic in &topics_list {
        n_topics_scanned += 1;
        let path = xaga_core::library::topic_path(data_dir, &topic.id);
        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warn: could not read {}: {e}", path.display());
                continue;
            }
        };
        let (fm, body) = match parse_document(&raw) {
            Some(parsed) => parsed,
            None => {
                eprintln!("warn: could not parse {}", path.display());
                continue;
            }
        };

        let mut changed = 0usize;
        let new_items: Vec<CollectionItem> = fm
            .items
            .iter()
            .map(|item| match item {
                CollectionItem::Entry { id } if annotation_map.contains_key(id) => {
                    changed += 1;
                    let reference = annotation_map[id].clone();
                    if dry_run {
                        eprintln!(
                            "  {}: entry {id} -> annotation (reference {reference})",
                            topic.id
                        );
                    }
                    CollectionItem::Annotation {
                        id: id.clone(),
                        reference,
                    }
                }
                other => other.clone(),
            })
            .collect();

        if changed == 0 {
            continue;
        }

        n_topics_changed += 1;
        n_repaired += changed;

        if dry_run {
            println!(
                "would repair '{}' ({}): {changed} item(s)",
                topic.title, topic.id
            );
            continue;
        }

        // Preserve timestamps — a membership-type correction is not a content edit (mirrors
        // set_topic_pinned). The member ids are unchanged, so the watcher's reconcile is a no-op.
        let updated_at = fm
            .updated_at
            .clone()
            .unwrap_or_else(|| fm.created_at.clone());
        let mut rebuilt = md_to_topic(Markdown {
            id: topic.id.clone(),
            fm,
            body,
        });
        rebuilt.updated_at = Some(updated_at);
        let rendered = topics::render_topic(&rebuilt, &new_items);
        match tokio::fs::write(&path, rendered.as_bytes()).await {
            Ok(_) => println!(
                "repaired '{}' ({}): {changed} item(s)",
                topic.title, topic.id
            ),
            Err(e) => eprintln!("error: failed to write {}: {e}", path.display()),
        }
    }

    let verb = if dry_run { "Would repair" } else { "Repaired" };
    println!();
    println!("Scanned {n_topics_scanned} topic(s).");
    println!(
        "{verb} {n_repaired} mis-recorded annotation item(s) across {n_topics_changed} topic(s)."
    );
}

async fn migrate(data_dir: &Path, dry_run: bool) {
    if dry_run {
        eprintln!("note: dry-run mode — no files will be written");
    }

    // ── Step 1: build annotation_id → article_id map from entries/*.md ────────

    let mut annotation_map: HashMap<String, String> = HashMap::new();
    let entries_dir = xaga_core::library::entries_dir(data_dir);

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
        let topic_path = xaga_core::library::topic_path(data_dir, &collection.id);
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
                                    eprintln!(
                                        "  (would delete stale LanceDB row for annotation {id})"
                                    );
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
                resolved_items
                    .iter()
                    .filter(|i| matches!(i, CollectionItem::Entry { .. }))
                    .count(),
                resolved_items
                    .iter()
                    .filter(|i| matches!(i, CollectionItem::Annotation { .. }))
                    .count(),
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
                Err(e) => eprintln!(
                    "error: failed to write topic for '{}': {e}",
                    collection.title
                ),
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
    println!(
        "  Annotations dropped:     {n_dropped}  (stale LanceDB rows deleted: {n_lancedb_deleted})"
    );
}
