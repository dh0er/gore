use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use gore_as::cache::header::CacheHeader;
use gore_as::cache::scan::scan_strings;
use gore_as::cache::splice::splice_auto;
use gore_as::cache::walk_modules::{module_count, module_region_end};

#[derive(Subcommand)]
pub enum AsCmd {
    /// Parse and print the outer cache header.
    DecodeHeader { file: PathBuf },
    /// Scan length-prefixed type-name strings (decode investigation aid).
    Walk {
        file: PathBuf,
        #[arg(long, default_value_t = 100)]
        max: usize,
    },
    /// Print module count + TAIL_OFF (the splice insertion point) for a cache.
    Info { file: PathBuf },
    /// Decompile functions whose name contains <needle> to structured AngelScript.
    Decompile {
        file: PathBuf,
        /// Substring filter on `module.Class::func` (default: all).
        #[arg(default_value = "")]
        needle: String,
        /// Max functions to print.
        #[arg(long, default_value_t = 20)]
        max: usize,
    },
    /// Emit ALL modules as recompilable .as into <outdir>, mirroring ScriptRelativeFilename.
    EmitAll { file: PathBuf, outdir: PathBuf },
    /// Emit recompilable .as for modules whose name contains <needle>.
    Emit {
        file: PathBuf,
        #[arg(default_value = "")]
        needle: String,
        #[arg(long, default_value_t = 5)]
        max: usize,
    },
    /// Dump StaticNames tail-table entries (the `n"..."` FName-literal pool indexed by
    /// `__STATIC_NAME(Id)`). With no indices: count + first 10 entries.
    StaticNames {
        file: PathBuf,
        /// Specific indices to print.
        indices: Vec<i64>,
    },
    /// Disassemble functions whose name contains <needle> to an asBC listing.
    Disasm {
        file: PathBuf,
        #[arg(default_value = "")]
        needle: String,
        #[arg(long, default_value_t = 20)]
        max: usize,
    },
    /// List uniquely patchable scalar assignments in generated `__InitDefaults` bytecode.
    /// Only an exact, audited SetV/LoadThisR/WRTV pattern is reported.
    DefaultSites {
        cache: PathBuf,
        /// Exact module-name filter.
        #[arg(long)]
        module: Option<String>,
        /// Exact class-name filter.
        #[arg(long)]
        class: Option<String>,
        /// Exact field-name filter.
        #[arg(long)]
        field: Option<String>,
        /// Emit one machine-readable JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Copy-on-write patch one `default-sites` scalar using semantic lookup plus raw CAS.
    PatchDefault {
        cache: PathBuf,
        /// Strict selector JSON copied from `default-sites --json`.
        #[arg(long, value_name = "SELECTOR.json")]
        selector: PathBuf,
        /// Complete current serialized immediate as lowercase hex (V1/V2/V4: 4 bytes; V8: 8).
        #[arg(long, value_name = "HEX")]
        expected_hex: String,
        /// Complete replacement serialized immediate as lowercase hex.
        #[arg(long, value_name = "HEX")]
        replacement_hex: String,
        /// New full cache path. Existing paths are never overwritten.
        #[arg(short, long)]
        out: PathBuf,
        /// Emit one machine-readable JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Sealed native GameplayTag-map inspection and patching commands. Both require exactly
    /// matching bounded Binds/USMAP evidence, discovered by game layout or GORE_AS_BINDS and
    /// GORE_AS_USMAP; missing, ambiguous, or mismatched evidence fails closed.
    #[command(flatten)]
    TagMap(TagMapCmd),
    /// Offline-check whether the optional diagnostics hook has one safe AOB match. Does not launch
    /// the game or change the installation.
    DiagnosticsCheck {
        /// Exact executable to scan (supports non-Steam/custom layouts).
        #[arg(long, conflicts_with = "game")]
        exe: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Compile AngelScript into a precompiled cache by driving the game's own
    /// `-as-generate-precompiled-data` flag. With no SRC, recompiles the loose `.as` already under
    /// `<game>/G1R/Script/`; with SRC, stages that tree first. With `-o`, writes the cache there and
    /// leaves the install untouched; without `-o`, installs the fresh cache in place (backing the
    /// previous one up to `*.gore-bak`). All the backup / stage / restore file handling is internal.
    Compile {
        /// Source `.as` tree (a directory) to compile. Omit to recompile the loose `.as` already
        /// installed under `<game>/G1R/Script/`.
        src: Option<PathBuf>,
        /// Write the compiled cache here and leave the game install untouched. Omit to install the
        /// fresh cache in place under `Script/`.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Game install root (the folder containing `G1R/`). Falls back to the configured game
        /// path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
        /// When installing in place, do NOT back up the previous cache.
        #[arg(long)]
        no_backup: bool,
        /// Disable the optional runtime compiler-diagnostic hook and use the normal generator.
        #[arg(long, conflicts_with = "diagnostics_hook")]
        no_diagnostics: bool,
        /// Explicit `gore-as-diagnostics-hook.dll`; otherwise use environment, sibling, then the
        /// integrity-checked embedded helper.
        #[arg(long, value_name = "DLL")]
        diagnostics_hook: Option<PathBuf>,
        /// Delay between game launch and diagnostics injection (loader warm-up).
        #[arg(
            long,
            default_value_t = 2000,
            value_name = "MS",
            value_parser = clap::value_parser!(u64).range(0..=30_000)
        )]
        diagnostics_inject_delay_ms: u64,
    },
    /// Compile one authored module into a deployable 1-module mini-cache. This wraps the complete
    /// Studio pipeline: emit the pristine source tree, overlay one `.as` file, drive the game
    /// compiler, extract the resulting module, and remap it back to the pristine cache.
    CompileModule {
        /// `add` for a new module or `edit` for an existing module.
        #[arg(long, value_parser = ["add", "edit"])]
        op: String,
        /// Expected module name. For `add`, the compiler-detected module name is reported and used.
        #[arg(long)]
        module: String,
        /// Safe path of the authored file relative to the game's `Script/` tree.
        #[arg(long)]
        rel_path: String,
        /// Authored `.as` source file to overlay.
        #[arg(long)]
        source: PathBuf,
        /// Persistent compiler workspace used for the emitted tree and intermediate regen cache.
        #[arg(long)]
        work_dir: PathBuf,
        /// Explicitly retain minimal rows for classes/functions/names absent from the pristine
        /// cache. Normally used with `--op add`; strict remapping remains the default.
        #[arg(long)]
        allow_new_symbols: bool,
        /// Output path for the remapped 1-module mini-cache.
        #[arg(short, long)]
        out: PathBuf,
        /// Game install root. Falls back to configured path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
        /// Disable the optional runtime compiler-diagnostic hook and use the normal generator.
        #[arg(long, conflicts_with = "diagnostics_hook")]
        no_diagnostics: bool,
        /// Explicit `gore-as-diagnostics-hook.dll`; otherwise use environment, sibling, then the
        /// integrity-checked embedded helper.
        #[arg(long, value_name = "DLL")]
        diagnostics_hook: Option<PathBuf>,
        /// Delay between game launch and diagnostics injection (loader warm-up).
        #[arg(
            long,
            default_value_t = 2000,
            value_name = "MS",
            value_parser = clap::value_parser!(u64).range(0..=30_000)
        )]
        diagnostics_inject_delay_ms: u64,
    },
    /// Replace an existing module (by name) in a base cache with a mini-cache's module.
    Replace {
        base: PathBuf,
        mini: PathBuf,
        target: String,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Splice a primitive-only mini-cache module into a base cache.
    Splice {
        /// Base cache (e.g. PrecompiledScript_Shipping.Cache).
        base: PathBuf,
        /// Mini-cache from -as-generate-precompiled-data (one primitive-only module).
        mini: PathBuf,
        /// Output path for the spliced cache.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Extract one module into a standalone 1-module mini-cache (module + full tail tables).
    /// Lets a dependency-heavy edited module be pulled from a full-tree regen and Replace'd
    /// into the vanilla base.
    Extract {
        /// Source cache (e.g. a full-tree regen).
        cache: PathBuf,
        /// Module name (the Modules TMap key) to extract.
        module: String,
        /// Output path for the 1-module mini-cache.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Extract one module from a regen cache AND remap its bytecode refs to a base (vanilla)
    /// cache's keys, normally emitting a 1-module mini with EMPTY tail tables. With
    /// --allow-new-symbols, the mini instead carries only the genuinely-new rows required by the
    /// module. The result can be Replace'd into the base without copying the regen's full tables.
    ExtractRemap {
        /// Regen cache (full-tree -as-generate-precompiled-data output) containing the edit.
        regen_cache: PathBuf,
        /// Module name (the Modules TMap key) to extract + remap.
        module: String,
        /// Base (vanilla) cache whose keys the module's refs are rewritten to.
        base_cache: PathBuf,
        /// Explicitly carry minimal tail-table rows for symbols absent from the base. Existing
        /// symbols still remap to vanilla; pointer/id collisions are re-keyed deterministically.
        #[arg(long)]
        allow_new_symbols: bool,
        /// Output path for the remapped 1-module mini-cache.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Semantic byte-faithfulness oracle: diff a VANILLA cache against a REGEN (re-compilation of
    /// our decompiled source) per function, after normalizing away build-noise (ref keys N1, jump
    /// absolutes N3, constant encodings N4; opt-in slot-allocation proofs N2). Classifies each aligned
    /// function IDENTICAL / BENIGN-DIFF / SEMANTIC-DIFF. See specs/semantic-oracle.md.
    Bytediff {
        /// Vanilla reference cache (e.g. samples/cache_A.Cache).
        vanilla: PathBuf,
        /// Regen cache (re-compilation of our decompiled .as tree).
        regen: PathBuf,
        /// Only diff modules whose name contains this substring.
        #[arg(long)]
        module: Option<String>,
        /// Only diff functions whose display name (module.Class::func) contains this substring.
        #[arg(long)]
        func: Option<String>,
        /// Filter output to a verdict: identical|benign|semantic (repeatable).
        #[arg(long = "verdict")]
        verdicts: Vec<String>,
        /// List which normalizers fired for BENIGN-DIFF functions (default: summary only).
        #[arg(long)]
        show_benign: bool,
        /// Instruction window (±N) around each SEMANTIC divergence.
        #[arg(long, default_value_t = 6)]
        context: usize,
        /// Enable OPT-IN fail-closed N2 slot-allocation normalization (default OFF; see FORMAT.md).
        #[arg(long = "norm-slots")]
        norm_slots: bool,
        /// Disable the N5 `FScopeCycleCounter` RAII profiler-scope strip (default ON; §B.2).
        #[arg(long = "no-norm-scope")]
        no_norm_scope: bool,
        /// Disable the N6 dominated boolean-cascade re-guard fold (default ON; §B.1).
        #[arg(long = "no-norm-reguard")]
        no_norm_reguard: bool,
        /// Write a machine-readable JSON scoreboard (per-verdict counts + alignment loss) here.
        #[arg(long)]
        json: Option<PathBuf>,
        /// Exit non-zero if any SEMANTIC-DIFF is found (CI gate).
        #[arg(long)]
        fail_on_semantic: bool,
    },
}

#[derive(Subcommand)]
pub enum TagMapCmd {
    /// List sealed native GameplayTag-to-float32 map-entry defaults.
    ///
    /// Requires exact bounded Binds.Cache and USMAP evidence. Discovery uses the game layout or
    /// GORE_AS_BINDS and GORE_AS_USMAP; missing, ambiguous, or mismatched evidence fails closed.
    TagMapSites {
        /// Regular cache to inspect (maximum 512 MiB). Requires exact Binds.Cache and USMAP via
        /// game layout or GORE_AS_BINDS/GORE_AS_USMAP; missing/ambiguous/mismatched evidence fails
        /// closed.
        cache: PathBuf,
        /// Exact module-name filter.
        #[arg(long)]
        module: Option<String>,
        /// Exact class-name filter.
        #[arg(long)]
        class: Option<String>,
        /// Exact field-name filter.
        #[arg(long)]
        field: Option<String>,
        /// Exact GameplayTag global name filter.
        #[arg(long)]
        tag: Option<String>,
        /// Emit one machine-readable JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Copy-on-write patch one sealed GameplayTag-to-float32 map entry using semantic CAS.
    ///
    /// Requires exact bounded Binds.Cache and USMAP evidence. Discovery uses the game layout or
    /// GORE_AS_BINDS and GORE_AS_USMAP; missing, ambiguous, or mismatched evidence fails closed.
    PatchTagMap {
        /// Regular cache to copy-on-write patch (maximum 512 MiB). Requires exact Binds.Cache and
        /// USMAP via game layout or GORE_AS_BINDS/GORE_AS_USMAP; missing/ambiguous/mismatched
        /// evidence fails closed.
        cache: PathBuf,
        /// Strict selector JSON extracted from `.sites[N].selector` in `tag-map-sites --json`.
        #[arg(long, value_name = "SELECTOR.json")]
        selector: PathBuf,
        /// Fresh current raw IEEE-754 float32 little-endian bytes: exactly 8 lowercase hex chars.
        #[arg(long, value_name = "HEX")]
        expected_hex: String,
        /// Replacement raw IEEE-754 float32 little-endian bytes: exactly 8 lowercase hex chars.
        #[arg(long, value_name = "HEX")]
        replacement_hex: String,
        /// New full cache path. Existing paths are never overwritten.
        #[arg(short, long)]
        out: PathBuf,
        /// Emit one machine-readable JSON document.
        #[arg(long)]
        json: bool,
    },
}

const DEFAULT_SELECTOR_MAX_BYTES: u64 = 64 * 1024;
const DEFAULT_USMAP_MAX_BYTES: u64 = 128 * 1024 * 1024;
const DEFAULT_BINDS_MAX_BYTES: u64 = 128 * 1024 * 1024;
const TAG_MAP_CACHE_MAX_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_USMAP_MAX_DIRECTORY_ENTRIES: usize = 1_024;
const DEFAULT_USMAP_MAX_CANDIDATES: usize = 16;
const TAG_MAP_SITES_REPORT_FORMAT: &str = "gore-as-tag-map-sites-v1";
const TAG_MAP_PATCH_REPORT_FORMAT: &str = "gore-as-tag-map-patch-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefaultSelectorJson {
    format: String,
    kind: String,
    module: String,
    class: String,
    field_owner: String,
    field: String,
    value_type: String,
    /// Required nullable field: `null` for direct/script ancestry, exact profile ID for a native
    /// ancestry proof. `serde_json::Value` intentionally distinguishes missing from explicit null.
    ancestry_profile: serde_json::Value,
}

impl DefaultSelectorJson {
    fn from_core(selector: &gore_as::cache::default_patch::DefaultSiteSelector) -> Self {
        Self {
            format: gore_as::cache::default_patch::DEFAULT_SITE_SELECTOR_FORMAT.to_owned(),
            kind: "scalar".to_owned(),
            module: selector.module.clone(),
            class: selector.class.clone(),
            field_owner: selector.field_owner.clone(),
            field: selector.field.clone(),
            value_type: selector.value_type.clone(),
            ancestry_profile: selector
                .ancestry_profile
                .as_ref()
                .map_or(serde_json::Value::Null, |profile| {
                    serde_json::Value::String(profile.clone())
                }),
        }
    }

    fn into_core(self) -> Result<gore_as::cache::default_patch::DefaultSiteSelector> {
        if self.format != gore_as::cache::default_patch::DEFAULT_SITE_SELECTOR_FORMAT {
            bail!(
                "AS_DEFAULT_SELECTOR: unsupported format {:?}; expected {:?}",
                self.format,
                gore_as::cache::default_patch::DEFAULT_SITE_SELECTOR_FORMAT
            );
        }
        if self.kind != "scalar" {
            bail!(
                "AS_DEFAULT_SELECTOR: unsupported kind {:?}; expected \"scalar\"",
                self.kind
            );
        }
        for (name, value) in [
            ("module", self.module.as_str()),
            ("class", self.class.as_str()),
            ("field_owner", self.field_owner.as_str()),
            ("field", self.field.as_str()),
            ("value_type", self.value_type.as_str()),
        ] {
            if value.is_empty() || value.trim() != value {
                bail!("AS_DEFAULT_SELECTOR: {name} must be nonempty and have no outer whitespace");
            }
        }
        let ancestry_profile = match self.ancestry_profile {
            serde_json::Value::Null => None,
            serde_json::Value::String(profile)
                if !profile.is_empty() && profile.trim() == profile =>
            {
                Some(profile)
            }
            serde_json::Value::String(_) => bail!(
                "AS_DEFAULT_SELECTOR: ancestry_profile must be null or a nonempty string with no outer whitespace"
            ),
            _ => bail!("AS_DEFAULT_SELECTOR: ancestry_profile must be null or a string"),
        };
        Ok(gore_as::cache::default_patch::DefaultSiteSelector {
            module: self.module,
            class: self.class,
            field_owner: self.field_owner,
            field: self.field,
            value_type: self.value_type,
            ancestry_profile,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TagMapSelectorJson {
    format: String,
    kind: String,
    module: String,
    class: String,
    field_owner: String,
    field: String,
    tag_module: String,
    tag_namespace: String,
    tag: String,
    tag_is_string: bool,
    value_type: String,
    map_proof_id: String,
    ancestry_profile: String,
}

impl TagMapSelectorJson {
    fn from_core(selector: &gore_as::cache::native_tag_map::NativeTagMapSelector) -> Self {
        Self {
            format: selector.format.clone(),
            kind: selector.kind.clone(),
            module: selector.module.clone(),
            class: selector.class.clone(),
            field_owner: selector.field_owner.clone(),
            field: selector.field.clone(),
            tag_module: selector.tag_module.clone(),
            tag_namespace: selector.tag_namespace.clone(),
            tag: selector.tag.clone(),
            tag_is_string: selector.tag_is_string,
            value_type: selector.value_type.clone(),
            map_proof_id: selector.map_proof_id.clone(),
            ancestry_profile: selector.ancestry_profile.clone(),
        }
    }

    fn into_core(self) -> Result<gore_as::cache::native_tag_map::NativeTagMapSelector> {
        use gore_as::cache::default_ancestry::{
            DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID, DEFAULT_NATIVE_ANCESTRY_PROFILE_ID,
        };
        use gore_as::cache::native_tag_map::{
            NATIVE_TAG_MAP_SELECTOR_FORMAT, NATIVE_TAG_MAP_SELECTOR_KIND, NATIVE_TAG_MAP_VALUE_TYPE,
        };
        if self.format != NATIVE_TAG_MAP_SELECTOR_FORMAT {
            bail!(
                "AS_TAG_MAP_SELECTOR: unsupported format {:?}; expected {:?}",
                self.format,
                NATIVE_TAG_MAP_SELECTOR_FORMAT
            );
        }
        if self.kind != NATIVE_TAG_MAP_SELECTOR_KIND {
            bail!(
                "AS_TAG_MAP_SELECTOR: unsupported kind {:?}; expected {:?}",
                self.kind,
                NATIVE_TAG_MAP_SELECTOR_KIND
            );
        }
        if self.value_type != NATIVE_TAG_MAP_VALUE_TYPE {
            bail!(
                "AS_TAG_MAP_SELECTOR: unsupported value_type {:?}; expected {:?}",
                self.value_type,
                NATIVE_TAG_MAP_VALUE_TYPE
            );
        }
        for (name, value) in [
            ("module", self.module.as_str()),
            ("class", self.class.as_str()),
            ("field_owner", self.field_owner.as_str()),
            ("field", self.field.as_str()),
            ("tag_namespace", self.tag_namespace.as_str()),
            ("tag", self.tag.as_str()),
            ("map_proof_id", self.map_proof_id.as_str()),
            ("ancestry_profile", self.ancestry_profile.as_str()),
        ] {
            if value.is_empty() || value.trim() != value {
                bail!("AS_TAG_MAP_SELECTOR: {name} must be nonempty and have no outer whitespace");
            }
        }
        if !self.tag_module.is_empty() {
            bail!("AS_TAG_MAP_SELECTOR: tag_module must be the empty string");
        }
        if self.tag_namespace != "GameplayTag" {
            bail!("AS_TAG_MAP_SELECTOR: tag_namespace must be exactly \"GameplayTag\"");
        }
        if self.tag_is_string {
            bail!("AS_TAG_MAP_SELECTOR: tag_is_string must be false");
        }
        if self.map_proof_id != DEFAULT_GAMEPLAY_TAG_FLOAT32_MAP_PROOF_ID {
            bail!(
                "AS_TAG_MAP_SELECTOR: map_proof_id does not match the sealed GameplayTag-float32 map proof"
            );
        }
        if self.ancestry_profile != DEFAULT_NATIVE_ANCESTRY_PROFILE_ID {
            bail!(
                "AS_TAG_MAP_SELECTOR: ancestry_profile does not match the sealed native ancestry profile"
            );
        }
        Ok(gore_as::cache::native_tag_map::NativeTagMapSelector {
            format: self.format,
            kind: self.kind,
            module: self.module,
            class: self.class,
            field_owner: self.field_owner,
            field: self.field,
            tag_module: self.tag_module,
            tag_namespace: self.tag_namespace,
            tag: self.tag,
            tag_is_string: self.tag_is_string,
            value_type: self.value_type,
            map_proof_id: self.map_proof_id,
            ancestry_profile: self.ancestry_profile,
        })
    }
}

#[derive(Serialize)]
struct DefaultSitesJson<'a> {
    format: &'static str,
    cache: CacheProofJson,
    site_count: usize,
    stats: DefaultStatsJson,
    sites: Vec<DefaultSiteJson<'a>>,
}

#[derive(Serialize)]
struct CacheProofJson {
    path: String,
    length: usize,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct EvidenceFileProofJson {
    path: String,
    length: usize,
    sha256: String,
}

#[derive(Serialize)]
struct DefaultStatsJson {
    init_functions: usize,
    branched_init_functions: usize,
    direct_windows: usize,
    unresolved_fields: usize,
    unresolved_types: usize,
    unsupported_types: usize,
    ambiguous_fields: usize,
}

#[derive(Serialize)]
struct DefaultSiteJson<'a> {
    selector: DefaultSelectorJson,
    value_type: &'a str,
    display_value: &'a str,
    encoding: &'static str,
    expected_hex: String,
    provenance: DefaultProvenanceJson<'a>,
}

#[derive(Serialize)]
struct DefaultProvenanceJson<'a> {
    function: &'a str,
    field_owner: &'a str,
    owner_type_id: String,
    member_offset: i32,
    pattern: &'static str,
    context_sha256: &'a str,
    opcode: &'static str,
    instruction_index: usize,
    instruction_offset_dwords: usize,
    operand_offset: usize,
    length: usize,
}

#[derive(Serialize)]
struct DefaultPatchJson<'a> {
    format: &'static str,
    status: &'static str,
    selector: DefaultSelectorJson,
    input: CacheProofJson,
    output: CacheProofJson,
    expected_hex: String,
    replacement_hex: String,
    provenance: DefaultProvenanceJson<'a>,
}

#[derive(Serialize)]
struct TagMapFingerprintJson {
    format: &'static str,
    sha256: String,
    scalar_operand_count: usize,
    tag_operand_count: usize,
    ancestry_profile: String,
    map_proof_id: String,
}

#[derive(Serialize)]
struct TagMapStatsJson {
    init_functions: usize,
    branched_init_functions: usize,
    raw_windows: usize,
    reference_proven_windows: usize,
    native_field_proven_windows: usize,
    missing_owner_types: usize,
    missing_properties: usize,
    property_owner_mismatches: usize,
    missing_tag_globals: usize,
    non_gameplay_tag_globals: usize,
    missing_callees: usize,
    non_exact_tmap_add_callees: usize,
    non_native_owner_identities: usize,
    unsealed_native_fields: usize,
    missing_target_classes: usize,
    target_ancestry_mismatches: usize,
    ambiguous_selectors: usize,
}

#[derive(Serialize)]
struct TagMapProvenanceJson<'a> {
    function: &'a str,
    context_sha256: &'a str,
    operand_offset: usize,
    length: usize,
    field_schema_proof_id: &'a str,
}

#[derive(Serialize)]
struct TagMapSiteJson<'a> {
    selector: TagMapSelectorJson,
    display_value: String,
    encoding: &'static str,
    expected_hex: String,
    provenance: TagMapProvenanceJson<'a>,
}

#[derive(Serialize)]
struct TagMapSitesJson<'a> {
    format: &'static str,
    cache: CacheProofJson,
    cache_guid: String,
    fingerprint: TagMapFingerprintJson,
    binds: EvidenceFileProofJson,
    usmap: EvidenceFileProofJson,
    site_count: usize,
    stats: TagMapStatsJson,
    sites: Vec<TagMapSiteJson<'a>>,
}

#[derive(Serialize)]
struct TagMapPatchJson<'a> {
    format: &'static str,
    status: &'static str,
    selector: TagMapSelectorJson,
    input: CacheProofJson,
    output: CacheProofJson,
    cache_guid: String,
    fingerprint: TagMapFingerprintJson,
    binds: EvidenceFileProofJson,
    usmap: EvidenceFileProofJson,
    expected_hex: String,
    replacement_hex: String,
    provenance: TagMapProvenanceJson<'a>,
}

