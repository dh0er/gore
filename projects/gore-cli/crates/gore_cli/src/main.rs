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
    /// Generate the gore-dump UE4SS mod (reads live CDO stat values in-game ->
    /// gore_game_data.json, the input to `sync`)
    DumpMod {
        /// Path to model.json (field schema; output of `dump`+`gui-model`)
        #[arg(long)]
        model: PathBuf,
        /// Path to item_catalog.json (the item allow-list)
        #[arg(long)]
        catalog: PathBuf,
        /// Mods directory to write the gore-dump/ folder into
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Read/edit localized text from the encrypted AlkimiaLocalization .lcache
    Loc {
        #[command(subcommand)]
        action: LocAction,
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
    /// Deploy the gore-lua shared SDK into the game's ue4ss/Mods/shared.
    DeployShared {
        /// Source shared/ dir (default: projects/gore-lua/shared).
        #[arg(long, default_value = "projects/gore-lua/shared")]
        src: std::path::PathBuf,
        /// Game dir containing ue4ss/Mods.
        #[arg(long)]
        game: std::path::PathBuf,
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

#[derive(Subcommand)]
enum LocAction {
    /// Auto-detect (or --lcache) the game's .lcache and write the shared
    /// gore-tools/loc_catalog.json (used by gore-save and gore-mod too)
    Extract {
        /// Path to the .lcache, the game dir, or a Steam library (else auto-detect)
        #[arg(long)]
        lcache: Option<PathBuf>,
        /// Skip the confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Show the shared loc catalog's status (ids, languages, source)
    Status,
    /// Decrypt the .lcache and write {id:{language:value}} JSON (all languages)
    Export {
        /// Path to AlkimiaLocalization_*.lcache
        #[arg(long)]
        lcache: PathBuf,
        /// Output loc_catalog.json
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// Keep empty values / ids with no text
        #[arg(long)]
        keep_empty: bool,
    },
    /// Apply {id:{language:value}} edits and re-encrypt the .lcache
    Import {
        /// Path to the .lcache to edit
        #[arg(long)]
        lcache: PathBuf,
        /// Path to edits JSON ({id:{language:value}})
        #[arg(long)]
        edits: PathBuf,
        /// Output .lcache (defaults to overwriting --lcache)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
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
        Commands::DumpMod { model, catalog, out } => cmd::dump_mod::run(model, catalog, out),
        Commands::Loc { action } => match action {
            LocAction::Extract { lcache, yes } => cmd::loc::extract(lcache, yes),
            LocAction::Status => cmd::loc::status(),
            LocAction::Export { lcache, out, keep_empty } => cmd::loc::export(lcache, out, keep_empty),
            LocAction::Import { lcache, edits, out } => cmd::loc::import(lcache, edits, out),
        },
        Commands::Scaffold { mod_name, out } => cmd::scaffold::run(mod_name, out),
        Commands::Gen { overrides, out, model } => cmd::gen::run(overrides, out, model),
        Commands::DeployShared { src, game } => cmd::deploy_shared::run(src, game),
        Commands::Package { mod_dir, out } => cmd::package::run(mod_dir, out),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
