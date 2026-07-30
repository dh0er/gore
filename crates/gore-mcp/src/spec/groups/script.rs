//! AngelScript precompiled-cache tooling — the largest group, and the only one that can start the
//! game.
//!
//! Two things here are unlike anything else in the table:
//!
//! - `compile` and `compile-module` drive the game's own compiler by launching the game executable.
//!   They are [`Safety::game_launch`] and unreachable without `--allow-game-launch`.
//! - `bytediff` spells `--json` as a *path* to write a report to, not as a switch that changes
//!   stdout. It is therefore an ordinary argument and is never passed implicitly; passing it
//!   automatically would create a file nobody asked for.
//!
//! Every `summary` and `help` string is copied verbatim from the corresponding clap doc comment.

use crate::spec::{
    ArgForm::{Long, LongRepeated, Positional, PositionalRepeated, Switch},
    ArgKind::{Bool, Enum, Hex, Int, IntList, Path, Str, StrList},
    ArgSpec, CommandSpec, GroupShape, GroupSpec, JsonSupport, Safety, T_COMPILE, T_FAST, T_LONG,
    T_NORMAL,
};

const CACHE_FILE: ArgSpec =
    ArgSpec::new("file", Positional { order: 0 }, Path, "Precompiled cache file to read.", true);

const CACHE_POSITIONAL: ArgSpec =
    ArgSpec::new("cache", Positional { order: 0 }, Path, "Cache to read or patch.", true);

const NEEDLE: ArgSpec = ArgSpec::new(
    "needle",
    Positional { order: 1 },
    Str,
    "Substring filter on `module.Class::func`.",
    false,
)
.with_default("empty, meaning all");

const GAME: ArgSpec = ArgSpec::new(
    "game",
    Long("game"),
    Path,
    "Game install root (the folder containing `G1R/`).",
    false,
)
.with_default("the configured game path, then Steam auto-detect");

const SELECTOR: ArgSpec = ArgSpec::new(
    "selector",
    Long("selector"),
    Path,
    "Strict selector JSON copied from the matching `--json` listing.",
    true,
);

const OUT_CACHE: ArgSpec = ArgSpec::new(
    "out",
    Long("out"),
    Path,
    "New full cache path. Existing paths are never overwritten.",
    true,
);

const MODULE_FILTER: ArgSpec =
    ArgSpec::new("module", Long("module"), Str, "Exact module-name filter.", false);
const CLASS_FILTER: ArgSpec =
    ArgSpec::new("class", Long("class"), Str, "Exact class-name filter.", false);
const FIELD_FILTER: ArgSpec =
    ArgSpec::new("field", Long("field"), Str, "Exact field-name filter.", false);

/// The diagnostics trio shared by `compile` and `compile-module`.
///
/// The hook captures AngelScript compiler errors from the running game, which is the only way an
/// agent gets to see why a compile failed — without it a failed compile returns no error text at
/// all. It stays available; `--allow-game-launch` is the gate.
const NO_DIAGNOSTICS: ArgSpec = ArgSpec::new(
    "no_diagnostics",
    Switch("no-diagnostics"),
    Bool,
    "Disable the optional runtime compiler-diagnostic hook and use the normal generator.",
    false,
);
const DIAGNOSTICS_HOOK: ArgSpec = ArgSpec::new(
    "diagnostics_hook",
    Long("diagnostics-hook"),
    Path,
    "Explicit `gore-as-diagnostics-hook.dll`; otherwise use environment, sibling, then the \
     integrity-checked embedded helper.",
    false,
);
const DIAGNOSTICS_DELAY: ArgSpec = ArgSpec::new(
    "diagnostics_inject_delay_ms",
    Long("diagnostics-inject-delay-ms"),
    Int { min: Some(0), max: Some(30_000) },
    "Delay between game launch and diagnostics injection (loader warm-up).",
    false,
)
.with_default("2000");

const DIAGNOSTICS_CONFLICT: &[&[&str]] = &[&["no_diagnostics", "diagnostics_hook"]];

const DECODE_HEADER_ARGS: &[ArgSpec] = &[CACHE_FILE];

const WALK_ARGS: &[ArgSpec] = &[
    CACHE_FILE,
    ArgSpec::new(
        "max",
        Long("max"),
        Int { min: Some(0), max: None },
        "Maximum number of strings to print.",
        false,
    )
    .with_default("100"),
];

