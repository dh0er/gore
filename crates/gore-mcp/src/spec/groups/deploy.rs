//! Textures, cooked data assets, mod bundles, and the multi-mod manager.
//!
//! This is where most of the commands that reach into the game installation live. Everything that
//! only *produces* a mod is [`Safety::write`]; everything that *installs* one is
//! [`Safety::mutate`], and `mgr reset` — which removes only a Manager-owned deployment — is
//! [`Safety::destructive`].
//!
//! Every `summary` and `help` string is copied verbatim from the corresponding clap doc comment.

use crate::spec::{
    ArgForm::{Long, Positional, Switch},
    ArgKind::{Bool, Int, Path, Str},
    ArgSpec, CommandSpec, Derived, GroupShape, GroupSpec, JsonSupport, Safety, T_FAST, T_LONG,
    T_NORMAL,
};

/// The optional game root, spelled the same way by many commands.
const GAME: ArgSpec = ArgSpec::new(
    "game",
    Long("game"),
    Path,
    "Game install root (the folder containing G1R/)",
    false,
)
.with_default("the configured game path, then Steam auto-detect");

// ---------------------------------------------------------------------------------------------
// gore_texture
// ---------------------------------------------------------------------------------------------

const TEXTURE_ASSET: ArgSpec = ArgSpec::new(
    "asset",
    Positional { order: 0 },
    Str,
    "Cooked asset path, e.g. /Game/UI/Textures/Common/T_HardwareCursor",
    true,
);

const TEXTURE_NAME: ArgSpec = ArgSpec::new(
    "name",
    Long("name"),
    Str,
    "Base name for the triplet, e.g. zzz_MyMod_P",
    true,
);

const TEXTURE_LIST_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Keep only asset paths containing this substring",
        false,
    ),
];

/// `paklist` is bounded by `--max` in the CLI itself, which is what keeps the installation's 4,577
/// pak entries from being clipped mid-path into a JSON document that no longer parses. The bound is
/// only useful if an agent can move it, so both narrowing flags are exposed here.
const TEXTURE_PAKLIST_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Keep only entry paths containing this substring (case-insensitive)",
        false,
    ),
    ArgSpec::new(
        "max",
        Long("max"),
        Int {
            min: Some(0),
            max: None,
        },
        "Max entries to print. The result states how many matched when it stops here; 0 lists \
         nothing and reports only the counts",
        false,
    )
    .with_default("100"),
];

const TEXTURE_EXTRACT_ARGS: &[ArgSpec] = &[
    GAME,
    TEXTURE_ASSET,
    ArgSpec::new("out", Long("out"), Path, "Output PNG path", true),
];

const TEXTURE_REPLACE_ARGS: &[ArgSpec] = &[
    GAME,
    TEXTURE_ASSET,
    ArgSpec::new(
        "image",
        Long("image"),
        Path,
        "Replacement PNG (RGBA8 / RGB8); dims need not match the original",
        true,
    ),
    ArgSpec::new(
        "mod_dir",
        Long("mod-dir"),
        Path,
        "Output mod dir; rewritten cooked files land under <mod_dir>/G1R/Content/…",
        true,
    ),
];

const TEXTURE_PACK_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "mod_dir",
        Long("mod-dir"),
        Path,
        "Mod dir laid out under its mount path (from `texture replace`)",
        true,
    ),
    TEXTURE_NAME,
    ArgSpec::new("out", Long("out"), Path, "Output dir for <name>.{utoc,ucas,pak}", true),
    ArgSpec::new(
        "compress",
        Switch("compress"),
        Bool,
        "Oodle-compress the .ucas blocks (opt-in). Default OFF: uncompressed containers are proven \
         to load in-game.",
        false,
    )
    .with_default("off"),
];

const TEXTURE_DEPLOY_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "triplet_dir",
        Long("triplet-dir"),
        Path,
        "Dir holding <name>.{utoc,ucas,pak} (the `texture pack` output dir)",
        true,
    ),
    TEXTURE_NAME,
];

