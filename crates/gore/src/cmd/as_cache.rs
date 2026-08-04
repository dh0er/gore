use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use gore_as::cache::default_evidence::{
    audited_builds, classify_candidate_failure, CandidateVerdict, EvidenceCounts,
    NativeEvidenceStatus, ObservedBuild, UsmapCandidate, UsmapProof,
};
use gore_as::cache::header::CacheHeader;
use gore_as::cache::scan::scan_strings;
use gore_as::cache::splice::splice_auto;
use gore_as::cache::walk_modules::{module_count, module_region_end};

#[derive(Subcommand)]
pub enum AsCmd {
    /// Parse and print the outer cache header.
    DecodeHeader { file: PathBuf },
    /// Scan length-prefixed type-name strings (decode investigation aid). The input must be a
    /// module cache: the scan starts after the outer header, so the `0x9e377abe` magic is
    /// checked first and an arbitrary blob is refused rather than scanned.
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
    /// Derive an installed build's generation row and qualification artifact. Reads the game and
    /// writes nothing: it proposes a row for a person to add, and says what it could not measure.
    Qualify {
        /// Game install root (the folder containing `G1R/`). Falls back to the configured game
        /// path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
        /// Exact `.usmap` reflection dump to qualify against. Omit to select one from the install,
        /// which refuses rather than choosing when two dumps both fit this executable.
        #[arg(long)]
        usmap: Option<PathBuf>,
        /// A previously published `story_catalog.v1` document, used to name the curated script
        /// modules and their sealed source. Omit when the build is already audited; the catalog is
        /// then built from the install itself.
        #[arg(long, value_name = "CATALOG.json")]
        catalog: Option<PathBuf>,
        /// Proposed `GenerationRow::id` for the draft. (default: `g1r-steam-<script cache GUID
        /// prefix>`, which is a placeholder — a Steam BuildID reads better.)
        #[arg(long, value_name = "ID")]
        id: Option<String>,
        /// Proposed `GenerationRow::label`, the banner a person reads. (default: derived from the
        /// id.)
        #[arg(long, value_name = "TEXT")]
        label: Option<String>,
        /// Emit one machine-readable JSON document.
        #[arg(long)]
        json: bool,
    },
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
const MODULE_CACHE_MAX_BYTES: u64 = 512 * 1024 * 1024;
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
        use gore_as::cache::default_ancestry::is_supported_gameplay_tag_float32_proof_pair;
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
        if !is_supported_gameplay_tag_float32_proof_pair(&self.ancestry_profile, &self.map_proof_id)
        {
            bail!(
                "AS_TAG_MAP_SELECTOR: ancestry_profile/map_proof_id is not one exact sealed generation pair"
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
    evidence: DefaultEvidenceJson,
    sites: Vec<DefaultSiteJson<'a>>,
}

/// The native-evidence verdict as a document, so `--json` never has to be reconciled against what
/// the same run printed on stderr.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum DefaultEvidenceJson {
    Qualified {
        generation_id: String,
        generation_label: String,
        ancestry_profile_id: String,
        usmap: Option<UsmapProofJson>,
    },
    UnsupportedGeneration {
        observed: ObservedBuildJson,
        audited: Vec<AuditedBuildJson>,
    },
    UsmapMissing {
        generation_id: Option<String>,
        examined: Vec<UsmapCandidateJson>,
    },
    UsmapAmbiguous {
        generation_id: String,
        matched: Vec<String>,
    },
    BindsUnavailable {
        reason: String,
    },
    SealDrift {
        generation_id: Option<String>,
        drift: String,
    },
    NotRequested,
}

#[derive(Serialize)]
struct UsmapProofJson {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct ObservedBuildJson {
    script_cache_guid: String,
    script_cache_length: usize,
    script_cache_sha256: String,
    binds_length: Option<usize>,
    binds_sha256: Option<String>,
}

#[derive(Serialize)]
struct AuditedBuildJson {
    id: &'static str,
    label: &'static str,
    ancestry_profile_id: &'static str,
    map_proof_id: &'static str,
}

#[derive(Serialize)]
struct UsmapCandidateJson {
    path: String,
    sha256: Option<String>,
    rejection: &'static str,
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
    evidence: DefaultEvidenceJson,
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
    load_native_api_with_proof(cache_file).map(|loaded| loaded.native)
}

/// A parsed `Binds.Cache` together with the measurements a refusal has to be able to quote.
struct LoadedBinds {
    native: gore_as::cache::binds::NativeApi,
    proof: EvidenceFileProofJson,
    len: usize,
    sha256: [u8; 32],
}

fn native_api_path(cache_file: &Path) -> Option<PathBuf> {
    Some(match std::env::var_os("GORE_AS_BINDS") {
        Some(path) => PathBuf::from(path),
        None => cache_file.parent()?.join("Binds.Cache"),
    })
}

fn load_native_api_with_proof(cache_file: &Path) -> Option<LoadedBinds> {
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
        Some(native) => {
            eprintln!("loaded native arities from {}", path.display());
            Some(LoadedBinds {
                native,
                proof,
                len: bytes.len(),
                sha256: Sha256::digest(&bytes).into(),
            })
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
    /// Why the sealed tuple did or did not qualify. Carried rather than printed here, so that every
    /// command prints it exactly once, in its own place, with its own counts.
    status: NativeEvidenceStatus,
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
    read_validated_cache(path, "AS_TAG_MAP_INPUT")
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

/// Prove a buffer really is an AngelScript module cache before anything walks it. `Binds.Cache` and
/// the other side tables carry no `CACHE_MAGIC` at 0x10, and every structural walker deliberately
/// skips the header and re-reads the module count from 0x14; without this gate they read an
/// arbitrary `FString` length out of unrelated bytes and blame a container parse for a wrong file.
/// Visible to the crate because `gore catalog knowledge --script-cache` feeds the same walkers.
pub(crate) fn validate_module_cache(path: &Path, bytes: &[u8], label: &'static str) -> Result<()> {
    CacheHeader::parse(bytes).with_context(|| {
        format!(
            "{label}: {} is not an AngelScript module cache — pass the game's \
             PrecompiledScript_Shipping.Cache or a mini-cache from 'gore as extract'",
            path.display()
        )
    })?;
    Ok(())
}

/// Read a bounded regular file and prove its outer header, under the caller's own `AS_*` code so
/// that a tag-map input is not diagnosed as a plain cache input.
fn read_validated_cache(path: &Path, label: &'static str) -> Result<Vec<u8>> {
    let bytes = read_regular_bounded(path, MODULE_CACHE_MAX_BYTES, label)?;
    validate_module_cache(path, &bytes, label)?;
    Ok(bytes)
}

/// Read a module cache and prove its outer header. The single entry point for every subcommand that
/// walks the `Modules` TMap or the seven global tail tables.
fn read_module_cache(path: &Path) -> Result<Vec<u8>> {
    read_validated_cache(path, "AS_CACHE_INPUT")
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
///
/// The verdict about the *build* is hoisted out of the USMAP loop. `from_schema_db` settles the
/// script-cache identity before it reads a single USMAP byte, so the first candidate that answers
/// "this is not an audited build" has answered for all of them — and printing that answer under a
/// candidate's path is what used to make an unaudited build look like a bad reflection dump.
fn load_default_mutation_evidence(cache_file: &Path, cache: &[u8]) -> DefaultMutationEvidence {
    let Some(loaded) = load_native_api_with_proof(cache_file) else {
        let reason = match native_api_path(cache_file) {
            Some(path) => format!("no usable Binds.Cache at {}", path.display()),
            None => "no Binds.Cache location could be derived for this script cache".to_owned(),
        };
        return DefaultMutationEvidence {
            native: None,
            ancestry: None,
            binds: None,
            usmap: None,
            status: NativeEvidenceStatus::BindsUnavailable { reason },
        };
    };
    let observed = ObservedBuild {
        // Every caller proves the outer header first; an all-zero GUID here would be a header this
        // command already refused to walk.
        script_cache_guid: CacheHeader::parse(cache).map_or([0u8; 16], |header| header.hash),
        script_cache_len: cache.len(),
        script_cache_sha256: Sha256::digest(cache).into(),
        binds_len: Some(loaded.len),
        binds_sha256: Some(loaded.sha256),
    };
    let binds = loaded.proof;
    let native = Some(loaded.native);
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
    let mut examined = Vec::new();
    let mut verdict = None;
    for path in candidates {
        if verdict.is_some() {
            break;
        }
        let display = path.display().to_string();
        let bytes = match read_default_usmap(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("warning: AS_DEFAULT_USMAP: {error:#}");
                examined.push(UsmapCandidate {
                    path: display,
                    sha256: None,
                    rejection: "could not be read",
                });
                continue;
            }
        };
        let proof = evidence_file_proof(&path, &bytes);
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let schemas = match gore_asset::SchemaDb::from_usmap(&bytes) {
            Ok(schemas) => schemas,
            Err(error) => {
                eprintln!("warning: AS_DEFAULT_USMAP: parsing {display}: {error}");
                examined.push(UsmapCandidate {
                    path: display,
                    sha256: Some(digest),
                    rejection: "is not a parseable schema map",
                });
                continue;
            }
        };
        match gore_as::cache::default_ancestry::DefaultNativeAncestry::from_schema_db(
            native_ref, cache, &schemas,
        ) {
            Ok(profile) => matches.push((display, profile, proof, digest)),
            Err(error) => match classify_candidate_failure(&error) {
                CandidateVerdict::UnsupportedCache => {
                    verdict = Some(NativeEvidenceStatus::UnsupportedGeneration {
                        observed: observed.clone(),
                        audited: audited_builds(),
                    });
                }
                CandidateVerdict::UnsupportedBinds => {
                    verdict = Some(NativeEvidenceStatus::BindsUnavailable {
                        reason: error.to_string(),
                    });
                }
                CandidateVerdict::SealDrift(drift) => {
                    verdict = Some(NativeEvidenceStatus::SealDrift {
                        generation_id: None,
                        drift,
                    });
                }
                CandidateVerdict::Rejected(rejection) => examined.push(UsmapCandidate {
                    path: display,
                    sha256: Some(digest),
                    rejection,
                }),
            },
        }
    }

    let (ancestry, usmap, status) = match (verdict, matches.len()) {
        (Some(status), _) => (None, None, status),
        (None, 1) => {
            let (path, profile, proof, sha256) = matches.pop().expect("one match");
            let status = NativeEvidenceStatus::qualified(
                profile.profile_id(),
                Some(UsmapProof { path, sha256 }),
            );
            (Some(profile), Some(proof), status)
        }
        (None, 0) => (
            None,
            None,
            NativeEvidenceStatus::UsmapMissing {
                generation_id: None,
                examined,
            },
        ),
        (None, _) => {
            let generation_id = NativeEvidenceStatus::generation_id_for_profile_id(
                matches[0].1.profile_id(),
            );
            let matched = matches.into_iter().map(|(path, ..)| path).collect();
            (
                None,
                None,
                NativeEvidenceStatus::UsmapAmbiguous {
                    generation_id,
                    matched,
                },
            )
        }
    };
    DefaultMutationEvidence {
        native,
        ancestry,
        binds: Some(binds),
        usmap,
        status,
    }
}

/// Print the native-evidence verdict once, above whatever the command is about to emit. Sites go to
/// stdout and this goes to stderr, so `--json` keeps a clean document and still states the cause.
fn print_native_evidence(status: &NativeEvidenceStatus, counts: Option<EvidenceCounts>) {
    let banner = status.banner(counts);
    if !banner.is_empty() {
        eprintln!("{banner}");
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
    let evidence = load_default_mutation_evidence(cache_file, cache);
    // Say what was refused before saying that the command needs it. Without this, a tag-map run on
    // an unaudited build reports only that evidence is required, never that the build is the reason.
    print_native_evidence(&evidence.status, None);
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

fn hex(bytes: &[u8]) -> String {
    gore_as::cache::default_patch::encode_hex(bytes)
}

fn default_evidence_json(status: &NativeEvidenceStatus) -> DefaultEvidenceJson {
    match status {
        NativeEvidenceStatus::Qualified {
            generation_id,
            generation_label,
            ancestry_profile_id,
            usmap,
        } => DefaultEvidenceJson::Qualified {
            generation_id: (*generation_id).to_owned(),
            generation_label: (*generation_label).to_owned(),
            ancestry_profile_id: (*ancestry_profile_id).to_owned(),
            usmap: usmap.as_ref().map(|proof| UsmapProofJson {
                path: proof.path.clone(),
                sha256: hex(&proof.sha256),
            }),
        },
        NativeEvidenceStatus::UnsupportedGeneration { observed, audited } => {
            DefaultEvidenceJson::UnsupportedGeneration {
                observed: ObservedBuildJson {
                    script_cache_guid: hex(&observed.script_cache_guid),
                    script_cache_length: observed.script_cache_len,
                    script_cache_sha256: hex(&observed.script_cache_sha256),
                    binds_length: observed.binds_len,
                    binds_sha256: observed.binds_sha256.as_ref().map(|sha| hex(sha)),
                },
                audited: audited
                    .iter()
                    .map(|build| AuditedBuildJson {
                        id: build.id,
                        label: build.label,
                        ancestry_profile_id: build.ancestry_profile_id,
                        map_proof_id: build.map_proof_id,
                    })
                    .collect(),
            }
        }
        NativeEvidenceStatus::UsmapMissing {
            generation_id,
            examined,
        } => DefaultEvidenceJson::UsmapMissing {
            generation_id: generation_id.map(str::to_owned),
            examined: examined
                .iter()
                .map(|candidate| UsmapCandidateJson {
                    path: candidate.path.clone(),
                    sha256: candidate.sha256.as_ref().map(|sha| hex(sha)),
                    rejection: candidate.rejection,
                })
                .collect(),
        },
        NativeEvidenceStatus::UsmapAmbiguous {
            generation_id,
            matched,
        } => DefaultEvidenceJson::UsmapAmbiguous {
            generation_id: (*generation_id).to_owned(),
            matched: matched.clone(),
        },
        NativeEvidenceStatus::BindsUnavailable { reason } => DefaultEvidenceJson::BindsUnavailable {
            reason: reason.clone(),
        },
        NativeEvidenceStatus::SealDrift {
            generation_id,
            drift,
        } => DefaultEvidenceJson::SealDrift {
            generation_id: generation_id.map(str::to_owned),
            drift: (*drift).to_owned(),
        },
        NativeEvidenceStatus::NotRequested => DefaultEvidenceJson::NotRequested,
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
/// Acquire the compile guard.
///
/// Split out for one reason: under `cfg(test)` it answers "is the game running?" from a thread-local
/// instead of from the real process list. These tests build a throwaway install in a temp directory
/// and assert on guard bookkeeping, which has nothing to do with what is running on the developer's
/// machine — but the production probe made the whole suite go red whenever Gothic happened to be
/// open, which is precisely when someone is likely to be testing modding tools. Production
/// behaviour is untouched.
#[cfg(not(test))]
fn acquire_compile_guard(game: &Path) -> Result<gore_as::compile::InstallMutationGuard, String> {
    gore_as::compile::acquire_compile_install_mutation(game)
}

#[cfg(test)]
fn acquire_compile_guard(game: &Path) -> Result<gore_as::compile::InstallMutationGuard, String> {
    gore_as::compile::acquire_compile_install_mutation_with_stated_game_process(game, || Ok(false))
}

fn guarded_pristine_script_cache(
    game: &Path,
) -> Result<(Vec<u8>, gore_as::compile::InstallMutationGuard)> {
    let mut guard = acquire_compile_guard(game)
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
            let bytes = read_module_cache(&file)?;
            let h = CacheHeader::parse(&bytes).context("parsing header")?;
            println!("hash       : {}", hex16(&h.hash));
            println!("magic      : {:#010x}", h.magic);
            println!("type_count : {}", h.type_count);
        }
        AsCmd::Walk { file, max } => {
            let bytes = read_module_cache(&file)?;
            for s in scan_strings(&bytes, CacheHeader::SIZE, max) {
                println!("0x{:08x}  len={:<4} {}", s.offset, s.len, s.text);
            }
        }
        AsCmd::Info { file } => {
            let bytes = read_module_cache(&file)?;
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
            let bytes = read_module_cache(&file)?;
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
            let bytes = read_module_cache(&file)?;
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
            let bytes = read_module_cache(&file)?;
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
            let bytes = read_module_cache(&file)?;
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
            let bytes = read_module_cache(&file)?;
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
            // Ahead of evidence loading: a non-cache input otherwise draws a `not usable
            // native-default evidence` warning before anything names the real mismatch.
            validate_module_cache(&cache, &bytes, "AS_DEFAULT_INPUT")?;
            let evidence = load_default_mutation_evidence(&cache, &bytes);
            let report = gore_as::cache::default_patch::default_sites_with_evidence(
                &bytes,
                evidence.native,
                evidence.ancestry,
                Some(evidence.status),
            )
            .context("AS_DEFAULT_INSPECT")?;
            // The counts are the whole report, so the reason for them belongs above them and above
            // the sites, not folded into a per-file warning further up.
            print_native_evidence(
                &report.evidence,
                Some(EvidenceCounts {
                    editable_sites: report.sites.len(),
                    direct_windows: report.stats.direct_windows,
                    unresolved_fields: report.stats.unresolved_fields,
                    unresolved_types: report.stats.unresolved_types,
                }),
            );
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
                    evidence: default_evidence_json(&report.evidence),
                    sites: sites.iter().map(|site| default_site_json(site)).collect(),
                };
                println!("{}", serde_json::to_string_pretty(&document)?);
            } else {
                if module.is_some() || class.is_some() || field.is_some() {
                    eprintln!(
                        "the counts above are for the whole cache; {} of them match the active \
                         --module/--class/--field filter",
                        sites.len()
                    );
                }
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
            validate_module_cache(&cache, &input, "AS_DEFAULT_INPUT")?;
            let evidence = load_default_mutation_evidence(&cache, &input);
            let patch = gore_as::cache::default_patch::patch_default_with_evidence(
                &input,
                evidence.native,
                evidence.ancestry,
                Some(evidence.status),
                &selector,
                &expected,
                &replacement,
            )
            .context("AS_DEFAULT_PATCH")?;
            print_native_evidence(&patch.evidence, None);
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
                    evidence: default_evidence_json(&patch.evidence),
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
        AsCmd::Qualify {
            game,
            usmap,
            catalog,
            id,
            label,
            json,
        } => run_qualify(game, usmap, catalog, id, label, json)?,
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
                source_override: None,
                work_dir,
                allow_new_symbols,
                base_override: Some(base_override),
                binds_override: None,
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
            let base_b = read_module_cache(&base)?;
            let mini_b = read_module_cache(&mini)?;
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
            let base_b = read_module_cache(&base)?;
            let mini_b = read_module_cache(&mini)?;
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
            let b = read_module_cache(&cache)?;
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
            let regen_b = read_module_cache(&regen_cache)?;
            let base_b = read_module_cache(&base_cache)?;
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
            // The two positionals are the easiest pair in the CLI to swap, so each keeps the role
            // label its own read carried before both moved to the shared helper.
            let v_bytes = read_module_cache(&vanilla).context("reading the vanilla cache")?;
            let r_bytes = read_module_cache(&regen).context("reading the regen cache")?;

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

// ------------------------------------------------------------------------------------------
// `gore as qualify` — deriving a generation row from an installation.
//
// The command exists because the last three generations were qualified by hand: a maintainer ran
// half a dozen suites, read numbers out of their output, and pasted twenty-four values into a
// struct literal across six files. Two things went wrong doing that, and both are designed against
// here. A USMAP dump from the previous build passes its own hash check while describing the wrong
// game, so the dump is chosen by evidence and an ambiguous directory is refused rather than
// resolved. And a digest cannot say that a parser dropped rows, so every digest is emitted next to
// the count it was taken over, compared against the nearest audited row.
//
// It never edits `crates/gore-generation/src/lib.rs`. Qualifying is not admitting.
// ------------------------------------------------------------------------------------------

const QUALIFY_REPORT_FORMAT: &str = "gore-as-qualify-v1";
const QUALIFY_EXECUTABLE_MAX_BYTES: u64 = 512 * 1024 * 1024;
const QUALIFY_CATALOG_MAX_BYTES: u64 = 16 * 1024 * 1024;
/// Shortest run of printable bytes in the executable treated as a name. Class names are far longer;
/// this only bounds how much noise the run table carries.
const QUALIFY_MIN_EXECUTABLE_RUN: usize = 4;
/// How many absent class names a rejected dump lists before the report stops naming them.
const QUALIFY_MAX_ABSENT_EXAMPLES: usize = 8;

#[derive(Serialize, Clone)]
struct QualifyFileJson {
    path: String,
    byte_len: u64,
    sha256: String,
}

#[derive(Serialize)]
struct QualifyInputsJson {
    executable: QualifyFileJson,
    shipping_cache: QualifyFileJson,
    binds_cache: QualifyFileJson,
    script_cache_guid: String,
    usmap: Option<QualifyFileJson>,
}

#[derive(Serialize)]
struct QualifyUsmapCandidateJson {
    path: String,
    sha256: String,
    byte_len: u64,
    class_rows: Option<usize>,
    /// Class names this dump declares that the executable never names. Zero is the only value that
    /// ties a dump to this build; anything else means the dump describes a different one.
    class_names_absent_from_executable: Option<usize>,
    absent_examples: Vec<String>,
    /// Of the native base classes this build's own script cache refers to, how many this dump has.
    native_bases_resolved: Option<usize>,
    native_bases_total: usize,
    verdict: &'static str,
}

#[derive(Serialize)]
struct QualifyUsmapSelectionJson {
    sealed: Option<QualifyFileJson>,
    reason: String,
    examined: Vec<QualifyUsmapCandidateJson>,
}

#[derive(Serialize)]
struct QualifyFieldJson {
    field: &'static str,
    value: Option<String>,
    derived_by: Option<&'static str>,
}

#[derive(Serialize)]
struct QualifyRowJson {
    id: String,
    fields: Vec<QualifyFieldJson>,
    missing: Vec<&'static str>,
    literal: String,
}

/// The committed artifact, in the exact shape `crates/gore-generation/qualifications/*.json` uses.
/// A value this run could not measure is `null` rather than absent, so the gap is visible in the
/// document a maintainer is about to commit.
#[derive(Serialize)]
struct QualifyArtifactJson {
    generation_id: String,
    label: String,
    native_ancestry_profile_id: Option<String>,
    gameplay_tag_float32_map_proof_id: Option<String>,
    scalar_default_operand_count: Option<u64>,
    gameplay_tag_float32_operand_count: Option<u64>,
    class_count: Option<u64>,
    gameplay_tag_float32_map_field_count: Option<u64>,
    unresolved_fields_with_ancestry: Option<u64>,
    direct_windows: Option<u64>,
    /// The two Binds row counts. A digest cannot say that a record shape stopped being recognised;
    /// these are what the next generation's run compares against, and they are recorded here
    /// because the first three generations were qualified before anything wrote them down.
    binds_field_row_count: Option<u64>,
    binds_class_path_row_count: Option<u64>,
    witnesses: std::collections::BTreeMap<&'static str, String>,
    notes: Vec<String>,
}

#[derive(Serialize)]
struct QualifyCountJson {
    name: &'static str,
    previous: Option<u64>,
    observed: u64,
    delta: Option<i64>,
    fell: bool,
    derived_by: String,
}

#[derive(Serialize)]
struct QualifyCuratedModuleJson {
    module: String,
    runtime_class: String,
    expected_byte_len: u64,
    expected_sha256: String,
    observed_byte_len: Option<u64>,
    observed_sha256: Option<String>,
    reproduces: bool,
}

#[derive(Serialize)]
struct QualifyCuratedJson {
    /// Where the curated module list and its seals came from, or why there is none.
    source: String,
    modules: Vec<QualifyCuratedModuleJson>,
    all_reproduce: bool,
}

#[derive(Serialize)]
struct QualifyUnavailableJson {
    field: &'static str,
    reason: String,
}

/// One value this run derived that the sealed row disagrees with. Only an audited build can produce
/// these, and on an audited build the list being empty is the whole proof: it means every number in
/// the row came back out of the game the row describes.
#[derive(Serialize)]
struct QualifyDivergenceJson {
    field: &'static str,
    sealed: String,
    derived: Option<String>,
}

#[derive(Serialize)]
struct QualifyJson {
    format: &'static str,
    game: String,
    already_audited_as: Option<&'static str>,
    compared_against: Option<&'static str>,
    inputs: QualifyInputsJson,
    usmap_selection: QualifyUsmapSelectionJson,
    counts: Vec<QualifyCountJson>,
    curated_records: QualifyCuratedJson,
    row: QualifyRowJson,
    qualification: QualifyArtifactJson,
    unavailable: Vec<QualifyUnavailableJson>,
    diverged_from_sealed_row: Vec<QualifyDivergenceJson>,
    complete: bool,
    still_to_do: Vec<String>,
}

/// The three sealed files, located the way every other command locates them.
struct QualifyPaths {
    root: PathBuf,
    executable: PathBuf,
    shipping_cache: PathBuf,
    binds_cache: PathBuf,
}

fn qualify_paths(game: Option<PathBuf>) -> Result<QualifyPaths> {
    let root = gore_loc::config::game_root(game).context("resolving game path")?;
    let g1r = if root.file_name().is_some_and(|name| name == "G1R") {
        root.clone()
    } else {
        root.join("G1R")
    };
    Ok(QualifyPaths {
        root,
        executable: g1r
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe"),
        shipping_cache: g1r.join("Script").join("PrecompiledScript_Shipping.Cache"),
        binds_cache: g1r.join("Script").join("Binds.Cache"),
    })
}

fn qualify_file_json(path: &Path, seal: &gore_story_catalog::ContentSeal) -> QualifyFileJson {
    QualifyFileJson {
        path: path.display().to_string(),
        byte_len: seal.byte_len,
        sha256: seal.sha256.to_string(),
    }
}

fn qualify_file_seal(seal: &gore_story_catalog::ContentSeal) -> gore_generation::FileSeal {
    gore_generation::FileSeal {
        byte_len: seal.byte_len,
        sha256: *seal.sha256.as_bytes(),
    }
}

/// Every run of printable bytes in the executable, in both the byte-per-character and the
/// UTF-16-little-endian encodings an Unreal binary mixes.
///
/// This is the only file-only tie between a `.usmap` and the executable it claims to describe: the
/// dump is generated on a player's machine and its filename is a claim, not a proof, so what a
/// candidate is checked against is whether the binary names the classes the candidate declares.
fn executable_name_runs(bytes: &[u8]) -> std::collections::HashSet<String> {
    let mut runs = std::collections::HashSet::new();
    let mut narrow = String::new();
    for &byte in bytes {
        if byte.is_ascii_graphic() {
            narrow.push(byte as char);
        } else {
            if narrow.len() >= QUALIFY_MIN_EXECUTABLE_RUN {
                runs.insert(std::mem::take(&mut narrow));
            } else {
                narrow.clear();
            }
        }
    }
    if narrow.len() >= QUALIFY_MIN_EXECUTABLE_RUN {
        runs.insert(narrow);
    }
    for start in [0usize, 1] {
        let mut wide = String::new();
        let mut cursor = start;
        while cursor + 1 < bytes.len() {
            let (low, high) = (bytes[cursor], bytes[cursor + 1]);
            if high == 0 && low.is_ascii_graphic() {
                wide.push(low as char);
            } else if wide.len() >= QUALIFY_MIN_EXECUTABLE_RUN {
                runs.insert(std::mem::take(&mut wide));
            } else {
                wide.clear();
            }
            cursor += 2;
        }
        if wide.len() >= QUALIFY_MIN_EXECUTABLE_RUN {
            runs.insert(wide);
        }
    }
    runs
}

/// Whether the executable names this identifier, allowing for the fact that a run is a whole
/// string literal and a class name is usually only part of one.
fn executable_names(runs: &std::collections::HashSet<String>, name: &str) -> bool {
    runs.contains(name) || runs.iter().any(|run| run.contains(name))
}

/// The native base-class names this build's own script cache refers to: every `super_class` that no
/// module in the cache declares. Reported, never decisive — a dump can be complete for the cache
/// and still describe a different executable.
fn script_cache_native_bases(modules: &[gore_as::cache::model::Module]) -> Vec<String> {
    let declared: std::collections::HashSet<&str> = modules
        .iter()
        .flat_map(|module| module.classes.iter().map(|class| class.name.as_str()))
        .collect();
    let mut bases: Vec<String> = modules
        .iter()
        .flat_map(|module| module.classes.iter())
        .filter_map(|class| class.super_class.as_deref())
        .filter(|base| !declared.contains(base))
        .map(str::to_owned)
        .collect();
    bases.sort_unstable();
    bases.dedup();
    bases
}

/// AngelScript spells an Unreal class with its engine prefix; the USMAP does not.
fn unprefixed_class_name(name: &str) -> &str {
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second))
            if matches!(first, 'U' | 'A' | 'F' | 'I') && second.is_ascii_uppercase() =>
        {
            &name[1..]
        }
        _ => name,
    }
}

/// The three parser-output digests a reflection dump determines, and the counts they were taken
/// over.
///
/// This is the join `DefaultNativeAncestry::from_schema_db` performs — the same `gore_asset`
/// accessors, in the same order, through the same row digest — minus every comparison against a
/// sealed row, because a build being qualified has not got one yet. That it really is the same join
/// is not a claim this file can make about itself:
/// `qualify_reproduces_the_sealed_values_of_the_generation_it_is_run_against` re-derives an audited
/// generation and requires both joined digests to come out equal to the ones the gate checks.
struct QualifyClassProfile {
    usmap_class_graph_sha256: [u8; 32],
    usmap_class_rows: usize,
    usmap_tag_map_declarations: usize,
    /// Absent when there is no Binds class bridge to join against. A digest over an empty row table
    /// is a perfectly valid digest, which is exactly why it must never be reported as a measurement.
    bridge: Option<QualifyClassBridge>,
}

/// What the Binds class bridge and the class graph say jointly. `class_paths` also names structs
/// and enums and types this dump has never heard of; they are not class ancestry and their exact
/// omission is part of what the resolved digest seals.
struct QualifyClassBridge {
    resolved_class_profile_sha256: [u8; 32],
    gameplay_tag_float32_map_profile_sha256: [u8; 32],
    bridged_class_count: usize,
    gameplay_tag_float32_map_field_count: usize,
}

fn derive_class_profiles(
    schemas: &gore_asset::SchemaDb,
    class_paths: Option<&std::collections::HashMap<String, String>>,
) -> std::result::Result<QualifyClassProfile, String> {
    let mut super_ids = vec![None; schemas.len()];
    let mut graph_rows = Vec::new();
    let mut usmap_tag_map_declarations = 0usize;
    for record in schemas
        .schemas()
        .iter()
        .filter(|record| record.kind == gore_asset::SchemaKind::Class)
    {
        let parent = schemas
            .exact_class_super_schema_id(record.id)
            .map_err(|error| format!("USMAP class graph: {}: {error}", record.qualified_name()))?;
        super_ids[record.id] = parent;
        let parent_name = parent
            .map(|id| {
                schemas
                    .schema(id)
                    .expect("resolved schema id")
                    .qualified_name()
            })
            .unwrap_or_default();
        graph_rows.push([record.qualified_name(), parent_name]);
        for property in &record.properties {
            if schemas
                .exact_declared_property_shape(record.id, &property.name)
                .map_err(|error| {
                    format!(
                        "USMAP class graph: {}.{}: {error}",
                        record.qualified_name(),
                        property.name
                    )
                })?
                == Some(gore_asset::schema::ExactDeclaredPropertyShape::GameplayTagFloat32Map)
            {
                usmap_tag_map_declarations += 1;
            }
        }
    }
    let usmap_class_rows = graph_rows.len();
    let usmap_class_graph_sha256 = gore_generation::qualify::canonical_rows_sha256(&mut graph_rows);

    let mut bridge = None;
    if let Some(class_paths) = class_paths.filter(|paths| !paths.is_empty()) {
        let mut claimed = std::collections::HashSet::new();
        let mut resolved_rows = Vec::new();
        let mut tag_map_rows = Vec::new();
        for (script_class, path) in class_paths {
            let id = match schemas.resolve_class(path) {
                Ok(id) => id,
                Err(gore_asset::SchemaError::SchemaNotFound { .. }
                | gore_asset::SchemaError::NotAClass(_)) => continue,
                Err(error) => {
                    return Err(format!("Binds class bridge: {script_class} -> {path}: {error}"));
                }
            };
            let canonical_path = schemas
                .schema(id)
                .expect("resolved schema id")
                .qualified_name();
            if canonical_path != *path {
                return Err(format!(
                    "Binds class bridge: {script_class} -> {path}: non-canonical case; exact \
                     schema path is {canonical_path}"
                ));
            }
            for property in &schemas.schema(id).expect("resolved schema id").properties {
                if schemas
                    .exact_declared_property_shape(id, &property.name)
                    .map_err(|error| {
                        format!(
                            "Binds class bridge: {script_class} -> {canonical_path}.{}: {error}",
                            property.name
                        )
                    })?
                    == Some(gore_asset::schema::ExactDeclaredPropertyShape::GameplayTagFloat32Map)
                {
                    tag_map_rows.push([
                        script_class.clone(),
                        canonical_path.clone(),
                        property.name.clone(),
                    ]);
                }
            }
            if !claimed.insert(id) {
                return Err(format!(
                    "Binds class bridge: two Binds class names resolve to USMAP schema \
                     {canonical_path}"
                ));
            }
            let parent = super_ids[id]
                .map(|parent| {
                    schemas
                        .schema(parent)
                        .expect("resolved parent id")
                        .qualified_name()
                })
                .unwrap_or_default();
            resolved_rows.push([script_class.clone(), canonical_path, parent]);
        }
        bridge = Some(QualifyClassBridge {
            bridged_class_count: resolved_rows.len(),
            gameplay_tag_float32_map_field_count: tag_map_rows.len(),
            resolved_class_profile_sha256: gore_generation::qualify::canonical_rows_sha256(
                &mut resolved_rows,
            ),
            gameplay_tag_float32_map_profile_sha256:
                gore_generation::qualify::canonical_rows_sha256(&mut tag_map_rows),
        });
    }

    Ok(QualifyClassProfile {
        usmap_class_graph_sha256,
        usmap_class_rows,
        usmap_tag_map_declarations,
        bridge,
    })
}

/// The next curated record set's id and the three labels that carry its version.
///
/// These are names rather than measurements, and they would normally be a person's job — except
/// that the record-set id is hashed into `record_set_seal`, so a draft that left it as a
/// placeholder would print two seals that stop being true the moment somebody names the record set.
/// Deriving the name is what makes deriving those two seals worth anything.
struct QualifyRecordSetNaming {
    record_set_id: String,
    catalog_label: String,
    record_seal_kind: String,
    catalog_seal_kind: String,
}

impl QualifyRecordSetNaming {
    fn succeeding(previous: &gore_generation::GenerationRow) -> Self {
        let (stem, version) = match previous
            .record_set_id
            .rsplit_once("-v")
            .and_then(|(stem, version)| version.parse::<u32>().ok().map(|version| (stem, version)))
        {
            Some((stem, version)) => (stem.to_owned(), version.saturating_add(1)),
            None => (previous.record_set_id.to_owned(), 2),
        };
        Self {
            record_set_id: format!("{stem}-v{version}"),
            catalog_label: format!("compiled curated V{version}"),
            record_seal_kind: format!("compiled curated V{version} record set"),
            catalog_seal_kind: format!("compiled curated V{version} catalog payload"),
        }
    }

    fn of(row: &gore_generation::GenerationRow) -> Self {
        Self {
            record_set_id: row.record_set_id.to_owned(),
            catalog_label: row.catalog_label.to_owned(),
            record_seal_kind: row.record_seal_kind.to_owned(),
            catalog_seal_kind: row.catalog_seal_kind.to_owned(),
        }
    }
}

fn qualify_seal_text(seal: &gore_generation::FileSeal) -> String {
    format!("{} bytes / sha256 {}", seal.byte_len, hex(&seal.sha256))
}

/// One `.usmap` candidate, measured against the executable and the script cache.
struct QualifyUsmapCandidate {
    path: PathBuf,
    byte_len: u64,
    sha256: [u8; 32],
    schemas: Option<gore_asset::SchemaDb>,
    class_rows: Option<usize>,
    absent: Vec<String>,
    absent_count: Option<usize>,
    native_bases_resolved: Option<usize>,
    verdict: &'static str,
}

fn assess_usmap_candidate(
    path: &Path,
    runs: &std::collections::HashSet<String>,
    native_bases: &[String],
) -> QualifyUsmapCandidate {
    let mut candidate = QualifyUsmapCandidate {
        path: path.to_path_buf(),
        byte_len: 0,
        sha256: [0; 32],
        schemas: None,
        class_rows: None,
        absent: Vec::new(),
        absent_count: None,
        native_bases_resolved: None,
        verdict: "could not be read",
    };
    let Ok(bytes) = read_default_usmap(path) else {
        return candidate;
    };
    candidate.byte_len = bytes.len() as u64;
    candidate.sha256 = Sha256::digest(&bytes).into();
    let Ok(schemas) = gore_asset::SchemaDb::from_usmap(&bytes) else {
        candidate.verdict = "is not a parseable schema map";
        return candidate;
    };
    let classes: Vec<&str> = schemas
        .schemas()
        .iter()
        .filter(|record| record.kind == gore_asset::SchemaKind::Class)
        .map(|record| record.name.as_str())
        .collect();
    candidate.class_rows = Some(classes.len());
    let absent: Vec<String> = classes
        .iter()
        .filter(|name| !executable_names(runs, name))
        .map(|name| (*name).to_owned())
        .collect();
    candidate.absent_count = Some(absent.len());
    candidate.absent = absent;
    candidate.native_bases_resolved = Some(
        native_bases
            .iter()
            .filter(|base| schemas.resolve_class(unprefixed_class_name(base)).is_ok())
            .count(),
    );
    candidate.verdict = if candidate.absent.is_empty() {
        "every class it declares is named by this executable"
    } else {
        "declares classes this executable never names"
    };
    candidate.schemas = Some(schemas);
    candidate
}

fn qualify_candidate_json(
    candidate: &QualifyUsmapCandidate,
    native_bases_total: usize,
) -> QualifyUsmapCandidateJson {
    QualifyUsmapCandidateJson {
        path: candidate.path.display().to_string(),
        sha256: hex(&candidate.sha256),
        byte_len: candidate.byte_len,
        class_rows: candidate.class_rows,
        class_names_absent_from_executable: candidate.absent_count,
        absent_examples: candidate
            .absent
            .iter()
            .take(QUALIFY_MAX_ABSENT_EXAMPLES)
            .cloned()
            .collect(),
        native_bases_resolved: candidate.native_bases_resolved,
        native_bases_total,
        verdict: candidate.verdict,
    }
}

/// Pick the one dump that belongs to this build, or say why no single dump does.
///
/// A candidate is scored by how many of its own class names the executable never mentions, and then
/// by how many of the script cache's native base classes it cannot resolve. On the 2026-07-31
/// update those two numbers are 5 and 4 for the previous generation's dump against 1 and 1 for the
/// re-dumped one, and the four extra names are exactly the four classes that build removed — so the
/// binary discriminates between the two dumps even though both parse and both hash to a sealed
/// value. The winner has to be strictly better than everything else: a tie is a refusal, because
/// choosing between a current dump and a stale one at random is how a profile ends up internally
/// consistent over the previous game's class graph.
///
/// There is deliberately no absolute threshold. A dump legitimately declares a handful of names the
/// binary never spells — `Default__Class` is a reflection artifact, not a class — so a fixed floor
/// would reject every dump ever produced. What guards the case where the only dump on disk is the
/// stale one is that it is byte-identical to a sealed dump for another executable, which the caller
/// checks separately.
fn select_qualify_usmap(
    candidates: &[QualifyUsmapCandidate],
    native_bases_total: usize,
) -> std::result::Result<usize, String> {
    let scored: Vec<(usize, (usize, usize))> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let absent = candidate.absent_count?;
            let unresolved = native_bases_total - candidate.native_bases_resolved.unwrap_or(0);
            Some((index, (absent, unresolved)))
        })
        .collect();
    let Some(&(best_index, best)) = scored.iter().min_by_key(|(_, score)| *score) else {
        return Err(
            "no .usmap in the install parsed as a schema map, so nothing can be tied to this \
             executable — re-dump it with UE4SS and run this again"
                .to_owned(),
        );
    };
    let tied = scored
        .iter()
        .filter(|(index, score)| *score == best && *index != best_index)
        .count();
    if tied > 0 {
        return Err(format!(
            "{} dumps fit this build equally well ({} class name(s) unnamed by the executable and \
             {} unresolved native base(s) each); refusing to choose — pass --usmap with the one you \
             re-dumped against this build",
            tied + 1,
            best.0,
            best.1
        ));
    }
    Ok(best_index)
}

/// The row this dump is sealed for when that row describes a *different* executable, or `None` when
/// some row pairs this exact dump with the executable in front of us.
///
/// Asking the first row that carries the seal is not the same question, and gets it wrong on the
/// generations that matter: two audited rows deliberately share one USMAP, because a build may move
/// the executable and the script cache without moving the reflection layout. Matching a seal
/// therefore proves nothing on its own. What makes a dump stale is that *every* row carrying it
/// names another executable — so a build whose own sealed dump is sitting right there could not
/// requalify without `--usmap`, which is the flag for asserting a reuse deliberately.
fn stale_usmap_row(
    usmap_sha256: &[u8; 32],
    executable_seal: &gore_generation::FileSeal,
) -> Option<&'static gore_generation::GenerationRow> {
    let mut carrying = gore_generation::rows().iter().filter(|row| row.usmap.sha256 == *usmap_sha256);
    let first = carrying.next()?;
    if first.executable == *executable_seal {
        return None;
    }
    match carrying.find(|row| row.executable == *executable_seal) {
        Some(_) => None,
        None => Some(first),
    }
}

/// The curated script modules the story catalog seals, and the source each one must reproduce.
struct QualifyCuratedRecord {
    module: String,
    runtime_class: String,
    seal: gore_story_catalog::ContentSeal,
}

fn qualify_curated_records(
    catalog: &gore_story_catalog::StoryCatalogFile,
) -> Result<Vec<QualifyCuratedRecord>> {
    let selections = catalog
        .authoring_selections()
        .map_err(|error| anyhow::anyhow!("AS_QUALIFY_CATALOG: {error}"))?;
    let mut records = Vec::new();
    let mut push = |selection: &gore_story_catalog::AuthoringClassSelection| {
        // `script-class:<module>/<class>` is the catalog's own provenance spelling; the module is
        // what `emit` indexes by, so this is the only place the two vocabularies meet.
        let module = selection
            .source_catalog_selector
            .strip_prefix("script-class:")
            .and_then(|rest| rest.rsplit_once('/'))
            .map(|(module, _)| module.to_owned());
        if let Some(module) = module {
            records.push(QualifyCuratedRecord {
                module,
                runtime_class: selection.runtime_class.clone(),
                seal: selection.source_seal.clone(),
            });
        }
    };
    for npc in &selections.npcs {
        push(&npc.character_definition);
        push(&npc.ai_agent_config);
        push(&npc.spawn_definition);
    }
    for parent in &selections.quest_parents {
        push(&parent.quest_class);
    }
    records.sort_by(|left, right| left.module.cmp(&right.module));
    records.dedup_by(|left, right| left.module == right.module);
    Ok(records)
}

fn read_qualification_number(artifact: &str, key: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(artifact)
        .ok()
        .and_then(|value| value.get(key).and_then(serde_json::Value::as_u64))
}

fn previous_qualification(row_id: &str) -> Option<&'static str> {
    gore_generation::QUALIFICATION_ARTIFACTS
        .iter()
        .find(|(id, _)| *id == row_id)
        .map(|(_, artifact)| *artifact)
}

/// Derive an installed build's generation row and qualification artifact.
///
/// Reads the installation and writes nothing. Every number it prints names the function that
/// produced it, and every number it could not produce is named too — a qualification that quietly
/// omitted a value would be worse than one that refused, because the omission is what gets pasted
/// into the table as a plausible-looking constant.
#[allow(clippy::too_many_lines)]
fn run_qualify(
    game: Option<PathBuf>,
    usmap: Option<PathBuf>,
    catalog: Option<PathBuf>,
    id: Option<String>,
    label: Option<String>,
    json: bool,
) -> Result<()> {
    let paths = qualify_paths(game)?;
    let generation = gore_story_catalog::capture_generation(
        &gore_story_catalog::GenerationPaths {
            executable: paths.executable.clone(),
            shipping_cache: paths.shipping_cache.clone(),
            binds_cache: paths.binds_cache.clone(),
        },
        gore_story_catalog::GenerationInputLimits::default(),
    )
    .map_err(|error| anyhow::anyhow!("AS_QUALIFY_INPUTS: {error}"))?;
    let executable_seal = qualify_file_seal(&generation.executable);
    let shipping_seal = qualify_file_seal(&generation.shipping_cache);
    let binds_seal = qualify_file_seal(&generation.binds_cache);
    let audited =
        gore_generation::row_for_file_seals(&executable_seal, &shipping_seal, &binds_seal);

    let cache = read_module_cache(&paths.shipping_cache)?;
    let script_cache_guid = CacheHeader::parse(&cache)
        .map_err(|error| anyhow::anyhow!("AS_QUALIFY_CACHE: {error}"))?
        .hash;
    let proposed_id = id.unwrap_or_else(|| match audited {
        Some(row) => row.id.to_owned(),
        None => format!("g1r-steam-{}", &hex16(&script_cache_guid)[..8]),
    });
    let proposed_label = label.unwrap_or_else(|| match audited {
        Some(row) => row.label.to_owned(),
        None => format!("Unqualified build, script cache GUID {}", hex16(&script_cache_guid)),
    });

    let executable_bytes = read_regular_bounded(
        &paths.executable,
        QUALIFY_EXECUTABLE_MAX_BYTES,
        "AS_QUALIFY_EXECUTABLE",
    )?;
    let runs = executable_name_runs(&executable_bytes);
    drop(executable_bytes);
    // A cache this build's own decoder cannot walk is a fact to report, not a reason to stop: the
    // file seals and the reflection dump are still qualifiable, and refusing here would hide them
    // behind the one input that failed.
    let modules = match gore_as::cache::model::parse_modules(&cache) {
        Ok(modules) => modules,
        Err(error) => {
            eprintln!("warning: AS_QUALIFY_MODULES: {error}");
            Vec::new()
        }
    };
    let native_bases = script_cache_native_bases(&modules);

    let asserted_usmap = usmap.is_some();
    let configured = usmap.or_else(|| std::env::var_os("GORE_AS_USMAP").map(PathBuf::from));
    let candidate_paths = default_usmap_candidates(&paths.shipping_cache, configured)?;
    let candidates: Vec<QualifyUsmapCandidate> = candidate_paths
        .iter()
        .map(|path| assess_usmap_candidate(path, &runs, &native_bases))
        .collect();
    let mut selection = select_qualify_usmap(&candidates, native_bases.len());
    // The trap this whole command exists for: a dump from the previous build hashes to that build's
    // seal and describes it faithfully, so nothing about the file itself is wrong. Two audited rows
    // legitimately share one dump, so this cannot be an error — but it cannot be silent either.
    if let (Ok(index), false) = (&selection, asserted_usmap) {
        if let Some(prior) = stale_usmap_row(&candidates[*index].sha256, &executable_seal) {
            selection = Err(format!(
                "the dump that fits is byte-for-byte the one sealed for {}, whose executable is \
                 not this one. UE4SS generates the dump on your machine and nothing forces it to \
                 be regenerated, so this is either a build that did not move the reflection \
                 layout or a stale dump describing the previous game. Re-dump it and run this \
                 again, or pass --usmap to assert the reuse deliberately",
                prior.id
            ));
        }
    }

    let sealed = selection.as_ref().ok().map(|index| &candidates[*index]);
    let sealed_seal = sealed.map(|candidate| gore_generation::FileSeal {
        byte_len: candidate.byte_len,
        sha256: candidate.sha256,
    });
    let selection_reason = match (&selection, sealed) {
        (Ok(_), Some(candidate)) => format!(
            "{} of the {} class(es) it declares are named by this executable, and it resolves {} of \
             the {} native base class(es) this build's own script cache refers to",
            candidate.class_rows.unwrap_or(0) - candidate.absent.len(),
            candidate.class_rows.unwrap_or(0),
            candidate.native_bases_resolved.unwrap_or(0),
            native_bases.len()
        ),
        (Err(reason), _) => reason.clone(),
        (Ok(_), None) => unreachable!("a selected index always names a candidate"),
    };

    let mut unavailable = Vec::new();
    let mut note =
        |field, reason: String| unavailable.push(QualifyUnavailableJson { field, reason });

    // The script cache's own three numbers, from the function the admission gate compares a row
    // against. Reading them anywhere else is the difference between a row that describes this cache
    // and a row that describes what somebody typed while looking at a test run.
    let fingerprint =
        match gore_as::cache::default_fingerprint::combined_default_cache_fingerprint(&cache) {
            Ok(fingerprint) => Some(fingerprint),
            Err(error) => {
                for field in [
                    "script_cache_mutation_stable_sha256",
                    "gameplay_tag_float32_operand_count",
                ] {
                    note(
                        field,
                        format!("the script cache could not be fingerprinted: {error}"),
                    );
                }
                None
            }
        };

    // Read `Binds.Cache` again rather than reusing the bytes any other pass held, then require the
    // re-read to hash to what `capture_generation` sealed. The two digests below are evidence about
    // this build only if the bytes they were taken over are the bytes the row will name.
    let binds_bytes = read_regular_bounded(
        &paths.binds_cache,
        DEFAULT_BINDS_MAX_BYTES,
        "AS_QUALIFY_BINDS",
    )?;
    let reread_binds_sha256: [u8; 32] = Sha256::digest(&binds_bytes).into();
    if reread_binds_sha256 != binds_seal.sha256 {
        bail!(
            "AS_QUALIFY_BINDS: {} changed while it was being qualified; refusing to seal digests \
             taken over bytes that are no longer the ones this run sealed",
            paths.binds_cache.display()
        );
    }
    let binds_profile = gore_as::cache::binds::derive_binds_profile(&binds_bytes);
    let binds_profile = if binds_profile.field_row_count == 0
        || binds_profile.class_path_row_count == 0
    {
        for field in ["binds_field_map_sha256", "binds_class_path_map_sha256"] {
            note(
                field,
                format!(
                    "the Binds record scan found {} field row(s) and {} class row(s); a digest over \
                     an empty table is valid and means nothing, so neither is reported",
                    binds_profile.field_row_count, binds_profile.class_path_row_count
                ),
            );
        }
        None
    } else {
        Some(binds_profile)
    };

    let class_profile = match sealed.and_then(|candidate| candidate.schemas.as_ref()) {
        Some(schemas) => match derive_class_profiles(
            schemas,
            binds_profile.as_ref().map(|profile| &profile.class_paths),
        ) {
            Ok(profile) => Some(profile),
            Err(error) => bail!("AS_QUALIFY_PROFILE: {error}"),
        },
        None => None,
    };
    let bridge = class_profile
        .as_ref()
        .and_then(|profile| profile.bridge.as_ref());
    let usmap_class_rows = class_profile
        .as_ref()
        .map(|profile| profile.usmap_class_rows);
    let usmap_tag_map_declarations = class_profile
        .as_ref()
        .map(|profile| profile.usmap_tag_map_declarations);

    let native = load_native_api(&paths.shipping_cache);
    let ancestry = match (sealed.and_then(|candidate| candidate.schemas.as_ref()), &native) {
        (Some(schemas), Some(native)) => {
            gore_as::cache::default_ancestry::DefaultNativeAncestry::from_schema_db(
                native, &cache, schemas,
            )
            .ok()
        }
        _ => None,
    };
    let report = match gore_as::cache::default_patch::default_sites_with_evidence(
        &cache,
        load_native_api(&paths.shipping_cache),
        ancestry.clone(),
        None,
    ) {
        Ok(report) => Some(report),
        Err(error) => {
            eprintln!("warning: AS_QUALIFY_SITES: {error}");
            None
        }
    };
    let stats = report.as_ref().map(|report| report.stats.clone());

    if class_profile.is_none() {
        for field in ["usmap_class_graph_sha256"] {
            note(
                field,
                "no reflection dump was tied to this executable, so there is no class graph to \
                 digest"
                    .to_owned(),
            );
        }
    }
    if bridge.is_none() {
        for field in [
            "resolved_class_profile_sha256",
            "gameplay_tag_float32_map_profile_sha256",
        ] {
            note(
                field,
                "the Binds class bridge and the USMAP class graph are joined to produce this, and \
                 one of the two was not available"
                    .to_owned(),
            );
        }
    }

    // The two published IDs, from the nine components measured off this installation, through the
    // same two functions the runtime gate runs. A qualification that spelled either of them any
    // other way would mint a row nothing on the mutation path could ever match, and the failure
    // would surface as a build the table admits and the tool refuses.
    let published = match (&fingerprint, &binds_profile, &class_profile, bridge, sealed_seal) {
        (Some(fingerprint), Some(binds), Some(class_profile), Some(bridge), Some(usmap)) => {
            let components = gore_generation::qualify::observed_profile_components(
                script_cache_guid,
                fingerprint.sha256,
                fingerprint.scalar_operand_count,
                fingerprint.tag_operand_count,
                binds_seal.sha256,
                binds.class_path_map_sha256,
                usmap.sha256,
                class_profile.usmap_class_graph_sha256,
                bridge.resolved_class_profile_sha256,
            );
            Some(gore_generation::qualify::derive_published_ids(
                &components,
                &bridge.gameplay_tag_float32_map_profile_sha256,
            ))
        }
        _ => {
            for field in [
                "native_ancestry_profile_id",
                "gameplay_tag_float32_map_proof_id",
            ] {
                note(
                    field,
                    "it is the digest of nine components and at least one of them was not derived; \
                     see the other entries here"
                        .to_owned(),
                );
            }
            None
        }
    };

    // A build already in the table is compared against the generation it succeeded, not against
    // itself: re-qualifying row three has to reproduce the 26339 -> 26399 and 6572 -> 6582 moves
    // that made it a re-seal, or the comparison proves nothing.
    let comparison_row = match audited {
        Some(row) => gore_generation::rows()
            .iter()
            .position(|candidate| candidate.id == row.id)
            .and_then(|index| index.checked_sub(1))
            .map(|index| &gore_generation::rows()[index]),
        None => gore_generation::qualify::nearest_row(
            Some(&binds_seal.sha256),
            sealed_seal.as_ref().map(|seal| &seal.sha256),
        ),
    };
    let previous = comparison_row.and_then(|row| previous_qualification(row.id));
    let previous_number =
        |key: &str| previous.and_then(|artifact| read_qualification_number(artifact, key));

    // The record set is named before it is sealed, because its id is inside the bytes the seal is
    // taken over. An audited build keeps the name it already published; a new one succeeds the
    // generation it is being compared against.
    let naming = match (audited, comparison_row) {
        (Some(row), _) => QualifyRecordSetNaming::of(row),
        (None, Some(row)) => QualifyRecordSetNaming::succeeding(row),
        (None, None) => QualifyRecordSetNaming {
            record_set_id: format!("{proposed_id}-curated-story-v1"),
            catalog_label: "compiled curated V1".to_owned(),
            record_seal_kind: "compiled curated V1 record set".to_owned(),
            catalog_seal_kind: "compiled curated V1 catalog payload".to_owned(),
        },
    };
    let curated_seals = gore_story_catalog::compile_curated_seals(&generation, &naming.record_set_id);
    if let Err(error) = &curated_seals {
        for field in ["record_set_seal", "catalog_payload_seal"] {
            note(
                field,
                format!("the curated record set did not compile for this generation: {error}"),
            );
        }
    }
    let curated_seal = |seal: &gore_story_catalog::ContentSeal| gore_generation::FileSeal {
        byte_len: seal.byte_len,
        sha256: *seal.sha256.as_bytes(),
    };

    // Nothing below is copied out of an audited row. Every measured value in this draft was derived
    // from the bytes on disk by the crate that owns the format it describes — including for a build
    // the table already carries, because re-deriving a sealed row is the only way a run can *check*
    // it rather than print it back.
    let mut draft = gore_generation::qualify::DraftRow {
        id: proposed_id.clone(),
        label: proposed_label.clone(),
        edition: generation.edition.clone(),
        executable: Some(executable_seal),
        shipping_cache: Some(shipping_seal),
        binds_cache: Some(binds_seal),
        usmap: sealed_seal,
        script_cache_guid: Some(script_cache_guid),
        script_cache_mutation_stable_sha256: fingerprint.map(|fingerprint| fingerprint.sha256),
        // The gate compares a row's scalar count against the fingerprint's, so that is where the
        // row's value comes from; `default_sites` counts the same windows through a different
        // filter and is the cross-check below rather than the source.
        scalar_default_operand_count: fingerprint
            .map(|fingerprint| fingerprint.scalar_operand_count)
            .or_else(|| stats.as_ref().map(|stats| stats.direct_windows)),
        gameplay_tag_float32_operand_count: fingerprint
            .map(|fingerprint| fingerprint.tag_operand_count),
        binds_field_map_sha256: binds_profile
            .as_ref()
            .map(|profile| profile.field_map_sha256),
        binds_class_path_map_sha256: binds_profile
            .as_ref()
            .map(|profile| profile.class_path_map_sha256),
        usmap_class_graph_sha256: class_profile
            .as_ref()
            .map(|profile| profile.usmap_class_graph_sha256),
        resolved_class_profile_sha256: bridge.map(|bridge| bridge.resolved_class_profile_sha256),
        gameplay_tag_float32_map_profile_sha256: bridge
            .map(|bridge| bridge.gameplay_tag_float32_map_profile_sha256),
        native_ancestry_profile_id: published.as_ref().map(|(profile, _)| profile.clone()),
        gameplay_tag_float32_map_proof_id: published.as_ref().map(|(_, proof)| proof.clone()),
        record_set_id: naming.record_set_id.clone(),
        record_set_seal: curated_seals
            .as_ref()
            .ok()
            .map(|seals| curated_seal(&seals.record_set_seal)),
        catalog_payload_seal: curated_seals
            .as_ref()
            .ok()
            .map(|seals| curated_seal(&seals.catalog_payload_seal)),
        catalog_label: naming.catalog_label.clone(),
        record_seal_kind: naming.record_seal_kind.clone(),
        catalog_seal_kind: naming.catalog_seal_kind.clone(),
        audited_item_generation: match audited {
            Some(row) => row.audited_item_generation.to_owned(),
            None => proposed_id.clone(),
        },
    };
    if let (Some(fingerprint), Some(stats)) = (&fingerprint, &stats) {
        // Two independent passes over the same cache count the same scalar windows behind different
        // filters. `docs/reference/game-updates.md` step 7 asks for exactly this comparison, and a
        // disagreement means the row would seal a number one of the two passes does not see.
        if fingerprint.scalar_operand_count != stats.direct_windows {
            bail!(
                "AS_QUALIFY_SCALAR_COUNT: the cache fingerprint counts {} direct scalar operand \
                 range(s) and default-site discovery counts {}; the two passes disagree about this \
                 cache, so neither number may be sealed",
                fingerprint.scalar_operand_count,
                stats.direct_windows
            );
        }
    }

    // Anything still missing that did not name its own reason. A value that is absent from the draft
    // and absent from `unavailable` is the one shape this document must not have: a gap nobody has
    // to explain is a gap a reader fills in from the row above.
    for field in draft.missing() {
        if !unavailable.iter().any(|entry| entry.field == field) {
            unavailable.push(QualifyUnavailableJson {
                field,
                reason: "this run neither derived it nor said why, which is a defect in `gore as \
                         qualify` rather than a fact about the build"
                    .to_owned(),
            });
        }
    }

    // The only check available to this command that it derived anything *correctly*. Everything
    // above came out of the installation; an audited row says what those same values were when a
    // person last looked at that installation, so on an audited build the two have to agree, field
    // by field. A run that could quietly disagree here is a run that would seal its own mistakes.
    let mut diverged: Vec<QualifyDivergenceJson> = Vec::new();
    if let Some(row) = audited {
        let mut compare = |field, sealed: String, derived: Option<String>| {
            if derived.as_deref() != Some(sealed.as_str()) {
                diverged.push(QualifyDivergenceJson {
                    field,
                    sealed,
                    derived,
                });
            }
        };
        compare(
            "usmap",
            qualify_seal_text(&row.usmap),
            draft.usmap.as_ref().map(qualify_seal_text),
        );
        compare(
            "script_cache_mutation_stable_sha256",
            hex(&row.script_cache_mutation_stable_sha256),
            draft
                .script_cache_mutation_stable_sha256
                .as_ref()
                .map(|digest| hex(digest)),
        );
        compare(
            "scalar_default_operand_count",
            row.scalar_default_operand_count.to_string(),
            draft
                .scalar_default_operand_count
                .map(|count| count.to_string()),
        );
        compare(
            "gameplay_tag_float32_operand_count",
            row.gameplay_tag_float32_operand_count.to_string(),
            draft
                .gameplay_tag_float32_operand_count
                .map(|count| count.to_string()),
        );
        for (field, sealed, derived) in [
            (
                "binds_field_map_sha256",
                &row.binds_field_map_sha256,
                &draft.binds_field_map_sha256,
            ),
            (
                "binds_class_path_map_sha256",
                &row.binds_class_path_map_sha256,
                &draft.binds_class_path_map_sha256,
            ),
            (
                "usmap_class_graph_sha256",
                &row.usmap_class_graph_sha256,
                &draft.usmap_class_graph_sha256,
            ),
            (
                "resolved_class_profile_sha256",
                &row.resolved_class_profile_sha256,
                &draft.resolved_class_profile_sha256,
            ),
            (
                "gameplay_tag_float32_map_profile_sha256",
                &row.gameplay_tag_float32_map_profile_sha256,
                &draft.gameplay_tag_float32_map_profile_sha256,
            ),
        ] {
            compare(field, hex(sealed), derived.as_ref().map(|d| hex(d)));
        }
        compare(
            "native_ancestry_profile_id",
            row.native_ancestry_profile_id.to_owned(),
            draft.native_ancestry_profile_id.clone(),
        );
        compare(
            "gameplay_tag_float32_map_proof_id",
            row.gameplay_tag_float32_map_proof_id.to_owned(),
            draft.gameplay_tag_float32_map_proof_id.clone(),
        );
        compare(
            "record_set_seal",
            qualify_seal_text(&row.record_set_seal),
            draft.record_set_seal.as_ref().map(qualify_seal_text),
        );
        compare(
            "catalog_payload_seal",
            qualify_seal_text(&row.catalog_payload_seal),
            draft.catalog_payload_seal.as_ref().map(qualify_seal_text),
        );
    }

    let mut counts = vec![qualify_count(
        "script-cache native base classes",
        None,
        native_bases.len() as u64,
        "gore_as::cache::model::parse_modules",
    )];
    if let Some(stats) = &stats {
        counts.push(qualify_count(
            "scalar default windows",
            previous_number("direct_windows"),
            stats.direct_windows as u64,
            "gore_as::cache::default_patch::default_sites",
        ));
        counts.push(qualify_count(
            "initializer functions",
            None,
            stats.init_functions as u64,
            "gore_as::cache::default_patch::default_sites",
        ));
    }
    if let Some(rows) = usmap_class_rows {
        counts.push(qualify_count(
            "USMAP class rows",
            None,
            rows as u64,
            "gore_asset::SchemaDb::from_usmap",
        ));
    }
    if let Some(declarations) = usmap_tag_map_declarations {
        counts.push(qualify_count(
            "USMAP tag-map field declarations",
            previous_number("gameplay_tag_float32_map_field_count"),
            declarations as u64,
            "gore_asset::SchemaDb::exact_declared_property_shape",
        ));
    }
    if let Some(profile) = &binds_profile {
        counts.push(qualify_count(
            "Binds plain-field rows",
            previous_number("binds_field_row_count"),
            profile.field_row_count as u64,
            "gore_as::cache::binds::derive_binds_profile",
        ));
        counts.push(qualify_count(
            "Binds class-path rows",
            previous_number("binds_class_path_row_count"),
            profile.class_path_row_count as u64,
            "gore_as::cache::binds::derive_binds_profile",
        ));
    }
    if let Some(bridge) = bridge {
        counts.push(qualify_count(
            "bridged classes",
            previous_number("class_count"),
            bridge.bridged_class_count as u64,
            "gore as qualify, Binds class bridge joined with the USMAP class graph",
        ));
    }

    // The curated records are what distinguishes a moved identity from moved content: the catalog
    // seals the emitted source of specific modules, so the only way to know a patch did not rewrite
    // them is to decompile each one out of THIS cache and compare.
    let curated_catalog = match &catalog {
        Some(path) => {
            let bytes = read_regular_bounded(path, QUALIFY_CATALOG_MAX_BYTES, "AS_QUALIFY_CATALOG")?;
            Some((
                gore_story_catalog::StoryCatalogFile::from_json(&bytes)
                    .map_err(|error| anyhow::anyhow!("AS_QUALIFY_CATALOG: {error}"))?,
                format!("--catalog {}", path.display()),
            ))
        }
        None => gore_story_catalog::build_known_catalog(
            &gore_story_catalog::GenerationPaths {
                executable: paths.executable.clone(),
                shipping_cache: paths.shipping_cache.clone(),
                binds_cache: paths.binds_cache.clone(),
            },
            gore_story_catalog::GenerationInputLimits::default(),
        )
        .ok()
        .map(|catalog| (catalog, "the installation's own audited story catalog".to_owned())),
    };
    let mut curated = QualifyCuratedJson {
        source: "none — this build is not audited and no --catalog was passed, so the curated \
                 source seals could not be named"
            .to_owned(),
        modules: Vec::new(),
        all_reproduce: false,
    };
    if let Some((catalog, source)) = curated_catalog {
        let records = qualify_curated_records(&catalog)?;
        let mut refs =
            gore_as::cache::refs::RefResolver::build(&cache).context("AS_QUALIFY_RESOLVER")?;
        let prepared = gore_as::cache::emit_all::PreparedEmit::new(
            &modules,
            &mut refs,
            load_native_api(&paths.shipping_cache),
        )
        .context("AS_QUALIFY_EMIT")?;
        for record in &records {
            let emitted = modules
                .iter()
                .position(|module| module.name == record.module)
                .and_then(|index| prepared.emit_module(index).ok());
            let observed = emitted.as_ref().map(|source| {
                (
                    source.len() as u64,
                    gore_story_catalog::Sha256Digest::from_bytes(
                        Sha256::digest(source.as_bytes()).into(),
                    ),
                )
            });
            let reproduces = observed.as_ref().is_some_and(|(len, digest)| {
                *len == record.seal.byte_len && *digest == record.seal.sha256
            });
            curated.modules.push(QualifyCuratedModuleJson {
                module: record.module.clone(),
                runtime_class: record.runtime_class.clone(),
                expected_byte_len: record.seal.byte_len,
                expected_sha256: record.seal.sha256.to_string(),
                observed_byte_len: observed.as_ref().map(|(len, _)| *len),
                observed_sha256: observed.as_ref().map(|(_, digest)| digest.to_string()),
                reproduces,
            });
        }
        curated.all_reproduce =
            !curated.modules.is_empty() && curated.modules.iter().all(|module| module.reproduces);
        curated.source = source;
        counts.push(qualify_count(
            "curated modules reproducing their sealed source",
            Some(curated.modules.len() as u64),
            curated.modules.iter().filter(|m| m.reproduces).count() as u64,
            "gore_as::cache::emit_all::PreparedEmit::emit_module",
        ));
    }

    let regressions: Vec<&'static str> = counts
        .iter()
        .filter(|count| count.fell)
        .map(|count| count.name)
        .collect();

    let hex_or = |value: &Option<[u8; 32]>| value.as_ref().map(|bytes| hex(bytes));
    let seal_text = |seal: &Option<gore_generation::FileSeal>| seal.as_ref().map(qualify_seal_text);
    let fields = vec![
        QualifyFieldJson { field: "id", value: Some(draft.id.clone()), derived_by: Some("--id, or the script-cache GUID prefix") },
        QualifyFieldJson { field: "label", value: Some(draft.label.clone()), derived_by: Some("--label") },
        QualifyFieldJson { field: "edition", value: Some(draft.edition.clone()), derived_by: Some("gore_story_catalog::capture_generation") },
        QualifyFieldJson { field: "executable", value: seal_text(&draft.executable), derived_by: Some("gore_story_catalog::capture_generation") },
        QualifyFieldJson { field: "shipping_cache", value: seal_text(&draft.shipping_cache), derived_by: Some("gore_story_catalog::capture_generation") },
        QualifyFieldJson { field: "binds_cache", value: seal_text(&draft.binds_cache), derived_by: Some("gore_story_catalog::capture_generation") },
        QualifyFieldJson { field: "usmap", value: seal_text(&draft.usmap), derived_by: Some("gore as qualify, USMAP selection") },
        QualifyFieldJson { field: "script_cache_guid", value: draft.script_cache_guid.as_ref().map(hex16), derived_by: Some("gore_as::cache::header::CacheHeader::parse") },
        QualifyFieldJson { field: "script_cache_mutation_stable_sha256", value: hex_or(&draft.script_cache_mutation_stable_sha256), derived_by: Some("gore_as::cache::default_fingerprint (crate-private)") },
        QualifyFieldJson { field: "scalar_default_operand_count", value: draft.scalar_default_operand_count.map(|value| value.to_string()), derived_by: Some("gore_as::cache::default_patch::default_sites") },
        QualifyFieldJson { field: "gameplay_tag_float32_operand_count", value: draft.gameplay_tag_float32_operand_count.map(|value| value.to_string()), derived_by: Some("gore_as::cache::default_fingerprint (crate-private)") },
        QualifyFieldJson { field: "binds_field_map_sha256", value: hex_or(&draft.binds_field_map_sha256), derived_by: Some("gore_as::cache::binds (crate-private)") },
        QualifyFieldJson { field: "binds_class_path_map_sha256", value: hex_or(&draft.binds_class_path_map_sha256), derived_by: Some("gore_as::cache::binds (crate-private)") },
        QualifyFieldJson { field: "usmap_class_graph_sha256", value: hex_or(&draft.usmap_class_graph_sha256), derived_by: Some("gore_generation::qualify::canonical_rows_sha256 over gore_asset::SchemaDb::exact_class_super_schema_id") },
        QualifyFieldJson { field: "resolved_class_profile_sha256", value: hex_or(&draft.resolved_class_profile_sha256), derived_by: Some("gore_as::cache::default_ancestry (crate-private)") },
        QualifyFieldJson { field: "gameplay_tag_float32_map_profile_sha256", value: hex_or(&draft.gameplay_tag_float32_map_profile_sha256), derived_by: Some("DefaultNativeAncestry::gameplay_tag_float32_map_profile_sha256") },
        QualifyFieldJson { field: "native_ancestry_profile_id", value: draft.native_ancestry_profile_id.clone(), derived_by: Some("DefaultNativeAncestry::from_schema_db, which re-derives it through gore_generation::derived_profile_sha256 before returning") },
        QualifyFieldJson { field: "gameplay_tag_float32_map_proof_id", value: draft.gameplay_tag_float32_map_proof_id.clone(), derived_by: Some("DefaultNativeAncestry::gameplay_tag_float32_map_proof_id") },
        QualifyFieldJson { field: "record_set_id", value: Some(draft.record_set_id.clone()), derived_by: None },
        QualifyFieldJson { field: "record_set_seal", value: seal_text(&draft.record_set_seal), derived_by: Some("gore_story_catalog (crate-private canonical record bytes)") },
        QualifyFieldJson { field: "catalog_payload_seal", value: seal_text(&draft.catalog_payload_seal), derived_by: Some("gore_story_catalog (crate-private canonical payload bytes)") },
        QualifyFieldJson { field: "catalog_label", value: Some(draft.catalog_label.clone()), derived_by: None },
        QualifyFieldJson { field: "record_seal_kind", value: Some(draft.record_seal_kind.clone()), derived_by: None },
        QualifyFieldJson { field: "catalog_seal_kind", value: Some(draft.catalog_seal_kind.clone()), derived_by: None },
        QualifyFieldJson { field: "audited_item_generation", value: Some(draft.audited_item_generation.clone()), derived_by: None },
    ];

    let mut witnesses = std::collections::BTreeMap::new();
    for (key, witness) in [
        ("native_ancestry_profile_id", "gore as qualify (gore_as::cache::default_ancestry::DefaultNativeAncestry::from_schema_db)"),
        ("gameplay_tag_float32_map_proof_id", "gore as qualify (DefaultNativeAncestry::gameplay_tag_float32_map_proof_id)"),
        ("scalar_default_operand_count", "gore as qualify (gore_as::cache::default_patch::default_sites)"),
        ("gameplay_tag_float32_operand_count", "gore as qualify (not derivable: gore_as::cache::default_fingerprint is crate-private)"),
        ("class_count", "gore as qualify (DefaultNativeAncestry::class_count)"),
        ("gameplay_tag_float32_map_field_count", "gore as qualify (gore_asset::SchemaDb::exact_declared_property_shape over every class the sealed dump declares)"),
        ("unresolved_fields_with_ancestry", "gore as qualify (gore_as::cache::default_patch::default_sites)"),
        ("direct_windows", "gore as qualify (gore_as::cache::default_patch::default_sites)"),
    ] {
        witnesses.insert(key, witness.to_owned());
    }
    let mut notes = vec![
        format!(
            "Derived by `gore as qualify` against {}. Every digest here comes from the crate that \
             owns the format it describes; nothing was recomputed by a second implementation.",
            paths.root.display()
        ),
        format!("USMAP: {selection_reason}."),
    ];
    if let Some(row) = comparison_row {
        notes.push(format!(
            "Counts compared against {}; a count that fell is listed under `counts[].fell` and \
             blocks acceptance, because a digest cannot say that a parser dropped rows.",
            row.id
        ));
    }
    if !unavailable.is_empty() {
        notes.push(format!(
            "{} row value(s) were not derivable from outside gore-as and gore-story-catalog; see \
             `unavailable`.",
            unavailable.len()
        ));
    }
    let artifact = QualifyArtifactJson {
        generation_id: draft.id.clone(),
        label: draft.label.clone(),
        native_ancestry_profile_id: draft.native_ancestry_profile_id.clone(),
        gameplay_tag_float32_map_proof_id: draft.gameplay_tag_float32_map_proof_id.clone(),
        scalar_default_operand_count: draft.scalar_default_operand_count.map(|v| v as u64),
        gameplay_tag_float32_operand_count: draft.gameplay_tag_float32_operand_count.map(|v| v as u64),
        class_count: ancestry.as_ref().map(|a| a.class_count() as u64),
        gameplay_tag_float32_map_field_count: usmap_tag_map_declarations.map(|v| v as u64),
        unresolved_fields_with_ancestry: ancestry
            .as_ref()
            .and(stats.as_ref())
            .map(|stats| stats.unresolved_fields as u64),
        direct_windows: stats.as_ref().map(|stats| stats.direct_windows as u64),
        binds_field_row_count: binds_profile
            .as_ref()
            .map(|profile| profile.field_row_count as u64),
        binds_class_path_row_count: binds_profile
            .as_ref()
            .map(|profile| profile.class_path_row_count as u64),
        witnesses,
        notes,
    };

    let curated_diverged = curated
        .modules
        .iter()
        .filter(|module| !module.reproduces)
        .count();
    let missing = draft.missing();
    // A divergence is only reachable on a build the table already describes, and there it is the
    // worst outcome this command has: the installation answered differently than the row a person
    // signed off. Fold it into `complete` so a run that disagrees with its own sealed row can never
    // present itself as a finished draft.
    let complete = missing.is_empty()
        && regressions.is_empty()
        && curated.all_reproduce
        && diverged.is_empty();
    let mut still_to_do = match (audited, ancestry.is_some()) {
        (Some(row), true) => vec![format!(
            "nothing: {} is already in the table. This run re-derived it, so read the values above \
             against the sealed row rather than pasting them",
            row.id
        )],
        _ => draft.still_to_do(),
    };
    if let (Some(row), None) = (audited, &ancestry) {
        still_to_do.insert(
            0,
            format!(
                "find out why the sealed native ancestry of {} did not qualify against the dump \
                 this run sealed: the build is audited, so this is a wrong dump or a drifted seal, \
                 not an unaudited game",
                row.id
            ),
        );
    }
    if curated.modules.is_empty() {
        still_to_do.insert(
            0,
            "name the curated records: no story catalog described this build, so a moved identity \
             cannot yet be told from moved content — re-run with --catalog pointing at the \
             previous generation's published story_catalog.v1"
                .to_owned(),
        );
    } else if curated_diverged > 0 {
        still_to_do.insert(
            0,
            format!(
                "read the diff for the {curated_diverged} curated module(s) whose emitted source \
                 did not reproduce: that is a real content change, not a transcription job"
            ),
        );
    }
    if !regressions.is_empty() {
        still_to_do.insert(
            0,
            format!(
                "explain the count(s) that fell before accepting anything: {}",
                regressions.join(", ")
            ),
        );
    }
    if !diverged.is_empty() {
        still_to_do.insert(
            0,
            format!(
                "this installation disagrees with the row that already describes it, in {}: {}. \
                 Either the dump is wrong or the seal is, and until that is settled nothing derived \
                 here can be trusted",
                if diverged.len() == 1 { "one field" } else { "several fields" },
                diverged
                    .iter()
                    .map(|entry| entry.field)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    let document = QualifyJson {
        format: QUALIFY_REPORT_FORMAT,
        game: paths.root.display().to_string(),
        already_audited_as: audited.map(|row| row.id),
        compared_against: comparison_row.map(|row| row.id),
        inputs: QualifyInputsJson {
            executable: qualify_file_json(&paths.executable, &generation.executable),
            shipping_cache: qualify_file_json(&paths.shipping_cache, &generation.shipping_cache),
            binds_cache: qualify_file_json(&paths.binds_cache, &generation.binds_cache),
            script_cache_guid: hex16(&script_cache_guid),
            usmap: sealed.map(|candidate| QualifyFileJson {
                path: candidate.path.display().to_string(),
                byte_len: candidate.byte_len,
                sha256: hex(&candidate.sha256),
            }),
        },
        usmap_selection: QualifyUsmapSelectionJson {
            sealed: sealed.map(|candidate| QualifyFileJson {
                path: candidate.path.display().to_string(),
                byte_len: candidate.byte_len,
                sha256: hex(&candidate.sha256),
            }),
            reason: selection_reason.clone(),
            examined: candidates
                .iter()
                .map(|candidate| qualify_candidate_json(candidate, native_bases.len()))
                .collect(),
        },
        counts,
        curated_records: curated,
        row: QualifyRowJson {
            id: draft.id.clone(),
            fields,
            missing: missing.clone(),
            literal: draft.to_rust_literal(),
        },
        qualification: artifact,
        unavailable,
        diverged_from_sealed_row: diverged,
        complete,
        still_to_do,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        print_qualify_report(&document);
    }
    // What the command refuses over, and what it merely reports. A row value nobody could derive is
    // reported, because the draft says so in a way that cannot be pasted. A dump that could not be
    // tied to this build, a count that fell, and a curated module whose source moved are refusals:
    // each of them is a way for a green-looking row to describe a different game.
    if let Err(reason) = selection {
        bail!("AS_QUALIFY_USMAP: {reason}");
    }
    if !regressions.is_empty() {
        bail!(
            "AS_QUALIFY_COUNTS: {} count(s) fell against {}: {} — a digest cannot say that a parser \
             dropped rows, so this is refused rather than sealed",
            regressions.len(),
            comparison_row.map_or("the previous generation", |row| row.id),
            regressions.join(", ")
        );
    }
    if curated_diverged > 0 {
        bail!(
            "AS_QUALIFY_CURATED: {curated_diverged} curated module(s) no longer reproduce their \
             sealed emitted source; the content moved, not only the identity"
        );
    }
    if let (Some(row), None) = (audited, &ancestry) {
        bail!(
            "AS_QUALIFY_ANCESTRY: this build is audited as {}, but the sealed native ancestry did \
             not qualify against the dump this run sealed — refusing to report a draft that mixes \
             derived values with that row's sealed ones",
            row.id
        );
    }
    Ok(())
}

/// The human rendering. Everything a person has to compare goes to stdout in the order they compare
/// it: what was sealed, which dump and why, the counts against the previous generation, and only
/// then the row literal.
fn print_qualify_report(document: &QualifyJson) {
    println!("qualifying {}", document.game);
    if let Some(audited) = document.already_audited_as {
        println!("  this build is already audited as {audited}; re-deriving to compare");
    }
    for (name, file) in [
        ("executable    ", &document.inputs.executable),
        ("shipping cache", &document.inputs.shipping_cache),
        ("binds cache   ", &document.inputs.binds_cache),
    ] {
        println!("  {name}  {:>12} bytes  {}", file.byte_len, file.sha256);
    }
    println!(
        "  script cache GUID  {}",
        document.inputs.script_cache_guid
    );
    println!("USMAP");
    match &document.usmap_selection.sealed {
        Some(file) => println!(
            "  sealed    {}\n            {} bytes  {}\n            {}",
            file.path, file.byte_len, file.sha256, document.usmap_selection.reason
        ),
        None => println!("  none sealed: {}", document.usmap_selection.reason),
    }
    for candidate in &document.usmap_selection.examined {
        println!(
            "  examined  {}  {}  {}",
            candidate.path, candidate.sha256, candidate.verdict
        );
        if let Some(absent) = candidate.class_names_absent_from_executable {
            println!(
                "            {absent} of {} class name(s) unnamed by the executable{}; {} of {} \
                 native base(s) resolved",
                candidate.class_rows.unwrap_or(0),
                if candidate.absent_examples.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", candidate.absent_examples.join(", "))
                },
                candidate.native_bases_resolved.unwrap_or(0),
                candidate.native_bases_total
            );
        }
    }
    println!("counts");
    for count in &document.counts {
        let previous = match count.previous {
            Some(previous) => format!("{previous:>8}"),
            None => "       -".to_owned(),
        };
        let delta = match count.delta {
            Some(delta) => format!("{delta:+}"),
            None => "new".to_owned(),
        };
        println!(
            "  {:<48} {previous} -> {:>8}  {delta}{}",
            count.name,
            count.observed,
            if count.fell { "  FELL" } else { "" }
        );
    }
    println!("curated records: {}", document.curated_records.source);
    for module in &document.curated_records.modules {
        println!(
            "  {}  {}  {}",
            if module.reproduces { "ok      " } else { "DIVERGED" },
            module.module,
            match (&module.observed_byte_len, &module.observed_sha256) {
                (Some(len), Some(sha)) => format!("{len} bytes / {sha}"),
                _ => "not emitted by this cache".to_owned(),
            }
        );
    }
    if !document.unavailable.is_empty() {
        println!("not derivable by this command");
        for entry in &document.unavailable {
            println!("  {}: {}", entry.field, entry.reason);
        }
    }
    println!("proposed row");
    for line in document.row.literal.lines() {
        println!("  {line}");
    }
    println!("still to do");
    for step in &document.still_to_do {
        println!("  - {step}");
    }
}

fn qualify_count(
    name: &'static str,
    previous: Option<u64>,
    observed: u64,
    derived_by: &str,
) -> QualifyCountJson {
    let comparison = gore_generation::qualify::CountComparison {
        name,
        previous,
        observed,
        witness: derived_by.to_owned(),
    };
    QualifyCountJson {
        name,
        previous,
        observed,
        delta: comparison.delta(),
        fell: comparison.fell(),
        derived_by: derived_by.to_owned(),
    }
}

#[cfg(test)]
mod default_cli_tests {
    use super::*;

    #[test]
    fn a_dump_two_generations_share_is_stale_for_neither_of_them() {
        // The guard asked the *first* row carrying the seal, and two audited rows deliberately
        // share one USMAP — a build may move the executable and the script cache without moving
        // the reflection layout. So the later of the two could not requalify against the very dump
        // sealed for it: the answer came from its predecessor's row and reported a stale dump.
        let sharing: Vec<_> = gore_generation::rows()
            .iter()
            .filter(|row| {
                gore_generation::rows()
                    .iter()
                    .filter(|other| other.usmap.sha256 == row.usmap.sha256)
                    .count()
                    > 1
            })
            .collect();
        assert!(
            sharing.len() >= 2,
            "this test is only meaningful while some dump is shared; the table has none"
        );

        // Every row that shares a dump must accept it as its own, whichever order the table is in.
        for row in &sharing {
            assert!(
                stale_usmap_row(&row.usmap.sha256, &row.executable).is_none(),
                "{} was told its own sealed dump belongs to another build",
                row.id
            );
        }

        // And a dump really is stale for an executable no row pairs it with.
        let foreign = gore_generation::FileSeal { byte_len: 1, sha256: [0x5a; 32] };
        let flagged = stale_usmap_row(&sharing[0].usmap.sha256, &foreign)
            .expect("an executable no row carries must not pass");
        assert_eq!(flagged.usmap.sha256, sharing[0].usmap.sha256);
    }

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

        let mut hotfix: serde_json::Value = serde_json::from_str(VALID_TAG_MAP).unwrap();
        hotfix["ancestry_profile"] = serde_json::Value::String(
            gore_generation::ROW_G1R_24169431.native_ancestry_profile_id.into(),
        );
        hotfix["map_proof_id"] = serde_json::Value::String(
            gore_generation::ROW_G1R_24169431.gameplay_tag_float32_map_proof_id
                .into(),
        );
        serde_json::from_value::<TagMapSelectorJson>(hotfix.clone())
            .unwrap()
            .into_core()
            .expect("the exact hotfix selector proof pair is supported");

        for (profile, proof) in [
            (
                gore_generation::ROW_G1R_1_0_3.native_ancestry_profile_id,
                gore_generation::ROW_G1R_24169431.gameplay_tag_float32_map_proof_id,
            ),
            (
                gore_generation::ROW_G1R_24169431.native_ancestry_profile_id,
                gore_generation::ROW_G1R_1_0_3.gameplay_tag_float32_map_proof_id,
            ),
        ] {
            let mut crossed = hotfix.clone();
            crossed["ancestry_profile"] = serde_json::Value::String(profile.into());
            crossed["map_proof_id"] = serde_json::Value::String(proof.into());
            assert!(serde_json::from_value::<TagMapSelectorJson>(crossed)
                .unwrap()
                .into_core()
                .is_err());
        }

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