const DECOMPILE_ARGS: &[ArgSpec] = &[
    CACHE_FILE,
    NEEDLE,
    ArgSpec::new(
        "max",
        Long("max"),
        Int { min: Some(0), max: None },
        "Max functions to print.",
        false,
    )
    .with_default("20"),
];

const EMIT_ALL_ARGS: &[ArgSpec] = &[
    CACHE_FILE,
    ArgSpec::new(
        "outdir",
        Positional { order: 1 },
        Path,
        "Output directory; the ScriptRelativeFilename layout is mirrored below it.",
        true,
    ),
];

const EMIT_ARGS: &[ArgSpec] = &[
    CACHE_FILE,
    NEEDLE,
    ArgSpec::new(
        "max",
        Long("max"),
        Int { min: Some(0), max: None },
        "Max modules to print.",
        false,
    )
    .with_default("5"),
];

const STATIC_NAMES_ARGS: &[ArgSpec] = &[
    CACHE_FILE,
    ArgSpec::new(
        "indices",
        PositionalRepeated { order: 1 },
        IntList,
        "Specific indices to print.",
        false,
    )
    .with_default("none, meaning the count plus the first 10 entries"),
];

const DISASM_ARGS: &[ArgSpec] = &[
    CACHE_FILE,
    NEEDLE,
    ArgSpec::new(
        "max",
        Long("max"),
        Int { min: Some(0), max: None },
        "Max functions to print.",
        false,
    )
    .with_default("20"),
];

const DEFAULT_SITES_ARGS: &[ArgSpec] =
    &[CACHE_POSITIONAL, MODULE_FILTER, CLASS_FILTER, FIELD_FILTER];

const PATCH_DEFAULT_ARGS: &[ArgSpec] = &[
    CACHE_POSITIONAL,
    SELECTOR,
    ArgSpec::new(
        "expected_hex",
        Long("expected-hex"),
        Hex,
        "Complete current serialized immediate as lowercase hex (V1/V2/V4: 4 bytes; V8: 8).",
        true,
    ),
    ArgSpec::new(
        "replacement_hex",
        Long("replacement-hex"),
        Hex,
        "Complete replacement serialized immediate as lowercase hex.",
        true,
    ),
    OUT_CACHE,
];

const TAG_MAP_SITES_ARGS: &[ArgSpec] = &[
    CACHE_POSITIONAL,
    MODULE_FILTER,
    CLASS_FILTER,
    FIELD_FILTER,
    ArgSpec::new("tag", Long("tag"), Str, "Exact GameplayTag global name filter.", false),
];

const PATCH_TAG_MAP_ARGS: &[ArgSpec] = &[
    CACHE_POSITIONAL,
    SELECTOR,
    ArgSpec::new(
        "expected_hex",
        Long("expected-hex"),
        Hex,
        "Fresh current raw IEEE-754 float32 little-endian bytes: exactly 8 lowercase hex chars.",
        true,
    ),
    ArgSpec::new(
        "replacement_hex",
        Long("replacement-hex"),
        Hex,
        "Replacement raw IEEE-754 float32 little-endian bytes: exactly 8 lowercase hex chars.",
        true,
    ),
    OUT_CACHE,
];

const DIAGNOSTICS_CHECK_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "exe",
        Long("exe"),
        Path,
        "Exact executable to scan (supports non-Steam/custom layouts). Conflicts with `game`.",
        false,
    ),
    GAME,
];

const COMPILE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "src",
        Positional { order: 0 },
        Path,
        "Source `.as` tree (a directory) to compile. Omit to recompile the loose `.as` already \
         installed under `<game>/G1R/Script/`.",
        false,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Write the compiled cache here and leave the game install untouched. Omitting this \
         installs the fresh cache in place under `Script/`.",
        false,
    ),
    GAME,
    ArgSpec::new(
        "no_backup",
        Switch("no-backup"),
        Bool,
        "When installing in place, do NOT back up the previous cache.",
        false,
    ),
    NO_DIAGNOSTICS,
    DIAGNOSTICS_HOOK,
    DIAGNOSTICS_DELAY,
];