/// Locate and load the native API arities from Binds.Cache: `GORE_AS_BINDS` env if set, else a
/// `Binds.Cache` sitting next to the input cache file. Absent/unparsable => None (no fallback).
fn load_native_api(cache_file: &std::path::Path) -> Option<gore_as::cache::binds::NativeApi> {
    load_native_api_with_proof(cache_file).map(|(api, _)| api)
}

fn native_api_path(cache_file: &Path) -> Option<PathBuf> {
    Some(match std::env::var_os("GORE_AS_BINDS") {
        Some(path) => PathBuf::from(path),
        None => cache_file.parent()?.join("Binds.Cache"),
    })
}

fn load_native_api_with_proof(
    cache_file: &Path,
) -> Option<(gore_as::cache::binds::NativeApi, EvidenceFileProofJson)> {
    let path = native_api_path(cache_file)?;
    let bytes = match read_regular_bounded(&path, DEFAULT_BINDS_MAX_BYTES, "AS_DEFAULT_BINDS") {
        Ok(bytes) => bytes,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            return None;
        }
        Err(error) => {
            eprintln!(
                "warning: failed to read bounded {}: {error:#}",
                path.display()
            );
            return None;
        }
    };
    let proof = evidence_file_proof(&path, &bytes);
    match gore_as::cache::binds::NativeApi::from_bytes(&bytes) {
        Some(api) => {
            eprintln!("loaded native arities from {}", path.display());
            Some((api, proof))
        }
        None => {
            eprintln!("warning: failed to parse {}", path.display());
            None
        }
    }
}

