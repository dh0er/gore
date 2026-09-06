//! Configuration, the catalog/reflection pipeline, the location lookup, and project scaffolding.
//!
//! Two of these four groups are synthetic. `gore` exposes twelve commands at its top level that
//! have no subcommand of their own; giving each a tool would make the least-reached half of the
//! CLI take up half the tool list. They are bundled here along the lines the guide already draws:
//! the catalog pipeline is one page (`catalogs-and-models`), project scaffolding is another.
//!
//! `gore_location` is not one of those bundles: `gore location` is a real subcommand family, and
//! it is the only tool here a model reaches for *while writing a mod* rather than while
//! regenerating data, so it keeps its own name in the tool list.
//!
//! Every `summary` and `help` string below is copied verbatim from the corresponding clap doc
//! comment so that a reviewer can diff this file against `crates/gore/src/main.rs` by eye.

use crate::spec::{
    ArgForm::{Long, Positional, PositionalRepeated, Switch},
    ArgKind::{Bool, Enum, Int, Path, Str, StrList},
    ArgSpec, CommandSpec, Derived, GroupShape, GroupSpec, JsonSupport, Safety, T_FAST, T_LONG,
    T_NORMAL,
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
    CommandSpec::new(
        "set",
        "Set a config value",
        CONFIG_SET_ARGS,
        Safety::write(),
        T_FAST,
    )
    .guide("getting-started"),
    CommandSpec::new(
        "get",
        "Print a single config value (exit non-zero if unset)",
        CONFIG_KEY_ONLY,
        Safety::read(),
        T_FAST,
    )
    .guide("getting-started"),
    CommandSpec::new(
        "unset",
        "Clear a single config value",
        CONFIG_KEY_ONLY,
        Safety::write(),
        T_FAST,
    )
    .guide("getting-started"),
    CommandSpec::new(
        "list",
        "Print all config values and, for the game path, the resolved root + source",
        &[],
        Safety::read(),
        T_FAST,
    )
    .guide("getting-started"),
    CommandSpec::new(
        "path",
        "Print the path of the config.json file",
        &[],
        Safety::read(),
        T_FAST,
    ),
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
// gore_doctor
// ---------------------------------------------------------------------------------------------

const DOCTOR_ARGS: &[ArgSpec] = &[ArgSpec::new(
    "game",
    Long("game"),
    Path,
    "Game install root (the folder containing G1R/). Falls back to the configured game path, then \
     Steam auto-detect",
    false,
)];

const DOCTOR_COMMANDS: &[CommandSpec] = &[CommandSpec::new(
    "doctor",
    "Diagnose the setup: what is installed, what is deployed, and what is wrong with it",
    DOCTOR_ARGS,
    Safety::read(),
    // Reads and hashes whatever the deploy record claims, exactly as `mgr status` does, so it
    // inherits that command's budget rather than the fast one its other checks would suggest.
    T_NORMAL,
)
.json(JsonSupport::Stdout)
.guide("getting-started")];

/// A tool of its own rather than a leaf of `gore_config`, which is nested under `gore config` and
/// would put this at `gore config doctor`. It also earns the place: this is the command to reach
/// for when a mod was built and deployed and the game shows nothing, and an agent that has to find
/// it inside another tool's subcommand list will not.
pub const DOCTOR: GroupSpec = GroupSpec {
    tool: "gore_doctor",
    title: "gore doctor",
    cli: "",
    summary: "One read-only pass over the setup, including whether the bundled standalone \
              AngelScript compiler is ready without launching the game: where the game is and where that came from, \
              whether UE4SS is installed (item and stat overrides silently do nothing without \
              it), which UE4SS mods are enabled, what is deployed, what is left over from an \
              interrupted run, and whether the shared text catalog still matches the install.",
    shape: GroupShape::Flat,
    commands: DOCTOR_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_find
// ---------------------------------------------------------------------------------------------

const FIND_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "query",
        PositionalRepeated { order: 0 },
        StrList,
        "Words to search for; several words all have to match, so no quoting is needed",
        true,
    ),
    ArgSpec::new(
        "domain",
        Long("domain"),
        Str,
        "Keep only one id namespace (e.g. item, npc, knowledge, texture, loc)",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int {
            min: Some(0),
            max: None,
        },
        "Max hits to print. The result says how many matched when it stops here",
        false,
    )
    .with_default("50"),
];