const COMPILE_MODULE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "op",
        Long("op"),
        Enum(&["add", "edit"]),
        "`add` for a new module or `edit` for an existing module.",
        true,
    ),
    ArgSpec::new(
        "module",
        Long("module"),
        Str,
        "Expected module name. For `add`, the compiler-detected module name is reported and used.",
        true,
    ),
    ArgSpec::new(
        "rel_path",
        Long("rel-path"),
        Str,
        "Safe path of the authored file relative to the game's `Script/` tree.",
        true,
    ),
    ArgSpec::new("source", Long("source"), Path, "Authored `.as` source file to overlay.", true),
    ArgSpec::new(
        "work_dir",
        Long("work-dir"),
        Path,
        "Persistent compiler workspace used for the emitted tree and intermediate regen cache.",
        true,
    ),
    ArgSpec::new(
        "allow_new_symbols",
        Switch("allow-new-symbols"),
        Bool,
        "Explicitly retain minimal rows for classes/functions/names absent from the pristine cache. \
         Normally used with `--op add`; strict remapping remains the default.",
        false,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path for the remapped 1-module mini-cache.",
        true,
    ),
    GAME,
    NO_DIAGNOSTICS,
    DIAGNOSTICS_HOOK,
    DIAGNOSTICS_DELAY,
];

const REPLACE_ARGS: &[ArgSpec] = &[
    ArgSpec::new("base", Positional { order: 0 }, Path, "Base cache to patch.", true),
    ArgSpec::new("mini", Positional { order: 1 }, Path, "Mini-cache holding the new module.", true),
    ArgSpec::new(
        "target",
        Positional { order: 2 },
        Str,
        "Name of the module in the base cache to replace.",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output path for the patched cache.", true),
];

const SPLICE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "base",
        Positional { order: 0 },
        Path,
        "Base cache (e.g. PrecompiledScript_Shipping.Cache).",
        true,
    ),
    ArgSpec::new(
        "mini",
        Positional { order: 1 },
        Path,
        "Mini-cache from -as-generate-precompiled-data (one primitive-only module).",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output path for the spliced cache.", true),
];

const EXTRACT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "cache",
        Positional { order: 0 },
        Path,
        "Source cache (e.g. a full-tree regen).",
        true,
    ),
    ArgSpec::new(
        "module",
        Positional { order: 1 },
        Str,
        "Module name (the Modules TMap key) to extract.",
        true,
    ),
    ArgSpec::new("out", Long("out"), Path, "Output path for the 1-module mini-cache.", true),
];

const EXTRACT_REMAP_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "regen_cache",
        Positional { order: 0 },
        Path,
        "Regen cache (full-tree -as-generate-precompiled-data output) containing the edit.",
        true,
    ),
    ArgSpec::new(
        "module",
        Positional { order: 1 },
        Str,
        "Module name (the Modules TMap key) to extract + remap.",
        true,
    ),
    ArgSpec::new(
        "base_cache",
        Positional { order: 2 },
        Path,
        "Base (vanilla) cache whose keys the module's refs are rewritten to.",
        true,
    ),
    ArgSpec::new(
        "allow_new_symbols",
        Switch("allow-new-symbols"),
        Bool,
        "Explicitly carry minimal tail-table rows for symbols absent from the base.",
        false,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path for the remapped 1-module mini-cache.",
        true,
    ),
];

const BYTEDIFF_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "vanilla",
        Positional { order: 0 },
        Path,
        "Vanilla reference cache.",
        true,
    ),
    ArgSpec::new(
        "regen",
        Positional { order: 1 },
        Path,
        "Regen cache (re-compilation of our decompiled .as tree).",
        true,
    ),
    ArgSpec::new(
        "module",
        Long("module"),
        Str,
        "Only diff modules whose name contains this substring.",
        false,
    ),
    ArgSpec::new(
        "func",
        Long("func"),
        Str,
        "Only diff functions whose display name (module.Class::func) contains this substring.",
        false,
    ),
    ArgSpec::new(
        "verdicts",
        LongRepeated("verdict"),
        StrList,
        "Filter output to these verdicts: identical, benign, semantic.",
        false,
    ),
    ArgSpec::new(
        "show_benign",
        Switch("show-benign"),
        Bool,
        "List which normalizers fired for BENIGN-DIFF functions (default: summary only).",
        false,
    ),
    ArgSpec::new(
        "context",
        Long("context"),
        Int { min: Some(0), max: None },
        "Instruction window (±N) around each SEMANTIC divergence.",
        false,
    )
    .with_default("6"),
    ArgSpec::new(
        "norm_slots",
        Switch("norm-slots"),
        Bool,
        "Enable OPT-IN fail-closed N2 slot-allocation normalization (default OFF).",
        false,
    ),
    ArgSpec::new(
        "no_norm_scope",
        Switch("no-norm-scope"),
        Bool,
        "Disable the N5 `FScopeCycleCounter` RAII profiler-scope strip (default ON).",
        false,
    ),
    ArgSpec::new(
        "no_norm_reguard",
        Switch("no-norm-reguard"),
        Bool,
        "Disable the N6 dominated boolean-cascade re-guard fold (default ON).",
        false,
    ),
    // Unlike every other `--json` in this toolkit, this one takes a path and writes a file.
    ArgSpec::new(
        "json",
        Long("json"),
        Path,
        "Write a machine-readable JSON scoreboard (per-verdict counts + alignment loss) to this \
         path. Unlike other commands, this is a file path, not a switch.",
        false,
    ),
    ArgSpec::new(
        "fail_on_semantic",
        Switch("fail-on-semantic"),
        Bool,
        "Exit non-zero if any SEMANTIC-DIFF is found (CI gate).",
        false,
    ),
];