struct DefaultMutationEvidence {
    native: Option<gore_as::cache::binds::NativeApi>,
    ancestry: Option<gore_as::cache::default_ancestry::DefaultNativeAncestry>,
    binds: Option<EvidenceFileProofJson>,
    usmap: Option<EvidenceFileProofJson>,
}

#[derive(Clone, Copy)]
enum DefaultEvidencePolicy {
    ScalarFallback,
    RequiredTagMap,
}

/// Resolve USMAP candidates without trusting a Steam location or versioned filename. An explicit
/// `GORE_AS_USMAP` is the sole candidate. Otherwise only regular `.usmap` files in the game-layout
/// directory relative to `<G1R>/Script/<cache>` are considered; their content seals remain the
/// authority.
fn default_usmap_candidates(
    cache_file: &Path,
    configured: Option<PathBuf>,
) -> Result<Vec<PathBuf>> {
    if let Some(path) = configured {
        return Ok(vec![path]);
    }
    let Some(script_dir) = cache_file.parent() else {
        return Ok(Vec::new());
    };
    if !script_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("Script"))
    {
        return Ok(Vec::new());
    }
    let Some(g1r_dir) = script_dir.parent() else {
        return Ok(Vec::new());
    };
    let directory = g1r_dir.join("Binaries").join("Win64").join("ue4ss");
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("AS_DEFAULT_USMAP: enumerating {}", directory.display()));
        }
    };
    let mut candidates = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= DEFAULT_USMAP_MAX_DIRECTORY_ENTRIES {
            bail!(
                "AS_DEFAULT_USMAP: {} contains more than {} entries",
                directory.display(),
                DEFAULT_USMAP_MAX_DIRECTORY_ENTRIES
            );
        }
        let entry = entry.with_context(|| {
            format!(
                "AS_DEFAULT_USMAP: enumerating an entry in {}",
                directory.display()
            )
        })?;
        let kind = entry.file_type().with_context(|| {
            format!(
                "AS_DEFAULT_USMAP: reading file type for {}",
                entry.path().display()
            )
        })?;
        let path = entry.path();
        if kind.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("usmap"))
        {
            if candidates.len() >= DEFAULT_USMAP_MAX_CANDIDATES {
                bail!(
                    "AS_DEFAULT_USMAP: {} contains more than {} .usmap candidates",
                    directory.display(),
                    DEFAULT_USMAP_MAX_CANDIDATES
                );
            }
            candidates.push(path);
        }
    }
    candidates.sort();
    Ok(candidates)
}

fn read_default_usmap(path: &Path) -> Result<Vec<u8>> {
    read_regular_bounded(path, DEFAULT_USMAP_MAX_BYTES, "AS_DEFAULT_USMAP")
}

fn read_tag_map_cache(path: &Path) -> Result<Vec<u8>> {
    read_regular_bounded(path, TAG_MAP_CACHE_MAX_BYTES, "AS_TAG_MAP_INPUT")
}

fn read_regular_bounded(path: &Path, limit: u64, label: &'static str) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("{label}: opening {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("{label}: reading metadata for {}", path.display()))?;
    if !metadata.is_file() {
        bail!("{label}: {} is not a regular file", path.display());
    }
    if metadata.len() > limit {
        bail!(
            "{label}: {} is {} bytes; limit is {}",
            path.display(),
            metadata.len(),
            limit
        );
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("{label}: reading {}", path.display()))?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > limit {
        bail!(
            "{label}: {} changed size while being read or exceeded the limit",
            path.display()
        );
    }
    Ok(bytes)
}

fn evidence_file_proof(path: &Path, bytes: &[u8]) -> EvidenceFileProofJson {
    EvidenceFileProofJson {
        path: path.display().to_string(),
        length: bytes.len(),
        sha256: gore_as::cache::default_patch::encode_hex(&Sha256::digest(bytes)),
    }
}

/// Load optional native mutation evidence. Every failure deliberately preserves the existing
/// scalar-only path: the sealed Binds data may still prove direct native field types, while no
/// native-grandparent ancestry is supplied.
fn load_default_mutation_evidence(
    cache_file: &Path,
    cache: &[u8],
    policy: DefaultEvidencePolicy,
) -> DefaultMutationEvidence {
    let Some((loaded_native, binds)) = load_native_api_with_proof(cache_file) else {
        return DefaultMutationEvidence {
            native: None,
            ancestry: None,
            binds: None,
            usmap: None,
        };
    };
    let native = Some(loaded_native);
    let native_ref = native.as_ref().expect("just populated native evidence");
    let configured = std::env::var_os("GORE_AS_USMAP").map(PathBuf::from);
    let candidates = match default_usmap_candidates(cache_file, configured) {
        Ok(candidates) => candidates,
        Err(error) => {
            eprintln!("warning: USMAP autodiscovery failed closed: {error:#}");
            Vec::new()
        }
    };
    let mut matches = Vec::new();
    for path in candidates {
        let result = (|| -> Result<_> {
            let bytes = read_default_usmap(&path)?;
            let proof = evidence_file_proof(&path, &bytes);
            let schemas = gore_asset::SchemaDb::from_usmap(&bytes)
                .map_err(anyhow::Error::from)
                .context("AS_DEFAULT_USMAP: parsing sealed schema map")?;
            let profile = gore_as::cache::default_ancestry::DefaultNativeAncestry::from_schema_db(
                native_ref, cache, &schemas,
            )
            .map_err(anyhow::Error::from)
            .context("AS_DEFAULT_ANCESTRY: validating cache/Binds/USMAP tuple")?;
            Ok((profile, proof))
        })();
        match result {
            Ok((profile, proof)) => matches.push((path, profile, proof)),
            Err(error) => eprintln!(
                "warning: {} is not usable native-default evidence: {error:#}",
                path.display()
            ),
        }
    }
    let (ancestry, usmap) = match matches.len() {
        1 => {
            let (path, profile, proof) = matches.pop().expect("one match");
            eprintln!(
                "loaded sealed native-default ancestry {} from {}",
                profile.profile_id(),
                path.display()
            );
            (Some(profile), Some(proof))
        }
        0 => {
            match policy {
                DefaultEvidencePolicy::ScalarFallback => eprintln!(
                    "native-default ancestry unavailable; using strict scalar-only fallback"
                ),
                DefaultEvidencePolicy::RequiredTagMap => eprintln!(
                    "warning: sealed native-default ancestry required for tag-map operation but unavailable"
                ),
            }
            (None, None)
        }
        count => {
            match policy {
                DefaultEvidencePolicy::ScalarFallback => eprintln!(
                    "warning: {count} sealed USMAP candidates matched; refusing ambiguous native-default ancestry"
                ),
                DefaultEvidencePolicy::RequiredTagMap => eprintln!(
                    "warning: {count} sealed USMAP candidates matched; tag-map operation requires exactly one"
                ),
            }
            (None, None)
        }
    };
    DefaultMutationEvidence {
        native,
        ancestry,
        binds: Some(binds),
        usmap,
    }
}

