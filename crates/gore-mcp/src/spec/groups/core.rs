//! Configuration, the catalog/reflection pipeline, and project scaffolding.
//!
//! Two of these three groups are synthetic. `gore` exposes eleven commands at its top level that
//! have no subcommand of their own; giving each a tool would make the least-reached half of the
//! CLI take up half the tool list. They are bundled here along the lines the guide already draws:
//! the catalog pipeline is one page (`catalogs-and-models`), project scaffolding is another.
//!
//! Every `summary` and `help` string below is copied verbatim from the corresponding clap doc
//! comment so that a reviewer can diff this file against `crates/gore/src/main.rs` by eye.

use crate::spec::{
    ArgForm::{Long, Positional},
    ArgKind::{Enum, Path, Str},
    ArgSpec, CommandSpec, GroupShape, GroupSpec, Safety, T_FAST, T_NORMAL,
};

/// The single validated config key, from the `ConfigKey` value enum. clap renders variants in
/// kebab-case, so `GamePath` reaches the command line as `game-path`.
const CONFIG_KEYS: &[&str] = &["game-path"];

/// From the `CatalogKind` value enum.
const CATALOG_KINDS: &[&str] = &["item", "npc", "knowledge"];

// ---------------------------------------------------------------------------------------------
// gore_config
// ---------------------------------------------------------------------------------------------

const CONFIG_KEY: ArgSpec = ArgSpec::new(
    "key",
    Positional { order: 0 },
    Enum(CONFIG_KEYS),
    "Config key to act on.",
    true,
);

const CONFIG_SET_ARGS: &[ArgSpec] = &[
    CONFIG_KEY,
    ArgSpec::new(
        "value",
        Positional { order: 1 },
        Str,
        "New value. A game path is stored absolutized, so a relative value is resolved now rather \
         than against whatever directory a later command runs from.",
        true,
    ),
];

const CONFIG_KEY_ONLY: &[ArgSpec] = &[CONFIG_KEY];

const CONFIG_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("set", "Set a config value", CONFIG_SET_ARGS, Safety::write(), T_FAST)
        .guide("getting-started"),
    CommandSpec::new(
        "get",
        "Print a single config value (exit non-zero if unset)",
        CONFIG_KEY_ONLY,
        Safety::read(),
        T_FAST,
    )
    .guide("getting-started"),
    CommandSpec::new("unset", "Clear a single config value", CONFIG_KEY_ONLY, Safety::write(), T_FAST)
        .guide("getting-started"),
    CommandSpec::new(
        "list",
        "Print all config values and, for the game path, the resolved root + source",
        &[],
        Safety::read(),
        T_FAST,
    )
    .guide("getting-started"),
    CommandSpec::new("path", "Print the path of the config.json file", &[], Safety::read(), T_FAST),
    CommandSpec::new(
        "detect",
        "Auto-detect the game via Steam and save it as game-path",
        &[],
        Safety::write(),
        T_FAST,
    )
    .guide("getting-started"),
];

pub const CONFIG: GroupSpec = GroupSpec {
    tool: "gore_config",
    title: "gore config",
    cli: "config",
    summary: "Read and write the shared per-user configuration — above all the game install path, \
              which almost every other command falls back to.",
    shape: GroupShape::Nested,
    commands: CONFIG_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_catalog  (synthetic: the reflection + catalog pipeline)
// ---------------------------------------------------------------------------------------------

const DUMP_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "sdk_dir",
        Positional { order: 0 },
        Path,
        "Path to the CXXHeaderDump/ directory",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output model.json path", true),
];

const STUBS_ARGS: &[ArgSpec] = &[
    ArgSpec::new("model", Positional { order: 0 }, Path, "Path to model.json", true),
    ArgSpec::new("out", Long("out"), Path, "Output directory for .lua stub files", true),
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Only emit classes whose name starts with PREFIX",
        false,
    ),
];

const CATALOG_ARGS: &[ArgSpec] = &[
    ArgSpec::new("kind", Long("kind"), Enum(CATALOG_KINDS), "Catalog kind to generate", true),
    ArgSpec::new("dump", Positional { order: 0 }, Path, "Path to UE4SS_ObjectDump.txt", true),
    ArgSpec::new(
        "script_cache",
        Long("script-cache"),
        Path,
        "Shipping script cache used to enrich knowledge captions (only affects --kind knowledge)",
        false,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output catalog JSON path", true),
];

const STORY_CATALOG_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "exe",
        Long("exe"),
        Path,
        "Exact game executable used by this installed generation.",
        true,
    ),
    ArgSpec::new(
        "cache",
        Long("cache"),
        Path,
        "Exact Shipping precompiled AngelScript cache.",
        true,
    ),
    ArgSpec::new("binds", Long("binds"), Path, "Exact Binds precompiled AngelScript cache.", true),
    ArgSpec::new("out", Long("out"), Path, "Output story_catalog.v1 JSON path.", true),
];

const GUI_MODEL_ARGS: &[ArgSpec] = &[
    ArgSpec::new("model", Long("model"), Path, "Path to model.json (output of `dump`)", true),
    ArgSpec::new("catalog", Long("catalog"), Path, "Path to item_catalog.json", true),
    ArgSpec::new("out", Long("out"), Path, "Output GUI model JSON path", true),
];

