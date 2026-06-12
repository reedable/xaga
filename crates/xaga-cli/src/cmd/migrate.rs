use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use clap::Args;
use xaga_core::entries::render_entry;
use xaga_core::feeds;
use xaga_core::topics;
use xaga_core::types::CollectionItem;
use xaga_core::util::{item_id, normalize_url, parse_document, url_hash12, Frontmatter};

/// Re-key the library after the `normalize_url` rule change (one-time; remove after use).
///
/// `normalize_url` now also strips the fragment, rewrites http→https, and strips
/// a leading `www.`, so `url_hash12` IDs derived from it no longer match files
/// saved under the old rules. This command:
///
///  1. renames `entries/{old}.md` → `entries/{new}.md`, merging entries whose
///     URLs now normalize to the same ID (longest body wins; tags and
///     annotations are unioned) and rebuilding unparseable legacy frontmatter,
///  2. rewrites entry IDs and annotation references in `topics/*.md`,
///  3. runs feed repair, which renames `feeds/*.atom` to the new hashes,
///  4. deletes `.entry_state_cache.json` so the app reseeds it with a full scan.
///
/// The embeddings index is deliberately not touched: the reconciler (app
/// startup, or `xaga-cli embeddings recover`) deletes rows whose files are
/// gone and re-embeds files it doesn't know — which is exactly the state this
/// migration leaves behind.
#[derive(Args)]
pub struct MigrateArgs {
    /// Print what would be done without writing any files
    #[arg(long)]
    pub dry_run: bool,
}

struct EntryFile {
    old_id: String,
    path: PathBuf,
    raw: String,
    fm: Frontmatter,
    body: String,
    /// True when `raw` was rebuilt from an unparseable legacy-template file
    /// (see `reparse_legacy_template`).
    repaired: bool,
}

/// The exact key set the pre-serde `render_entry` template (replaced in
/// commit f744989) could emit. Files written by it are not valid YAML when a
/// value contains quotes, `": "`, or newlines, so they cannot be parsed —
/// but they can be read back *as the template*: fixed keys at column 0,
/// values verbatim.
const LEGACY_SCALAR_KEYS: &[&str] = &["source", "title", "created", "published", "updated", "summary", "favicon"];
const LEGACY_LIST_KEYS: &[&str] = &["aliases", "tags"];