const TEXTURE_INDEX_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output path (defaults to an immutable, generation-specific shared cache)",
        false,
    )
    .with_default("a generation-specific cache in the shared data dir"),
];

const TEXTURE_UNDEPLOY_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "name",
        Long("name"),
        Str,
        "Base name of the deployed triplet, e.g. zzz_MyMod_P",
        true,
    ),
];

/// `texture story-images` reads a plain directory, so it shares nothing with the container args.
const TEXTURE_STORY_IMAGES_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "game",
        Long("game"),
        Str,
        "Path to the game install dir (contains G1R/Content/Paks/…)",
        false,
    ),
    ArgSpec::new(
        "filter",
        Long("filter"),
        Str,
        "Keep only paths containing this substring (case-insensitive), e.g. `Glossary/Creatures`",
        false,
    ),
    ArgSpec::new(
        "absolute",
        Switch("absolute"),
        Bool,
        "Print the absolute file path instead of the name below the images dir",
        false,
    ),
];

const TEXTURE_COMMANDS: &[CommandSpec] = &[
    // The loose side of the same question `list` answers for containers: glossary portraits,
    // tutorial pictures and loading-screen art are plain files under `G1R/Story/Conversation`,
    // and `list` will never show them.
    CommandSpec::new(
        "story-images",
        "List the game's loose story images (glossary portraits, tutorial pictures, writings, \
         loading-screen art)",
        TEXTURE_STORY_IMAGES_ARGS,
        Safety::read(),
        T_FAST,
    )
    .guide("textures"),
    CommandSpec::new(
        "list",
        "List Texture2D assets in the game container",
        TEXTURE_LIST_ARGS,
        Safety::read(),
        T_LONG,
    )
    .guide("textures"),
    // The only command here that reads the plain `.pak` containers rather than the IoStore side.
    // It answers what `texture list` cannot: whether a path is already packed, and therefore
    // whether replacing that file on disk would ever be read.
    CommandSpec::new(
        "paklist",
        "List what the game's own .pak containers carry, so a loose-file destination can be \
         checked before it is written",
        TEXTURE_PAKLIST_ARGS,
        Safety::read(),
        T_FAST,
    )
    .json(JsonSupport::Stdout)
    .guide("textures"),
    // Writes two files: the PNG named by `out`, and a metadata sidecar at
    // `out.with_extension("png.json")` that no argument mentions (cmd/texture.rs).
    CommandSpec::new(
        "extract",
        "Extract a texture's top mip to a PNG",
        TEXTURE_EXTRACT_ARGS,
        Safety::write_truncating(&["out"]).also_writes(&[("out", Derived::Extension("png.json"))]),
        T_NORMAL,
    )
    .guide("textures"),
    // Writes `<mod_dir>/<mount path>/<leaf>.{uasset,uexp}` and either writes or *deletes* the
    // sibling `.ubulk` (cmd/texture.rs). Those paths come from `mount_dir(mod_dir, asset)`, a
    // mapping this layer would have to reimplement to check — so, like `gen`, it is gated instead
    // of described. Replacing the same asset twice is the ordinary case that overwrites.
    CommandSpec::new(
        "replace",
        "Replace a texture with a PNG, writing rewritten cooked files into a mod dir",
        TEXTURE_REPLACE_ARGS,
        Safety::mutate(),
        T_NORMAL,
    )
    .gated_because(
        "writes the rewritten `.uasset` and `.uexp` into the mod dir and deletes the sibling \
         `.ubulk`, replacing whatever an earlier replace of the same asset left there",
    )
    .guide("textures"),
    CommandSpec::new(
        "pack",
        "Pack a mod dir of cooked files into a Zen triplet (.utoc/.ucas/.pak)",
        TEXTURE_PACK_ARGS,
        // Producing a triplet in a scratch directory is a plain write; producing it in the
        // game's `~mods` is a deployment, which is what `texture deploy` is gated for.
        Safety::write().installs_via(&["out"]),
        T_LONG,
    )
    .guide("textures"),
    CommandSpec::new(
        "deploy",
        "Deploy a Zen triplet into the game's ~mods override folder",
        TEXTURE_DEPLOY_ARGS,
        Safety::mutate(),
        T_NORMAL,
    )
    .gated_because(
        "copies the triplet into the game's `~mods` folder, which is the installation the game \
         loads from on the next start",
    )
    .guide("textures"),
    // A full walk of every container in the installation. Slow by nature, and the output is large.
    CommandSpec::new(
        "index",
        "Build the texture index (asset->package_id) and cache it to the shared dir",
        TEXTURE_INDEX_ARGS,
        // With `out` this is a plain `TextureIndex::save`, an `fs::write` over that path. Without
        // it, the index goes to the shared data directory and `pin_and_prune_managed_texture_cache`
        // deletes the other cached generations there — shared state the GUI apps read, which is the
        // same reason `loc extract` is gated. The existing in-place idiom says exactly that.
        Safety::write_or_in_place(&["out"]),
        T_LONG,
    )
    .guide("textures"),
    CommandSpec::new(
        "undeploy",
        "Remove a previously-deployed triplet from the game's ~mods folder",
        TEXTURE_UNDEPLOY_ARGS,
        Safety::mutate(),
        T_NORMAL,
    )
    .gated_because("deletes the deployed triplet out of the game's `~mods` folder")
    .guide("textures"),
];

