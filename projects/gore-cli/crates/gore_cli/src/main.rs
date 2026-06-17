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
        /// Path to UE4SS_ObjectDump.txt (optional)
        #[arg(long = "object-dump")]
        object_dump: Option<PathBuf>,
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
    /// (Re)generate item_catalog.json from model.json or sdk-dir
    Catalog {
        /// Path to model.json or CXXHeaderDump/ sdk-dir
        input: PathBuf,
        /// Output catalog JSON path
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
        Commands::Dump { sdk_dir, object_dump, out } => {
            cmd::dump::run(sdk_dir, object_dump, out)
        }
        Commands::Stubs { model, out, filter } => cmd::stubs::run(model, out, filter),
        Commands::Catalog { input, out } => cmd::catalog::run(input, out),
        Commands::Scaffold { mod_name, out } => cmd::scaffold::run(mod_name, out),
        Commands::Gen { overrides, out, model } => cmd::gen::run(overrides, out, model),
        Commands::Package { mod_dir, out } => cmd::package::run(mod_dir, out),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
