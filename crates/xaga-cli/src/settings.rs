use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const RC_FILE: &str = ".xagarc";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: Option<PathBuf>,
}

/// Path to ~/.xagarc
pub fn rc_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(RC_FILE))
}

/// Load ~/.xagarc if it exists. Missing file returns an empty Config.
/// Prints a warning to stderr if the file exists but cannot be parsed.
pub fn load() -> Config {
    let Some(path) = rc_path() else {
        return Config::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    match toml::from_str(&text) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("warning: {} is malformed, using defaults: {e}", path.display());
            Config::default()
        }
    }
}

/// Write a Config back to ~/.xagarc.
pub fn save(config: &Config) -> Result<(), String> {
    let path = rc_path().ok_or("cannot determine home directory")?;
    let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    xaga_core::fs::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Resolve the data directory with explicit priority:
///   1. --data-dir CLI flag
///   2. $XAGA_DATA_DIR env var  (handled by clap before we get here)
///   3. ~/.xagarc  data_dir
///   4. Platform data dir / xaga
///   5. .xaga  (last-resort fallback — warns to stderr)
pub fn resolve_data_dir(cli_override: Option<PathBuf>, rc: &Config) -> PathBuf {
    cli_override
        .or_else(|| rc.data_dir.clone())
        .or_else(|| dirs::data_dir().map(|d| d.join("xaga")))
        .unwrap_or_else(|| {
            eprintln!(
                "warning: cannot determine platform data directory; \
                 using .xaga in the current directory. \
                 Set a permanent path with `config set data-dir <path>`."
            );
            PathBuf::from(".xaga")
        })
}

/// Pretty-print the active config for `config show`.
pub fn display(rc: &Config, resolved: &Path) {
    println!("rc file:  {}", rc_path().map(|p| p.display().to_string()).unwrap_or_default());
    println!(
        "data_dir: {}  ({})",
        resolved.display(),
        if rc.data_dir.as_deref() == Some(resolved) {
            "from ~/.xagarc"
        } else {
            "default"
        }
    );
}
