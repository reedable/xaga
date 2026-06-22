use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use clap::{Args, Subcommand};
use xaga_core::embeddings::{default_model_dir, load_default_embedder, EmbeddingsDb, LockFile, DEFAULT_MODEL_REPO, MODEL_FILES};
use xaga_core::recovery;

use crate::die;

const DOWNLOAD_CMD: &str = "xaga-cli embeddings download-model";

async fn make_embedder() -> Arc<xaga_core::embeddings::Embedder> {
    match load_default_embedder().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("       Run:  {DOWNLOAD_CMD}");
            std::process::exit(1);
        }
    }
}

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct EmbeddingsArgs {
    #[command(subcommand)]
    pub subcommand: EmbeddingsCommand,
}

#[derive(Subcommand)]
pub enum EmbeddingsCommand {
    /// Show the number of embedded documents
    Status,
    /// List all embedded documents (source URL + title)
    List,
    /// Reconcile LanceDB with the filesystem (adds missing, removes stale; covers docs + feeds)
    Recover,
    /// Drop and recreate the embeddings table (clears all indexed data)
    Reset,
    /// Reset the embeddings table then re-embed all documents and feeds from scratch
    Reindex,
    /// Embed a single file and report device, timing, and vector norm
    Embed {
        /// Path to the markdown file to embed
        path: PathBuf,
    },
    /// Download the embedding model from HuggingFace
    DownloadModel {
        /// HuggingFace repo ID to download (e.g. "Qdrant/all-MiniLM-L6-v2-onnx")
        #[arg(long, default_value = DEFAULT_MODEL_REPO)]
        repo: String,
    },
    /// Sample pairwise similarities and print a histogram to help choose thresholds
    Calibrate {
        /// Number of items to sample (compared against the full corpus)
        #[arg(long, default_value = "50")]
        sample: usize,
        /// Minimum similarity to include in the output
        #[arg(long, default_value = "0.3")]
        min: f32,
    },
}

/// Acquires the library write lock or exits with a clear error.
///
/// The returned `LockFile` must be kept alive for the duration of the write
/// operation — dropping it releases the lock.
fn require_write_lock(data_dir: &Path) -> LockFile {
    LockFile::try_acquire(LockFile::path_for(data_dir)).unwrap_or_else(|| {
        eprintln!("error: the Xaga UI is running against this library.");
        eprintln!("       Stop the UI before running write commands.");
        std::process::exit(1);
    })
}