const SYNC_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "dump",
        Long("dump"),
        Path,
        "Path to game_data.json (output of the gore-dump mod)",
        true,
    ),
    ArgSpec::new(
        "catalog",
        Long("catalog"),
        Path,
        "Path to item_catalog.json (the item allow-list)",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output GUI model JSON path", true),
];

const DUMP_MOD_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "model",
        Long("model"),
        Path,
        "Path to model.json (field schema; output of `dump`+`gui-model`)",
        true,
    ),
    ArgSpec::new(
        "catalog",
        Long("catalog"),
        Path,
        "Path to item_catalog.json (the item allow-list)",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Mods directory to write the gore-dump/ folder into", true),
];

const CATALOG_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "dump",
        "Parse UE4SS SDK dump into gore-reflect reflection model JSON",
        DUMP_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "stubs",
        "Generate LuaLS/EmmyLua type stubs from model.json",
        STUBS_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "catalog",
        "Generate a catalog JSON from a UE4SS object dump",
        CATALOG_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "story-catalog",
        "Build a strict, generation-sealed NPC and quest-parent catalog.",
        STORY_CATALOG_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "gui-model",
        "Convert a gore reflection model into a gore-mod GUI shape JSON",
        GUI_MODEL_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "sync",
        "Refresh the gore-mod GUI model from a runtime game-data dump (with real default values), \
         produced in-game by the gore-dump UE4SS mod",
        SYNC_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "dump-mod",
        "Generate the gore-dump UE4SS mod (reads live CDO stat values in-game -> \
         gore_game_data.json, the input to `sync`)",
        DUMP_MOD_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("catalogs-and-models"),
];

pub const CATALOG: GroupSpec = GroupSpec {
    tool: "gore_catalog",
    title: "gore catalog pipeline",
    cli: "",
    summary: "The reflection and catalog pipeline: turn a UE4SS SDK/object dump into the \
              model.json, catalogs and GUI shapes the rest of the toolkit reads. These are \
              regeneration steps, run once per game build, not per mod.",
    shape: GroupShape::Flat,
    commands: CATALOG_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_project  (synthetic: making and shipping a UE4SS Lua mod)
// ---------------------------------------------------------------------------------------------

const SCAFFOLD_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "mod_name",
        Positional { order: 0 },
        Str,
        "Mod name (becomes the directory name under mods-dir). Must be a single path component.",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Mods directory (e.g. ue4ss/Mods/)", true),
];

const GEN_ARGS: &[ArgSpec] = &[
    ArgSpec::new("overrides", Positional { order: 0 }, Path, "Path to overrides.toml", true),
    ArgSpec::new("out", Long("out"), Path, "Mods directory to write the mod folder into", true),
    ArgSpec::new(
        "model",
        Long("model"),
        Path,
        "Path to model.json for validation (optional; skips validation if absent)",
        false,
    ),
];

const PACKAGE_ARGS: &[ArgSpec] = &[
    ArgSpec::new("mod_dir", Positional { order: 0 }, Path, "Path to the mod directory", true),
    ArgSpec::new("out", Long("out"), Path, "Output zip path", true),
];

const DEPLOY_SHARED_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "src",
        Long("src"),
        Path,
        "Source shared/ dir. Defaults to a copy located relative to the gore executable.",
        false,
    ),
    ArgSpec::new(
        "game",
        Long("game"),
        Path,
        "Game install root (the folder containing G1R/). Falls back to the configured game path, \
         then Steam auto-detect.",
        false,
    )
    .with_default("the configured game path, then Steam auto-detect"),
];

const PROJECT_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "scaffold",
        "Create a UE4SS Lua mod skeleton directory",
        SCAFFOLD_ARGS,
        Safety::write(),
        T_FAST,
    )
    .guide("items"),
    CommandSpec::new(
        "gen",
        "Compile overrides.toml into a UE4SS Lua mod",
        GEN_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("items"),
    CommandSpec::new(
        "package",
        "Zip a mod folder into distributable UE4SS layout",
        PACKAGE_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("items"),
    // The only command in this group that reaches into the installation: it copies the shared Lua
    // SDK into the game's ue4ss/Mods/shared.
    CommandSpec::new(
        "deploy-shared",
        "Deploy the gore-lua shared SDK into the game's ue4ss/Mods/shared.",
        DEPLOY_SHARED_ARGS,
        Safety::mutate(),
        T_NORMAL,
    )
    .guide("items"),
];

pub const PROJECT: GroupSpec = GroupSpec {
    tool: "gore_project",
    title: "gore Lua mod project",
    cli: "",
    summary: "Author and ship a UE4SS Lua mod: scaffold a skeleton, compile overrides.toml into \
              Lua, zip it for distribution, and install the shared Lua SDK the generated mods need.",
    shape: GroupShape::Flat,
    commands: PROJECT_COMMANDS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_group_sizes_match_the_cli() {
        assert_eq!(CONFIG.commands.len(), 6);
        assert_eq!(CATALOG.commands.len(), 7);
        assert_eq!(PROJECT.commands.len(), 4);
    }

    #[test]
    fn deploy_shared_is_the_only_installation_mutating_command_here() {
        let mutating: Vec<&str> = [CONFIG, CATALOG, PROJECT]
            .iter()
            .flat_map(|group| group.commands.iter())
            .filter(|command| command.safety.worst_case().needs_write_permission())
            .map(|command| command.sub)
            .collect();
        assert_eq!(mutating, vec!["deploy-shared"]);
    }
}
