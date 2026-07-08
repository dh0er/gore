//! `gore config` — read/write the shared, per-user tool configuration
//! (`<shared>/config.json`). Currently the game install path.

use anyhow::{bail, Result};
use clap::{Subcommand, ValueEnum};
use gore_loc::config::{self, Config};

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a config value
    Set { key: ConfigKey, value: String },
    /// Print a single config value (exit non-zero if unset)
    Get { key: ConfigKey },
    /// Clear a single config value
    Unset { key: ConfigKey },
    /// Print all config values and, for the game path, the resolved root + source
    List,
    /// Print the path of the config.json file
    Path,
    /// Auto-detect the game via Steam and save it as game-path
    Detect,
}

/// The known, validated config keys. Add a variant here + a `Config` field to
/// extend the config surface.
#[derive(Clone, Copy, ValueEnum)]
pub enum ConfigKey {
    /// Game install path (an install root or the .exe)
    GamePath,
}

pub fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Set { key, value } => {
            let mut cfg = config::load();
            match key {
                ConfigKey::GamePath => cfg.game_path = Some(value.clone()),
            }
            config::save(&cfg)?;
            println!("set game-path = {value}");
            Ok(())
        }
        ConfigAction::Get { key } => {
            let cfg = config::load();
            match value_of(&cfg, key) {
                Some(v) => {
                    println!("{v}");
                    Ok(())
                }
                None => bail!("game-path is not set"),
            }
        }
        ConfigAction::Unset { key } => {
            let mut cfg = config::load();
            match key {
                ConfigKey::GamePath => cfg.game_path = None,
            }
            config::save(&cfg)?;
            println!("unset game-path");
            Ok(())
        }
        ConfigAction::List => {
            let cfg = config::load();
            let raw = cfg.game_path.clone().unwrap_or_else(|| "(unset)".into());
            println!("game-path = {raw}");
            match config::game_root(None) {
                Ok(root) => {
                    let source = if cfg.game_path.is_some() { "config" } else { "auto-detect" };
                    println!("resolved game root = {} (source: {source})", root.display());
                }
                Err(_) => println!("resolved game root = (unresolved)"),
            }
            println!("config file = {}", config::config_path().display());
            Ok(())
        }
        ConfigAction::Path => {
            println!("{}", config::config_path().display());
            Ok(())
        }
        ConfigAction::Detect => {
            match gore_loc::discover::find_game_root() {
                Some(root) => {
                    let mut cfg: Config = config::load();
                    let val = root.display().to_string();
                    cfg.game_path = Some(val.clone());
                    config::save(&cfg)?;
                    println!("detected and saved game-path = {val}");
                    Ok(())
                }
                None => bail!("could not auto-detect a Steam install; set it manually with 'gore config set game-path <path>'"),
            }
        }
    }
}

fn value_of(cfg: &Config, key: ConfigKey) -> Option<String> {
    match key {
        ConfigKey::GamePath => cfg.game_path.clone(),
    }
}
