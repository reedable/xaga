//! Restore absent article summaries from locally stored Atom, without fetching.
use std::collections::HashMap;
use std::path::Path;

use super::{documents, entries::EntryError};
use crate::library_writer::{LibraryWriter, WriteToken};
use crate::{atom, fs, library, md, util};

#[derive(Debug, Default)]
pub struct RepairReport {
    pub repaired: usize,
    pub would_repair: usize,
    pub skipped: usize,
    pub unavailable: usize,
    pub errors: Vec<String>,
}

pub async fn repair_summaries(writer: &LibraryWriter, dry_run: bool) -> RepairReport {
    let dir = writer.library_dir();
    let mut report = RepairReport::default();
    let mut summaries = HashMap::new();
    let outcome =
        fs::for_each_file(
            &library::feeds_dir(dir),
            "atom",
            |path, raw| match atom::parse_atom(&raw) {
                Ok(feed) => {
                    for entry in feed.entries() {
                        let entry = atom::atom_to_entry(entry);
                        if let Some(summary) = entry.description.filter(|s| !s.trim().is_empty()) {
                            if let Ok(url) = util::normalize_url(&entry.url) {
                                summaries.entry(url).or_insert(summary);
                            }
                        }
                    }
                }
                Err(e) => report.errors.push(format!("{}: {e}", path.display())),
            },
        )
        .await;
    if outcome == fs::WalkOutcome::Unreadable {
        report
            .errors
            .push("Could not read all local Atom files".into());
    }

    let mut candidates = Vec::new();
    let outcome = fs::for_each_file(&library::entries_dir(dir), "md", |path, raw| {
        let result = (|| -> Result<(), EntryError> {
            if md::has_summary_field(&raw)? {
                report.skipped += 1;
                return Ok(());
            }
            let (fm, _) = md::parse_document(&raw)?;
            let url = util::normalize_url(&fm.source)?;
            if let Some(summary) = summaries.get(&url) {
                if dry_run {
                    // Exercise the same serializer and output parser as the write path.
                    md::patch_document(&raw, &[("summary", serde_json::json!(summary))], None)?;
                    report.would_repair += 1;
                } else {
                    candidates.push((util::item_id(&path), summary.clone()));
                }
            } else {
                report.unavailable += 1;
            }
            Ok(())
        })();
        if let Err(e) = result {
            report.errors.push(format!("{}: {e}", path.display()));
        }
    })
    .await;
    if outcome == fs::WalkOutcome::Unreadable {
        report
            .errors
            .push("Could not read all saved article files".into());
    }
    if dry_run {
        return report;
    }
    for (id, summary) in candidates {
        match writer.repair_summary(id.clone(), summary).await {
            Ok(true) => report.repaired += 1,
            Ok(false) => report.skipped += 1,
            Err(e) => report.errors.push(format!("{id}: {e}")),
        }
    }
    report
}

