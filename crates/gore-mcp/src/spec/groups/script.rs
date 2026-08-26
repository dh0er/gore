//! AngelScript precompiled-cache tooling — the largest group, and the only one that can start the
//! game.
//!
//! Two things here are unlike anything else in the table:
//!
//! - `compile` and `compile-module` expose an explicit standalone/game backend policy. Their
//!   worst case remains a game launch, but the per-call gate lets explicit strict standalone run
//!   without game-launch consent. Ordinary build outputs need no install-write consent either;
//!   an output explicitly aimed into the game installation remains protected.
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

const CACHE_FILE: ArgSpec = ArgSpec::new(
    "file",
    Positional { order: 0 },
    Path,
    "Precompiled cache file to read.",
    true,
);

const CACHE_POSITIONAL: ArgSpec = ArgSpec::new(
    "cache",
    Positional { order: 0 },
    Path,
    "Cache to read or patch.",
    true,
);

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

const MODULE_FILTER: ArgSpec = ArgSpec::new(
    "module",
    Long("module"),
    Str,
    "Exact module-name filter.",
    false,
);
const CLASS_FILTER: ArgSpec = ArgSpec::new(
    "class",
    Long("class"),
    Str,
    "Exact class-name filter.",
    false,
);
const FIELD_FILTER: ArgSpec = ArgSpec::new(
    "field",
    Long("field"),
    Str,
    "Exact field-name filter.",
    false,
);

/// The diagnostics trio shared by `compile` and `compile-module`.
///
/// The hook captures AngelScript compiler errors from the running game, which is the only way an
/// agent gets to see why a compile failed — without it a failed compile returns no error text at
/// all. It stays available; a game-capable backend is gated by `--allow-game-launch` plus
/// `--allow-write`, while strict standalone never loads this runtime hook.
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
    Int {
        min: Some(0),
        max: Some(30_000),
    },
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
        Int {
            min: Some(0),
            max: None,
        },
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
        Int {
            min: Some(0),
            max: None,
        },
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
        Int {
            min: Some(0),
            max: None,
        },
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
        Int {
            min: Some(0),
            max: None,
        },
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
    ArgSpec::new(
        "tag",
        Long("tag"),
        Str,
        "Exact GameplayTag global name filter.",
        false,
    ),
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

const QUALIFY_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "usmap",
        Long("usmap"),
        Path,
        "Exact `.usmap` reflection dump to qualify against. Omit to select one from the install, \
         which refuses rather than choosing when two dumps both fit this executable.",
        false,
    ),
    ArgSpec::new(
        "catalog",
        Long("catalog"),
        Path,
        "A previously published `story_catalog.v1` document, used to name the curated script \
         modules and their sealed source. Omit when the build is already audited; the catalog is \
         then built from the install itself.",
        false,
    ),
    ArgSpec::new(
        "id",
        Long("id"),
        Str,
        "Proposed `GenerationRow::id` for the draft.",
        false,
    )
    .with_default("`g1r-steam-<script cache GUID prefix>`, a placeholder"),
    ArgSpec::new(
        "label",
        Long("label"),
        Str,
        "Proposed `GenerationRow::label`, the banner a person reads.",
        false,
    )
    .with_default("derived from the id"),
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
        "Complete authoritative `.as` source tree. Missing base modules are explicit deletes.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Publish the complete cache here with atomic no-clobber semantics. Must be outside the \
         game installation.",
        true,
    ),
    ArgSpec::new(
        "work_dir",
        Long("work-dir"),
        Path,
        "Existing private workspace outside the game installation.",
        true,
    ),
    GAME,
    ArgSpec::new(
        "backend",
        Long("backend"),
        Enum(&["game", "standalone", "standalone-then-game"]),
        "Product-owned compiler selection. Standalone package paths and hashes are never \
         accepted from this command line.",
        false,
    )
    .with_default("standalone-then-game"),
    ArgSpec::new(
        "generation_receipt",
        Long("generation-receipt"),
        Path,
        "Publish a product-authoritative full-graph receipt when the installed target matches an \
         embedded qualified compiler package.",
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
        "Existing persistent compiler workspace outside the game installation, used for the \
         emitted tree and intermediate compiler cache.",
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
    ArgSpec::new(
        "backend",
        Long("backend"),
        Enum(&["game", "standalone", "standalone-then-game"]),
        "Compiler policy. GORE resolves its catalog-authenticated standalone package \
         automatically; the game compiler is the visible fallback by default.",
        false,
    )
    .with_default("standalone-then-game"),
    ArgSpec::new(
        "development_standalone_sidecar",
        Long("development-standalone-sidecar"),
        Path,
        "Development-only native standalone sidecar override. All development override values \
         are required together and conflict with `generation_receipt`.",
        false,
    ),
    ArgSpec::new(
        "development_standalone_sidecar_sha256",
        Long("development-standalone-sidecar-sha256"),
        Hex,
        "Development-only SHA-256 of the exact override sidecar.",
        false,
    ),
    ArgSpec::new(
        "development_compiler_profile_manifest",
        Long("development-compiler-profile-manifest"),
        Path,
        "Development-only typed compiler-profile manifest.",
        false,
    ),
    ArgSpec::new(
        "development_compiler_profile_root",
        Long("development-compiler-profile-root"),
        Path,
        "Development-only root containing every sealed compiler-profile payload.",
        false,
    ),
    ArgSpec::new(
        "development_standalone_scratch_root",
        Long("development-standalone-scratch-root"),
        Path,
        "Development-only existing private scratch root used by the override sidecar.",
        false,
    ),
    ArgSpec::new(
        "generation_receipt",
        Long("generation-receipt"),
        Path,
        "Publish a local V1 no-clobber receipt after automatic product-package authentication.",
        false,
    ),
    NO_DIAGNOSTICS,
    DIAGNOSTICS_HOOK,
    DIAGNOSTICS_DELAY,
];