pub const TEXTURE: GroupSpec = GroupSpec {
    tool: "gore_texture",
    title: "gore texture",
    cli: "texture",
    summary: "Extract and replace game textures in the UE5 IoStore containers. The workflow is \
              list → extract → replace → pack → deploy; only the last two touch the installation. \
              paklist answers a separate question: which paths the game's own .pak containers \
              already carry, and therefore which loose files on disk can never be read.",
    shape: GroupShape::Nested,
    commands: TEXTURE_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_asset
// ---------------------------------------------------------------------------------------------

const ASSET_EXTRACT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "game",
        Long("game"),
        Path,
        "Gothic 1 Remake install root containing `G1R/`.",
        true,
    ),
    ArgSpec::new(
        "asset",
        Long("asset"),
        Str,
        "Exact cooked package path, beginning with `/Game/` and without an extension.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "New output directory; it must not exist and is never placed in the game tree.",
        true,
    ),
];

const ASSET_INSPECT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "uasset",
        Long("uasset"),
        Path,
        "Input legacy `.uasset`; the sibling `.uexp` is required.",
        true,
    ),
    ArgSpec::new(
        "usmap",
        Long("usmap"),
        Path,
        "Exact `.usmap` used to decode this package generation.",
        true,
    ),
    ArgSpec::new(
        "export_index",
        Long("export-index"),
        Int {
            min: Some(0),
            max: None,
        },
        "Inspect only this export; unsupported/missing selected exports are fatal.",
        false,
    ),
];

const ASSET_PATCH_FIXED_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "uasset",
        Long("uasset"),
        Path,
        "Input legacy `.uasset`; it is never modified.",
        true,
    ),
    ArgSpec::new(
        "usmap",
        Long("usmap"),
        Path,
        "Exact `.usmap` named by the selector.",
        true,
    ),
    ArgSpec::new(
        "extract_receipt",
        Long("extract-receipt"),
        Path,
        "Exact extract receipt that seals this package pair and USMAP generation.",
        true,
    ),
    ArgSpec::new(
        "selector",
        Long("selector"),
        Path,
        "JSON containing a selector, descriptor, or one inspect leaf object.",
        true,
    ),
    ArgSpec::new(
        "expected_hex",
        Long("expected-hex"),
        crate::spec::ArgKind::Hex,
        "Exact current raw little-endian wire bytes; must agree with the selector.",
        true,
    ),
    ArgSpec::new(
        "replacement_hex",
        Long("replacement-hex"),
        crate::spec::ArgKind::Hex,
        "Exact replacement wire bytes; no gameplay/domain validation is implied.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "New `.uasset` output; its sibling `.uexp` is created without clobbering.",
        true,
    ),
];