pub(crate) async fn repair_summary(
    dir: &Path,
    id: &str,
    summary: &str,
    token: WriteToken,
) -> Result<bool, EntryError> {
    if !super::entries::is_safe_id(id) {
        return Err(EntryError::InvalidId);
    }
    let path = library::entry_path(dir, id);
    // Read and check inside the writer's entry lane. Never create a missing article.
    let raw = fs::read_md_capped(&path).await?;
    if md::has_summary_field(&raw)? || summary.trim().is_empty() {
        return Ok(false);
    }
    documents::upsert(
        &path,
        &[("summary", serde_json::json!(summary))],
        None,
        token,
    )
    .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article(summary: &str) -> String {
        format!(
            "---\nsource: https://example.org/article\ntitle: Saved\ncreated: '2026-01-01'\ntags: [research, bookmark]\nannotations:\n  - id: note\n    text: [selection]\n    occurrence: [0]\n    color: yellow\n    note: Keep this note\n    created_at: '2026-01-01'\n{summary}---\n\nOriginal body\n\n"
        )
    }

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(library::entries_dir(dir.path())).unwrap();
        std::fs::create_dir(library::feeds_dir(dir.path())).unwrap();
        dir
    }

    fn writer(dir: &Path) -> LibraryWriter {
        LibraryWriter::start(dir.to_owned(), library::acquire_lock(dir).unwrap())
    }

    #[tokio::test]
    async fn restores_local_atom_summary_and_preserves_other_fields_and_body() {
        let dir = fixture();
        let path = library::entry_path(dir.path(), "saved");
        let original = article("");
        std::fs::write(&path, &original).unwrap();
        // Match by normalized URL, not the publisher's Atom id or local filename.
        std::fs::write(library::feeds_dir(dir.path()).join("feed.atom"), r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Feed</title><id>feed</id><updated>2026-01-01T00:00:00Z</updated><entry><title>Different title</title><id>publisher-id</id><updated>2026-01-01T00:00:00Z</updated><link href="http://www.example.org/article/" rel="alternate"/><summary type="html">&lt;img src="https://example.org/comic.png"&gt;
Original summary&#13;&lt;br/&gt;
Last line</summary></entry></feed>"#).unwrap();
        let writer = writer(dir.path());
        let dry_report = repair_summaries(&writer, true).await;
        assert_eq!(dry_report.would_repair, 1);
        assert_eq!(dry_report.repaired, 0);
        assert!(dry_report.errors.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let report = repair_summaries(&writer, false).await;
        assert_eq!(report.repaired, 1, "{report:?}");
        assert!(report.errors.is_empty(), "{report:?}");
        let saved = std::fs::read_to_string(&path).unwrap();
        let mut expected = md::test_frontmatter_values(&original);
        expected.insert(
            "summary".into(),
            noyalib::Value::String(
                "<img src=\"https://example.org/comic.png\">\nOriginal summary\r<br/>\nLast line".into(),
            ),
        );
        assert_eq!(md::test_frontmatter_values(&saved), expected);
        assert!(saved.ends_with("---\n\nOriginal body\n\n"));
        let report = repair_summaries(&writer, false).await;
        assert_eq!(report.repaired, 0);
        assert_eq!(report.skipped, 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn existing_summary_values_are_untouched_inside_writer_lane() {
        let dir = fixture();
        let writer = writer(dir.path());
        for field in [
            "summary: Saved\n",
            "summary: ''\n",
            "summary: null\n",
            "summary: '  '\n",
        ] {
            let path = library::entry_path(dir.path(), "saved");
            let original = article(field);
            std::fs::write(&path, &original).unwrap();
            assert!(
                !writer
                    .repair_summary("saved".into(), "Recovered".into())
                    .await
                    .unwrap()
            );
            assert_eq!(std::fs::read_to_string(path).unwrap(), original);
        }
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn unavailable_summary_and_malformed_files_are_not_written() {
        let dir = fixture();
        let path = library::entry_path(dir.path(), "saved");
        let original = article("");
        std::fs::write(&path, &original).unwrap();
        let broken_path = library::entry_path(dir.path(), "broken");
        std::fs::write(&broken_path, "---\ntags: [broken\n---\nBody").unwrap();
        std::fs::write(library::feeds_dir(dir.path()).join("broken.atom"), "<feed>").unwrap();
        let writer = writer(dir.path());
        let report = repair_summaries(&writer, false).await;
        assert_eq!(report.repaired, 0);
        assert_eq!(report.unavailable, 1);
        assert_eq!(report.errors.len(), 2, "{report:?}");
        assert!(
            !writer
                .repair_summary("saved".into(), "".into())
                .await
                .unwrap()
        );
        assert!(
            writer
                .repair_summary("missing".into(), "Recovered".into())
                .await
                .is_err()
        );
        assert!(
            writer
                .repair_summary("broken".into(), "Recovered".into())
                .await
                .is_err()
        );
        assert!(!library::entry_path(dir.path(), "missing").exists());
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
        assert_eq!(
            std::fs::read_to_string(broken_path).unwrap(),
            "---\ntags: [broken\n---\nBody"
        );
        writer.shutdown().await;
    }
}