enum LegacyCtx {
    Scalar(&'static str),
    List(&'static str),
}

/// Reads a document the legacy template wrote and rebuilds it through serde.
///
/// A field's value is everything from after `key:` up to the next known key
/// at column 0 — extracted verbatim, never YAML-parsed, so embedded quotes
/// and colons cannot break it; raw newlines are folded to spaces. `aliases`
/// is discarded (rebuilt from the title), `tags` items are preserved. Any
/// unknown top-level key (e.g. `annotations`, which only the serde writer
/// emits) means the file is not the legacy template — bail rather than
/// guess. The caller keeps the result only if the rebuilt document parses.
fn reparse_legacy_template(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start();
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let yaml = &rest[..end];
    let body = rest[end + 5..].trim_matches('\n');

    let mut fields: HashMap<&'static str, String> = HashMap::new();
    let mut tags: Vec<String> = Vec::new();
    let mut ctx: Option<LegacyCtx> = None;

    for line in yaml.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');

        if !indented {
            let colon = line.find(':');
            let key = colon.map(|c| &line[..c]);
            if let Some(&k) = key.and_then(|key| LEGACY_SCALAR_KEYS.iter().find(|&&k| k == key)) {
                fields.insert(k, line[k.len() + 1..].trim().to_string());
                ctx = Some(LegacyCtx::Scalar(k));
                continue;
            }
            if let Some(&k) = key.and_then(|key| LEGACY_LIST_KEYS.iter().find(|&&k| k == key)) {
                if !line[k.len() + 1..].trim().is_empty() {
                    return None; // inline list — not the legacy template
                }
                ctx = Some(LegacyCtx::List(k));
                continue;
            }
            // `annotations` is only ever emitted by the serde writer, so this
            // is not a legacy-template file.
            if key == Some("annotations") {
                return None;
            }
            // Any other column-0 line — even one containing a colon — can
            // only be a continuation of the open value: the legacy template
            // has no fields beyond the known keys. With no value open, the
            // file is not the legacy template.
            if ctx.is_none() {
                return None;
            }
        }

        match &ctx {
            Some(LegacyCtx::Scalar(k)) => {
                let value = fields.get_mut(k).expect("scalar ctx implies entry");
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(line.trim());
            }
            Some(LegacyCtx::List(k)) => {
                if *k == "tags" {
                    if let Some(item) = line.trim_start().strip_prefix("- ") {
                        tags.push(item.trim().to_string());
                    }
                }
                // aliases items (and their continuation lines) are discarded:
                // render_entry rebuilds aliases from the title.
            }
            None => return None,
        }
    }

    let non_empty = |k: &str| fields.get(k).map(|v| v.trim()).filter(|v| !v.is_empty()).map(str::to_string);
    let source = non_empty("source")?;
    let title = non_empty("title")?;
    let created = non_empty("created")?;
    let published = non_empty("published");
    let updated = non_empty("updated");
    let summary = non_empty("summary");
    let favicon = non_empty("favicon");

    Some(render_entry(
        &source,
        &title,
        &created,
        published.as_deref(),
        updated.as_deref(),
        summary.as_deref(),
        &[],
        &tags,
        favicon.as_deref(),
        body,
    ))
}

pub async fn handle(args: MigrateArgs, feeds_dir: &Path, data_dir: &Path) {
    let dry_run = args.dry_run;
    if dry_run {
        eprintln!("note: dry-run mode — no files will be written");
    }

    // ── Step 1: scan entries/ and group files by their re-keyed id ───────────

    let entries_dir = data_dir.join("entries");
    let mut groups: HashMap<String, Vec<EntryFile>> = HashMap::new();

    match tokio::fs::read_dir(&entries_dir).await {
        Ok(mut dir) => {
            while let Ok(Some(dirent)) = dir.next_entry().await {
                let path = dirent.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let old_id = item_id(&path);
                let raw = match tokio::fs::read_to_string(&path).await {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("warn: could not read {}: {e}", path.display());
                        continue;
                    }
                };
                let (raw, fm, body, repaired) = match parse_document(&raw) {
                    Some((fm, body)) => (raw, fm, body, false),
                    None => {
                        let repaired_raw = reparse_legacy_template(&raw)
                            .and_then(|fixed| parse_document(&fixed).map(|d| (fixed, d)));
                        match repaired_raw {
                            Some((fixed, (fm, body))) => (fixed, fm, body, true),
                            None => {
                                eprintln!("warn: could not parse {} — skipped", path.display());
                                continue;
                            }
                        }
                    }
                };
                if fm.source.is_empty() {
                    eprintln!("warn: {} has no source URL — skipped", path.display());
                    continue;
                }
                let new_id = match normalize_url(&fm.source) {
                    Ok(n) => url_hash12(&n),
                    Err(e) => {
                        eprintln!("warn: {} has an invalid source URL ({e}) — skipped", path.display());
                        continue;
                    }
                };
                groups.entry(new_id).or_default().push(EntryFile { old_id, path, raw, fm, body, repaired });
            }
        }
        Err(_) => println!("No entries directory found."),
    }

    // ── Step 2: plan renames and merges ──────────────────────────────────────

    // Every path that belongs to some group. A file on disk at a target path
    // that is NOT in this set was skipped above (unparseable) — writing over
    // it would destroy data we could not read, so its group is skipped too.
    let group_paths: HashSet<PathBuf> = groups.values().flatten().map(|f| f.path.clone()).collect();

    let mut new_ids: Vec<String> = groups.keys().cloned().collect();
    new_ids.sort();

    // Every (old id → new id) move, for rewriting references in topics.
    let mut rekeys: Vec<(String, String)> = Vec::new();
    // Deletions happen before writes: a group's target path may currently hold
    // another group's old file, whose content is already in memory.
    let mut to_delete: Vec<PathBuf> = Vec::new();
    let mut to_write: Vec<(PathBuf, String)> = Vec::new();

    let mut n_unchanged = 0usize;
    let mut n_renamed = 0usize;
    let mut n_merged_groups = 0usize;
    let mut n_merged_files = 0usize;
    let mut n_repaired = 0usize;