pub async fn handle(cmd: EmbeddingsCommand, data_dir: &Path) {
    match cmd {
        EmbeddingsCommand::Status => match EmbeddingsDb::open(data_dir).await {
            Ok(db) => match db.embedding_status().await {
                Ok(rows) => {
                    let total: usize = rows.iter().map(|r| r.total).sum();
                    let embedded: usize = rows.iter().map(|r| r.embedded).sum();
                    println!("{total} row(s): {embedded} embedded, {} pending", total - embedded);
                    for r in &rows {
                        println!(
                            "  {:<11} {:>7} total  {:>7} embedded  {:>7} pending",
                            r.kind,
                            r.total,
                            r.embedded,
                            r.total - r.embedded,
                        );
                    }
                }
                Err(e) => die(e),
            },
            Err(e) => die(e),
        },

        EmbeddingsCommand::List => match EmbeddingsDb::open(data_dir).await {
            Ok(db) => match db.list().await {
                Ok(docs) if docs.is_empty() => println!("No documents embedded yet."),
                Ok(docs) => {
                    for doc in &docs {
                        println!("{}", doc.url);
                        println!("  {}", doc.title);
                    }
                }
                Err(e) => die(e),
            },
            Err(e) => die(e),
        },

        EmbeddingsCommand::Embed { path } => {
            let _lock = require_write_lock(data_dir);
            let t0 = Instant::now();
            let embedder = make_embedder().await;
            println!("device:       {}", embedder.device_label());
            println!("model loaded: {:.2?}", t0.elapsed());

            let content = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => die(e),
            };
            let (title, body) = content.split_once('\n').unwrap_or(("", &content));
            let title = title.trim_start_matches('#').trim();

            let t1 = Instant::now();
            let vector = match embedder.embed(title, body) {
                Ok(v) => v,
                Err(e) => die(e),
            };
            let elapsed = t1.elapsed();

            let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("embed time:   {elapsed:.2?}");
            println!("dimensions:   {}", vector.len());
            println!("vector norm:  {norm:.6}  (should be ≈1.0)");
        }

        EmbeddingsCommand::Recover => {
            let _lock = require_write_lock(data_dir);
            let db = match EmbeddingsDb::open(data_dir).await {
                Ok(db) => db,
                Err(e) => die(e),
            };
            let embedder = make_embedder().await;
            let report = recovery::execute::run_full_recovery(data_dir, &embedder, &db).await;
            println!("recover: {} embedded, {} deleted", report.embedded, report.deleted);
            for err in &report.errors {
                eprintln!("  warning: {err}");
            }
        }

        EmbeddingsCommand::Reset => {
            let _lock = require_write_lock(data_dir);
            EmbeddingsDb::destroy(data_dir).unwrap_or_else(|e| die(e));
            match EmbeddingsDb::open(data_dir).await {
                Ok(_) => println!("Embeddings index cleared."),
                Err(e) => die(e),
            }
        }

        EmbeddingsCommand::Reindex => {
            let _lock = require_write_lock(data_dir);
            EmbeddingsDb::destroy(data_dir).unwrap_or_else(|e| die(e));
            let db = match EmbeddingsDb::open(data_dir).await {
                Ok(db) => db,
                Err(e) => die(e),
            };
            let embedder = make_embedder().await;
            let report = recovery::execute::run_full_recovery(data_dir, &embedder, &db).await;
            println!("reindex: {} embedded, {} deleted", report.embedded, report.deleted);
            for err in &report.errors {
                eprintln!("  warning: {err}");
            }
        }

        EmbeddingsCommand::DownloadModel { repo } => {
            download_model(&repo).await;
        }

        EmbeddingsCommand::Calibrate { sample, min } => {
            let db = match EmbeddingsDb::open(data_dir).await {
                Ok(db) => db,
                Err(e) => die(e),
            };
            let total = match db.count().await {
                Ok(n) => n,
                Err(e) => die(e),
            };
            if total < 2 {
                println!("Not enough documents embedded to calibrate (need ≥ 2).");
                return;
            }
            let actual_sample = sample.min(total);
            println!("Sampling {actual_sample} items against {total} total…");

            let mut scores = match db.sample_similarities(actual_sample, min).await {
                Ok(s) => s,
                Err(e) => die(e),
            };

            if scores.is_empty() {
                println!("No pairs found with similarity ≥ {min:.2}.");
                return;
            }

            scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let total_pairs = scores.len();

            // Histogram: bins of width 0.05 from `min` (rounded down) to 1.0.
            const BIN_WIDTH: f32 = 0.05;
            let first_bin = ((min / BIN_WIDTH).floor() as usize).max(0);
            let num_bins = ((1.0f32 - first_bin as f32 * BIN_WIDTH) / BIN_WIDTH).ceil() as usize + 1;
            let mut bins = vec![0usize; num_bins];
            for &s in &scores {
                let idx = ((s / BIN_WIDTH).floor() as usize).saturating_sub(first_bin);
                let idx = idx.min(bins.len() - 1);
                bins[idx] += 1;
            }

            let max_count = *bins.iter().max().unwrap_or(&1).max(&1);
            const BAR_WIDTH: usize = 40;

            println!("\nSimilarity distribution ({total_pairs} pairs ≥ {min:.2}):\n");
            for (i, &count) in bins.iter().enumerate() {
                let lo = (first_bin + i) as f32 * BIN_WIDTH;
                let hi = lo + BIN_WIDTH;
                let bar_len = (count * BAR_WIDTH / max_count).max(if count > 0 { 1 } else { 0 });
                let bar: String = "█".repeat(bar_len);
                let pct = count as f32 / total_pairs as f32 * 100.0;
                println!("  {lo:.2}–{hi:.2}  {bar:<40}  {count:>6}  ({pct:5.1}%)");
            }

            // Percentiles.
            let pct = |p: f32| -> f32 {
                let idx = ((p / 100.0) * (scores.len() - 1) as f32).round() as usize;
                scores[idx.min(scores.len() - 1)]
            };
            println!("\nPercentiles (of all sampled pairs ≥ {min:.2}):");
            println!(
                "  p50: {:.3}   p75: {:.3}   p90: {:.3}   p95: {:.3}   p99: {:.3}",
                pct(50.0), pct(75.0), pct(90.0), pct(95.0), pct(99.0)
            );
        }
    }
}

async fn download_model(repo: &str) {
    let model_dir = default_model_dir();

    if let Err(e) = std::fs::create_dir_all(&model_dir) {
        eprintln!("error: could not create model directory: {e}");
        std::process::exit(1);
    }

    println!("Downloading model from {repo} into {}", model_dir.display());

    let client = reqwest::Client::builder()
        .build()
        .unwrap_or_else(|e| die(e));

    for &filename in MODEL_FILES {
        let dest = model_dir.join(filename);
        if dest.is_file() {
            println!("  {filename}  (already present, skipping)");
            continue;
        }
        let url = format!("https://huggingface.co/{repo}/resolve/main/{filename}");
        print!("  {filename} … ");
        match fetch_file(&client, &url).await {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&dest, &bytes) {
                    eprintln!("\nerror: failed to write {filename}: {e}");
                    std::process::exit(1);
                }
                println!("{} MB", bytes.len() / 1_000_000);
            }
            Err(e) => {
                eprintln!("\nerror: failed to download {filename}: {e}");
                std::process::exit(1);
            }
        }
    }

    println!("Done. You can now run:  {DOWNLOAD_CMD_DONE}");
}

const DOWNLOAD_CMD_DONE: &str = "xaga-cli embeddings reindex";

async fn fetch_file(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, reqwest::Error> {
    Ok(client.get(url).send().await?.error_for_status()?.bytes().await?.to_vec())
}