// Dedicated MCP routes expose the ordinary product-owned standalone workflow without a backend
// selector. Besides making the useful arguments directly typed, this lets clients see truthful
// non-destructive annotations instead of inheriting the game-launching worst case of `gore_as`.
const STANDALONE_COMPILE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "src",
        Positional { order: 0 },
        Path,
        "Complete authoritative `.as` source tree. Missing base modules are explicit deletes.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Publish the complete cache here with atomic no-clobber semantics. Must be outside the game installation.",
        true,
    ),
    ArgSpec::new(
        "work_dir",
        Long("work-dir"),
        Path,
        "Existing private workspace outside the game installation.",
        true,
    ),
    GAME,
    ArgSpec::new(
        "generation_receipt",
        Long("generation-receipt"),
        Path,
        "Publish a product-authoritative full-graph receipt when the installed target matches the bundled compiler.",
        false,
    ),
];

const STANDALONE_COMPILE_MODULE_ARGS: &[ArgSpec] = &[
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
    ArgSpec::new(
        "source",
        Long("source"),
        Path,
        "Authored `.as` source file to overlay.",
        true,
    ),
    ArgSpec::new(
        "work_dir",
        Long("work-dir"),
        Path,
        "Existing persistent compiler workspace outside the game installation.",
        true,
    ),
    ArgSpec::new(
        "allow_new_symbols",
        Switch("allow-new-symbols"),
        Bool,
        "Retain minimal rows for classes, functions, and names absent from the pristine cache.",
        false,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path for the remapped one-module mini-cache. Existing paths are refused.",
        true,
    ),
    GAME,
    ArgSpec::new(
        "generation_receipt",
        Long("generation-receipt"),
        Path,
        "Publish a local no-clobber receipt after bundled-compiler authentication.",
        false,
    ),
];