const FIND_COMMANDS: &[CommandSpec] = &[CommandSpec::new(
    "find",
    "Search the bundled catalogs and the effect register: class names, ids, and what they do",
    FIND_ARGS,
    Safety::read(),
    T_FAST,
)
.json(JsonSupport::Stdout)
.guide("find")];

/// A tool of its own, for the reason `gore_doctor` is: it is flat at the CLI, and it is the one
/// command a model reaches for *before* it knows which family owns the thing it was asked about.
/// Buried inside `gore_catalog` — a group about regenerating data once per game build — it would
/// not be found by anybody who needed it.
pub const FIND: GroupSpec = GroupSpec {
    tool: "gore_find",
    title: "gore find",
    cli: "",
    summary: "One lookup over both offline layers: the bundled item/NPC/knowledge catalogs, which \
              say what exists, and the effect register, which says what an id was observed to do \
              in game. Matches ids, categories, class paths and register text always, and display \
              names when the shared text catalog has been extracted — every result states which \
              of those two it was, because a name search that skipped its index answers \"no such \
              item\" about an item that is there.",
    shape: GroupShape::Flat,
    commands: FIND_COMMANDS,
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
    ArgSpec::new(
        "model",
        Positional { order: 0 },
        Path,
        "Path to model.json",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output directory for .lua stub files",
        true,
    ),
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Only emit classes whose name starts with PREFIX",
        false,
    ),
];

const CATALOG_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "kind",
        Long("kind"),
        Enum(CATALOG_KINDS),
        "Catalog kind to generate",
        true,
    ),
    ArgSpec::new(
        "dump",
        Positional { order: 0 },
        Path,
        "Path to UE4SS_ObjectDump.txt",
        true,
    ),
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
    ArgSpec::new(
        "binds",
        Long("binds"),
        Path,
        "Exact Binds precompiled AngelScript cache.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output story_catalog.v1 JSON path.",
        true,
    ),
];

const LOCATION_CATALOG_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "source",
        Positional { order: 0 },
        Path,
        r"Path to G1R\Script\Map\MainMap\InteractionSpots.json (default: inside the resolved game)",
        false,
    )
    .with_default("the InteractionSpots.json of the resolved game install"),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output location_catalog.json path",
        true,
    ),
];

const GUI_MODEL_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "model",
        Long("model"),
        Path,
        "Path to model.json (output of `dump`)",
        true,
    ),
    ArgSpec::new(
        "catalog",
        Long("catalog"),
        Path,
        "Path to item_catalog.json",
        true,
    ),
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
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Mods directory to write the gore-dump/ folder into",
        true,
    ),
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
        // Writes one `.lua` per class into `out`, overwriting whatever is there. The class names
        // come from the model file this layer does not read -- so the files cannot be named ahead
        // of time, but an empty or absent `out` still has nothing in them to lose.
        Safety::write().clobbers_dir(&["out"]),
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
        "location-catalog",
        "Build the named-location catalog from the game's InteractionSpots.json",
        LOCATION_CATALOG_ARGS,
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
        // Always writes the fixed `gore-dump/` folder inside the directory it is given.
        Safety::write()
            .also_writes(&[("out", Derived::Child("gore-dump"))])
            .installs_via(&["out"]),
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
// gore_location
// ---------------------------------------------------------------------------------------------

const LOCATION_RESOLVE_ARGS: &[ArgSpec] = &[ArgSpec::new(
    "name",
    Positional { order: 0 },
    Str,
    "Spot name, e.g. FP_OC_STAND_YARD_1 (case-insensitive)",
    true,
)];

const LOCATION_LIST_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "area",
        Long("area"),
        Str,
        "Keep only spots in this area code (e.g. OC). See the `areas` table of the catalog",
        false,
    ),
    ArgSpec::new(
        "prefix",
        Long("prefix"),
        Str,
        "Keep only spots whose name starts with this (e.g. FP)",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int {
            min: Some(0),
            max: None,
        },
        "Max names to print. The result says how many matched when it stops here",
        false,
    )
    .with_default("200"),
];