    for new_id in &new_ids {
        let mut files = groups.remove(new_id).unwrap();
        let target = entries_dir.join(format!("{new_id}.md"));
        n_repaired += files.iter().filter(|f| f.repaired).count();

        if files.len() == 1 && files[0].old_id == *new_id {
            // Re-keying is a no-op, but a repaired file is rewritten in place
            // so the app can read it again.
            if files[0].repaired {
                let f = files.pop().unwrap();
                println!("repair [{}]  {}", f.old_id, f.fm.source);
                to_write.push((f.path, f.raw));
            } else {
                n_unchanged += 1;
            }
            continue;
        }

        if !group_paths.contains(&target)
            && tokio::fs::try_exists(&target).await.unwrap_or(false)
        {
            eprintln!(
                "warn: target {} exists but could not be parsed — group ({}) skipped",
                target.display(),
                files.iter().map(|f| f.old_id.as_str()).collect::<Vec<_>>().join(", "),
            );
            continue;
        }

        if files.len() == 1 {
            let f = files.pop().unwrap();
            println!("rename [{}] → [{}]  {}", f.old_id, new_id, f.fm.source);
            to_delete.push(f.path);
            to_write.push((target, f.raw));
            rekeys.push((f.old_id, new_id.clone()));
            n_renamed += 1;
            continue;
        }

        // Merge: longest body wins (tie → smallest old id, for determinism);
        // tags and annotations are unioned onto the winner.
        files.sort_by(|a, b| b.body.len().cmp(&a.body.len()).then(a.old_id.cmp(&b.old_id)));
        let (winner, losers) = files.split_first().unwrap();

        let mut tags = winner.fm.tags.clone();
        let mut annotations = winner.fm.annotations.clone();
        for l in losers {
            for t in &l.fm.tags {
                if !tags.contains(t) {
                    tags.push(t.clone());
                }
            }
            for a in &l.fm.annotations {
                if !annotations.iter().any(|x| x.id == a.id) {
                    annotations.push(a.clone());
                }
            }
        }

        println!(
            "merge  [{}] ⊃ [{}] → [{}]  {}",
            winner.old_id,
            losers.iter().map(|f| f.old_id.as_str()).collect::<Vec<_>>().join(", "),
            new_id,
            winner.fm.source,
        );

        let content = render_entry(
            &winner.fm.source,
            &winner.fm.title,
            &winner.fm.created_at,
            winner.fm.published_at.as_deref(),
            winner.fm.updated_at.as_deref(),
            winner.fm.summary.as_deref(),
            &annotations,
            &tags,
            winner.fm.favicon.as_deref(),
            &winner.body,
        );

        for f in &files {
            if f.path != target {
                to_delete.push(f.path.clone());
            }
        }
        to_write.push((target, content));

        for f in &files {
            if f.old_id != *new_id {
                rekeys.push((f.old_id.clone(), new_id.clone()));
            }
        }
        n_merged_groups += 1;
        n_merged_files += files.len();
    }

    if !dry_run {
        for path in &to_delete {
            if let Err(e) = tokio::fs::remove_file(path).await {
                eprintln!("error: could not delete {}: {e}", path.display());
            }
        }
        for (path, content) in &to_write {
            if let Err(e) = tokio::fs::write(path, content.as_bytes()).await {
                eprintln!("error: could not write {}: {e}", path.display());
            }
        }
    }

    // ── Step 3: rewrite entry ids / annotation references in topics/*.md ─────

    let id_map: HashMap<&str, &str> =
        rekeys.iter().map(|(old, new)| (old.as_str(), new.as_str())).collect();

    let mut n_topics_rewritten = 0usize;
    let mut n_items_remapped = 0usize;
    let mut n_items_deduped = 0usize;