const REPLACE_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "base",
        Positional { order: 0 },
        Path,
        "Base cache to patch.",
        true,
    ),
    ArgSpec::new(
        "mini",
        Positional { order: 1 },
        Path,
        "Mini-cache holding the new module.",
        true,
    ),
    ArgSpec::new(
        "target",
        Positional { order: 2 },
        Str,
        "Name of the module in the base cache to replace.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path for the patched cache.",
        true,
    ),
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
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path for the spliced cache.",
        true,
    ),
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
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path for the 1-module mini-cache.",
        true,
    ),
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
        Int {
            min: Some(0),
            max: None,
        },
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
        "Scan length-prefixed type-name strings (decode investigation aid). The input must be a \
         module cache: the scan starts after the outer header, so the `0x9e377abe` magic is \
         checked first and an arbitrary blob is refused rather than scanned.",
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
        // Writes every decompiled module under `outdir` with `fs::write`. The module paths come from
        // the cache being read, so hand edits in that tree are truncated and nothing here can check
        // for them first -- but an empty or absent `outdir` has no hand edits to lose.
        Safety::write().clobbers_dir(&["outdir"]),
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
        Safety::write().installs_via(&["out"]),
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
        Safety::write().installs_via(&["out"]),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("angelscript-defaults"),
    // Reads the installation and writes nothing — not even the row it proposes, which is why a
    // maintainer still has to paste it. `Safety::read()` is therefore exact rather than generous.
    CommandSpec::new(
        "qualify",
        "Derive an installed build's generation row and qualification artifact. Reads the game and \
         writes nothing: it proposes a row for a person to add, and says what it could not measure.",
        QUALIFY_ARGS,
        Safety::read(),
        T_LONG,
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
        "Compile a complete authoritative AngelScript tree into a new precompiled cache. Uses the \
         requested standalone/game policy; only game or explicit fallback may launch the game.",
        COMPILE_ARGS,
        Safety::game_launch_except("backend", "standalone"),
        T_COMPILE,
    )
    .at_most_one(DIAGNOSTICS_CONFLICT)
    .guide("scripts"),
    CommandSpec::new(
        "compile-module",
        "Compile one authored module into a deployable 1-module mini-cache. Wraps the complete \
         Studio pipeline with an explicit standalone/game policy.",
        COMPILE_MODULE_ARGS,
        Safety::game_launch_except("backend", "standalone")
            .writes_into(&["out", "generation_receipt"]),
        T_COMPILE,
    )
    .at_most_one(DIAGNOSTICS_CONFLICT)
    .guide("scripts"),
    // These five publish with a plain `std::fs::write` over whatever is at the destination
    // (cmd/as_cache.rs) -- unlike `patch-default` and `patch-tag-map`, which refuse an occupied
    // output themselves. `bytediff` is the odd one: its `--json` is a report path, not a switch.
    CommandSpec::new(
        "replace",
        "Replace an existing module (by name) in a base cache with a mini-cache's module.",
        REPLACE_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "splice",
        "Splice a primitive-only mini-cache module into a base cache.",
        SPLICE_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "extract",
        "Extract one module into a standalone 1-module mini-cache (module + full tail tables).",
        EXTRACT_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "extract-remap",
        "Extract one module from a regen cache AND remap its bytecode refs to a base (vanilla) \
         cache's keys.",
        EXTRACT_REMAP_ARGS,
        Safety::write_truncating(&["out"]),
        T_NORMAL,
    )
    .guide("scripts"),
    CommandSpec::new(
        "bytediff",
        "Semantic byte-faithfulness oracle: diff a VANILLA cache against a REGEN per function, \
         after normalizing away build noise. Classifies each aligned function IDENTICAL / \
         BENIGN-DIFF / SEMANTIC-DIFF.",
        BYTEDIFF_ARGS,
        Safety::write_truncating(&["json"]),
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
              Compilation has an explicit standalone/game policy: strict standalone runs offline \
              without game-launch consent, while outputs aimed into the installation remain protected; \
              game and fallback-capable calls require game-launch and install-write consent.",
    shape: GroupShape::Nested,
    commands: AS_COMMANDS,
};

const STANDALONE_COMPILE_COMMANDS: &[CommandSpec] = &[CommandSpec::new(
    "compile",
    "Compile a complete AngelScript tree with GORE's bundled standalone compiler. This never starts the game or stages files in the installation.",
    STANDALONE_COMPILE_ARGS,
    Safety::write(),
    T_COMPILE,
)
.forced(&["--backend", "standalone"])
.hides_cli_flags(&[
    "no-diagnostics",
    "diagnostics-hook",
    "diagnostics-inject-delay-ms",
])
.guide("scripts")];

pub const AS_COMPILE: GroupSpec = GroupSpec {
    tool: "gore_as_compile",
    title: "gore as compile (standalone)",
    cli: "as",
    summary: "Strict standalone full-tree AngelScript compilation with native diagnostics and no game-launch or install-write consent.",
    shape: GroupShape::Nested,
    commands: STANDALONE_COMPILE_COMMANDS,
};