const LOCATION_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "resolve",
        "Look one spot name up: area, coordinates and yaw, or the near names it was not",
        LOCATION_RESOLVE_ARGS,
        Safety::read(),
        T_FAST,
    )
    .json(JsonSupport::Stdout)
    .guide("catalogs-and-models"),
    CommandSpec::new(
        "list",
        "List spot names, narrowed by area code and/or name prefix",
        LOCATION_LIST_ARGS,
        Safety::read(),
        T_FAST,
    )
    .json(JsonSupport::Stdout)
    .guide("catalogs-and-models"),
];

pub const LOCATION: GroupSpec = GroupSpec {
    tool: "gore_location",
    title: "gore location lookup",
    cli: "location",
    summary: "Check a waypoint or interaction-spot name against the catalog bundled in this \
              binary — no game install, no dump, no regeneration step. The teleport helpers \
              resolve an unknown FName to nothing at all and log nothing, so a typo is a silent \
              no-op in game; this is where it turns into an error instead.",
    shape: GroupShape::Nested,
    commands: LOCATION_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_dialog
// ---------------------------------------------------------------------------------------------

/// Shared by every leaf: which cache to read, and where the game is.
const DIALOG_CACHE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "cache",
        Long("cache"),
        Path,
        "Exact script cache to read. Defaults to the one in the resolved game install",
        false,
    )
    .with_default("the script cache of the resolved game install"),
    ArgSpec::new(
        "game",
        Long("game"),
        Path,
        "Game install root (the folder containing `G1R/`)",
        false,
    )
    .with_default("the configured game path, then Steam auto-detect"),
];