const AS_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "decode-header",
        "Parse and print the outer cache header.",
        DECODE_HEADER_ARGS,
        Safety::read(),
        T_FAST,
    )
    .guide("scripts"),
    CommandSpec::new(
        "walk",
        "Scan length-prefixed type-name strings (decode investigation aid).",
        WALK_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "info",
        "Print module count + TAIL_OFF (the splice insertion point) for a cache.",
        DECODE_HEADER_ARGS,
        Safety::read(),
        T_FAST,
    )
    .guide("scripts"),
    CommandSpec::new(
        "decompile",
        "Decompile functions whose name contains <needle> to structured AngelScript.",
        DECOMPILE_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "emit-all",
        "Emit ALL modules as recompilable .as into <outdir>, mirroring ScriptRelativeFilename.",
        EMIT_ALL_ARGS,
        Safety::write(),
        T_LONG,
    )
    .guide("scripts"),
    CommandSpec::new(
        "emit",
        "Emit recompilable .as for modules whose name contains <needle>.",
        EMIT_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "static-names",
        "Dump StaticNames tail-table entries (the `n\"…\"` FName-literal pool indexed by \
         `__STATIC_NAME(Id)`).",
        STATIC_NAMES_ARGS,
        Safety::read(),
        T_FAST,
    )
    .guide("scripts"),
    CommandSpec::new(
        "disasm",
        "Disassemble functions whose name contains <needle> to an asBC listing.",
        DISASM_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "default-sites",
        "List uniquely patchable scalar assignments in generated `__InitDefaults` bytecode.",
        DEFAULT_SITES_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("angelscript-defaults"),
    CommandSpec::new(
        "patch-default",
        "Copy-on-write patch one `default-sites` scalar using semantic lookup plus raw CAS.",
        PATCH_DEFAULT_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("angelscript-defaults"),
    CommandSpec::new(
        "tag-map-sites",
        "List sealed native GameplayTag-to-float32 map-entry defaults. Requires exact bounded \
         Binds/USMAP evidence and fails closed without it.",
        TAG_MAP_SITES_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("angelscript-defaults"),
    CommandSpec::new(
        "patch-tag-map",
        "Copy-on-write patch one sealed GameplayTag-to-float32 map entry using semantic CAS.",
        PATCH_TAG_MAP_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("angelscript-defaults"),
    CommandSpec::new(
        "diagnostics-check",
        "Offline-check whether the optional diagnostics hook has one safe AOB match. Does not \
         launch the game or change the installation.",
        DIAGNOSTICS_CHECK_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .at_most_one(&[&["exe", "game"]])
    .guide("scripts"),
    CommandSpec::new(
        "compile",
        "Compile AngelScript into a precompiled cache by driving the game's own \
         `-as-generate-precompiled-data` flag. Launches the game.",
        COMPILE_ARGS,
        Safety::game_launch().in_place_without(&["out"]),
        T_COMPILE,
    )
    .at_most_one(DIAGNOSTICS_CONFLICT)
    .guide("scripts"),
    CommandSpec::new(
        "compile-module",
        "Compile one authored module into a deployable 1-module mini-cache. Wraps the complete \
         Studio pipeline and launches the game.",
        COMPILE_MODULE_ARGS,
        Safety::game_launch(),
        T_COMPILE,
    )
    .at_most_one(DIAGNOSTICS_CONFLICT)
    .guide("scripts"),
    CommandSpec::new(
        "replace",
        "Replace an existing module (by name) in a base cache with a mini-cache's module.",
        REPLACE_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "splice",
        "Splice a primitive-only mini-cache module into a base cache.",
        SPLICE_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "extract",
        "Extract one module into a standalone 1-module mini-cache (module + full tail tables).",
        EXTRACT_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "extract-remap",
        "Extract one module from a regen cache AND remap its bytecode refs to a base (vanilla) \
         cache's keys.",
        EXTRACT_REMAP_ARGS,
        Safety::write(),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "bytediff",
        "Semantic byte-faithfulness oracle: diff a VANILLA cache against a REGEN per function, \
         after normalizing away build noise. Classifies each aligned function IDENTICAL / \
         BENIGN-DIFF / SEMANTIC-DIFF.",
        BYTEDIFF_ARGS,
        Safety::write(),
        T_LONG,
    )
    .json(JsonSupport::OutputFile)
    .guide("scripts"),
];

pub const AS: GroupSpec = GroupSpec {
    tool: "gore_as",
    title: "gore as",
    cli: "as",
    summary: "AngelScript precompiled-cache tooling: inspect and decompile the shipped script \
              cache, patch scalar defaults in place, and compile authored modules back in. \
              Inspection is free; `compile` and `compile-module` launch the game and need \
              --allow-game-launch.",
    shape: GroupShape::Nested,
    commands: AS_COMMANDS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Class;

    #[test]
    fn the_group_size_matches_the_cli() {
        assert_eq!(AS.commands.len(), 20);
    }

    #[test]
    fn exactly_the_two_compilers_launch_the_game() {
        let launching: Vec<&str> = AS
            .commands
            .iter()
            .filter(|command| command.safety.worst_case() == Class::GameLaunch)
            .map(|command| command.sub)
            .collect();
        assert_eq!(launching, vec!["compile", "compile-module"]);
    }

    /// The generator's own deadline, from `gore-as` (`compile.rs`: `Duration::from_secs(30 * 60)`).
    const INNER_GENERATOR_TIMEOUT_SECS: u64 = 30 * 60;

    #[test]
    fn a_compile_outlives_the_generator_deadline_it_wraps() {
        // The outer clock starts before preflight and staging; the inner one only when the game is
        // launched. Equal budgets mean the outer kill always lands first on a long compile, and a
        // killed wrapper never reaches `CompileTransaction::restore_install` — the installation is
        // left staged. Whatever the numbers become, the outer one has to be the larger.
        for sub in ["compile", "compile-module"] {
            let command = AS.command(sub).expect("exists");
            assert!(
                command.timeout_secs > INNER_GENERATOR_TIMEOUT_SECS,
                "`as {sub}` is capped at {}s, which does not outlast the generator's own {}s",
                command.timeout_secs,
                INNER_GENERATOR_TIMEOUT_SECS
            );
        }
    }

    #[test]
    fn the_diagnostics_hook_stays_reachable_behind_the_game_launch_gate() {
        // Capturing AngelScript compiler errors is the only way an agent learns why a compile
        // failed; forcing it off would leave every failure silent. `--allow-game-launch` is the
        // gate, and it is not narrowed further here.
        for sub in ["compile", "compile-module"] {
            let command = AS.command(sub).expect("command exists");
            assert!(command.arg("diagnostics_hook").is_some(), "{sub}");
            assert!(command.forced_argv.is_empty(), "{sub} must not force diagnostics off");
        }
    }

    #[test]
    fn bytediff_declares_json_as_a_path_and_never_passes_it_implicitly() {
        let bytediff = AS.command("bytediff").expect("bytediff exists");
        assert_eq!(bytediff.json, JsonSupport::OutputFile);
        assert_eq!(bytediff.arg("json").expect("json argument").kind, Path);
        assert!(!bytediff.arg("json").expect("json argument").required);
    }

    #[test]
    fn the_tag_map_commands_are_flattened_into_this_group_like_clap_does() {
        // `TagMapCmd` is `#[command(flatten)]`-ed into `AsCmd`, so its two variants are ordinary
        // `gore as …` subcommands rather than a nested level.
        assert!(AS.command("tag-map-sites").is_some());
        assert!(AS.command("patch-tag-map").is_some());
    }

    #[test]
    fn only_compile_can_overwrite_the_installed_cache() {
        let in_place: Vec<&str> = AS
            .commands
            .iter()
            .filter(|command| command.safety.in_place_without.is_some())
            .map(|command| command.sub)
            .collect();
        assert_eq!(in_place, vec!["compile"]);
    }
}