struct RequiredTagMapEvidence {
    ancestry: gore_as::cache::default_ancestry::DefaultNativeAncestry,
    binds: EvidenceFileProofJson,
    usmap: EvidenceFileProofJson,
}

fn load_required_tag_map_evidence(
    cache_file: &Path,
    cache: &[u8],
) -> Result<RequiredTagMapEvidence> {
    let evidence =
        load_default_mutation_evidence(cache_file, cache, DefaultEvidencePolicy::RequiredTagMap);
    Ok(RequiredTagMapEvidence {
        ancestry: evidence.ancestry.context(
            "AS_TAG_MAP_ANCESTRY: sealed cache/Binds/USMAP evidence is required; refusing fallback",
        )?,
        binds: evidence.binds.context(
            "AS_TAG_MAP_BINDS: bounded sealed Binds evidence is required; refusing fallback",
        )?,
        usmap: evidence.usmap.context(
            "AS_TAG_MAP_USMAP: bounded sealed USMAP evidence is required; refusing fallback",
        )?,
    })
}

fn default_provenance_json(
    site: &gore_as::cache::default_patch::DefaultSite,
) -> DefaultProvenanceJson<'_> {
    DefaultProvenanceJson {
        function: &site.function,
        field_owner: &site.field_owner,
        owner_type_id: format!("0x{:x}", site.owner_type_id as u32),
        member_offset: site.member_offset,
        pattern: site.pattern.as_str(),
        context_sha256: &site.context_sha256,
        opcode: site.opcode,
        instruction_index: site.instruction_index,
        instruction_offset_dwords: site.instruction_offset_dw,
        operand_offset: site.operand_offset,
        length: site.encoding.width(),
    }
}

fn default_site_json(site: &gore_as::cache::default_patch::DefaultSite) -> DefaultSiteJson<'_> {
    DefaultSiteJson {
        selector: DefaultSelectorJson::from_core(&site.selector),
        value_type: &site.value_type,
        display_value: &site.display_value,
        encoding: site.encoding.as_str(),
        expected_hex: gore_as::cache::default_patch::encode_hex(&site.expected),
        provenance: default_provenance_json(site),
    }
}

fn tag_map_fingerprint_json(
    report: &gore_as::cache::native_tag_map::NativeTagMapReport,
) -> TagMapFingerprintJson {
    TagMapFingerprintJson {
        format: report.fingerprint_format(),
        sha256: gore_as::cache::default_patch::encode_hex(&report.fingerprint_sha256()),
        scalar_operand_count: report.scalar_operand_count(),
        tag_operand_count: report.tag_operand_count(),
        ancestry_profile: report.ancestry_profile_id().to_owned(),
        map_proof_id: report.map_proof_id().to_owned(),
    }
}

fn tag_map_stats_json(
    stats: &gore_as::cache::native_tag_map::NativeTagMapStats,
) -> TagMapStatsJson {
    TagMapStatsJson {
        init_functions: stats.init_functions(),
        branched_init_functions: stats.branched_init_functions(),
        raw_windows: stats.raw_windows(),
        reference_proven_windows: stats.reference_proven_windows(),
        native_field_proven_windows: stats.native_field_proven_windows(),
        missing_owner_types: stats.missing_owner_types(),
        missing_properties: stats.missing_properties(),
        property_owner_mismatches: stats.property_owner_mismatches(),
        missing_tag_globals: stats.missing_tag_globals(),
        non_gameplay_tag_globals: stats.non_gameplay_tag_globals(),
        missing_callees: stats.missing_callees(),
        non_exact_tmap_add_callees: stats.non_exact_tmap_add_callees(),
        non_native_owner_identities: stats.non_native_owner_identities(),
        unsealed_native_fields: stats.unsealed_native_fields(),
        missing_target_classes: stats.missing_target_classes(),
        target_ancestry_mismatches: stats.target_ancestry_mismatches(),
        ambiguous_selectors: stats.ambiguous_selectors(),
    }
}

fn tag_map_provenance_json(
    site: &gore_as::cache::native_tag_map::NativeTagMapSite,
) -> TagMapProvenanceJson<'_> {
    let range = site.operand_range();
    TagMapProvenanceJson {
        function: site.function(),
        context_sha256: site.context_sha256(),
        operand_offset: range.start,
        length: range.len(),
        field_schema_proof_id: site.field_schema_proof_id(),
    }
}

fn tag_map_site_json(
    site: &gore_as::cache::native_tag_map::NativeTagMapSite,
) -> TagMapSiteJson<'_> {
    let expected = site.expected();
    TagMapSiteJson {
        selector: TagMapSelectorJson::from_core(site.selector()),
        display_value: f32::from_le_bytes(expected).to_string(),
        encoding: "le_f32",
        expected_hex: gore_as::cache::default_patch::encode_hex(&expected),
        provenance: tag_map_provenance_json(site),
    }
}

fn cache_proof(path: &Path, bytes: &[u8]) -> CacheProofJson {
    CacheProofJson {
        path: path.display().to_string(),
        length: bytes.len(),
        sha256: gore_as::cache::default_patch::encode_hex(&Sha256::digest(bytes)),
    }
}

fn read_default_selector(path: &Path) -> Result<DefaultSelectorJson> {
    read_bounded_selector(path, "AS_DEFAULT_SELECTOR")
}

fn read_tag_map_selector(path: &Path) -> Result<TagMapSelectorJson> {
    read_bounded_selector(path, "AS_TAG_MAP_SELECTOR")
}