const DIALOG_LIST_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "filter",
        Positional { order: 0 },
        Str,
        "Keep only conversations whose participant or module contains this text",
        false,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_TREE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "Participant identifier (`om_stt_viper_302`), part of one, or a module name",
        true,
    ),
    ArgSpec::new(
        "lang",
        Long("lang"),
        Str,
        "Localization column, or a language family (`german`, `english`)",
        false,
    )
    .with_default("english"),
    ArgSpec::new(
        "depth",
        Long("depth"),
        Int {
            min: Some(0),
            max: None,
        },
        "Stop after this much sub-dialog nesting",
        false,
    ),
    ArgSpec::new(
        "ids",
        crate::spec::ArgForm::Switch("ids"),
        crate::spec::ArgKind::Bool,
        "Print class names and localization keys next to the text",
        false,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_SHOW_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "topic",
        Positional { order: 0 },
        Str,
        "Topic class name, with or without the generated `U` prefix",
        true,
    ),
    ArgSpec::new(
        "lang",
        Long("lang"),
        Str,
        "Localization column, or a language family (`german`, `english`)",
        false,
    )
    .with_default("english"),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_EXPORT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Empty output directory. Created if absent; existing files are never overwritten",
        true,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_CHECKOUT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "Participant identifier (`om_stt_viper_302`), part of one, or a module name",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Empty working directory for the editable source (including complete reconstructed class \
         defaults), its pristine copy, and the manifest; existing work is never overwritten",
        true,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_CHECK_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "dir",
        Positional { order: 0 },
        Path,
        "The directory `checkout` wrote",
        true,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_STAGE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "dir",
        Positional { order: 0 },
        Path,
        "The directory `checkout` wrote",
        true,
    ),
    ArgSpec::new(
        "mod_name",
        Long("mod-name"),
        Str,
        "Portable single-component mod name for the mini-cache and bundle spec this edit stages",
        false,
    )
    .with_default("MyDialogEdit"),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "Participant identifier (`om_stt_viper_302`), part of one, or a module name",
        true,
    ),
    ArgSpec::new(
        "lang",
        Long("lang"),
        Str,
        "Localization column, or a language family (`german`, `english`)",
        false,
    )
    .with_default("german"),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output edits JSON, ready for `gore loc import --edits`",
        true,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_NEW_TOPIC_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "Participant identifier (`om_stt_viper_302`), part of one, or a module name",
        true,
    ),
    ArgSpec::new(
        "caption",
        Long("caption"),
        Str,
        "The menu text, as an untranslated literal",
        false,
    ),
    ArgSpec::new(
        "caption_key",
        Long("caption-key"),
        Str,
        "The menu text's localization key, for a translatable option",
        false,
    ),
    ArgSpec::new(
        "class",
        Long("class"),
        Str,
        "AngelScript class name for the new option",
        false,
    )
    .with_default("UChoice<mod name>"),
    ArgSpec::new(
        "subdialog_of",
        Long("subdialog-of"),
        Str,
        "Existing topic whose single `Subdialog(...)` call should receive the new same-module \
         class. Omit this to scaffold a natively discovered same-module root topic",
        false,
    ),
    ArgSpec::new(
        "subdialog_position",
        Long("subdialog-position"),
        Int {
            min: Some(1),
            max: Some(20),
        },
        "Optional 1-based position among the parent's populated Subdialog entries. Existing entries at that position and later shift right. Requires subdialog_of. By default a recognized trailing TEXT_BACK entry stays last; otherwise the new child appends",
        false,
    ),
    ArgSpec::new(
        "priority_rank",
        Long("priority-rank"),
        Int {
            min: Some(-2_147_483_648),
            max: Some(2_147_483_647),
        },
        "Menu-order rank. Roots choose an automatic rank before a recognized final End/Back entry, otherwise before the trailing rank group; sub-topics default to 0 and keep equal-rank order from --subdialog-position. Pass a value to override it; -1 has the game's forced-topic semantics and is never chosen automatically",
        false,
    ),
    ArgSpec::new(
        "mod_name",
        Long("mod-name"),
        Str,
        "Mod name, used for the default class name and the later staged bundle",
        false,
    )
    .with_default("MyDialogMod"),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Empty output directory for the edited same-module source, its pristine copy, and the \
         edit manifest",
        true,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_NEW_CONVERSATION_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "Exact NPC identifier with one loaded per-NPC conversation-settings module; a separate unbound add module is refused",
        true,
    ),
    ArgSpec::new(
        "caption",
        Long("caption"),
        Str,
        "The first option's menu text, as an untranslated literal",
        false,
    ),
    ArgSpec::new(
        "caption_key",
        Long("caption-key"),
        Str,
        "The first option's localization key, for a translatable caption",
        false,
    ),
    ArgSpec::new(
        "class",
        Long("class"),
        Str,
        "AngelScript class name for the first option",
        false,
    )
    .with_default("UChoice<mod name>"),
    ArgSpec::new(
        "priority_rank",
        Long("priority-rank"),
        Int {
            min: Some(-2_147_483_648),
            max: Some(2_147_483_647),
        },
        "Menu-order rank for the first option. Pass -1 only when intentionally authoring a forced topic",
        false,
    )
    .with_default("2"),
    ArgSpec::new(
        "mod_name",
        Long("mod-name"),
        Str,
        "Mod name, used for the default class name and the later staged bundle",
        false,
    )
    .with_default("MyDialogMod"),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Empty output directory for the same-module conversation source, its pristine/base \
         material, and the checked manifest",
        true,
    ),
    DIALOG_CACHE_ARGS[0],
    DIALOG_CACHE_ARGS[1],
];

