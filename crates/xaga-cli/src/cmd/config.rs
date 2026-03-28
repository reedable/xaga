use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};

use crate::{die, settings};

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub subcommand: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Print the active configuration
    Show,
    /// Persist a setting to ~/.xagarc
    Set(ConfigSetArgs),
}

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct ConfigSetArgs {
    #[command(subcommand)]
    pub key: ConfigKey,
}

#[derive(Subcommand)]
pub enum ConfigKey {
    /// Set the data directory
    DataDir { path: PathBuf },
}

pub fn handle(cmd: ConfigCommand, rc: &settings::Config, resolved_data_dir: &Path) {
    match cmd {
        ConfigCommand::Show => settings::display(rc, resolved_data_dir),
        ConfigCommand::Set(args) => match args.key {
            ConfigKey::DataDir { path } => {
                let mut updated = settings::load();
                updated.data_dir = Some(path.clone());
                match settings::save(&updated) {
                    Ok(()) => println!(
                        "data_dir set to {} in {}",
                        path.display(),
                        settings::rc_path()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ),
                    Err(e) => die(e),
                }
            }
        },
    }
}