fn read_bounded_selector<T>(path: &Path, label: &'static str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let file = std::fs::File::open(path)
        .with_context(|| format!("{label}: opening {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("{label}: reading metadata for {}", path.display()))?;
    if !metadata.is_file() {
        bail!(
            "{label}: selector is not a regular file: {}",
            path.display()
        );
    }
    if metadata.len() > DEFAULT_SELECTOR_MAX_BYTES {
        bail!(
            "{label}: selector is {} bytes; limit is {}",
            metadata.len(),
            DEFAULT_SELECTOR_MAX_BYTES
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(DEFAULT_SELECTOR_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("{label}: reading {}", path.display()))?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > DEFAULT_SELECTOR_MAX_BYTES {
        bail!(
            "{label}: selector changed size while reading or exceeded the {}-byte limit",
            DEFAULT_SELECTOR_MAX_BYTES,
        );
    }
    serde_json::from_slice(&bytes)
        .with_context(|| format!("{label}: parsing strict JSON from {}", path.display()))
}

fn decode_default_hex(value: &str, label: &'static str) -> Result<Vec<u8>> {
    if !matches!(value.len(), 8 | 16)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("{label}: expected exactly 8 or 16 lowercase hexadecimal characters without 0x");
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = hex_nibble(pair[0]).expect("validated hex");
        let low = hex_nibble(pair[1]).expect("validated hex");
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn decode_tag_map_hex(value: &str, label: &'static str) -> Result<[u8; 4]> {
    if value.len() != 8
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("{label}: expected exactly 8 lowercase hexadecimal characters without 0x");
    }
    let bytes = decode_default_hex(value, label)?;
    Ok(bytes.try_into().expect("validated tag-map hex width"))
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn publish_default_cache_noclobber(path: &Path, bytes: &[u8]) -> Result<Vec<u8>> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        bail!(
            "AS_DEFAULT_OUTPUT: output parent is not an existing directory: {}",
            parent.display()
        );
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent).with_context(|| {
        format!(
            "AS_DEFAULT_OUTPUT: creating temporary file in {}",
            parent.display()
        )
    })?;
    temporary
        .write_all(bytes)
        .context("AS_DEFAULT_OUTPUT: writing verified cache")?;
    temporary
        .as_file_mut()
        .sync_all()
        .context("AS_DEFAULT_OUTPUT: syncing verified cache")?;
    publish_default_temp_noclobber(temporary, path, parent)?;

    // The receipt must prove what was actually published, not merely what was held in memory
    // before the rename. Reopen by the final name and independently check all three invariants.
    let persisted = read_regular_bounded(path, bytes.len() as u64, "AS_DEFAULT_OUTPUT")
        .with_context(|| {
            format!(
                "AS_DEFAULT_OUTPUT: reopening published cache {} for verification",
                path.display()
            )
        })?;
    let expected_sha256 = Sha256::digest(bytes);
    let persisted_sha256 = Sha256::digest(&persisted);
    let length_matches = persisted.len() == bytes.len();
    let hash_matches = persisted_sha256 == expected_sha256;
    let bytes_match = persisted == bytes;
    if !length_matches || !hash_matches || !bytes_match {
        bail!(
            "AS_DEFAULT_OUTPUT: persisted verification failed for {}: expected length {}, got {}; expected sha256 {}, got {}; byte_equal={}",
            path.display(),
            bytes.len(),
            persisted.len(),
            gore_as::cache::default_patch::encode_hex(&expected_sha256),
            gore_as::cache::default_patch::encode_hex(&persisted_sha256),
            bytes_match
        );
    }
    Ok(persisted)
}

fn tag_map_post_publish_error(path: &Path, error: anyhow::Error) -> anyhow::Error {
    error.context(format!(
        "AS_TAG_MAP_OUTPUT: publication of {} succeeded, but subsequent semantic verification failed; treat it as an unverified recovery artifact and inspect or remove it manually; it is never auto-deleted",
        path.display()
    ))
}

fn tag_map_publish_error(path: &Path, error: anyhow::Error) -> anyhow::Error {
    error.context(format!(
        "AS_TAG_MAP_OUTPUT: publication or durable byte verification failed; {} may already exist as an unverified recovery artifact; inspect or remove it manually if present; it is never auto-deleted",
        path.display()
    ))
}

#[cfg(not(windows))]
fn publish_default_temp_noclobber(
    temporary: tempfile::NamedTempFile,
    path: &Path,
    parent: &Path,
) -> Result<()> {
    temporary.persist_noclobber(path).map_err(|error| {
        anyhow::anyhow!(
            "AS_DEFAULT_OUTPUT: output already exists or cannot be published without clobbering {}: {}",
            path.display(),
            error.error
        )
    })?;
    sync_default_output_parent(parent)
}

#[cfg(windows)]
fn publish_default_temp_noclobber(
    temporary: tempfile::NamedTempFile,
    path: &Path,
    _parent: &Path,
) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, SetFileAttributesW, FILE_ATTRIBUTE_NORMAL, MOVEFILE_WRITE_THROUGH,
    };

    // `persist_noclobber` is race-safe but does not request a durable directory-entry update on
    // Windows. Consume the synced temp into a cleanup-owning path, normalize its temporary
    // attribute, and publish with WRITE_THROUGH. Deliberately omit REPLACE_EXISTING so a racing
    // creator wins and can never be overwritten.
    let temporary = temporary.into_temp_path();
    let source: Vec<u16> = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both buffers remain stable and NUL-terminated for the duration of each call.
    unsafe {
        if SetFileAttributesW(source.as_ptr(), FILE_ATTRIBUTE_NORMAL) == 0 {
            return Err(std::io::Error::last_os_error()).with_context(|| {
                format!(
                    "AS_DEFAULT_OUTPUT: normalizing temporary cache before publishing {}",
                    path.display()
                )
            });
        }
        if MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        ) == 0
        {
            let error = std::io::Error::last_os_error();
            bail!(
                "AS_DEFAULT_OUTPUT: output already exists or cannot be published durably without clobbering {}: {}",
                path.display(),
                error
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
fn sync_default_output_parent(parent: &Path) -> Result<()> {
    std::fs::File::open(parent)
        .with_context(|| {
            format!(
                "AS_DEFAULT_OUTPUT: opening parent directory {} for sync",
                parent.display()
            )
        })?
        .sync_all()
        .with_context(|| {
            format!(
                "AS_DEFAULT_OUTPUT: syncing parent directory {}",
                parent.display()
            )
        })
}

#[cfg(not(any(unix, windows)))]
fn sync_default_output_parent(_parent: &Path) -> Result<()> {
    Ok(())
}

/// Acquire cross-tool ownership before selecting the deployment-aware pristine cache. The same
/// guard is then transferred into gore-as, so deploy/undeploy cannot change the authoritative base
/// between this read and compiler use.
fn guarded_pristine_script_cache(
    game: &Path,
) -> Result<(Vec<u8>, gore_as::compile::InstallMutationGuard)> {
    let mut guard = gore_as::compile::acquire_compile_install_mutation(game)
        .map_err(anyhow::Error::msg)
        .context("acquiring the AngelScript install-mutation guard")?;
    match gore_mod::pristine_script_cache(game) {
        Ok(base) => Ok((base, guard)),
        Err(error) => {
            let primary = format!("reading the drift-aware pristine script cache: {error}");
            match guard.release() {
                Ok(()) => Err(anyhow::Error::msg(primary)),
                Err(release) => {
                    guard.preserve_for_manual_recovery();
                    bail!(
                        "COMPILE_RECOVERY_REQUIRED: {primary}; additionally failed to release the \
                         pre-held install-mutation guard: {release}"
                    )
                }
            }
        }
    }
}

pub fn run(cmd: AsCmd) -> Result<()> {
    match cmd {
        AsCmd::DecodeHeader { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let h = CacheHeader::parse(&bytes).context("parsing header")?;
            println!("hash       : {}", hex16(&h.hash));
            println!("magic      : {:#010x}", h.magic);
            println!("type_count : {}", h.type_count);
        }
        AsCmd::Walk { file, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            for s in scan_strings(&bytes, CacheHeader::SIZE, max) {
                println!("0x{:08x}  len={:<4} {}", s.offset, s.len, s.text);
            }
        }
        AsCmd::Info { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let tail = module_region_end(&bytes).context("walking modules")?;
            println!("modules  : {}", module_count(&bytes));
            println!("tail_off : {:#x}", tail);
            println!("eof      : {:#x}", bytes.len());
            println!(
                "tail_len : {} bytes (global ref tables)",
                bytes.len() - tail
            );
        }
        AsCmd::Decompile { file, needle, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            // Mirror `emit`/`emit-all`: load the class hierarchy and native arity table so
            // decompile output matches emitted source (subclass casts, native-call trimming).
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            gore_as::cache::emit_all::prepare_resolver_semantics(
                &mods,
                &mut refs,
                load_native_api(&file),
            );
            let funcs =
                gore_as::cache::walk_modules::collect_function_bytecodes(&bytes).context("walk")?;
            let mut n = 0;
            for f in funcs.iter().filter(|f| f.func.contains(&needle)) {
                if n >= max {
                    break;
                }
                println!("{}", gore_as::cache::structure::decompile(f, &refs));
                n += 1;
            }
            eprintln!("({n} function(s))");
        }
        AsCmd::EmitAll { file, outdir } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            let stats = gore_as::cache::emit_all::emit_all_tree(
                &mods,
                &mut refs,
                load_native_api(&file),
                &outdir,
            )
            .with_context(|| format!("emitting to {}", outdir.display()))?;
            eprintln!(
                "emitted {} modules / {} body-bearing functions to {} ({} cache function records; {} modules / {} functions contain a stubbed body)",
                stats.written,
                stats.functions,
                outdir.display(),
                stats.cache_function_records,
                stats.stubbed,
                stats.stubbed_functions
            );
        }
        AsCmd::Emit { file, needle, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            let prepared = gore_as::cache::emit_all::PreparedEmit::new(
                &mods,
                &mut refs,
                load_native_api(&file),
            )
            .context("prepare emitted modules")?;
            let mut n = 0;
            for (module_index, _) in mods
                .iter()
                .enumerate()
                .filter(|(_, module)| module.name.contains(&needle))
            {
                if n >= max {
                    break;
                }
                println!("{}", prepared.emit_module(module_index)?);
                n += 1;
            }
            eprintln!("({n} module(s))");
        }
        AsCmd::StaticNames { file, indices } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            println!("StaticNames count: {}", refs.static_name_count());
            let show: Vec<i64> = if indices.is_empty() {
                (0..10).collect()
            } else {
                indices
            };
            for i in show {
                match refs.static_name(i) {
                    Some(n) => println!("  [{i}] {n:?}"),
                    None => println!("  [{i}] <out of range>"),
                }
            }
        }
        AsCmd::Disasm { file, needle, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let funcs =
                gore_as::cache::walk_modules::collect_function_bytecodes(&bytes).context("walk")?;
            let mut n = 0;
            for f in funcs.iter().filter(|f| f.func.contains(&needle)) {
                if n >= max {
                    break;
                }
                match gore_as::cache::disasm::disassemble(&f.bytecode) {
                    Ok(ins) => println!("// {}\n{}", f.func, gore_as::cache::disasm::listing(&ins)),
                    Err(e) => println!("// {} — {e}", f.func),
                }
                n += 1;
            }
            eprintln!("({n} function(s))");
        }
        AsCmd::DefaultSites {
            cache,
            module,
            class,
            field,
            json,
        } => {
            let bytes = std::fs::read(&cache)
                .with_context(|| format!("AS_DEFAULT_INPUT: reading {}", cache.display()))?;
            let evidence = load_default_mutation_evidence(
                &cache,
                &bytes,
                DefaultEvidencePolicy::ScalarFallback,
            );
            let report = gore_as::cache::default_patch::default_sites_with_native_ancestry(
                &bytes,
                evidence.native,
                evidence.ancestry,
            )
            .context("AS_DEFAULT_INSPECT")?;
            let sites: Vec<_> = report
                .sites
                .iter()
                .filter(|site| {
                    module
                        .as_deref()
                        .is_none_or(|value| site.selector.module == value)
                        && class
                            .as_deref()
                            .is_none_or(|value| site.selector.class == value)
                        && field
                            .as_deref()
                            .is_none_or(|value| site.selector.field == value)
                })
                .collect();
            if json {
                let document = DefaultSitesJson {
                    format: gore_as::cache::default_patch::DEFAULT_SITES_REPORT_FORMAT,
                    cache: CacheProofJson {
                        path: cache.display().to_string(),
                        length: report.cache_len,
                        sha256: report.cache_sha256,
                    },
                    site_count: sites.len(),
                    stats: DefaultStatsJson {
                        init_functions: report.stats.init_functions,
                        branched_init_functions: report.stats.branched_init_functions,
                        direct_windows: report.stats.direct_windows,
                        unresolved_fields: report.stats.unresolved_fields,
                        unresolved_types: report.stats.unresolved_types,
                        unsupported_types: report.stats.unsupported_types,
                        ambiguous_fields: report.stats.ambiguous_fields,
                    },
                    sites: sites.iter().map(|site| default_site_json(site)).collect(),
                };
                println!("{}", serde_json::to_string_pretty(&document)?);
            } else {
                for site in &sites {
                    let selector =
                        serde_json::to_string(&DefaultSelectorJson::from_core(&site.selector))?;
                    println!(
                        "SITE\tmodule={}\tclass={}\tfield={}\ttype={}\tvalue={}\texpected_hex={}\tselector={}",
                        site.selector.module,
                        site.selector.class,
                        site.selector.field,
                        site.value_type,
                        site.display_value,
                        gore_as::cache::default_patch::encode_hex(&site.expected),
                        selector
                    );
                }
                eprintln!(
                    "{} editable site(s); {} direct window(s), {} branched initializer(s), {} unresolved field(s), {} unresolved type(s), {} unsupported type(s), {} ambiguous field(s)",
                    sites.len(),
                    report.stats.direct_windows,
                    report.stats.branched_init_functions,
                    report.stats.unresolved_fields,
                    report.stats.unresolved_types,
                    report.stats.unsupported_types,
                    report.stats.ambiguous_fields
                );
            }
        }
        AsCmd::PatchDefault {
            cache,
            selector,
            expected_hex,
            replacement_hex,
            out,
            json,
        } => {
            match std::fs::symlink_metadata(&out) {
                Ok(_) => bail!(
                    "AS_DEFAULT_OUTPUT: output already exists; refusing to publish without clobbering {}",
                    out.display()
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("AS_DEFAULT_OUTPUT: checking {}", out.display()));
                }
            }
            let selector = read_default_selector(&selector)?.into_core()?;
            let expected = decode_default_hex(&expected_hex, "AS_DEFAULT_EXPECTED")?;
            let replacement = decode_default_hex(&replacement_hex, "AS_DEFAULT_REPLACEMENT")?;
            let input = std::fs::read(&cache)
                .with_context(|| format!("AS_DEFAULT_INPUT: reading {}", cache.display()))?;
            let evidence = load_default_mutation_evidence(
                &cache,
                &input,
                DefaultEvidencePolicy::ScalarFallback,
            );
            let patch = gore_as::cache::default_patch::patch_default_with_native_ancestry(
                &input,
                evidence.native,
                evidence.ancestry,
                &selector,
                &expected,
                &replacement,
            )
            .context("AS_DEFAULT_PATCH")?;
            let persisted_output = publish_default_cache_noclobber(&out, &patch.bytes)?;

            if json {
                let document = DefaultPatchJson {
                    format: gore_as::cache::default_patch::DEFAULT_PATCH_REPORT_FORMAT,
                    status: "patched",
                    selector: DefaultSelectorJson::from_core(&patch.after.selector),
                    input: cache_proof(&cache, &input),
                    output: cache_proof(&out, &persisted_output),
                    expected_hex: gore_as::cache::default_patch::encode_hex(&patch.before.expected),
                    replacement_hex: gore_as::cache::default_patch::encode_hex(
                        &patch.after.expected,
                    ),
                    provenance: default_provenance_json(&patch.after),
                };
                println!("{}", serde_json::to_string_pretty(&document)?);
            } else {
                println!(
                    "PATCHED\tmodule={}\tclass={}\tfield={}\texpected_hex={}\treplacement_hex={}\toffset={}\tlength={}\tout={}",
                    patch.after.selector.module,
                    patch.after.selector.class,
                    patch.after.selector.field,
                    gore_as::cache::default_patch::encode_hex(&patch.before.expected),
                    gore_as::cache::default_patch::encode_hex(&patch.after.expected),
                    patch.after.operand_offset,
                    patch.after.encoding.width(),
                    out.display()
                );
            }
        }
        AsCmd::TagMap(TagMapCmd::TagMapSites {
            cache,
            module,
            class,
            field,
            tag,
            json,
        }) => {
            let bytes = read_tag_map_cache(&cache)?;
            let evidence = load_required_tag_map_evidence(&cache, &bytes)?;
            let report =
                gore_as::cache::native_tag_map::inspect_native_tag_maps(&bytes, &evidence.ancestry)
                    .context("AS_TAG_MAP_INSPECT")?;
            let sites: Vec<_> = report
                .sites()
                .iter()
                .filter(|site| {
                    module
                        .as_deref()
                        .is_none_or(|value| site.target_module() == value)
                        && class
                            .as_deref()
                            .is_none_or(|value| site.target_class() == value)
                        && field.as_deref().is_none_or(|value| site.field() == value)
                        && tag.as_deref().is_none_or(|value| site.tag_name() == value)
                })
                .collect();
            if json {
                let document = TagMapSitesJson {
                    format: TAG_MAP_SITES_REPORT_FORMAT,
                    cache: cache_proof(&cache, &bytes),
                    cache_guid: hex16(&report.cache_guid()),
                    fingerprint: tag_map_fingerprint_json(&report),
                    binds: evidence.binds,
                    usmap: evidence.usmap,
                    site_count: sites.len(),
                    stats: tag_map_stats_json(report.stats()),
                    sites: sites.iter().map(|site| tag_map_site_json(site)).collect(),
                };
                println!("{}", serde_json::to_string_pretty(&document)?);
            } else {
                for site in &sites {
                    let selector =
                        serde_json::to_string(&TagMapSelectorJson::from_core(site.selector()))?;
                    println!(
                        "TAG_MAP_SITE\tmodule={}\tclass={}\tfield_owner={}\tfield={}\ttag={}\tvalue={}\texpected_hex={}\tcontext_sha256={}\tselector={}",
                        site.target_module(),
                        site.target_class(),
                        site.owner(),
                        site.field(),
                        site.tag_name(),
                        f32::from_le_bytes(site.expected()),
                        gore_as::cache::default_patch::encode_hex(&site.expected()),
                        site.context_sha256(),
                        selector
                    );
                }
                eprintln!(
                    "{} proven tag-map site(s); {} raw window(s), {} reference-proven, {} native-field-proven, {} ambiguous selector group(s)",
                    sites.len(),
                    report.stats().raw_windows(),
                    report.stats().reference_proven_windows(),
                    report.stats().native_field_proven_windows(),
                    report.stats().ambiguous_selectors()
                );
            }
        }
        AsCmd::TagMap(TagMapCmd::PatchTagMap {
            cache,
            selector,
            expected_hex,
            replacement_hex,
            out,
            json,
        }) => {
            match std::fs::symlink_metadata(&out) {
                Ok(_) => bail!(
                    "AS_TAG_MAP_OUTPUT: output already exists; refusing to publish without clobbering {}",
                    out.display()
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("AS_TAG_MAP_OUTPUT: checking {}", out.display()));
                }
            }
            let selector = read_tag_map_selector(&selector)?.into_core()?;
            let expected = decode_tag_map_hex(&expected_hex, "AS_TAG_MAP_EXPECTED")?;
            let replacement = decode_tag_map_hex(&replacement_hex, "AS_TAG_MAP_REPLACEMENT")?;
            let input = read_tag_map_cache(&cache)?;
            let evidence = load_required_tag_map_evidence(&cache, &input)?;
            let patch = gore_as::cache::native_tag_map::patch_native_tag_map(
                &input,
                &evidence.ancestry,
                &selector,
                &expected,
                &replacement,
            )
            .context("AS_TAG_MAP_PATCH")?;
            let persisted_output = match publish_default_cache_noclobber(&out, patch.bytes()) {
                Ok(persisted) => persisted,
                Err(error) => return Err(tag_map_publish_error(&out, error)),
            };

            let finalized = (|| -> Result<String> {
                let persisted_report = gore_as::cache::native_tag_map::inspect_native_tag_maps(
                    &persisted_output,
                    &evidence.ancestry,
                )
                .context("AS_TAG_MAP_OUTPUT: reinspecting persisted cache")?;
                let matches: Vec<_> = persisted_report
                    .sites()
                    .iter()
                    .filter(|site| site.selector() == &selector)
                    .collect();
                let verified = match matches.as_slice() {
                    [site] => *site,
                    other => bail!(
                        "AS_TAG_MAP_OUTPUT: persisted selector was rediscovered {} times",
                        other.len()
                    ),
                };
                if verified.expected() != replacement
                    || verified.operand_range() != patch.after().operand_range()
                {
                    bail!(
                        "AS_TAG_MAP_OUTPUT: persisted selector does not prove the replacement at the original range"
                    );
                }
                if json {
                    let document = TagMapPatchJson {
                        format: TAG_MAP_PATCH_REPORT_FORMAT,
                        status: "patched",
                        selector: TagMapSelectorJson::from_core(verified.selector()),
                        input: cache_proof(&cache, &input),
                        output: cache_proof(&out, &persisted_output),
                        cache_guid: hex16(&persisted_report.cache_guid()),
                        fingerprint: tag_map_fingerprint_json(&persisted_report),
                        binds: evidence.binds.clone(),
                        usmap: evidence.usmap.clone(),
                        expected_hex: gore_as::cache::default_patch::encode_hex(
                            &patch.before().expected(),
                        ),
                        replacement_hex: gore_as::cache::default_patch::encode_hex(
                            &verified.expected(),
                        ),
                        provenance: tag_map_provenance_json(verified),
                    };
                    serde_json::to_string_pretty(&document)
                        .context("AS_TAG_MAP_OUTPUT: serializing verified patch receipt")
                } else {
                    let range = verified.operand_range();
                    Ok(format!(
                        "PATCHED_TAG_MAP\tmodule={}\tclass={}\tfield={}\ttag={}\texpected_hex={}\treplacement_hex={}\tcontext_sha256={}\toffset={}\tlength={}\tout={}",
                        verified.target_module(),
                        verified.target_class(),
                        verified.field(),
                        verified.tag_name(),
                        gore_as::cache::default_patch::encode_hex(&patch.before().expected()),
                        gore_as::cache::default_patch::encode_hex(&verified.expected()),
                        verified.context_sha256(),
                        range.start,
                        range.len(),
                        out.display()
                    ))
                }
            })();
            match finalized {
                Ok(rendered) => println!("{rendered}"),
                Err(error) => return Err(tag_map_post_publish_error(&out, error)),
            }
        }
        AsCmd::DiagnosticsCheck { exe, game } => {
            let exe = match exe {
                Some(exe) => exe,
                None => {
                    let root = gore_loc::config::game_root(game).context("resolving game path")?;
                    let g1r = if root.file_name().is_some_and(|name| name == "G1R") {
                        root
                    } else {
                        root.join("G1R")
                    };
                    g1r.join("Binaries")
                        .join("Win64")
                        .join("G1R-Win64-Shipping.exe")
                }
            };
            let probe = gore_as::diagnostics::probe_executable(&exe)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("scanning {}", exe.display()))?;
            println!("exe: {}", exe.display());
            println!("sha256: {}", probe.sha256);
            println!("signature matches: {}", probe.match_count);
            for rva in &probe.matched_rvas {
                println!("matched RVA: 0x{rva:x}");
            }
            println!(
                "callback structure: {}",
                match (probe.match_count, probe.callback_shape_verified) {
                    (1, true) => "verified",
                    (1, false) => "mismatch",
                    _ => "not checked (signature not unique)",
                }
            );
            if probe.match_count != 1 {
                anyhow::bail!(
                    "diagnostics hook unavailable: signature matched {} times in {} (need exactly 1; normal `gore as compile` will fall back)",
                    probe.match_count,
                    exe.display()
                );
            }
            if !probe.callback_shape_verified {
                anyhow::bail!(
                    "diagnostics hook unavailable: the unique signature in {} did not match the verified asSMessageInfo callback structure (normal `gore as compile` will fall back)",
                    exe.display()
                );
            }
            println!("diagnostics hook compatible");
        }
        AsCmd::Compile {
            src,
            out,
            game,
            no_backup,
            no_diagnostics,
            diagnostics_hook,
            diagnostics_inject_delay_ms,
        } => {
            let game = gore_loc::config::game_root(game).context("resolving game path")?;
            let opts = gore_as::compile::PrecompileOpts {
                game_dir: game,
                src,
                out,
                backup: !no_backup,
            };
            let diagnostics = gore_as::diagnostics::DiagnosticsOptions {
                disabled: no_diagnostics,
                hook_dll: diagnostics_hook,
                inject_delay: std::time::Duration::from_millis(diagnostics_inject_delay_ms),
            };
            let cache = gore_as::compile::precompile_with_diagnostics(&opts, &diagnostics)
                .map_err(anyhow::Error::msg)?;
            match std::fs::metadata(&cache) {
                Ok(m) => println!("compiled -> {} ({} bytes)", cache.display(), m.len()),
                Err(_) => println!("game ran, but no cache found at {}", cache.display()),
            }
        }
        AsCmd::CompileModule {
            op,
            module,
            rel_path,
            source,
            work_dir,
            allow_new_symbols,
            out,
            game,
            no_diagnostics,
            diagnostics_hook,
            diagnostics_inject_delay_ms,
        } => {
            let game = gore_loc::config::game_root(game).context("resolving game path")?;
            let (base_override, guard) = guarded_pristine_script_cache(&game)?;
            let opts = gore_as::compile::CompileOpts {
                game_dir: game,
                op,
                module_name: module,
                rel_path,
                as_path: source,
                work_dir,
                allow_new_symbols,
                base_override: Some(base_override),
            };
            let diagnostics = gore_as::diagnostics::DiagnosticsOptions {
                disabled: no_diagnostics,
                hook_dll: diagnostics_hook,
                inject_delay: std::time::Duration::from_millis(diagnostics_inject_delay_ms),
            };
            let report = gore_as::compile::compile_module_with_diagnostics_report_with_guard(
                &opts,
                &diagnostics,
                guard,
            );
            let restore = report.install_restore_disposition();
            let compiled = match report.outcome {
                gore_as::compile::CompileModuleReportOutcome::Compiled(output)
                    if restore == gore_as::compile::InstallRestoreDisposition::RestoredExact =>
                {
                    output
                }
                gore_as::compile::CompileModuleReportOutcome::Compiled(_) => bail!(
                    "COMPILE_RECOVERY_REQUIRED: compiler output was produced without proving an \
                     exact game-install restore"
                ),
                gore_as::compile::CompileModuleReportOutcome::Failed(error)
                    if matches!(
                        restore,
                        gore_as::compile::InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
                            | gore_as::compile::InstallRestoreDisposition::RecoveryRequiredRestoreFailed
                    ) =>
                {
                    bail!("COMPILE_RECOVERY_REQUIRED: {error}")
                }
                gore_as::compile::CompileModuleReportOutcome::Failed(error) => {
                    return Err(anyhow::Error::new(error)).context("compiling module");
                }
            };
            let mini = std::fs::read(&compiled.mini_path).with_context(|| {
                format!(
                    "reading compiled mini-cache {}",
                    compiled.mini_path.display()
                )
            })?;
            if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::write(&out, &mini).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "compiled module {:?} -> {} ({} bytes)",
                compiled.module_name,
                out.display(),
                mini.len()
            );
        }
        AsCmd::Replace {
            base,
            mini,
            target,
            out,
        } => {
            let base_b =
                std::fs::read(&base).with_context(|| format!("reading {}", base.display()))?;
            let mini_b =
                std::fs::read(&mini).with_context(|| format!("reading {}", mini.display()))?;
            let n = module_count(&base_b);
            let res = gore_as::cache::splice::replace_module(&base_b, &mini_b, &target)
                .context("replace")?;
            std::fs::write(&out, &res).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "replaced {:?}: {} modules (unchanged) ; {} -> {} bytes ; wrote {}",
                target,
                n,
                base_b.len(),
                res.len(),
                out.display()
            );
        }
        AsCmd::Splice { base, mini, out } => {
            let base_b =
                std::fs::read(&base).with_context(|| format!("reading {}", base.display()))?;
            let mini_b =
                std::fs::read(&mini).with_context(|| format!("reading {}", mini.display()))?;
            let before = module_count(&base_b);
            let spliced = splice_auto(&base_b, &mini_b).context("splicing")?;
            std::fs::write(&out, &spliced).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "spliced: {} modules -> {} ; {} -> {} bytes ; wrote {}",
                before,
                module_count(&spliced),
                base_b.len(),
                spliced.len(),
                out.display()
            );
        }
        AsCmd::Extract { cache, module, out } => {
            let b =
                std::fs::read(&cache).with_context(|| format!("reading {}", cache.display()))?;
            let n = module_count(&b);
            let mini = gore_as::cache::splice::extract_module(&b, &module).context("extract")?;
            std::fs::write(&out, &mini).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "extracted {:?} from {} modules -> 1-module mini ; {} bytes ; wrote {}",
                module,
                n,
                mini.len(),
                out.display()
            );
        }
        AsCmd::ExtractRemap {
            regen_cache,
            module,
            base_cache,
            allow_new_symbols,
            out,
        } => {
            let regen_b = std::fs::read(&regen_cache)
                .with_context(|| format!("reading {}", regen_cache.display()))?;
            let base_b = std::fs::read(&base_cache)
                .with_context(|| format!("reading {}", base_cache.display()))?;
            let n = module_count(&regen_b);
            let mini =
                gore_as::cache::splice::extract_module(&regen_b, &module).context("extract")?;
            let (remapped, counts) = gore_as::cache::remap::remap_module_to_base_with_options(
                &mini,
                &base_b,
                gore_as::cache::remap::RemapOptions { allow_new_symbols },
            )
            .context("remap")?;
            std::fs::write(&out, &remapped)
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "extract-remap {:?} from {} modules -> remapped 1-module mini ; {} bytes ; wrote {}",
                module,
                n,
                remapped.len(),
                out.display()
            );
            println!(
                "refs remapped: {} total (bytecode: global={} func_ptr={} type_ptr={} func_id={} type_id={} ; embedded: type_ptr={} func_id={})",
                counts.total(),
                counts.global_ptr,
                counts.func_ptr,
                counts.type_ptr,
                counts.func_id,
                counts.type_id,
                counts.embed_type_ptr,
                counts.embed_func_id
            );
        }
        AsCmd::Bytediff {
            vanilla,
            regen,
            module,
            func,
            verdicts,
            show_benign,
            context,
            norm_slots,
            no_norm_scope,
            no_norm_reguard,
            json,
            fail_on_semantic,
        } => {
            use gore_as::cache::bytediff::{self, Filters, NormOpts, Verdict};
            let v_bytes = std::fs::read(&vanilla)
                .with_context(|| format!("reading vanilla {}", vanilla.display()))?;
            let r_bytes = std::fs::read(&regen)
                .with_context(|| format!("reading regen {}", regen.display()))?;

            let opts = NormOpts {
                n2_slots: norm_slots,
                n5_scope: !no_norm_scope,
                n6_reguard: !no_norm_reguard,
                ..Default::default()
            };
            let filters = Filters {
                module: module.clone(),
                func: func.clone(),
            };

            let report =
                bytediff::run(&v_bytes, &r_bytes, &opts, &filters, context).context("bytediff")?;

            // Verdict filter for per-function output (empty = show all).
            let want = |v: Verdict| -> bool {
                if verdicts.is_empty() {
                    return true;
                }
                verdicts.iter().any(|s| match s.as_str() {
                    "identical" => v == Verdict::Identical,
                    "benign" => v == Verdict::Benign,
                    "semantic" => v == Verdict::Semantic,
                    _ => false,
                })
            };

            // Per-function lines. SEMANTIC always prints its window; BENIGN prints its fired
            // normalizers only under --show-benign; IDENTICAL prints a one-liner when explicitly
            // requested via --verdict identical (otherwise summarized to keep 162k-fn runs sane).
            let show_identical_lines = verdicts.iter().any(|s| s == "identical");
            for d in &report.diffs {
                if !want(d.verdict) {
                    continue;
                }
                match d.verdict {
                    Verdict::Identical => {
                        if show_identical_lines {
                            println!(
                                "{}  IDENTICAL  (v={} ops, r={} ops)",
                                d.name, d.v_ops, d.r_ops
                            );
                        }
                    }
                    Verdict::Benign => {
                        let labels = d.fired.labels();
                        if show_benign {
                            println!(
                                "{}  BENIGN-DIFF  [{}]  (v={} ops, r={} ops)",
                                d.name,
                                labels.join(" "),
                                d.v_ops,
                                d.r_ops
                            );
                        }
                    }
                    Verdict::Semantic => {
                        println!(
                            "{}  SEMANTIC-DIFF  (v={} ops, r={} ops)",
                            d.name, d.v_ops, d.r_ops
                        );
                        if let Some(h) = &d.hint {
                            println!("    hint: {h}");
                        }
                        if let Some(w) = &d.window {
                            print!("{w}");
                        }
                    }
                }
            }

            // Alignment loss (always reported — a dropped/added symbol is a severe defect).
            for m in &report.only_in_vanilla_modules {
                println!("ONLY-IN-VANILLA module: {m}");
            }
            for m in &report.only_in_regen_modules {
                println!("ONLY-IN-REGEN module: {m}");
            }
            if func.is_none() {
                for f in &report.only_in_vanilla_funcs {
                    println!("ONLY-IN-VANILLA func: {f}");
                }
                for f in &report.only_in_regen_funcs {
                    println!("ONLY-IN-REGEN func: {f}");
                }
            }

            // Summary scoreboard.
            let n_ident = report.count(Verdict::Identical);
            let n_benign = report.count(Verdict::Benign);
            let n_sem = report.count(Verdict::Semantic);
            let aligned = report.diffs.len();
            let b1 = if aligned > 0 {
                100.0 * (n_ident + n_benign) as f64 / aligned as f64
            } else {
                100.0
            };
            eprintln!("---- bytediff scoreboard ----");
            eprintln!("aligned functions : {aligned}");
            eprintln!("  IDENTICAL       : {n_ident}");
            eprintln!("  BENIGN-DIFF     : {n_benign}");
            eprintln!("  SEMANTIC-DIFF   : {n_sem}");
            eprintln!(
                "alignment loss    : {} module(s) only-in-vanilla, {} only-in-regen, {} func(s) only-in-vanilla, {} only-in-regen",
                report.only_in_vanilla_modules.len(),
                report.only_in_regen_modules.len(),
                report.only_in_vanilla_funcs.len(),
                report.only_in_regen_funcs.len()
            );
            eprintln!("B1 byte-faithful  : {b1:.2}%  (IDENTICAL+BENIGN / aligned)");
            // Per-normalizer fire counts across BENIGN functions.
            let (mut c1, mut c2, mut c3, mut c4) = (0usize, 0usize, 0usize, 0usize);
            let (mut c5, mut c6) = (0usize, 0usize);
            for d in &report.diffs {
                if d.verdict == Verdict::Benign {
                    c1 += d.fired.n1_refs as usize;
                    c2 += d.fired.n2_slots as usize;
                    c3 += d.fired.n3_jumps as usize;
                    c4 += d.fired.n4_consts as usize;
                    c5 += d.fired.n5_scope as usize;
                    c6 += d.fired.n6_reguard as usize;
                }
            }
            eprintln!(
                "normalizer fires  : N1:refs={c1} N2:slots={c2} N3:jumps={c3} N4:consts={c4} N5:scope={c5} N6:reguard={c6}"
            );

            if let Some(jpath) = &json {
                let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
                let mut sem_list = String::from("[");
                let mut first = true;
                for d in &report.diffs {
                    if d.verdict == Verdict::Semantic {
                        if !first {
                            sem_list.push(',');
                        }
                        first = false;
                        let hint = d.hint.as_deref().unwrap_or("");
                        sem_list.push_str(&format!(
                            "{{\"name\":\"{}\",\"v_ops\":{},\"r_ops\":{},\"hint\":\"{}\"}}",
                            esc(&d.name),
                            d.v_ops,
                            d.r_ops,
                            esc(hint)
                        ));
                    }
                }
                sem_list.push(']');
                let json_out = format!(
                    "{{\n  \"aligned\": {aligned},\n  \"identical\": {n_ident},\n  \"benign\": {n_benign},\n  \"semantic\": {n_sem},\n  \"b1_byte_faithful_pct\": {b1:.4},\n  \"only_in_vanilla_modules\": {},\n  \"only_in_regen_modules\": {},\n  \"only_in_vanilla_funcs\": {},\n  \"only_in_regen_funcs\": {},\n  \"normalizer_fires\": {{\"n1_refs\": {c1}, \"n2_slots\": {c2}, \"n3_jumps\": {c3}, \"n4_consts\": {c4}, \"n5_scope\": {c5}, \"n6_reguard\": {c6}}},\n  \"semantic_list\": {sem_list}\n}}\n",
                    report.only_in_vanilla_modules.len(),
                    report.only_in_regen_modules.len(),
                    report.only_in_vanilla_funcs.len(),
                    report.only_in_regen_funcs.len(),
                );
                std::fs::write(jpath, &json_out)
                    .with_context(|| format!("writing json {}", jpath.display()))?;
                eprintln!("wrote JSON scoreboard to {}", jpath.display());
            }

            if fail_on_semantic && report.any_semantic() {
                anyhow::bail!("{n_sem} SEMANTIC-DIFF function(s) found (--fail-on-semantic)");
            }
        }
    }
    Ok(())
}