const DIALOG_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "list",
        "List the conversations the game ships, with their participants and size",
        DIALOG_LIST_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("dialog-trees"),
    CommandSpec::new(
        "tree",
        "Print one NPC's complete dialog tree",
        DIALOG_TREE_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("dialog-trees"),
    CommandSpec::new(
        "show",
        "Print one topic in full: caption, rules, visibility, and body",
        DIALOG_SHOW_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("dialog-trees"),
    // One edits document at a path the caller picks, overwritten if it is already there.
    CommandSpec::new(
        "text",
        "Write one NPC's dialog text as a `gore loc import` edits document",
        DIALOG_TEXT_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("dialog-trees"),
    // The module's source, an untouched copy of it, and a manifest, under a directory the caller
    // picks. The game install is only read.
    CommandSpec::new(
        "checkout",
        "Check out one conversation as editable AngelScript, including complete reconstructed \
         defaults for Caption, PriorityRank, Rules, and topic flags",
        DIALOG_CHECKOUT_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .guide("dialog-authoring"),
    CommandSpec::new(
        "check",
        "Fail closed unless a conversation workspace preserves every required class/default and \
         satisfies the supported edit/remap contract",
        DIALOG_CHECK_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("dialog-authoring"),
    // One build spec inside the directory `checkout` already created.
    CommandSpec::new(
        "stage",
        "Write the build spec for a checked workspace and print its strict standalone `--op \
         edit` compile command, adding `--allow-new-symbols` only when required",
        DIALOG_STAGE_ARGS,
        Safety::write()
            .also_writes(&[("dir", Derived::Child("spec.json"))])
            .writes_into(&["dir"]),
        T_NORMAL,
    )
    .guide("dialog-authoring"),
    // A same-module editable source, pristine copy, and manifest under a caller-picked empty
    // directory. The game install is only read.
    CommandSpec::new(
        "new-topic",
        "Scaffold a new topic inside an NPC's existing conversation module: leave it as a native \
         root, or wire it into one existing Subdialog call with --subdialog-of",
        DIALOG_NEW_TOPIC_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .exactly_one(&[&["caption", "caption_key"]])
    .guide("dialog-authoring"),
    // One same-module conversation source plus its base material and manifest under a
    // caller-picked empty directory. The game install is only read.
    CommandSpec::new(
        "new-conversation",
        "Scaffold a complete conversation where the NPC has no root topic by extending its exact \
         already-loaded per-NPC conversation-settings module; refuse an unbound new module",
        DIALOG_NEW_CONVERSATION_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .exactly_one(&[&["caption", "caption_key"]])
    .guide("dialog-authoring"),
    // One JSON file per conversation, under data-derived collision-free names, in a directory the
    // caller picks. The CLI refuses occupied directories and per-file collisions itself.
    CommandSpec::new(
        "export",
        "Write every conversation to a directory, one JSON file each",
        DIALOG_EXPORT_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .guide("dialog-trees"),
];

pub const DIALOG: GroupSpec = GroupSpec {
    tool: "gore_dialog",
    title: "gore dialog reader and authoring guard",
    cli: "dialog",
    summary:
        "Read the dialog trees declared by the installed script cache, edit method bodies and \
              complete reconstructed defaults, and stage checked same-module topic or complete \
              conversation additions. Localization, compilation, packaging, deployment, and \
              runtime proof remain separate steps; read commands report cache declarations, not \
              save-state behavior.",
    shape: GroupShape::Nested,
    commands: DIALOG_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_npc
// ---------------------------------------------------------------------------------------------

/// Shared by every leaf that reads the script cache. Two are not among them: `list` answers from
/// the catalog bundled in this binary, and `text` only writes the document it is handed a name for.
/// Neither looks at an installation.
const NPC_CACHE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "cache",
        Long("cache"),
        Path,
        "Read this script cache instead of the installed one",
        false,
    )
    .with_default("the script cache of the resolved game install"),
    // No `with_default` here, unlike `DIALOG_CACHE_ARGS`: this help text already names the
    // fallback chain, and a hint would render the same sentence twice in a row.
    ArgSpec::new(
        "game",
        Long("game"),
        Path,
        "Game install root. Falls back to configured path, then Steam auto-detect",
        false,
    ),
];

const NPC_LIST_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "filter",
        Positional { order: 0 },
        Str,
        "Keep only entries whose id or class contains this text",
        false,
    ),
    ArgSpec::new(
        "category",
        Long("category"),
        Str,
        "Keep only one category (human, creature, other)",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int {
            min: Some(0),
            max: None,
        },
        "Max rows to print",
        false,
    )
    .with_default("50"),
];

const NPC_SHOW_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "Exact NPC id, for example OC_STT_Diego",
        true,
    ),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
];

const NPC_SITES_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "level",
        Long("level"),
        Str,
        "Keep only sites whose level-script module contains this text",
        false,
    ),
    ArgSpec::new(
        "free",
        Switch("free"),
        Bool,
        "Keep only world points nobody is spawned at",
        false,
    ),
    ArgSpec::new(
        "occupied",
        Switch("occupied"),
        Bool,
        "Keep only world points that already spawn somebody",
        false,
    ),
    ArgSpec::new(
        "npc",
        Long("npc"),
        Str,
        "Keep only sites that spawn this character",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int {
            min: Some(0),
            max: None,
        },
        "Max rows to print",
        false,
    )
    .with_default("50"),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
];