    let topics_dir = data_dir.join("topics");
    if let Ok(mut dir) = tokio::fs::read_dir(&topics_dir).await {
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let path = dirent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let topic_id = item_id(&path);
            let raw = match tokio::fs::read_to_string(&path).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("warn: could not read {}: {e}", path.display());
                    continue;
                }
            };
            let Some((fm, body)) = parse_document(&raw) else {
                eprintln!("warn: could not parse {} — skipped", path.display());
                continue;
            };

            let mut changed = false;
            // (item, original entry id, was remapped) — a merge can turn two
            // distinct Entry items into the same id; such duplicates are
            // dropped. Pre-existing duplicates (same original id) are kept.
            let mut kept: Vec<(CollectionItem, String, bool)> = Vec::new();

            for item in &fm.items {
                match item {
                    CollectionItem::Entry { id } => {
                        let (new_item, remapped) = match id_map.get(id.as_str()) {
                            Some(new_id) => (CollectionItem::Entry { id: (*new_id).to_string() }, true),
                            None => (item.clone(), false),
                        };
                        let dup = kept.iter().any(|(k, orig, k_remapped)| {
                            matches!(k, CollectionItem::Entry { id: kid } if kid == new_item.id())
                                && (remapped || *k_remapped)
                                && orig != id
                        });
                        if dup {
                            n_items_deduped += 1;
                            changed = true;
                            continue;
                        }
                        if remapped {
                            n_items_remapped += 1;
                            changed = true;
                        }
                        kept.push((new_item, id.clone(), remapped));
                    }
                    CollectionItem::Annotation { id, reference } => {
                        let new_item = match id_map.get(reference.as_str()) {
                            Some(new_ref) => {
                                n_items_remapped += 1;
                                changed = true;
                                CollectionItem::Annotation {
                                    id: id.clone(),
                                    reference: (*new_ref).to_string(),
                                }
                            }
                            None => item.clone(),
                        };
                        kept.push((new_item, id.clone(), false));
                    }
                }
            }

            if !changed {
                continue;
            }

            let items: Vec<CollectionItem> = kept.into_iter().map(|(item, _, _)| item).collect();
            println!("topic  [{}] {} — references updated", topic_id, fm.title);
            if !dry_run {
                let updated_at = fm.updated_at.as_deref().unwrap_or(&fm.created_at).to_string();
                if let Err(e) = topics::save_topic_with_timestamps(
                    data_dir,
                    &topic_id,
                    &fm.title,
                    &body,
                    fm.threshold,
                    items,
                    &fm.created_at,
                    &updated_at,
                )
                .await
                {
                    eprintln!("error: could not rewrite topic {}: {e}", path.display());
                    continue;
                }
            }
            n_topics_rewritten += 1;
        }
    }

    // ── Step 4: repair feeds (renames .atom files to the new hashes) ─────────

    println!();
    println!("Feeds:");
    if dry_run {
        println!("  would run feed repair (renames .atom files whose hash no longer matches; normalises entry links)");
    } else {
        match feeds::repair_feeds(feeds_dir).await {
            Ok(results) if results.is_empty() => println!("  no feed files found"),
            Ok(results) => super::feeds::print_repair_results(&results),
            Err(e) => eprintln!("warn: feed repair failed: {e}"),
        }
    }

    // ── Step 5: drop the entry state cache (keyed by old ids) ────────────────

    let cache_path = data_dir.join(".entry_state_cache.json");
    let mut cache_deleted = false;
    if !rekeys.is_empty() && tokio::fs::try_exists(&cache_path).await.unwrap_or(false) {
        if dry_run {
            cache_deleted = true;
        } else {
            match tokio::fs::remove_file(&cache_path).await {
                Ok(()) => cache_deleted = true,
                Err(e) => eprintln!("warn: could not delete {}: {e}", cache_path.display()),
            }
        }
    }

    // ── Step 6: summary ───────────────────────────────────────────────────────

    let verb = if dry_run { "Would re-key" } else { "Re-keyed" };
    println!();
    println!("{verb} entries: {n_renamed} renamed, {n_merged_files} merged into {n_merged_groups} ({n_unchanged} unchanged, {n_repaired} legacy frontmatter rebuilt).");
    println!("  Topics rewritten:        {n_topics_rewritten}  ({n_items_remapped} items remapped, {n_items_deduped} duplicates dropped)");
    if cache_deleted {
        let action = if dry_run { "would be deleted" } else { "deleted" };
        println!("  Entry state cache:       {action} — next app start reseeds it with a full scan");
    }
    if !rekeys.is_empty() || n_repaired > 0 {
        println!("  Embeddings index:        untouched — run `xaga-cli embeddings recover` (or start the app) to re-sync");
    }
}