fn hex16(b: &[u8; 16]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod default_cli_tests {
    use super::*;

    #[test]
    fn compile_module_cli_keeps_one_guard_across_pristine_selection() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let script = game.join("G1R/Script");
        std::fs::create_dir_all(&script).unwrap();
        std::fs::write(
            script.join("PrecompiledScript_Shipping.Cache"),
            b"authoritative-pristine",
        )
        .unwrap();

        let (base, mut guard) = guarded_pristine_script_cache(&game).unwrap();
        assert_eq!(base, b"authoritative-pristine");
        let contender = gore_as::compile::InstallMutationGuard::acquire(&game, "gore-mod:deploy")
            .expect_err("deploy must remain blocked after the authoritative read");
        assert!(
            contender.contains("install mutation is active"),
            "got: {contender}"
        );
        assert!(guard.path().exists());
        guard.release().unwrap();
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn compile_module_cli_releases_guard_when_pristine_selection_fails() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        std::fs::create_dir_all(&game).unwrap();

        let error = guarded_pristine_script_cache(&game)
            .unwrap_err()
            .to_string();
        assert!(error.contains("pristine script cache"), "got: {error}");
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    const VALID: &str = r#"{
        "format":"gore-as-default-site-v4",
        "kind":"scalar",
        "module":"Items.Food",
        "class":"UApple",
        "field_owner":"UItemDefinition",
        "field":"m_Value",
        "value_type":"int",
        "ancestry_profile":null
    }"#;

    const VALID_TAG_MAP: &str = r#"{
        "format":"gore-as-native-tag-map-selector-v1",
        "kind":"gameplay-tag-float32-map-entry",
        "module":"Items.GenericItems.WeaponsOneHandedGeneric",
        "class":"UItMw_1H_Sword_Old_01",
        "field_owner":"UWeaponDefinition",
        "field":"m_DamageBase",
        "tag_module":"",
        "tag_namespace":"GameplayTag",
        "tag":"Item_Damage_Physical_Edge",
        "tag_is_string":false,
        "value_type":"float32",
        "map_proof_id":"sha256:f20ce5ce571f3d121046ac1942e0705cfb30c3761a3e390cd5d77ea2c16159cc",
        "ancestry_profile":"sha256:98da5430f213b0107bd7361fa3c78316bf5320fbd15a53a9258d50d8d3ac9ed5"
    }"#;

    #[test]
    fn selector_json_is_strict_and_semantic_only() {
        let selector: DefaultSelectorJson = serde_json::from_str(VALID).unwrap();
        let core = selector.into_core().unwrap();
        assert_eq!(core.module, "Items.Food");
        assert_eq!(core.class, "UApple");
        assert_eq!(core.field_owner, "UItemDefinition");
        assert_eq!(core.field, "m_Value");
        assert_eq!(core.value_type, "int");
        assert_eq!(core.ancestry_profile, None);
        assert!(!VALID.contains("offset"));

        let missing_owner = VALID.replace("        \"field_owner\":\"UItemDefinition\",\n", "");
        assert!(serde_json::from_str::<DefaultSelectorJson>(&missing_owner).is_err());

        let unknown = VALID.replace(
            "\"field\":\"m_Value\"",
            "\"field\":\"m_Value\",\"operand_offset\":123",
        );
        assert!(serde_json::from_str::<DefaultSelectorJson>(&unknown).is_err());
        let mut missing_type: serde_json::Value = serde_json::from_str(VALID).unwrap();
        missing_type.as_object_mut().unwrap().remove("value_type");
        assert!(serde_json::from_value::<DefaultSelectorJson>(missing_type).is_err());

        let mut missing_profile: serde_json::Value = serde_json::from_str(VALID).unwrap();
        missing_profile
            .as_object_mut()
            .unwrap()
            .remove("ancestry_profile");
        assert!(serde_json::from_value::<DefaultSelectorJson>(missing_profile).is_err());

        let native = VALID.replace(
            "\"ancestry_profile\":null",
            "\"ancestry_profile\":\"sha256:sealed\"",
        );
        assert_eq!(
            serde_json::from_str::<DefaultSelectorJson>(&native)
                .unwrap()
                .into_core()
                .unwrap()
                .ancestry_profile
                .as_deref(),
            Some("sha256:sealed")
        );

        let wrong_format = VALID.replace("gore-as-default-site-v4", "future-v5");
        assert!(serde_json::from_str::<DefaultSelectorJson>(&wrong_format)
            .unwrap()
            .into_core()
            .is_err());
    }

    #[test]
    fn publish_reopens_verified_bytes_and_keeps_noclobber() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("patched.Cache");
        let expected = b"verified cache bytes";

        let persisted = publish_default_cache_noclobber(&output, expected).unwrap();
        assert_eq!(persisted, expected);
        assert_eq!(std::fs::read(&output).unwrap(), expected);

        let error = publish_default_cache_noclobber(&output, b"replacement").unwrap_err();
        assert!(error.to_string().contains("without clobbering"));
        assert_eq!(std::fs::read(&output).unwrap(), expected);
    }

    #[test]
    fn raw_cas_hex_is_canonical_and_fixed_width() {
        assert_eq!(
            decode_default_hex("04000000", "TEST").unwrap(),
            [4, 0, 0, 0]
        );
        assert_eq!(
            decode_default_hex("0000000000709740", "TEST").unwrap(),
            [0, 0, 0, 0, 0, 0x70, 0x97, 0x40]
        );
        for invalid in ["04", "0400000", "0400000000", "0x04000000", "AB000000"] {
            assert!(decode_default_hex(invalid, "TEST").is_err(), "{invalid}");
        }
    }

    #[test]
    fn tag_map_selector_json_is_complete_strict_and_constant_bound() {
        let selector: TagMapSelectorJson = serde_json::from_str(VALID_TAG_MAP).unwrap();
        let core = selector.clone().into_core().unwrap();
        assert_eq!(core.field, "m_DamageBase");
        assert_eq!(core.tag, "Item_Damage_Physical_Edge");
        assert_eq!(core.tag_module, "");
        assert_eq!(core.tag_namespace, "GameplayTag");
        assert!(!core.tag_is_string);

        let mut missing: serde_json::Value = serde_json::from_str(VALID_TAG_MAP).unwrap();
        missing.as_object_mut().unwrap().remove("map_proof_id");
        assert!(serde_json::from_value::<TagMapSelectorJson>(missing).is_err());
        let unknown = VALID_TAG_MAP.replace(
            "\"field\":\"m_DamageBase\"",
            "\"field\":\"m_DamageBase\",\"context_sha256\":\"forged\"",
        );
        assert!(serde_json::from_str::<TagMapSelectorJson>(&unknown).is_err());

        for (field, invalid) in [
            ("format", "future-v2"),
            ("kind", "other"),
            ("value_type", "int"),
            ("tag_module", "Angelscript"),
            ("tag_namespace", "Other"),
            ("map_proof_id", "sha256:stale"),
            ("ancestry_profile", "sha256:stale"),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(VALID_TAG_MAP).unwrap();
            value[field] = serde_json::Value::String(invalid.into());
            assert!(
                serde_json::from_value::<TagMapSelectorJson>(value)
                    .unwrap()
                    .into_core()
                    .is_err(),
                "{field}"
            );
        }
        let mut string_tag: serde_json::Value = serde_json::from_str(VALID_TAG_MAP).unwrap();
        string_tag["tag_is_string"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<TagMapSelectorJson>(string_tag)
            .unwrap()
            .into_core()
            .is_err());
    }

    #[test]
    fn tag_map_selector_file_and_hex_are_bounded_and_exact() {
        let directory = tempfile::tempdir().unwrap();
        let selector = directory.path().join("selector.json");
        std::fs::write(&selector, VALID_TAG_MAP).unwrap();
        assert!(read_tag_map_selector(&selector).is_ok());
        assert!(read_tag_map_selector(directory.path()).is_err());

        let oversized = directory.path().join("oversized.json");
        let file = std::fs::File::create(&oversized).unwrap();
        file.set_len(DEFAULT_SELECTOR_MAX_BYTES + 1).unwrap();
        assert!(read_tag_map_selector(&oversized).is_err());

        assert_eq!(
            decode_tag_map_hex("00002041", "TEST").unwrap(),
            10.0f32.to_le_bytes()
        );
        for invalid in [
            "0000204",
            "000020410",
            "0000204100000000",
            "0x002041",
            "0000204A",
        ] {
            assert!(decode_tag_map_hex(invalid, "TEST").is_err(), "{invalid}");
        }
    }

    #[test]
    fn usmap_autodiscovery_is_layout_relative_and_content_agnostic() {
        let root = tempfile::tempdir().unwrap();
        let g1r = root.path().join("G1R");
        let script = g1r.join("Script");
        let maps = g1r.join("Binaries/Win64/ue4ss");
        std::fs::create_dir_all(&script).unwrap();
        std::fs::create_dir_all(&maps).unwrap();
        std::fs::write(maps.join("b.USMAP"), b"unknown-b").unwrap();
        std::fs::write(maps.join("a.usmap"), b"unknown-a").unwrap();
        std::fs::write(maps.join("ignored.txt"), b"not a map").unwrap();
        std::fs::create_dir(maps.join("directory.usmap")).unwrap();

        let cache = script.join("PrecompiledScript_Shipping.Cache");
        assert_eq!(
            default_usmap_candidates(&cache, None).unwrap(),
            vec![maps.join("a.usmap"), maps.join("b.USMAP")]
        );
        let explicit = root.path().join("custom.bin");
        assert_eq!(
            default_usmap_candidates(&cache, Some(explicit.clone())).unwrap(),
            vec![explicit]
        );
        assert!(
            default_usmap_candidates(&root.path().join("elsewhere.Cache"), None)
                .unwrap()
                .is_empty()
        );

        for index in 0..=DEFAULT_USMAP_MAX_CANDIDATES {
            std::fs::write(maps.join(format!("overflow-{index}.usmap")), b"map").unwrap();
        }
        assert!(default_usmap_candidates(&cache, None).is_err());

        let flood = tempfile::tempdir().unwrap();
        let flood_script = flood.path().join("G1R/Script");
        let flood_maps = flood.path().join("G1R/Binaries/Win64/ue4ss");
        std::fs::create_dir_all(&flood_script).unwrap();
        std::fs::create_dir_all(&flood_maps).unwrap();
        for index in 0..=DEFAULT_USMAP_MAX_DIRECTORY_ENTRIES {
            std::fs::write(flood_maps.join(format!("entry-{index}.txt")), b"x").unwrap();
        }
        assert!(default_usmap_candidates(&flood_script.join("cache.Cache"), None).is_err());
    }

    #[test]
    fn usmap_reads_are_regular_bounded_and_exact_length() {
        let root = tempfile::tempdir().unwrap();
        let small = root.path().join("small.usmap");
        std::fs::write(&small, b"exact bytes").unwrap();
        assert_eq!(read_default_usmap(&small).unwrap(), b"exact bytes");
        assert!(read_default_usmap(root.path()).is_err());

        let large = root.path().join("large.usmap");
        let file = std::fs::File::create(&large).unwrap();
        file.set_len(DEFAULT_USMAP_MAX_BYTES + 1).unwrap();
        assert!(read_default_usmap(&large).is_err());
    }
}