const NPC_NEW_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "id",
        Positional { order: 0 },
        Str,
        "Id of the new character, for example MY_NPC",
        true,
    ),
    ArgSpec::new(
        "from",
        Long("from"),
        Str,
        "The shipped character to derive from: its looks, stats and voice",
        true,
    ),
    ArgSpec::new(
        "guild",
        Long("guild"),
        Str,
        "Replace the faction with this guild base, for example OldCamp_Guard",
        false,
    ),
    ArgSpec::new(
        "at",
        Long("at"),
        Str,
        "World point to spawn at, from `gore npc sites`",
        true,
    ),
    ArgSpec::new(
        "waypoint",
        Long("waypoint"),
        Str,
        "Waypoint for the daily routine",
        false,
    ),
    ArgSpec::new(
        "trader",
        crate::spec::ArgForm::Switch("trader"),
        crate::spec::ArgKind::Bool,
        "Add an empty trader configuration",
        false,
    ),
    ArgSpec::new(
        "modular_visuals",
        crate::spec::ArgForm::Switch("modular-visuals"),
        crate::spec::ArgKind::Bool,
        "Build the looks from parts at runtime instead of borrowing a prebaked model. No shipped \
         character does this; unproven",
        false,
    ),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output workspace directory; must not exist",
        true,
    ),
];

const NPC_DELETE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "The character to remove",
        true,
    ),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output workspace directory; must not exist",
        true,
    ),
];

const NPC_CLONE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "source",
        Positional { order: 0 },
        Str,
        "The shipped character to copy",
        true,
    ),
    ArgSpec::new(
        "id",
        Long("id"),
        Str,
        "Id of the new character, for example MY_NPC",
        true,
    ),
    ArgSpec::new(
        "guild",
        Long("guild"),
        Str,
        "Replace the faction with this guild base, for example OldCamp_Guard",
        false,
    ),
    ArgSpec::new(
        "at",
        Long("at"),
        Str,
        "World point to spawn at, from `gore npc sites`",
        true,
    ),
    ArgSpec::new(
        "waypoint",
        Long("waypoint"),
        Str,
        "Waypoint for the daily routine",
        false,
    ),
    ArgSpec::new(
        "trader",
        Switch("trader"),
        Bool,
        "Add an empty trader configuration",
        false,
    ),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output workspace directory; must not exist",
        true,
    ),
];

const NPC_CHECKOUT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "npc",
        Positional { order: 0 },
        Str,
        "The character to edit, for example OC_STT_Diego",
        true,
    ),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output workspace directory; must not exist",
        true,
    ),
];

const NPC_CHECK_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "dir",
        Positional { order: 0 },
        Path,
        "The workspace directory written by `new` or `delete`",
        true,
    ),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
];

const NPC_STAGE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "dir",
        Positional { order: 0 },
        Path,
        "The workspace directory written by `new` or `delete`",
        true,
    ),
    ArgSpec::new(
        "tree",
        Long("tree"),
        Path,
        "Where to keep the emitted source tree between runs. Required for a new character",
        false,
    ),
    ArgSpec::new(
        "mod_name",
        Long("mod-name"),
        Str,
        "Name of the mod being built",
        false,
    )
    .with_default("MyNpcMod"),
    NPC_CACHE_ARGS[0],
    NPC_CACHE_ARGS[1],
];

const NPC_TEXT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "id",
        Positional { order: 0 },
        Str,
        "The character id, for example MY_NPC",
        true,
    ),
    ArgSpec::new(
        "name",
        Long("name"),
        Str,
        "The name to show above the character's dialog lines",
        true,
    ),
    ArgSpec::new(
        "english",
        Long("english"),
        Str,
        "Also set the English columns to this name",
        false,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output file; must not exist",
        true,
    ),
];