const STANDALONE_COMPILE_MODULE_COMMANDS: &[CommandSpec] = &[CommandSpec::new(
    "compile-module",
    "Compile one authored AngelScript module with GORE's bundled standalone compiler. This never starts the game; ordinary build outputs need no consent, while outputs aimed into the installation remain protected.",
    STANDALONE_COMPILE_MODULE_ARGS,
    Safety::write().writes_into(&["out", "generation_receipt"]),
    T_COMPILE,
)
.forced(&["--backend", "standalone"])
.hides_cli_flags(&[
    "development-standalone-sidecar",
    "development-standalone-sidecar-sha256",
    "development-compiler-profile-manifest",
    "development-compiler-profile-root",
    "development-standalone-scratch-root",
    "no-diagnostics",
    "diagnostics-hook",
    "diagnostics-inject-delay-ms",
])
.guide("scripts")];

pub const AS_COMPILE_MODULE: GroupSpec = GroupSpec {
    tool: "gore_as_compile_module",
    title: "gore as compile-module (standalone)",
    cli: "as",
    summary: "Strict standalone one-module AngelScript compilation with native diagnostics, no game-launch consent, and install protection only when an output targets the game tree.",
    shape: GroupShape::Nested,
    commands: STANDALONE_COMPILE_MODULE_COMMANDS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Class;

    #[test]
    fn the_group_size_matches_the_cli() {
        assert_eq!(AS.commands.len(), 21);
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

    #[test]
    fn strict_standalone_is_offline_but_game_capable_policies_keep_the_worst_case() {
        for sub in ["compile", "compile-module"] {
            let command = AS.command(sub).expect("compiler command");
            let standalone = serde_json::json!({ "backend": "standalone" })
                .as_object()
                .expect("object")
                .clone();
            let game = serde_json::json!({ "backend": "game" })
                .as_object()
                .expect("object")
                .clone();
            let fallback = serde_json::json!({ "backend": "standalone-then-game" })
                .as_object()
                .expect("object")
                .clone();

            assert_eq!(command.safety.effective(&standalone), Class::Write, "{sub}");
            assert_eq!(command.safety.effective(&game), Class::GameLaunch, "{sub}");
            assert_eq!(
                command.safety.effective(&fallback),
                Class::GameLaunch,
                "{sub}"
            );
            assert_eq!(
                command.safety.effective(&serde_json::Map::new()),
                Class::GameLaunch,
                "{sub}"
            );
        }
    }

    #[test]
    fn product_compilers_default_to_standalone_then_game_and_manual_paths_are_development_only() {
        for sub in ["compile", "compile-module"] {
            let backend = AS
                .command(sub)
                .expect("compiler command")
                .arg("backend")
                .expect("backend argument");
            assert_eq!(backend.default_hint, Some("standalone-then-game"), "{sub}");
        }

        let module = AS.command("compile-module").expect("compile-module");
        for name in [
            "development_standalone_sidecar",
            "development_standalone_sidecar_sha256",
            "development_compiler_profile_manifest",
            "development_compiler_profile_root",
            "development_standalone_scratch_root",
        ] {
            assert!(module.arg(name).is_some(), "missing {name}");
        }
        for product_forbidden in [
            "standalone_sidecar",
            "standalone_sidecar_sha256",
            "compiler_profile_manifest",
            "compiler_profile_root",
            "standalone_scratch_root",
        ] {
            assert!(
                module.arg(product_forbidden).is_none(),
                "normal product schema still exposes {product_forbidden}"
            );
        }
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
    fn the_diagnostics_hook_stays_reachable_for_game_capable_backends() {
        // Capturing AngelScript compiler errors is the only way an agent learns why a compile
        // failed; forcing it off would leave every game-backed failure silent. Strict standalone
        // uses native diagnostics and never loads this hook.
        for sub in ["compile", "compile-module"] {
            let command = AS.command(sub).expect("command exists");
            assert!(command.arg("diagnostics_hook").is_some(), "{sub}");
            assert!(
                command.forced_argv.is_empty(),
                "{sub} must not force diagnostics off"
            );
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
    fn compiler_commands_never_claim_an_in_place_cache_overwrite() {
        let in_place: Vec<&str> = AS
            .commands
            .iter()
            .filter(|command| command.safety.in_place_without.is_some())
            .map(|command| command.sub)
            .collect();
        assert!(in_place.is_empty());
    }
}
