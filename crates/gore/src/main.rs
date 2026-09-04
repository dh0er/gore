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
        /// Shipping script cache used to enrich knowledge captions
        #[arg(long, value_name = "CACHE")]
        script_cache: Option<PathBuf>,
        /// Output catalog JSON path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Build a strict, generation-sealed NPC and quest-parent catalog.
    StoryCatalog {
        /// Exact game executable used by this installed generation.
        #[arg(long)]
        exe: PathBuf,
        /// Exact Shipping precompiled AngelScript cache.
        #[arg(long)]
        cache: PathBuf,
        /// Exact Binds precompiled AngelScript cache.
        #[arg(long)]
        binds: PathBuf,
        /// Output story_catalog.v1 JSON path.
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Build the named-location catalog from the game's InteractionSpots.json
    LocationCatalog {
        /// Path to G1R\Script\Map\MainMap\InteractionSpots.json (default: inside the resolved game)
        source: Option<PathBuf>,
        /// Output location_catalog.json path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
    /// Look named locations up in the bundled catalog (no game install needed)
    Location {
        #[command(subcommand)]
        action: cmd::location::LocationAction,
    },
    /// Read and safely author the game's dialog trees, topics, and complete conversations
    Dialog {
        #[command(subcommand)]
        action: cmd::dialog::DialogAction,
    },
    /// Read the game's characters, and author bounded changes to them
    Npc {
        #[command(subcommand)]
        action: cmd::npc::NpcAction,
    },
    /// Search the bundled catalogs and the effect register: class names, ids, and what they do
    Find {
        /// Words to search for; several words all have to match, so no quoting is needed
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// Keep only one id namespace (e.g. item, npc, knowledge, texture, loc)
        #[arg(long)]
        domain: Option<String>,
        /// Max hits to print. The result says how many matched when it stops here
        #[arg(long, default_value_t = cmd::find::DEFAULT_MAX)]
        max: usize,
        /// Emit one JSON document instead of the human-readable blocks
        #[arg(long)]
        json: bool,
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
        /// Game install root (the folder containing G1R/). Falls back to the
        /// configured game path, then Steam auto-detect.
        #[arg(long)]
        game: Option<std::path::PathBuf>,
    },
    /// AngelScript precompiled-cache tooling (decode/emit/splice/decompile).
    As {
        #[command(subcommand)]
        cmd: cmd::as_cache::AsCmd,
    },
    /// Extract, inspect, copy-on-write patch, and pack cooked DataAssets.
    Asset {
        #[command(subcommand)]
        action: cmd::asset::AssetAction,
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
    /// Inspect, extract, and copy-on-write edit voice-over ZIP archives
    Voice {
        #[command(subcommand)]
        action: cmd::voice::VoiceAction,
    },
    /// Build, inspect, deploy, and undeploy a unified mod bundle
    Mod {
        #[command(subcommand)]
        action: ModAction,
    },
    /// Extract/replace game textures (Gothic 1 Remake, UE5 IoStore)
    Texture {
        #[command(subcommand)]
        action: cmd::texture::TextureAction,
    },
    /// Multi-mod manager: library, loadout, readiness/recovery, apply, status and reset
    Mgr {
        #[command(subcommand)]
        action: cmd::mgr::MgrAction,
    },
    /// Read/write the shared per-user config (game path, …)
    Config {
        #[command(subcommand)]
        action: cmd::config::ConfigAction,
    },
    /// Serve this toolkit over the Model Context Protocol (stdio JSON-RPC) for AI agents
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// Render the built-in guide for offline reading
    Guide {
        #[command(subcommand)]
        action: GuideAction,
    },
    /// Diagnose the setup: what is installed, what is deployed, and what is wrong with it
    Doctor {
        /// Game install root (the folder containing G1R/). Falls back to the configured
        /// game path, then Steam auto-detect
        #[arg(long)]
        game: Option<PathBuf>,
        /// Emit one JSON document instead of the human-readable report
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum GuideAction {
    /// Rank guide and reference sections against a query, best first — the ranking the MCP server serves
    Search {
        /// Words to search for; several words are one query, so no quoting is needed
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// Maximum number of hits. Clamped to 25, as in the MCP tool
        #[arg(long, default_value_t = cmd::guide::DEFAULT_SEARCH_LIMIT, value_name = "N")]
        limit: usize,
    },
    /// Write the whole guide as one self-contained HTML file — sidebar, filter, no external assets
    Html {
        /// Output path
        #[arg(short = 'o', long, default_value = "guide.html")]
        out: PathBuf,
        /// Commit or branch that links leaving the guide tree are pinned to
        #[arg(long, default_value = cmd::guide::DEFAULT_REPO_REF, value_name = "REF")]
        repo_ref: String,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Run the MCP server on stdin/stdout. Speaks JSON-RPC; it is not an interactive shell.
    Serve {
        /// Pre-approve commands that change the game installation or overwrite an existing file,
        /// so they run without confirming with you. Without this they are still allowed — the
        /// agent's client asks you first. Use it where nobody is watching (CI, batch runs).
        /// Also settable as GORE_MCP_ALLOW_WRITE=1, which is the only route a plugin has
        #[arg(long)]
        allow_write: bool,
        /// Pre-approve compiler backends that may launch the game (`game` and
        /// `standalone-then-game`). Explicit strict `standalone` is offline and never needs this.
        /// Also settable as GORE_MCP_ALLOW_GAME_LAUNCH=1
        #[arg(long)]
        allow_game_launch: bool,
        /// Never ask, and refuse anything that would need confirming. The strict posture, for a
        /// server exposed to an agent whose calls nobody reviews.
        /// Also settable as GORE_MCP_NO_CONSENT_PROMPTS=1
        #[arg(long)]
        no_consent_prompts: bool,
        /// Override every per-command wall-clock cap, in seconds (0 keeps the defaults)
        #[arg(long, default_value_t = 0, value_name = "SECS")]
        timeout_secs: u64,
        /// Cap on captured stdout per command, in KiB (0 keeps the default)
        #[arg(long, default_value_t = 256, value_name = "KIB")]
        max_output_kib: usize,
    },
    /// Print the tool definitions the server would advertise, then exit
    Tools,
}

#[derive(Subcommand)]
enum ModAction {
    /// Build a bundle dir from a BuildSpec JSON
    Build {
        /// Path to the build spec JSON; asset paths inside it resolve against its directory
        #[arg(long)]
        spec: PathBuf,
        /// Output directory (the bundle is written to <out>/<mod-name>)
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// Path to model.json for validation (optional; skips validation if absent)
        #[arg(long)]
        model: Option<PathBuf>,
    },
    /// Validate and summarize a built GORE bundle without importing or deploying it
    Inspect {
        /// Bundle directory or ZIP containing one GORE bundle
        bundle: PathBuf,
        /// Emit one bounded JSON report instead of human-readable text
        #[arg(long)]
        json: bool,
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
    /// List the .bank files the configured install carries, with each one's sample count
    Banks {
        /// Game root (the folder containing G1R/)
        #[arg(long)]
        game: Option<PathBuf>,
        /// Emit one JSON document instead of the human-readable table
        #[arg(long)]
        json: bool,
        /// Override the bank encryption key (defaults to the Gothic 1 Remake key)
        #[arg(long)]
        key: Option<String>,
    },
    /// List a bank's samples (name, codec, sample rate, channels, duration)
    List {
        /// Path to a .bank file
        #[arg(long)]
        bank: PathBuf,
        /// Keep only sample names containing this substring (case-insensitive)
        #[arg(long)]
        filter: Option<String>,
        /// Max samples to print. The result states how many matched when it stops here; 0 lists
        /// nothing and reports only the counts
        #[arg(long, default_value_t = 100)]
        max: usize,
        /// Emit one JSON document instead of the human-readable table
        #[arg(long)]
        json: bool,
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
        /// Extract every sample whose name contains this substring (case-insensitive)
        #[arg(long)]
        filter: Option<String>,
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
    /// gore/loc_catalog.json (used by gore-save and gore-mod too)
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
        /// Path to AlkimiaLocalization_*.lcache, the game dir, or a Steam library (else
        /// auto-detect)
        #[arg(long)]
        lcache: Option<PathBuf>,
        /// Output loc_catalog.json
        #[arg(short = 'o', long)]
        out: PathBuf,
        /// Keep empty values / ids with no text
        #[arg(long)]
        keep_empty: bool,
    },
    /// Apply {id:{language:value}} edits and re-encrypt the .lcache
    Import {
        /// Path to the .lcache to edit, the game dir, or a Steam library (else auto-detect)
        #[arg(long)]
        lcache: Option<PathBuf>,
        /// Path to edits JSON ({id:{language:value}})
        #[arg(long)]
        edits: PathBuf,
        /// Output .lcache (defaults to overwriting the cache that was read)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        /// Add ids absent from the input .lcache (default: reject them)
        #[arg(long)]
        add_missing: bool,
    },
}

// Clap constructs the complete nested command graph before parsing. The GORE CLI intentionally
// exposes many large command families, and debug Windows binaries can exceed the platform's 1 MiB
// main-thread stack while building that graph. Give parsing and dispatch explicit headroom so
// adding an unrelated subcommand cannot make every invocation fail before argument parsing.
const CLI_THREAD_STACK_BYTES: usize = 8 * 1024 * 1024;

fn main() {
    let worker = std::thread::Builder::new()
        .name("gore-cli".into())
        .stack_size(CLI_THREAD_STACK_BYTES)
        .spawn(run_cli)
        .unwrap_or_else(|error| {
            eprintln!("error: failed to start CLI worker: {error}");
            std::process::exit(1);
        });
    if let Err(payload) = worker.join() {
        std::panic::resume_unwind(payload);
    }
}

fn run_cli() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Dump { sdk_dir, out } => cmd::dump::run(sdk_dir, out),
        Commands::Stubs { model, out, filter } => cmd::stubs::run(model, out, filter),
        Commands::Catalog {
            kind,
            dump,
            script_cache,
            out,
        } => cmd::catalog::run(kind, dump, script_cache, out),
        Commands::StoryCatalog {
            exe,
            cache,
            binds,
            out,
        } => cmd::story_catalog::run(exe, cache, binds, out),
        Commands::LocationCatalog { source, out } => cmd::location_catalog::run(source, out),
        Commands::Location { action } => cmd::location::run(action),
        Commands::Dialog { action } => cmd::dialog::run(action),
        Commands::Npc { action } => cmd::npc::run(action),
        Commands::Find {
            query,
            domain,
            max,
            json,
        } => cmd::find::run(query, domain, max, json),
        Commands::GuiModel {
            model,
            catalog,
            out,
        } => cmd::gui_model::run(model, catalog, out),
        Commands::Sync { dump, catalog, out } => cmd::sync::run(dump, catalog, out),
        Commands::DumpMod {
            model,
            catalog,
            out,
        } => cmd::dump_mod::run(model, catalog, out),
        Commands::Loc { action } => match action {
            LocAction::Extract { lcache, yes } => cmd::loc::extract(lcache, yes),
            LocAction::Status => cmd::loc::status(),
            LocAction::Export {
                lcache,
                out,
                keep_empty,
            } => cmd::loc::export(lcache, out, keep_empty),
            LocAction::Import {
                lcache,
                edits,
                out,
                add_missing,
            } => cmd::loc::import(lcache, edits, out, add_missing),
        },
        Commands::Scaffold { mod_name, out } => cmd::scaffold::run(mod_name, out),
        Commands::Gen {
            overrides,
            out,
            model,
        } => cmd::gen::run(overrides, out, model),
        Commands::DeployShared { src, game } => cmd::deploy_shared::run(src, game),
        Commands::As { cmd } => cmd::as_cache::run(cmd),
        Commands::Asset { action } => cmd::asset::run(action),
        Commands::Package { mod_dir, out } => cmd::package::run(mod_dir, out),
        Commands::Audio { action } => match action {
            AudioAction::Banks { game, json, key } => cmd::audio::banks(game, json, key),
            AudioAction::List {
                bank,
                filter,
                max,
                json,
                key,
            } => cmd::audio::list(bank, filter, max, json, key),
            AudioAction::Extract {
                bank,
                out,
                sample,
                filter,
                key,
            } => cmd::audio::extract(bank, out, sample, filter, key),
            AudioAction::Replace {
                map,
                bank,
                out,
                key,
            } => cmd::audio::replace(map, bank, out, key),
            AudioAction::Restore { bank } => cmd::audio::restore(bank),
            AudioAction::ExportPatch { map, out } => cmd::audio::export_patch(map, out),
            AudioAction::ApplyPatch {
                patch,
                bank,
                out,
                key,
            } => cmd::audio::apply_patch(patch, bank, out, key),
        },
        Commands::Voice { action } => cmd::voice::run(action),
        Commands::Mod { action } => match action {
            ModAction::Build { spec, out, model } => cmd::modcmd::build(spec, out, model),
            ModAction::Inspect { bundle, json } => cmd::modcmd::inspect(bundle, json),
            ModAction::Deploy { bundle, game } => cmd::modcmd::deploy(bundle, game),
            ModAction::Undeploy { game } => cmd::modcmd::undeploy(game),
        },
        Commands::Texture { action } => cmd::texture::run(action),
        Commands::Mgr { action } => cmd::mgr::run(action),
        Commands::Config { action } => cmd::config::run(action),
        Commands::Mcp { action } => match action {
            McpAction::Serve {
                allow_write,
                allow_game_launch,
                no_consent_prompts,
                timeout_secs,
                max_output_kib,
            } => cmd::mcp::serve(cmd::mcp::ServeOptions {
                allow_write,
                allow_game_launch,
                no_consent_prompts,
                timeout_secs,
                max_output_kib,
            }),
            McpAction::Tools => cmd::mcp::tools(),
        },
        Commands::Guide { action } => match action {
            GuideAction::Search { query, limit } => cmd::guide::search(&query, limit),
            GuideAction::Html { out, repo_ref } => cmd::guide::html_file(out, &repo_ref),
        },
        Commands::Doctor { game, json } => cmd::doctor::run(game, json),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod story_catalog_cli_tests {
    use super::*;

    #[test]
    fn story_catalog_requires_all_generation_inputs_and_new_output() {
        let cli = Cli::try_parse_from([
            "gore",
            "story-catalog",
            "--exe",
            "G1R.exe",
            "--cache",
            "Shipping.Cache",
            "--binds",
            "Binds.Cache",
            "--out",
            "story_catalog.v1.json",
        ])
        .unwrap();
        let Commands::StoryCatalog {
            exe,
            cache,
            binds,
            out,
        } = cli.command
        else {
            panic!("expected story-catalog command");
        };
        assert_eq!(exe, PathBuf::from("G1R.exe"));
        assert_eq!(cache, PathBuf::from("Shipping.Cache"));
        assert_eq!(binds, PathBuf::from("Binds.Cache"));
        assert_eq!(out, PathBuf::from("story_catalog.v1.json"));

        assert!(Cli::try_parse_from([
            "gore",
            "story-catalog",
            "--exe",
            "G1R.exe",
            "--cache",
            "Shipping.Cache",
            "--out",
            "story_catalog.v1.json",
        ])
        .is_err());
    }
}

#[cfg(test)]
mod catalog_cli_tests {
    use super::*;

    #[test]
    fn knowledge_catalog_accepts_optional_script_cache() {
        let cli = Cli::try_parse_from([
            "gore",
            "catalog",
            "--kind",
            "knowledge",
            "UE4SS_ObjectDump.txt",
            "--script-cache",
            "PrecompiledScript_Shipping.Cache",
            "--out",
            "knowledge_catalog.json",
        ])
        .unwrap();
        let Commands::Catalog {
            kind: cmd::catalog::CatalogKind::Knowledge,
            dump,
            script_cache,
            out,
        } = cli.command
        else {
            panic!("expected knowledge catalog command");
        };
        assert_eq!(dump, PathBuf::from("UE4SS_ObjectDump.txt"));
        assert_eq!(
            script_cache,
            Some(PathBuf::from("PrecompiledScript_Shipping.Cache"))
        );
        assert_eq!(out, PathBuf::from("knowledge_catalog.json"));
    }
}

#[cfg(test)]
mod loc_cli_tests {
    use super::*;

    fn parse_import_add_missing(extra: &[&str]) -> bool {
        let mut args = vec![
            "gore",
            "loc",
            "import",
            "--lcache",
            "base.lcache",
            "--edits",
            "edits.json",
        ];
        args.extend_from_slice(extra);
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Loc {
            action: LocAction::Import { add_missing, .. },
        } = cli.command
        else {
            panic!("expected loc import command");
        };
        add_missing
    }

    #[test]
    fn loc_import_add_missing_is_opt_in() {
        assert!(!parse_import_add_missing(&[]));
        assert!(parse_import_add_missing(&["--add-missing"]));
    }
}

#[cfg(test)]
mod voice_cli_tests {
    use super::*;

    #[test]
    fn voice_list_and_index_alias_parse_with_json() {
        for command in ["list", "index"] {
            let cli = Cli::try_parse_from([
                "gore",
                "voice",
                command,
                "--archive",
                "voices.zip",
                "--json",
            ])
            .unwrap();
            assert!(matches!(
                cli.command,
                Commands::Voice {
                    action: cmd::voice::VoiceAction::List { json: true, .. }
                }
            ));
        }
    }

    #[test]
    fn voice_list_bounds_default_to_a_hundred_entries_and_hide_directories() {
        // The bound only protects anybody if it applies to the call nobody thought about, so pin
        // the defaults here rather than leaving them to a `default_value_t` that can be edited
        // without anything noticing.
        let cli =
            Cli::try_parse_from(["gore", "voice", "list", "--archive", "voices.zip"]).unwrap();
        let Commands::Voice {
            action:
                cmd::voice::VoiceAction::List {
                    filter,
                    max,
                    directories,
                    ..
                },
        } = cli.command
        else {
            panic!("expected voice list command");
        };
        assert_eq!(max, 100);
        assert_eq!(filter, None);
        assert!(!directories);
    }

    #[test]
    fn voice_entry_selector_requires_exactly_one_mode() {
        let base = [
            "gore",
            "voice",
            "extract",
            "--archive",
            "voices.zip",
            "--out",
            "extracted",
        ];
        assert!(Cli::try_parse_from(base).is_err());

        let mut basename = base.to_vec();
        basename.extend(["--basename", "line.ogg"]);
        assert!(Cli::try_parse_from(basename).is_ok());

        let mut path = base.to_vec();
        path.extend(["--path", "NPC/Line.ogg"]);
        assert!(Cli::try_parse_from(path).is_ok());

        let mut both = base.to_vec();
        both.extend(["--basename", "line.ogg", "--path", "NPC/Line.ogg"]);
        assert!(Cli::try_parse_from(both).is_err());
    }

    #[test]
    fn voice_mutations_require_a_new_output_path() {
        let no_output = [
            "gore",
            "voice",
            "add",
            "--archive",
            "voices.zip",
            "--path",
            "NPC/New.ogg",
            "--ogg",
            "new.ogg",
        ];
        assert!(Cli::try_parse_from(no_output).is_err());
    }

    #[test]
    fn voice_apply_manifest_and_alias_parse() {
        for command in ["apply-manifest", "apply"] {
            let cli = Cli::try_parse_from([
                "gore",
                "voice",
                command,
                "--archive",
                "voices.zip",
                "--manifest",
                "voice-manifest.json",
                "--out",
                "modded.zip",
            ])
            .unwrap();
            assert!(matches!(
                cli.command,
                Commands::Voice {
                    action: cmd::voice::VoiceAction::ApplyManifest { .. }
                }
            ));
        }

        assert!(Cli::try_parse_from([
            "gore",
            "voice",
            "apply-manifest",
            "--archive",
            "voices.zip",
            "--manifest",
            "voice-manifest.json",
        ])
        .is_err());
    }
}

#[cfg(test)]
mod as_cli_tests {
    use super::*;

    fn parse_extract_remap_allow_new_symbols(extra: &[&str]) -> bool {
        let mut args = vec![
            "gore",
            "as",
            "extract-remap",
            "regen.cache",
            "MyModule",
            "base.cache",
            "--out",
            "mini.cache",
        ];
        args.extend_from_slice(extra);
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::As {
            cmd:
                cmd::as_cache::AsCmd::ExtractRemap {
                    allow_new_symbols, ..
                },
        } = cli.command
        else {
            panic!("expected as extract-remap command");
        };
        allow_new_symbols
    }

    #[test]
    fn as_extract_remap_new_symbols_are_opt_in() {
        assert!(!parse_extract_remap_allow_new_symbols(&[]));
        assert!(parse_extract_remap_allow_new_symbols(&[
            "--allow-new-symbols"
        ]));
    }

    fn parse_compile_module(
        extra: &[&str],
    ) -> (String, String, bool, cmd::as_cache::AsCompilerBackendV1) {
        let mut args = vec![
            "gore",
            "as",
            "compile-module",
            "--op",
            "add",
            "--module",
            "GoreMods.Example",
            "--rel-path",
            "GoreMods/Example.as",
            "--source",
            "Example.as",
            "--work-dir",
            "work",
            "--out",
            "Example.mini.Cache",
        ];
        args.extend_from_slice(extra);
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::As {
            cmd:
                cmd::as_cache::AsCmd::CompileModule {
                    op,
                    module,
                    allow_new_symbols,
                    compiler,
                    ..
                },
        } = cli.command
        else {
            panic!("expected as compile-module command");
        };
        (op, module, allow_new_symbols, compiler.backend)
    }

    #[test]
    fn as_compile_module_defaults_to_standalone_with_visible_game_fallback() {
        assert_eq!(
            parse_compile_module(&[]),
            (
                "add".into(),
                "GoreMods.Example".into(),
                false,
                cmd::as_cache::AsCompilerBackendV1::StandaloneThenGame,
            )
        );
        assert!(parse_compile_module(&["--allow-new-symbols"]).2);
        assert_eq!(
            parse_compile_module(&["--backend", "standalone"]).3,
            cmd::as_cache::AsCompilerBackendV1::Standalone
        );
        assert_eq!(
            parse_compile_module(&["--backend", "game"]).3,
            cmd::as_cache::AsCompilerBackendV1::Game
        );
    }

    #[test]
    fn as_compile_module_product_receipt_needs_no_manual_package_paths() {
        let args = [
            "gore",
            "as",
            "compile-module",
            "--op",
            "add",
            "--module",
            "M",
            "--rel-path",
            "M.as",
            "--source",
            "M.as",
            "--work-dir",
            "work",
            "--out",
            "M.cache",
            "--generation-receipt",
            "module.receipt.json",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::As {
            cmd: cmd::as_cache::AsCmd::CompileModule { compiler, .. },
        } = cli.command
        else {
            panic!("expected as compile-module command");
        };
        assert_eq!(
            compiler.generation_receipt,
            Some(PathBuf::from("module.receipt.json"))
        );
        assert!(compiler.standalone_sidecar.is_none());
        assert!(compiler.compiler_profile_manifest.is_none());
    }

    #[test]
    fn as_compile_module_keeps_manual_packages_in_a_complete_development_override() {
        let base = [
            "gore",
            "as",
            "compile-module",
            "--op",
            "add",
            "--module",
            "M",
            "--rel-path",
            "M.as",
            "--source",
            "M.as",
            "--work-dir",
            "work",
            "--out",
            "M.cache",
        ];
        let mut incomplete = base.to_vec();
        incomplete.extend_from_slice(&["--development-standalone-sidecar", "sidecar.exe"]);
        assert!(Cli::try_parse_from(incomplete).is_err());

        let mut complete = base.to_vec();
        complete.extend_from_slice(&[
            "--backend",
            "standalone",
            "--development-standalone-sidecar",
            "sidecar.exe",
            "--development-standalone-sidecar-sha256",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--development-compiler-profile-manifest",
            "profile.json",
            "--development-compiler-profile-root",
            "profile-root",
            "--development-standalone-scratch-root",
            "scratch",
        ]);
        assert!(Cli::try_parse_from(complete.clone()).is_ok());
        complete.extend_from_slice(&["--generation-receipt", "receipt.json"]);
        assert!(Cli::try_parse_from(complete).is_err());

        let mut legacy_product_flags = base.to_vec();
        legacy_product_flags.extend_from_slice(&["--standalone-sidecar", "sidecar.exe"]);
        assert!(Cli::try_parse_from(legacy_product_flags).is_err());
    }

    #[test]
    fn as_compile_module_rejects_unknown_operation() {
        let args = [
            "gore",
            "as",
            "compile-module",
            "--op",
            "remove",
            "--module",
            "M",
            "--rel-path",
            "M.as",
            "--source",
            "M.as",
            "--work-dir",
            "work",
            "--out",
            "M.cache",
        ];
        assert!(Cli::try_parse_from(args).is_err());
    }

    #[test]
    fn as_compile_exposes_diagnostics_controls() {
        let cli = Cli::try_parse_from([
            "gore",
            "as",
            "compile",
            "scripts",
            "--out",
            "compiled.cache",
            "--work-dir",
            "compiler-work",
            "--diagnostics-hook",
            "custom.dll",
            "--diagnostics-inject-delay-ms",
            "2500",
        ])
        .unwrap();
        let Commands::As {
            cmd:
                cmd::as_cache::AsCmd::Compile {
                    no_diagnostics,
                    diagnostics_hook,
                    diagnostics_inject_delay_ms,
                    ..
                },
        } = cli.command
        else {
            panic!("expected as compile command");
        };
        assert!(!no_diagnostics);
        assert_eq!(diagnostics_hook, Some(PathBuf::from("custom.dll")));
        assert_eq!(diagnostics_inject_delay_ms, 2500);

        assert!(Cli::try_parse_from([
            "gore",
            "as",
            "compile",
            "scripts",
            "--out",
            "compiled.cache",
            "--work-dir",
            "compiler-work",
            "--no-diagnostics",
            "--diagnostics-hook",
            "custom.dll",
        ])
        .is_err());
        assert!(Cli::try_parse_from([
            "gore",
            "as",
            "compile",
            "scripts",
            "--out",
            "compiled.cache",
            "--work-dir",
            "compiler-work",
            "--diagnostics-inject-delay-ms",
            "30001",
        ])
        .is_err());
    }

    #[test]
    fn as_compile_requires_complete_source_output_and_workspace_and_exposes_product_backend() {
        let cli = Cli::try_parse_from([
            "gore",
            "as",
            "compile",
            "scripts",
            "--out",
            "compiled.cache",
            "--work-dir",
            "compiler-work",
            "--backend",
            "standalone-then-game",
            "--generation-receipt",
            "compiled.receipt.json",
        ])
        .unwrap();
        let Commands::As {
            cmd:
                cmd::as_cache::AsCmd::Compile {
                    src,
                    out,
                    work_dir,
                    compiler,
                    ..
                },
        } = cli.command
        else {
            panic!("expected as compile command");
        };
        assert_eq!(src, PathBuf::from("scripts"));
        assert_eq!(out, PathBuf::from("compiled.cache"));
        assert_eq!(work_dir, PathBuf::from("compiler-work"));
        assert_eq!(
            compiler.backend,
            cmd::as_cache::AsCompilerBackendV1::StandaloneThenGame
        );
        assert_eq!(
            compiler.generation_receipt,
            Some(PathBuf::from("compiled.receipt.json"))
        );

        for args in [
            vec!["gore", "as", "compile", "scripts", "--work-dir", "work"],
            vec!["gore", "as", "compile", "scripts", "--out", "out.cache"],
            vec![
                "gore",
                "as",
                "compile",
                "--out",
                "out.cache",
                "--work-dir",
                "work",
            ],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn as_compile_defaults_to_standalone_with_visible_game_fallback() {
        let cli = Cli::try_parse_from([
            "gore",
            "as",
            "compile",
            "scripts",
            "--out",
            "compiled.cache",
            "--work-dir",
            "compiler-work",
        ])
        .unwrap();
        let Commands::As {
            cmd: cmd::as_cache::AsCmd::Compile { compiler, .. },
        } = cli.command
        else {
            panic!("expected as compile command");
        };
        assert_eq!(
            compiler.backend,
            cmd::as_cache::AsCompilerBackendV1::StandaloneThenGame
        );
    }

    #[test]
    fn as_diagnostics_check_accepts_an_explicit_nonsteam_exe() {
        let cli = Cli::try_parse_from([
            "gore",
            "as",
            "diagnostics-check",
            "--exe",
            "D:/Custom/G1R.exe",
        ])
        .unwrap();
        let Commands::As {
            cmd: cmd::as_cache::AsCmd::DiagnosticsCheck { exe, game },
        } = cli.command
        else {
            panic!("expected as diagnostics-check command");
        };
        assert_eq!(exe, Some(PathBuf::from("D:/Custom/G1R.exe")));
        assert!(game.is_none());
    }

    #[test]
    fn as_default_sites_exposes_exact_filters_and_json() {
        let cli = Cli::try_parse_from([
            "gore",
            "as",
            "default-sites",
            "base.cache",
            "--module",
            "Items.Food",
            "--class",
            "UApple",
            "--field",
            "m_Value",
            "--json",
        ])
        .unwrap();
        let Commands::As {
            cmd:
                cmd::as_cache::AsCmd::DefaultSites {
                    cache,
                    module,
                    class,
                    field,
                    json,
                },
        } = cli.command
        else {
            panic!("expected as default-sites command");
        };
        assert_eq!(cache, PathBuf::from("base.cache"));
        assert_eq!(module.as_deref(), Some("Items.Food"));
        assert_eq!(class.as_deref(), Some("UApple"));
        assert_eq!(field.as_deref(), Some("m_Value"));
        assert!(json);
    }

    #[test]
    fn as_patch_default_requires_selector_cas_and_new_output() {
        let cli = Cli::try_parse_from([
            "gore",
            "as",
            "patch-default",
            "base.cache",
            "--selector",
            "site.json",
            "--expected-hex",
            "04000000",
            "--replacement-hex",
            "05000000",
            "--out",
            "patched.cache",
            "--json",
        ])
        .unwrap();
        let Commands::As {
            cmd:
                cmd::as_cache::AsCmd::PatchDefault {
                    cache,
                    selector,
                    expected_hex,
                    replacement_hex,
                    out,
                    json,
                },
        } = cli.command
        else {
            panic!("expected as patch-default command");
        };
        assert_eq!(cache, PathBuf::from("base.cache"));
        assert_eq!(selector, PathBuf::from("site.json"));
        assert_eq!(expected_hex, "04000000");
        assert_eq!(replacement_hex, "05000000");
        assert_eq!(out, PathBuf::from("patched.cache"));
        assert!(json);

        assert!(Cli::try_parse_from([
            "gore",
            "as",
            "patch-default",
            "base.cache",
            "--selector",
            "site.json",
            "--expected-hex",
            "04000000",
            "--replacement-hex",
            "05000000",
        ])
        .is_err());
    }
}