const NPC_COMMANDS: &[CommandSpec] = &[
    // Answered from the catalog compiled into this binary, like `find`, so it needs no install.
    CommandSpec::new(
        "list",
        "List the characters the game ships",
        NPC_LIST_ARGS,
        Safety::read(),
        T_FAST,
    )
    .json(JsonSupport::Stdout)
    .guide("npc-authoring"),
    // Resolves the class chain out of the emitted source, which means emitting the few modules
    // that carry it — seconds, not the minutes a whole-tree emit would cost.
    CommandSpec::new(
        "show",
        "Print one character in full: its class chain, where it spawns, and what it inherits",
        NPC_SHOW_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("npc-authoring"),
    CommandSpec::new(
        "sites",
        "List the world points the level scripts can place characters at",
        NPC_SITES_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("npc-authoring"),
    // A workspace under a caller-picked directory: the character's own module, the level script one
    // spawn line longer, an untouched copy of that script, and the manifest. The install is only
    // read, and the CLI refuses a directory that already exists, so there is nothing to ask about.
    CommandSpec::new(
        "new",
        "Author a new character derived from a shipped one",
        NPC_NEW_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .guide("npc-authoring"),
    CommandSpec::new(
        "delete",
        "Stop a shipped character from being placed in the world",
        NPC_DELETE_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .guide("npc-authoring"),
    CommandSpec::new(
        "clone",
        "Clone a shipped character. Same result as `new`, named for what it does",
        NPC_CLONE_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .guide("npc-authoring"),
    CommandSpec::new(
        "checkout",
        "Check out a shipped character's own module for edits that preserve its existing default targets",
        NPC_CHECKOUT_ARGS,
        Safety::write().writes_into(&["out"]),
        T_NORMAL,
    )
    .guide("npc-authoring"),
    CommandSpec::new(
        "check",
        "Check an authored workspace against the current compile contract",
        NPC_CHECK_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .guide("npc-authoring"),
    // One build spec inside the directory `new` or `delete` already created — and, for a new
    // character, the emitted source tree in `--tree` first. That emit is around 19 minutes on the
    // first run of a game version and is reused afterwards, which is why this leaf carries the long
    // budget rather than the one its siblings share.
    CommandSpec::new(
        "stage",
        "Build the source tree and print the commands that compile an authored character",
        NPC_STAGE_ARGS,
        Safety::write()
            .also_writes(&[("dir", Derived::Child("spec.json"))])
            .writes_into(&["dir"]),
        T_LONG,
    )
    .guide("npc-authoring"),
    // One edits document at a path the caller picks. Reads nothing at all: the localization id of a
    // character is its id in lowercase, so the document is derivable from the arguments alone.
    CommandSpec::new(
        "text",
        "Write a character's display name as a `gore loc import --edits` document",
        NPC_TEXT_ARGS,
        Safety::write().writes_into(&["out"]),
        T_FAST,
    )
    .guide("npc-authoring"),
];

/// The character surface. A character is not a record in this game but a chain of AngelScript
/// classes — a spawn definition names an AI config, which names a character definition — so the
/// question "what is this NPC" is only answerable by walking that chain, and `show` is where it
/// gets walked. `sites` answers the other half, where the level scripts place them; `new` and
/// `delete` write that placement, and `check` and `stage` are what stands between an authored
/// workspace and a compile.
pub const NPC: GroupSpec = GroupSpec {
    tool: "gore_npc",
    title: "gore character reader and authoring guard",
    cli: "npc",
    summary: "Read the game's characters from the class chain the script cache declares — which \
              ones exist, what one of them inherits from its AI config and character definition, \
              and which world points the level scripts spawn it from — then author a new one, or \
              stop a shipped one from being placed. `list` answers from the catalog bundled in \
              this binary and needs no installation, `text` writes a localization document from \
              its arguments alone, and everything else reads the script cache. Authoring produces \
              a workspace and a build spec: compiling, packaging, deploying, and any evidence that \
              the character actually appears in game are separate steps this group does not take.",
    shape: GroupShape::Nested,
    commands: NPC_COMMANDS,
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
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Mods directory (e.g. ue4ss/Mods/)",
        true,
    ),
];

const GEN_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "overrides",
        Positional { order: 0 },
        Path,
        "Path to overrides.toml",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Mods directory to write the mod folder into",
        true,
    ),
    ArgSpec::new(
        "model",
        Long("model"),
        Path,
        "Path to model.json for validation (optional; skips validation if absent)",
        false,
    ),
];

const PACKAGE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "mod_dir",
        Positional { order: 0 },
        Path,
        "Path to the mod directory",
        true,
    ),
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
        // The CLI refuses only when `Scripts/main.lua` is already there, so an existing
        // non-Lua UE4SS mod under the same name is entered and its `enabled.txt` truncated.
        // Unlike `gen`, the folder is fully derivable -- `<out>/<mod_name>` -- so a fresh name
        // still needs no flag and only the collision is gated.
        Safety::write()
            .also_writes(&[("out", Derived::ChildOfArg("mod_name"))])
            .installs_via(&["out"]),
        T_FAST,
    )
    .guide("items"),
    CommandSpec::new(
        "gen",
        "Compile overrides.toml into a UE4SS Lua mod",
        GEN_ARGS,
        // `out` is a Mods directory that always exists, and the folder actually rewritten is
        // `<out>/<name from overrides.toml>` -- a path this layer cannot compute without parsing
        // that file. cmd/gen.rs rewrites enabled.txt and Scripts/main.lua unconditionally, so an
        // existing mod (generated or hand-edited) is replaced. Not gateable, therefore gated.
        Safety::mutate(),
        T_NORMAL,
    )
    .gated_because(
        "rewrites the `enabled.txt` and `Scripts/main.lua` of the mod folder named inside \
         overrides.toml, whether that mod was generated or written by hand",
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
    .gated_because(
        "copies the shared Lua SDK into the game's own `ue4ss/Mods/shared`, replacing the copy \
         installed there",
    )
    .guide("items"),
];

pub const PROJECT: GroupSpec = GroupSpec {
    tool: "gore_project",
    title: "gore Lua mod project",
    cli: "",
    summary:
        "Author and ship a UE4SS Lua mod: scaffold a skeleton, compile overrides.toml into \
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
        assert_eq!(FIND.commands.len(), 1);
        assert_eq!(CATALOG.commands.len(), 8);
        assert_eq!(LOCATION.commands.len(), 2);
        assert_eq!(PROJECT.commands.len(), 4);
    }

    #[test]
    fn the_config_commands_are_deliberately_ungated() {
        // `set`, `unset` and `detect` do rewrite an existing config.json, which the rest of the
        // table would call a mutation. They are the one deliberate exception, for two reasons.
        //
        // What they change is a preference, not content: one absolute path, visible through
        // `config list`, restored by running `set` again. Nothing a user or the game produced is
        // lost. And the primer tells the model to reach for exactly this when a command cannot
        // find the game — gating it would leave that advice unfollowable and turn the single most
        // common setup failure into a dead end the agent cannot clear.
        //
        // The exception is stated in the guide, not just here, because it is the one place the
        // "rewriting an existing file needs --allow-write" rule does not hold.
        for sub in ["set", "unset", "detect"] {
            let command = CONFIG.command(sub).expect("exists");
            assert!(
                !command.safety.worst_case().needs_write_permission(),
                "`config {sub}` is gated; if that is intended, the guide and primer must say so"
            );
            assert!(command.safety.installs_via.is_empty());
            assert!(command.safety.truncates.is_empty());
        }
    }

    #[test]
    fn exactly_the_commands_whose_targets_cannot_be_checked_are_gated() {
        // `deploy-shared` copies the SDK into the game's `ue4ss/Mods`; `gen` rewrites
        // `<out>/<name from overrides.toml>`, and TOML is not something this layer parses. Neither
        // path can be computed from the arguments, so both are gated outright.
        //
        // `stubs` is deliberately not here any more. It writes one `.lua` per class under names it
        // takes from the model file, which is just as unpreflightable — but the directory those
        // files land in is not, so it is gated on an occupied `out` instead of on every call.
        let mutating: Vec<&str> = [CONFIG, FIND, CATALOG, LOCATION, PROJECT]
            .iter()
            .flat_map(|group| group.commands.iter())
            .filter(|command| command.safety.worst_case().needs_write_permission())
            .map(|command| command.sub)
            .collect();
        assert_eq!(mutating, vec!["gen", "deploy-shared"]);

        let stubs = CATALOG.command("stubs").expect("exists");
        assert!(!stubs.safety.worst_case().needs_write_permission());
        assert_eq!(stubs.safety.clobbers_dir, &["out"]);

        let dialog_export = DIALOG.command("export").expect("exists");
        assert_eq!(dialog_export.safety.writes_into, &["out"]);
        assert!(dialog_export.safety.clobbers_dir.is_empty());
    }
}
