use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cmd;

#[derive(Parser)]
#[command(name = "gore-cli", about = "Gothic 1 Remake mod tooling CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse UE4SS SDK dump into gore-core reflection model JSON
    Dump {
        /// Path to the CXXHeaderDump/ directory
        sdk_dir: PathBuf,
        /// Output model.json path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Generate LuaLS/EmmyLua type stubs from model.json
    Stubs {
        /// Path to model.json
        model: PathBuf,
        /// Output directory for .lua stub files
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// Only emit classes whose name starts with PREFIX
        #[arg(long)]
        filter: Option<String>,
    },
    /// Generate a catalog JSON from a UE4SS object dump
    Catalog {
        /// Catalog kind to generate
        #[arg(long, value_name = "KIND")]
        kind: cmd::catalog::CatalogKind,
        /// Path to UE4SS_ObjectDump.txt
        dump: PathBuf,
        /// Output catalog JSON path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Convert a gore-cli reflection model into a gore-mod GUI shape JSON
    GuiModel {
        /// Path to model.json (output of `gore-cli dump`)
        #[arg(long)]
        model: PathBuf,
        /// Path to item_catalog.json
        #[arg(long)]
        catalog: PathBuf,
        /// Output GUI model JSON path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Refresh the gore-mod GUI model from a runtime game-data dump (with real
    /// default values), produced in-game by the gore-dump UE4SS mod
    Sync {
        /// Path to game_data.json (output of the gore-dump mod)
        #[arg(long)]
        dump: PathBuf,
        /// Path to item_catalog.json (the item allow-list)
        #[arg(long)]
        catalog: PathBuf,
        /// Output GUI model JSON path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Create a UE4SS Lua mod skeleton directory
    Scaffold {
        /// Mod name (becomes the directory name under mods-dir)
        mod_name: String,
        /// Mods directory (e.g. ue4ss/Mods/)
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Compile overrides.toml into a UE4SS Lua mod
    Gen {
        /// Path to overrides.toml
        overrides: PathBuf,
        /// Mods directory to write the mod folder into
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// Path to model.json for validation (optional; skips validation if absent)
        #[arg(long)]
        model: Option<PathBuf>,
    },
    /// Zip a mod folder into distributable UE4SS layout
    Package {
        /// Path to the mod directory
        mod_dir: PathBuf,
        /// Output zip path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Dump { sdk_dir, out } => cmd::dump::run(sdk_dir, out),
        Commands::Stubs { model, out, filter } => cmd::stubs::run(model, out, filter),
        Commands::Catalog { kind, dump, out } => cmd::catalog::run(kind, dump, out),
        Commands::GuiModel { model, catalog, out } => cmd::gui_model::run(model, catalog, out),
        Commands::Sync { dump, catalog, out } => cmd::sync::run(dump, catalog, out),
        Commands::Scaffold { mod_name, out } => cmd::scaffold::run(mod_name, out),
        Commands::Gen { overrides, out, model } => cmd::gen::run(overrides, out, model),
        Commands::Package { mod_dir, out } => cmd::package::run(mod_dir, out),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
