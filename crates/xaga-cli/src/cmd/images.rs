use std::path::Path;

use clap::{Args, Subcommand};
use xaga_core::library;
use xaga_core::persistence::{articles, images};

use crate::die;

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct ImagesArgs {
    #[command(subcommand)]
    pub subcommand: ImagesCommand,
}

#[derive(Subcommand)]
pub enum ImagesCommand {
    /// Remove stored entry images that no entry references: whole directories
    /// for entries that no longer exist, and individual files a surviving
    /// entry's body no longer points at. Local per-entry GC already runs on
    /// every save; this is the manual, whole-library backstop.
    Gc {
        /// Report what would be removed without deleting anything.
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle(cmd: ImagesCommand, data_dir: &Path) {
    match cmd {
        ImagesCommand::Gc { dry_run } => gc(data_dir, dry_run).await,
    }
}

async fn gc(data_dir: &Path, dry_run: bool) {
    // Deleting files is a library write; hold the single-writer lock so the GUI
    // or another CLI cannot be mid-save on the same library.
    let _lock = match library::acquire_lock(data_dir) {
        Some(lock) => lock,
        None => die("another process holds the library lock"),
    };

    let images_dir = library::images_dir(data_dir);
    let entries = match std::fs::read_dir(&images_dir) {
        Ok(entries) => entries,
        // No images directory means nothing has been stored yet.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("No stored images.");
            return;
        }
        Err(e) => die(e),
    };

    let mut removed_dirs = 0usize;
    let mut removed_files = 0usize;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => die(e),
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();

        // Does the owning entry still exist? read_article searches the entry
        // directories; NotFound (or any read error) means the images are orphans.
        match articles::read_article(data_dir, &id).await {
            Ok(article) => {
                // The body references ext-less url-hashes; a file `<hash>.<ext>`
                // is live iff its stem is referenced. The favicon is referenced
                // from frontmatter rather than the body, so it has to be added
                // by hand or every sweep would delete it.
                let mut keep = images::referenced_image_hashes(&id, &article.content);
                if let Some(hash) = article
                    .favicon
                    .as_deref()
                    .and_then(library::parse_entry_image_ref)
                    .filter(|(ref_id, _)| *ref_id == id)
                    .map(|(_, hash)| hash.to_string())
                {
                    keep.insert(hash);
                }
                let files = match std::fs::read_dir(&path) {
                    Ok(files) => files,
                    Err(e) => die(e),
                };
                for file in files {
                    let file = match file {
                        Ok(f) => f,
                        Err(e) => die(e),
                    };
                    let name = file.file_name().to_string_lossy().into_owned();
                    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&name);
                    if keep.contains(stem) {
                        continue;
                    }
                    let file_path = file.path();
                    if dry_run {
                        println!("would remove file {}", file_path.display());
                    } else if let Err(e) = std::fs::remove_file(&file_path) {
                        die(e);
                    }
                    removed_files += 1;
                }
            }
            Err(_) => {
                if dry_run {
                    println!("would remove orphan directory {}", path.display());
                } else if let Err(e) = std::fs::remove_dir_all(&path) {
                    die(e);
                }
                removed_dirs += 1;
            }
        }
    }

    let verb = if dry_run { "would remove" } else { "removed" };
    println!("{verb} {removed_dirs} orphan director{} and {removed_files} unreferenced file{}",
        if removed_dirs == 1 { "y" } else { "ies" },
        if removed_files == 1 { "" } else { "s" },
    );
}
