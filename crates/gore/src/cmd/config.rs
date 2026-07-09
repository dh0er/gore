//! `gore config` — read/write the shared, per-user tool configuration
//! (`<shared>/config.json`). Currently the game install path.

use anyhow::{bail, Result};
use clap::{Subcommand, ValueEnum};
use gore_loc::config::{self, Config};
use std::path::PathBuf;

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
            let stored = match key {
                // Persist an absolute path: a relative value (e.g. `.`) would
                // otherwise be re-resolved against whatever directory a later
                // command happens to run from.
                ConfigKey::GamePath => {
                    let s = absolutize(&value);
                    cfg.game_path = Some(s.clone());
                    s
                }
            };
            config::save(&cfg)?;
            println!("set game-path = {stored}");
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
                    // Match game_root's own view: a blank/whitespace game_path is
                    // treated as unset, so the root came from auto-detect, not config.
                    let from_config = cfg
                        .game_path
                        .as_deref()
                        .is_some_and(|s| !s.trim().is_empty());
                    let source = if from_config { "config" } else { "auto-detect" };
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

/// Resolve a possibly-relative user path to an absolute one (joined against the
/// current directory) so a stored config value doesn't depend on the cwd of a
/// later command. Kept lexical — no symlink / `..` resolution — to avoid the
/// Windows `\\?\` verbatim prefix that `canonicalize` produces, which some
/// downstream tooling mishandles.
fn absolutize(value: &str) -> String {
    let raw = PathBuf::from(value);
    let abs = if raw.is_absolute() {
        raw
    } else {
        std::env::current_dir().map(|d| d.join(&raw)).unwrap_or(raw)
    };
    abs.to_string_lossy().into_owned()
}
