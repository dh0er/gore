use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cmd;

#[derive(Parser)]
#[command(name = "gore", about = "GORE Command Line Tools", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse UE4SS SDK dump into gore-reflect reflection model JSON
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
        /// Source shared/ dir. Defaults to a copy located relative to the gore-cli
        /// executable (cwd-independent); pass this when running from an unusual layout.
        #[arg(long)]
        src: Option<std::path::PathBuf>,
        /// Game dir containing ue4ss/Mods. Defaults to the configured game path or
        /// Steam auto-detect when omitted (see `gore config`).
        #[arg(long)]
        game: Option<std::path::PathBuf>,
    },
    /// AngelScript precompiled-cache tooling (decode/emit/splice/decompile).
    As {
        #[command(subcommand)]
        cmd: cmd::as_cache::AsCmd,
    },
    /// Zip a mod folder into distributable UE4SS layout
    Package {
        /// Path to the mod directory
        mod_dir: PathBuf,
        /// Output zip path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Read/replace audio in the game's encrypted FMOD sound banks (.bank)
    Audio {
        #[command(subcommand)]
        action: AudioAction,
    },
    /// Build / deploy / undeploy a unified mod bundle (overrides + loc + audio + textures + scripts)
    Mod {
        #[command(subcommand)]
        action: ModAction,
    },
    /// Extract/replace game textures (Gothic 1 Remake, UE5 IoStore)
    Texture {
        #[command(subcommand)]
        action: cmd::texture::TextureAction,
    },
    /// Multi-mod manager: library + loadout + conflicts + composed deploy
    Mgr {
        #[command(subcommand)]
        action: cmd::mgr::MgrAction,
    },
    /// Read/write the shared per-user config (game path, …)
    Config {
        #[command(subcommand)]
        action: cmd::config::ConfigAction,
    },
}

#[derive(Subcommand)]
enum ModAction {
    /// Build a bundle dir from a BuildSpec JSON
    Build {
        /// Path to the build spec JSON
        #[arg(long)]
        spec: PathBuf,
        /// Output directory (the bundle is written to <out>/<mod-name>)
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Deploy a built bundle to the game install
    Deploy {
        /// Path to the bundle directory
        #[arg(long)]
        bundle: PathBuf,
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Undeploy the active mod (restore backups)
    Undeploy {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum AudioAction {
    /// List a bank's samples (name, codec, sample rate, channels, duration)
    List {
        /// Path to a .bank file
        #[arg(long)]
        bank: PathBuf,
        /// Override the bank encryption key (defaults to the Gothic 1 Remake key)
        #[arg(long)]
        key: Option<String>,
    },
    /// Extract samples to WAV (.wav) for listening/editing
    Extract {
        /// Path to a .bank file
        #[arg(long)]
        bank: PathBuf,
        /// Output directory for .wav files
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// A single sample name, or "all" (default)
        #[arg(long)]
        sample: Option<String>,
        /// Override the bank encryption key
        #[arg(long)]
        key: Option<String>,
    },
    /// Replace samples with new audio (WAV) via PCM injection
    Replace {
        /// Path to map JSON: { "SampleName": "path/to/new.wav", ... } (WAV paths relative to it)
        #[arg(long)]
        map: PathBuf,
        /// Path to the .bank to modify
        #[arg(long)]
        bank: PathBuf,
        /// Output .bank (default: overwrite --bank in place, backing up to *.gore-bak)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        /// Override the bank encryption key
        #[arg(long)]
        key: Option<String>,
    },
    /// Restore a bank from its *.gore-bak backup
    Restore {
        /// Path to the .bank to restore
        #[arg(long)]
        bank: PathBuf,
    },
    /// Build a shareable audio patch zip (manifest + replacement WAVs, no game audio)
    ExportPatch {
        /// Path to map JSON: { "SampleName": "path/to/new.wav", ... }
        #[arg(long)]
        map: PathBuf,
        /// Output patch .zip
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Apply a patch zip (from export-patch) to a bank
    ApplyPatch {
        /// Path to the patch .zip
        #[arg(long)]
        patch: PathBuf,
        /// Path to the .bank to modify
        #[arg(long)]
        bank: PathBuf,
        /// Output .bank (default: overwrite --bank in place, backing up to *.gore-bak)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        /// Override the bank encryption key
        #[arg(long)]
        key: Option<String>,
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
        Commands::As { cmd } => cmd::as_cache::run(cmd),
        Commands::Package { mod_dir, out } => cmd::package::run(mod_dir, out),
        Commands::Audio { action } => match action {
            AudioAction::List { bank, key } => cmd::audio::list(bank, key),
            AudioAction::Extract { bank, out, sample, key } => cmd::audio::extract(bank, out, sample, key),
            AudioAction::Replace { map, bank, out, key } => cmd::audio::replace(map, bank, out, key),
            AudioAction::Restore { bank } => cmd::audio::restore(bank),
            AudioAction::ExportPatch { map, out } => cmd::audio::export_patch(map, out),
            AudioAction::ApplyPatch { patch, bank, out, key } => cmd::audio::apply_patch(patch, bank, out, key),
        },
        Commands::Mod { action } => match action {
            ModAction::Build { spec, out } => cmd::modcmd::build(spec, out),
            ModAction::Deploy { bundle, game } => cmd::modcmd::deploy(bundle, game),
            ModAction::Undeploy { game } => cmd::modcmd::undeploy(game),
        },
        Commands::Texture { action } => cmd::texture::run(action),
        Commands::Mgr { action } => cmd::mgr::run(action),
        Commands::Config { action } => cmd::config::run(action),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