const ASSET_PACK_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "game",
        Long("game"),
        Path,
        "Gothic 1 Remake install root used only for the global script-object store.",
        true,
    ),
    ArgSpec::new(
        "uasset",
        Long("uasset"),
        Path,
        "Input legacy `.uasset`; its `.uexp` and optional same-stem sidecars are read.",
        true,
    ),
    ArgSpec::new(
        "patch_receipt",
        Long("patch-receipt"),
        Path,
        "Exact patch receipt produced beside `--uasset` by `patch-fixed`.",
        true,
    ),
    ArgSpec::new(
        "asset",
        Long("asset"),
        Str,
        "Exact target package path, beginning with `/Game/` and without an extension.",
        true,
    ),
    ArgSpec::new(
        "name",
        Long("name"),
        Str,
        "Safe filename stem for `<NAME>.{utoc,ucas,pak}`.",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "New output directory; it must not exist and is never placed in the game tree.",
        true,
    ),
];

const ASSET_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "extract",
        "Extract one current IoStore package into a sealed legacy package directory.",
        ASSET_EXTRACT_ARGS,
        Safety::write(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("dataassets"),
    CommandSpec::new(
        "inspect",
        "List structurally editable fixed-width leaves in a legacy split package.",
        ASSET_INSPECT_ARGS,
        Safety::read(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("dataassets"),
    CommandSpec::new(
        "patch-fixed",
        "Apply one snapshot-bound raw wire edit to a new package pair.",
        ASSET_PATCH_FIXED_ARGS,
        // The CLI checks only that the output is absent. `asset extract` additionally refuses a
        // destination in the game tree (`prepare_absent_output_directory`); this command does
        // not, so a fresh `.uasset` path below G1R would publish the pair, its sidecars and
        // the receipt straight into the live installation.
        Safety::write().installs_via(&["out"]),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("dataassets"),
    CommandSpec::new(
        "pack",
        "Pack one legacy package as an additive, undeployed Zen triplet.",
        ASSET_PACK_ARGS,
        // Same shape as `texture pack`: the triplet is an artifact to deploy later, unless it
        // is written straight into the install.
        Safety::write().installs_via(&["out"]),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    .guide("dataassets"),
];

pub const ASSET: GroupSpec = GroupSpec {
    tool: "gore_asset",
    title: "gore asset",
    cli: "asset",
    summary: "Edit cooked DataAssets by exact byte replacement, sealed end to end by receipts: \
              extract seals the package, inspect finds an editable leaf, patch-fixed swaps its \
              bytes under a content check, pack produces an undeployed triplet.",
    shape: GroupShape::Nested,
    commands: ASSET_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_mod
// ---------------------------------------------------------------------------------------------

const MOD_BUILD_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "spec",
        Long("spec"),
        Path,
        "Path to the build spec JSON; asset paths inside it resolve against its directory",
        true,
    ),
    ArgSpec::new(
        "out",
        Long("out"),
        Path,
        "Output directory (the bundle is written to <out>/<mod-name>)",
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

const MOD_INSPECT_ARGS: &[ArgSpec] = &[ArgSpec::new(
    "bundle",
    Positional { order: 0 },
    Path,
    "Bundle directory or ZIP containing one GORE bundle",
    true,
)];

const MOD_DEPLOY_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "bundle",
        Long("bundle"),
        Path,
        "Path to the bundle directory",
        true,
    ),
    GAME,
];

const MOD_UNDEPLOY_ARGS: &[ArgSpec] = &[GAME];

const MOD_INSPECT_COMMAND: CommandSpec = CommandSpec::new(
    "inspect",
    "Validate and summarize a built GORE bundle without importing or deploying it",
    MOD_INSPECT_ARGS,
    Safety::read(),
    T_NORMAL,
)
.json(JsonSupport::Stdout)
.guide("bundles");

const MOD_INSPECT_COMMANDS: &[CommandSpec] = &[MOD_INSPECT_COMMAND];

const MOD_COMMANDS: &[CommandSpec] = &[
    // `gore_mod::write_bundle` calls `remove_dir_all` on `<out>/<meta.name from the spec JSON>`
    // before rebuilding it (`cmd/modcmd.rs` joins exactly that, unsanitized), so a colliding mod
    // name erases whatever was there. The name lives inside the spec file -- one pointer away, so
    // the gate reads it and asks about the *rebuild* rather than about every build. A first build
    // into a directory that does not hold that bundle yet destroys nothing and now says so.
    CommandSpec::new(
        "build",
        "Build a bundle dir from a BuildSpec JSON",
        MOD_BUILD_ARGS,
        Safety::write()
            .also_writes(&[(
                "out",
                Derived::ChildNamedInJson {
                    arg: "spec",
                    pointer: "/meta/name",
                },
            )])
            .installs_via(&["out"]),
        T_LONG,
    )
    .guide("bundles"),
    MOD_INSPECT_COMMAND,
    CommandSpec::new(
        "deploy",
        "Deploy a built bundle to the game install",
        MOD_DEPLOY_ARGS,
        Safety::mutate(),
        T_LONG,
    )
    .gated_because(
        "installs the bundle into the game, backing up every file it replaces so that `undeploy` \
         can put them back",
    )
    .guide("bundles"),
    CommandSpec::new(
        "undeploy",
        "Undeploy the active mod (restore backups)",
        MOD_UNDEPLOY_ARGS,
        Safety::mutate(),
        T_NORMAL,
    )
    .gated_because(
        "removes the deployed bundle from the game and restores the files it backed up when it \
         was deployed",
    )
    .guide("bundles"),
];

pub const MOD: GroupSpec = GroupSpec {
    tool: "gore_mod",
    title: "gore mod",
    cli: "mod",
    summary:
        "Build, inspect, deploy, and undeploy a unified mod bundle — overrides, localization, \
              audio, voice, textures and scripts in one spec and one lifecycle.",
    shape: GroupShape::Nested,
    commands: MOD_COMMANDS,
};

/// Dedicated read-only route for inspecting a built bundle. The mixed `gore_mod` namespace must
/// advertise its Deploy/Undeploy worst case, while this alias can truthfully tell MCP clients that
/// it only reads the selected bundle and temporary inspection storage.
pub const MOD_INSPECT: GroupSpec = GroupSpec {
    tool: "gore_mod_inspect",
    title: "gore mod inspect",
    cli: "mod",
    summary: "Read-only validation and bounded inspection of one built GORE bundle. This does not \
              import, deploy, or resolve a game installation.",
    shape: GroupShape::Nested,
    commands: MOD_INSPECT_COMMANDS,
};

// ---------------------------------------------------------------------------------------------
// gore_mgr
// ---------------------------------------------------------------------------------------------

const LIBRARY: ArgSpec = ArgSpec::new("library", Long("library"), Path, "Library dir", false)
    .with_default("the shared per-user manager library");

const LOADOUT: ArgSpec = ArgSpec::new("loadout", Long("loadout"), Path, "Loadout file", false)
    .with_default("the shared per-user loadout");

const MGR_ID: ArgSpec = ArgSpec::new("id", Positional { order: 0 }, Str, "Library entry id", true);

const MGR_IMPORT_ARGS: &[ArgSpec] = &[
    ArgSpec::new(
        "path",
        Positional { order: 0 },
        Path,
        "Source folder / .zip / game file to import",
        true,
    ),
    LIBRARY,
    LOADOUT,
];

const MGR_LIBRARY_ONLY_ARGS: &[ArgSpec] = &[LIBRARY, LOADOUT];

const MGR_ID_ARGS: &[ArgSpec] = &[MGR_ID, LIBRARY, LOADOUT];

const MGR_ORDER_ARGS: &[ArgSpec] = &[
    MGR_ID,
    ArgSpec::new(
        "pos",
        Positional { order: 1 },
        Int {
            min: Some(0),
            max: None,
        },
        "New 0-based position (clamped to the last slot); 0 mounts first and loses conflicts",
        true,
    ),
    LIBRARY,
    LOADOUT,
];

const MGR_GAME_ARGS: &[ArgSpec] = &[GAME, LIBRARY, LOADOUT];

const MGR_RESET_ARGS: &[ArgSpec] = &[GAME];

const MGR_RECOVER_ARGS: &[ArgSpec] = &[
    GAME,
    ArgSpec::new(
        "expected_guard_id",
        Long("expected-guard-id"),
        Str,
        "Exact opaque action token reported by `mgr preflight`",
        true,
    ),
];

const MGR_PREFLIGHT_COMMAND: CommandSpec = CommandSpec::new(
    "preflight",
    "Inspect Manager readiness and recovery evidence without changing anything",
    MGR_GAME_ARGS,
    Safety::read(),
    T_NORMAL,
)
.json(JsonSupport::Stdout)
.guide("mod-manager");

const MGR_PREFLIGHT_COMMANDS: &[CommandSpec] = &[MGR_PREFLIGHT_COMMAND];

const MGR_COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "import",
        "Import a mod (folder, .zip, or single game file) into the library",
        MGR_IMPORT_ARGS,
        // A verified source/content/entry-id match may update an existing Manager entry. The
        // crash-safe publication either leaves an unchanged import alone or promotes the new tree
        // and cleans the previous payload before returning. It never touches the game installation.
        Safety::manager_write(),
        T_NORMAL,
    )
    .gated_because(
        "can replace a verified source, content, or entry-id match in the Manager library; changed \
         content is cleaned up during crash-safe publication, while an unchanged re-import is a \
         no-op",
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "list",
        "List library mods joined to their loadout state (enabled/order)",
        MGR_LIBRARY_ONLY_ARGS,
        // Not `read()`, despite doing nothing but print. `import::list` runs
        // `recover_interrupted_replacements` first, which finishes an import that was cut short:
        // it either renames the backup back into place or calls `transaction.cleanup()`, which
        // discards the superseded entry for good. It stays ungated — refusing to *list* a library
        // would be absurd — but nothing may advertise it as read-only when it can delete.
        Safety::manager_reconcile(),
        T_FAST,
    )
    .guide("mod-manager"),
    // Deletes the imported mod from the library, then rewrites the loadout without it. Nothing
    // here is created, and nothing puts it back: the user has to re-import from wherever the mod
    // originally came from, which they may no longer have. `enable`, `disable` and `order` stay
    // ungated Manager edits because each is undone by its own inverse; this one is not.
    CommandSpec::new(
        "remove",
        "Remove a mod from the library (and drop it from the loadout)",
        MGR_ID_ARGS,
        Safety::destructive(),
        T_FAST,
    )
    .gated_because(
        "deletes the mod from the library and drops it from the loadout, and nothing here puts it \
         back: it has to be imported again from wherever it came from",
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "enable",
        "Enable a loadout entry (it will deploy on the next apply)",
        MGR_ID_ARGS,
        Safety::manager_edit(),
        T_FAST,
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "disable",
        "Disable a loadout entry (it will not deploy)",
        MGR_ID_ARGS,
        Safety::manager_edit(),
        T_FAST,
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "order",
        "Move a loadout entry to a new position (0 = mounts first, loses conflicts)",
        MGR_ORDER_ARGS,
        Safety::manager_edit(),
        T_FAST,
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "analyze",
        "Report conflicts among the enabled loadout mods",
        MGR_LIBRARY_ONLY_ARGS,
        // Not `read()`, despite doing nothing but print. `import::list` runs
        // `recover_interrupted_replacements` first, which finishes an import that was cut short:
        // it either renames the backup back into place or calls `transaction.cleanup()`, which
        // discards the superseded entry for good. It stays ungated — refusing to *list* a library
        // would be absurd — but nothing may advertise it as read-only when it can delete.
        Safety::manager_reconcile(),
        T_NORMAL,
    )
    .guide("mod-manager"),
    MGR_PREFLIGHT_COMMAND,
    CommandSpec::new(
        "recover",
        "Recover one exact abandoned Manager mutation selected by its preflight token",
        MGR_RECOVER_ARGS,
        Safety::mutate(),
        T_LONG,
    )
    .json(JsonSupport::Stdout)
    // The CLI asks y/N by default. MCP stdin carries JSON-RPC and cannot answer that prompt, so the
    // server's own consent gate stands in for it and the child receives the explicit confirmation.
    .forced(&["--yes"])
    .gated_because(
        "repairs the exact abandoned Manager installation change selected by the preflight token \
         and may restore its recorded pristine state or preserve a change that already completed",
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "apply",
        "Compose the enabled loadout into one deployment against the game",
        MGR_GAME_ARGS,
        Safety::mutate(),
        T_LONG,
    )
    .gated_because(
        "deploys the whole enabled loadout into the game installation, replacing what is deployed \
         there now",
    )
    .guide("mod-manager"),
    CommandSpec::new(
        "status",
        "Show whether the game is in sync with the target loadout",
        MGR_GAME_ARGS,
        // Status opens the authoritative Store snapshot. Like list/analyze, that may persist a
        // valid loadout reconciliation, so clients must not advertise the call as read-only. It is
        // intentionally ungated: the repair only canonicalizes Manager-owned state.
        Safety::manager_reconcile(),
        T_NORMAL,
    )
    .json(JsonSupport::Stdout)
    .guide("mod-manager"),
    // Removes only a Manager-owned deployment and restores its pristine base. A Studio deployment
    // is refused and preserved, including if ownership changes during the operation.
    CommandSpec::new(
        "reset",
        "Undeploy the active Manager deployment (never removes a Studio deploy)",
        MGR_RESET_ARGS,
        Safety::destructive(),
        T_LONG,
    )
    .gated_because(
        "undeploys the active Manager deployment and restores its pristine base while refusing to \
         remove a Studio deployment",
    )
    .guide("mod-manager"),
];

pub const MGR: GroupSpec = GroupSpec {
    tool: "gore_mgr",
    title: "gore mgr",
    cli: "mgr",
    summary: "Run several mods at once: import and order them, inspect readiness, conflicts, and \
              status, recover an interrupted Manager change, and apply or reset the whole loadout \
              as one composed deployment.",
    shape: GroupShape::Nested,
    commands: MGR_COMMANDS,
};

/// A dedicated read-only route for the most common Manager diagnosis. `gore_mgr` must advertise
/// the worst case of its Apply/Reset leaves, which makes clients put a generic write approval in
/// front of even this strictly read-only call. The original `gore_mgr`/`preflight` route remains
/// available for compatibility; new clients can use this accurately annotated alias directly.
pub const MGR_PREFLIGHT: GroupSpec = GroupSpec {
    tool: "gore_mgr_preflight",
    title: "gore mgr preflight",
    cli: "mgr",
    summary: "Read-only Manager readiness and recovery evidence. This never applies, repairs, or \
              resets anything.",
    shape: GroupShape::Nested,
    commands: MGR_PREFLIGHT_COMMANDS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Class;

    #[test]
    fn the_group_sizes_match_the_cli() {
        assert_eq!(TEXTURE.commands.len(), 9);
        assert_eq!(ASSET.commands.len(), 4);
        assert_eq!(MOD.commands.len(), 4);
        assert_eq!(MOD_INSPECT.commands.len(), 1);
        assert_eq!(MOD_INSPECT.worst_case(), Class::Read);
        assert_eq!(MGR.commands.len(), 12);
    }

    #[test]
    fn manager_reads_that_may_reconcile_state_are_not_advertised_as_read_only() {
        // `mgr list` and `mgr analyze` may finish an interrupted library replacement. All three
        // open the authoritative Store snapshot, which may persist a valid loadout reconciliation.
        // Some clients auto-approve read-only tools; these must not claim that annotation.
        for sub in ["list", "analyze", "status"] {
            let command = MGR.command(sub).expect("exists");
            assert_ne!(
                command.safety.worst_case(),
                Class::Read,
                "`mgr {sub}` may reconcile Manager state and cannot be advertised as read-only"
            );
            // Still ungated: canonical Store reconciliation is part of taking an authoritative
            // Manager snapshot, not a user-requested install or library mutation.
            assert!(!command.safety.worst_case().needs_write_permission());
        }

        let preflight = MGR.command("preflight").expect("exists");
        assert_eq!(preflight.safety.worst_case(), Class::Read);
        assert_eq!(preflight.json, JsonSupport::Stdout);

        assert_eq!(MGR_PREFLIGHT.commands.len(), 1);
        assert_eq!(MGR_PREFLIGHT.commands[0].sub, "preflight");
        assert_eq!(MGR_PREFLIGHT.worst_case(), Class::Read);
    }

    #[test]
    fn manager_recovery_is_token_bound_gated_and_noninteractive_over_mcp() {
        let recover = MGR.command("recover").expect("exists");
        assert_eq!(recover.safety.worst_case(), Class::Mutate);
        assert!(recover.safety.worst_case().needs_write_permission());
        assert_eq!(recover.json, JsonSupport::Stdout);
        assert_eq!(recover.forced_argv, ["--yes"]);
        assert!(recover.arg("expected_guard_id").is_some());

        let reset = MGR.command("reset").expect("exists");
        assert!(
            reset
                .gated_because
                .is_some_and(|reason| reason.contains("Studio deployment")),
            "reset consent must disclose its Manager-only ownership boundary"
        );
    }

    #[test]
    fn only_commands_that_delete_user_content_are_destructive() {
        // `reset` restores a pristine install; `remove` deletes an imported mod from the library
        // and nothing puts it back. Both take something away that the user supplied.
        let destructive: Vec<&str> = [TEXTURE, ASSET, MOD, MGR]
            .iter()
            .flat_map(|group| group.commands.iter())
            .filter(|command| command.safety.worst_case() == Class::Destructive)
            .map(|command| command.sub)
            .collect();
        assert_eq!(destructive, vec!["remove", "reset"]);
    }

    #[test]
    fn exactly_the_installing_and_deleting_commands_need_allow_write() {
        // The two `deploy`/`undeploy` pairs are texture and mod; `recover`, `apply`, and `reset` are
        // the manager's install-wide operations. The rest are here because they overwrite or
        // delete paths this layer cannot check: `remove` destroys library content, `import` may
        // replace a verified source/content/entry-id match, and `texture replace` rewrites cooked
        // files under a mount-mapped path. `texture index` is here only in its `out`-less form, where it
        // publishes into the shared data directory and prunes the other cached generations;
        // `worst_case` reports that shape.
        //
        // `mod build` is deliberately absent. It also clears a directory before rebuilding it, but
        // the directory is `<out>/<meta.name from the spec>` and the spec is JSON, so the gate can
        // read the name and ask about the rebuild rather than about the first build.
        let gated: Vec<&str> = [TEXTURE, ASSET, MOD, MGR]
            .iter()
            .flat_map(|group| group.commands.iter())
            .filter(|command| command.safety.worst_case().needs_write_permission())
            .map(|command| command.sub)
            .collect();
        assert_eq!(
            gated,
            vec![
                "replace", "deploy", "index", "undeploy", "deploy", "undeploy", "import", "remove",
                "recover", "apply", "reset"
            ]
        );

        let build = MOD.command("build").expect("exists");
        assert!(!build.safety.worst_case().needs_write_permission());
        assert!(build
            .safety
            .derives
            .iter()
            .any(|(_, how)| how.reads_a_file()));
    }

    #[test]
    fn the_asset_group_is_uniformly_machine_readable() {
        // Every asset command has a `--json` switch, and the server always passes it, so the whole
        // receipt-driven workflow is structured output end to end.
        for command in ASSET.commands {
            assert_eq!(
                command.json,
                JsonSupport::Stdout,
                "{} is not JSON",
                command.sub
            );
        }
    }
}
