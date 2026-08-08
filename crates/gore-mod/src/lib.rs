//! gore-mod — assemble one unified mod **bundle** (item overrides + localized text + FMOD audio +
//! voice ZIP edits and more) and deploy/undeploy it to the game.
//!
//! Pipeline: `BuildSpec` → [`build_bundle`] → bundle dir (`gore-mod.json` manifest + payloads)
//! → [`deploy`]/[`undeploy`]. Each content domain is a manifest **component** with its own
//! deploy mechanism (UE4SS Lua = runtime mod; loc + audio + voice ZIPs = loose-file patches against
//! the user's own pristine game files, with `*.gore-bak` backups). The manifest is the
//! hand-off contract for a future stand-alone mod-manager; this crate does single-mod deploy.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use gore_modgen::gen::{gen_lua, MetaConfig, OverridesConfig, SingleOverride};

pub mod dialog;
pub mod mgr;

pub use dialog::DialogTopicSpec;

pub type Files = BTreeMap<String, Vec<u8>>;

// Bundle-side voice inputs are untrusted. Check the file length before allocating and retain a
// bounded-reader check for files that grow after metadata is read. Keep this manifest limit in
// sync with the manager importer; Ogg payloads use gore-vo's public/default processing limit.
const MAX_VOICE_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
/// Hard deployment/build-wide cap for simultaneously resident source Ogg payloads. Rewritten ZIP
/// candidates are streamed to disk, so voice memory remains bounded by this value plus gore-vo's
/// bounded index/ZIP working state rather than growing by another archive-sized `Vec` per language.
const MAX_PENDING_VOICE_OGG_BYTES: u64 = 256 * 1024 * 1024;
const MAX_BUNDLE_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DEPLOY_RECORD_BYTES: u64 = 16 * 1024 * 1024;
const MAX_AUDIO_WAV_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SCRIPT_MINI_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TEXTURE_IMAGE_BYTES: u64 = 256 * 1024 * 1024;
/// A loose game file is opaque bytes we never decode, so the cap only has to keep one bundle
/// payload from exhausting the disk it is staged on. Nothing about the format is assumed.
const MAX_LOOSE_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_PRISTINE_PATCH_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_UE4SS_TREE_ENTRIES: u64 = 250_000;
const MAX_UE4SS_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_UE4SS_TREE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_GAME_EXECUTABLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

// ── Errors ───────────────────────────────────────────────────────────────────
#[derive(Debug, thiserror::Error)]
pub enum ModError {
    #[error("io: {0}")]
    Io(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("loc: {0}")]
    Loc(#[from] gore_loc::loc::LcacheError),
    #[error("fmod: {0}")]
    Fmod(String),
    #[error("voice archive: {0}")]
    Voice(String),
    #[error("{0}")]
    Other(String),
}
type Result<T> = std::result::Result<T, ModError>;

fn io<E: std::fmt::Display>(ctx: &str) -> impl FnOnce(E) -> ModError + '_ {
    move |e| ModError::Io(format!("{ctx}: {e}"))
}

// ── Spec / manifest types ──────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMeta {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
}

/// One audio sample replacement: put `wav_path`'s audio in place of `sample` in `bank`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioReplacement {
    pub bank: String,   // e.g. "SFX.bank"
    pub sample: String, // FSB5 sample name in that bank
    pub wav_path: String,
}

/// One texture replacement: put `image_path` (a PNG) in place of cooked `asset` (in-game path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureReplacement {
    pub asset: String,      // e.g. "/Game/UI/Textures/Common/T_HardwareCursor"
    pub image_path: String, // a PNG on disk
}

/// One LOOSE game-file replacement: put `source_path`'s bytes in place of the file the game reads
/// from disk at `game_path`.
///
/// `game_path` is forward-slash and relative to the game install root (the directory that contains
/// `G1R`), for example `G1R/Content/Slate/Cursors/Normal/Normal.PNG`. This is the escape hatch for
/// content Unreal loads from the filesystem rather than from the IoStore containers or an archive,
/// and it is deliberately REPLACE-only: the target must already exist, so undeploy can put the
/// original back from the `*.gore-bak` snapshot the same way it does for the .lcache and the banks.
/// Only a bounded set of destinations is accepted — see the guide's bundle page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LooseFileReplacement {
    /// Game-root-relative, forward-slash destination, e.g. `G1R/Content/Slate/.../Normal.PNG`.
    pub game_path: String,
    /// Replacement file on disk. Resolved relative to the build spec's own directory.
    pub source_path: String,
}

/// One AngelScript module mod: splice (`op = "add"`) or replace (`op = "edit"`) the compiled
/// 1-module mini-cache at `mini_cache` into the precompiled-script cache at deploy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptModule {
    pub op: String,          // "add" | "edit"
    pub module_name: String, // the Modules TMap key (used for "edit"/replace)
    pub mini_cache: String,  // path to the compiled 1-module mini-cache on disk
}

/// One entry in a bundle's `scripts/manifest.json`: `mini` is a bundle-relative path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntry {
    pub op: String,
    pub module: String,
    pub mini: String,
}

/// One source Ogg edit for a voice ZIP archive in a [`BuildSpec`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceArchiveEdit {
    /// ZIP filename under `G1R/Story/VoiceOver`, for example `german_new.zip`.
    pub archive: String,
    pub op: VoicePatchOp,
    /// Complete member path inside the ZIP. Forward slashes are required.
    pub archive_path: String,
    /// Source Ogg file on disk.
    pub ogg_path: String,
    /// Optional authoring-time identity of the pristine archive and targeted member. When every
    /// edit supplies one, the bundle uses voice manifest format 2 and deploy refuses to apply it
    /// to any other archive snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<VoiceArchiveObservation>,
}

/// One path-free, byte-backed replacement for an existing member of a pristine voice archive.
///
/// This is the hardened input contract for managed authoring stores: the operation is always
/// `replace`, the Ogg payload is already owned by the caller, and an exact archive/member
/// observation is mandatory. [`build_sealed_voice_bundle`] rejects an `Absent` member proof, so
/// this type cannot be used to smuggle an additive archive edit into a managed voice bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedVoiceArchiveReplace {
    /// ZIP filename under `G1R/Story/VoiceOver`, for example `german_new.zip`.
    pub archive: String,
    /// Complete existing member path inside the ZIP. Forward slashes are required.
    pub archive_path: String,
    /// Complete replacement Ogg payload.
    pub ogg: Vec<u8>,
    /// Exact authoring-time identity of the pristine archive and existing member.
    pub observation: VoiceArchiveObservation,
}

/// Authoring-time identity of one pristine voice archive and the member an edit targets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceArchiveObservation {
    pub archive_size: u64,
    /// Lowercase, 64-character hexadecimal SHA-256 of the complete pristine ZIP.
    pub archive_sha256: String,
    pub member_proof: VoiceMemberProof,
}

/// Exact authoring-time state of the member targeted by a voice edit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceMemberProof {
    Present {
        /// The source member must be non-empty and match this exact uncompressed length.
        uncompressed_size: u64,
        crc32: u32,
    },
    Absent,
}

/// Supported voice archive operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoicePatchOp {
    Add,
    Replace,
}

/// Exact identity of one replacement Ogg payload embedded in a sealed Voice bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoicePayloadSeal {
    pub byte_len: u64,
    /// Lowercase, 64-character hexadecimal SHA-256 of the complete Ogg payload.
    pub sha256: String,
}

/// Exact content identity of the game executable a managed Voice bundle was authored against.
///
/// Format-3 Voice manifests carry this seal once at manifest level. Deployment reopens the fixed
/// installed executable with no-follow semantics and checks the complete bounded content both
/// while preparing the Voice rewrite and immediately before the first game mutation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceExecutableGenerationSeal {
    pub byte_len: u64,
    /// Lowercase, 64-character hexadecimal SHA-256 of the complete game executable.
    pub sha256: String,
}

/// Canonical content identity of every regular file in one exact managed Voice bundle tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceBundleTreeSeal {
    pub byte_len: u64,
    /// Lowercase, 64-character hexadecimal SHA-256 using the
    /// `gore-mod.voice-bundle-tree.v1` canonical tree algorithm.
    pub sha256: String,
}

/// Read-only selection of the authenticated pristine archive deployment itself would rebuild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceArchiveInspectionSource {
    pub path: PathBuf,
    /// `true` when the installed live archive drifted from the active deployment and is therefore
    /// the new post-hotfix pristine source rather than the older authenticated backup.
    pub drifted: bool,
}

/// Retained, component-by-component no-follow binding of the fixed
/// install/G1R/Story/VoiceOver directory.
///
/// The initial traversal retains no-delete anchors until every component is proven. The returned
/// guard then keeps a no-follow identity handle with rename-compatible sharing so legitimate
/// atomic backup/deploy writes below VoiceOver remain possible. Consumers rebind and compare that
/// identity at authority boundaries.
#[derive(Debug, Clone)]
pub struct VoiceOverPathGuard {
    install_root: PathBuf,
    directory: std::sync::Arc<mgr::model::RenameDirectoryGuard>,
}

impl VoiceOverPathGuard {
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn same_identity(&self, other: &Self) -> bool {
        self.directory.identity() == other.directory.identity()
            && self.install_root == other.install_root
    }

    /// Resolve one archive using the same authenticated pristine-source rules as deployment while
    /// retaining the no-follow VoiceOver directory binding for the caller's subsequent inspection.
    pub fn resolve_pristine_archive(&self, archive: &str) -> Result<VoiceArchiveInspectionSource> {
        resolve_pristine_voice_archive_with_guard(self, archive)
    }
}

/// One entry in a bundle's versioned `voice/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoicePatchEntry {
    pub archive: String,
    pub op: VoicePatchOp,
    pub archive_path: String,
    /// Bundle-root-relative Ogg payload path.
    pub ogg: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<VoiceArchiveObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_seal: Option<VoicePayloadSeal>,
}

/// Stable on-disk contract for a voice archive patch component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoicePatchManifest {
    pub format: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_generation: Option<VoiceExecutableGenerationSeal>,
    pub edits: Vec<VoicePatchEntry>,
}

/// Declarative build input — the union of the editor domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSpec {
    pub meta: ModMeta,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default)]
    pub overrides: Vec<SingleOverride>,
    /// `{ locId: { setName: text } }`
    #[serde(default)]
    pub loc_edits: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub audio: Vec<AudioReplacement>,
    #[serde(default)]
    pub texture: Vec<TextureReplacement>,
    /// Loose game files to replace on disk — reaches content living outside the IoStore containers
    /// and the voice/FMOD archives, but only where no mounted pak already carries the same path.
    #[serde(default)]
    pub files: Vec<LooseFileReplacement>,
    /// The same destinations, claimed from an ADDITIVE `~mods` pak instead of overwritten on disk.
    /// Which mechanism a bundle uses is declared here and never inferred from the install: a
    /// bundle that switched routes per machine would owe a different undeploy on every machine.
    #[serde(default)]
    pub pak_files: Vec<LooseFileReplacement>,
    #[serde(default)]
    pub scripts: Vec<ScriptModule>,
    /// Authored AngelScript topics to register at the target conversation's natural UI boundary.
    /// This delivery mechanism does not certify selection-side save or knowledge behavior.
    #[serde(default)]
    pub dialog_topics: Vec<DialogTopicSpec>,
    #[serde(default)]
    pub voice: Vec<VoiceArchiveEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    /// A UE4SS Lua mod folder at `path`, deployed to `ue4ss/Mods/<name>`.
    Ue4ssLua {
        name: String,
        path: String,
        /// The `Class.Field` CDO targets this mod overrides (sorted, deduped) — the
        /// mod-manager's conflict-detection contract. `#[serde(default)]` keeps old
        /// manifests parseable; omitted from the JSON when empty to keep byte-noise low.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targets: Vec<String>,
        /// `true` when `targets` is only the known, precise subset of this script's footprint.
        /// Missing values default to `false` so format-1 manifests remain parseable. New manifests
        /// always serialize the value: an explicit `false` with no known targets must remain
        /// distinguishable from a legacy targetless script, which the manager treats as opaque.
        #[serde(default)]
        opaque: bool,
    },
    /// Declarative loc edits at `path` (`{id:{set:text}}`), applied to the .lcache.
    LocPatch { path: String },
    /// Audio patch dir at `path` (manifest.json + wavs), applied to `banks`.
    AudioPatch { path: String, banks: Vec<String> },
    /// Texture patch dir at `path` (manifest.json + pngs); deploy cooks + packs a Zen triplet
    /// into `~mods` for `assets`. Additive — no in-place game-file patch, no `*.gore-bak`.
    TexturePatch { path: String, assets: Vec<String> },
    /// Loose-file payloads at `path` (manifest.json + opaque payload files); deploy replaces each
    /// game-root-relative destination in `targets` in place, with a `*.gore-bak` backup.
    /// Replace-only: every target must already exist in the install.
    FilePatch { path: String, targets: Vec<String> },
    /// Loose-file payloads at `path` (manifest.json + opaque payload files); deploy packs them into
    /// ONE additive `~mods` pak that claims each game-root-relative destination in `targets` from
    /// the pak filesystem. Additive — no in-place write and no `*.gore-bak`; undeploy deletes the
    /// pak. Unlike [`Component::FilePatch`] the destinations need not exist on disk.
    PakFilePatch { path: String, targets: Vec<String> },
    /// AngelScript mini-caches at `path` (manifest.json + `*.cache`); deploy splices/replaces
    /// them into `PrecompiledScript_Shipping.Cache` in place, with a `*.gore-bak` backup.
    AngelScriptPatch { path: String },
    /// Versioned manifest + Ogg payloads; deploy rewrites ZIP archives under
    /// `G1R/Story/VoiceOver` transactionally.
    VoiceArchivePatch { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub format: u32,
    #[serde(rename = "mod")]
    pub mod_meta: ModMeta,
    pub components: Vec<Component>,
}

const MOD_MANIFEST_FORMAT_BASE: u32 = 1;
const MOD_MANIFEST_FORMAT_PAK_FILE_PATCH: u32 = 2;

fn component_manifest_format(component: &Component) -> u32 {
    match component {
        Component::Ue4ssLua { .. }
        | Component::LocPatch { .. }
        | Component::AudioPatch { .. }
        | Component::TexturePatch { .. }
        | Component::FilePatch { .. }
        | Component::AngelScriptPatch { .. }
        | Component::VoiceArchivePatch { .. } => MOD_MANIFEST_FORMAT_BASE,
        Component::PakFilePatch { .. } => MOD_MANIFEST_FORMAT_PAK_FILE_PATCH,
    }
}

fn mod_manifest_format_for_components(components: &[Component]) -> u32 {
    components
        .iter()
        .map(component_manifest_format)
        .max()
        .unwrap_or(MOD_MANIFEST_FORMAT_BASE)
}

/// Validate the root bundle manifest's minimum-consumer contract.
///
/// Format 1 is the base component set and deliberately excludes [`Component::PakFilePatch`].
/// Format 2 adds that component and therefore requires at least one such declaration. The exact
/// correspondence is intentional: consumers must not silently interpret a newer component under
/// an older format number, and a gratuitous format bump must not exclude otherwise-capable format-1
/// consumers. There is no migration or fallback path for unknown or mismatched manifests.
pub(crate) fn validate_mod_manifest_format(manifest: &ModManifest) -> Result<()> {
    let required_format = mod_manifest_format_for_components(&manifest.components);
    match (manifest.format, required_format) {
        (actual, required) if actual == required => Ok(()),
        (MOD_MANIFEST_FORMAT_BASE, MOD_MANIFEST_FORMAT_PAK_FILE_PATCH) => Err(ModError::Other(
            "gore-mod manifest format 1 does not support pak_file_patch components".into(),
        )),
        (MOD_MANIFEST_FORMAT_PAK_FILE_PATCH, MOD_MANIFEST_FORMAT_BASE) => Err(ModError::Other(
            "gore-mod manifest format 2 requires at least one pak_file_patch component".into(),
        )),
        (format, _) => Err(ModError::Other(format!(
            "unsupported gore-mod manifest format {format} (want 1 or 2)"
        ))),
    }
}

pub struct Bundle {
    pub files: Files,
    pub manifest: ModManifest,
}

// Hand-written because a bundle carries whole WAVs and Oggs: the derive would dump every payload
// byte into the panic message of a test that expected a build to fail and got one that succeeded.
// The name and the staged paths are what identifies a bundle; the bytes never are.
impl std::fmt::Debug for Bundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bundle")
            .field("mod", &self.manifest.mod_meta.name)
            .field("components", &self.manifest.components.len())
            .field("files", &self.files.keys().collect::<Vec<_>>())
            .finish()
    }
}

enum VoiceBuildPayload {
    File(String),
    Bytes(Vec<u8>),
}

struct VoiceBuildInput {
    archive: String,
    op: VoicePatchOp,
    archive_path: String,
    payload: VoiceBuildPayload,
    observation: Option<VoiceArchiveObservation>,
}

fn voice_payload_byte_limit(retained: u64, per_ogg_limit: u64) -> Result<u64> {
    MAX_PENDING_VOICE_OGG_BYTES
        .checked_sub(retained)
        .map(|remaining| remaining.min(per_ogg_limit))
        .ok_or_else(|| ModError::Other("voice Ogg memory budget underflow".into()))
}

/// Lower voice inputs into the one stable bundle component shared by the compatibility
/// [`BuildSpec`] path and the hardened byte-backed path. Payload bytes are moved directly into the
/// bundle, keeping the aggregate resident-byte accounting identical for file and in-memory input.
fn lower_voice_component(
    files: &mut Files,
    components: &mut Vec<Component>,
    inputs: Vec<VoiceBuildInput>,
    executable_generation: Option<VoiceExecutableGenerationSeal>,
) -> Result<()> {
    if inputs.is_empty() {
        return Err(ModError::Other(
            "voice bundle requires at least one replacement".into(),
        ));
    }
    let observed = inputs[0].observation.is_some();
    if inputs
        .iter()
        .any(|edit| edit.observation.is_some() != observed)
    {
        return Err(ModError::Other(
            "voice build mixes sealed and legacy edits; every edit must either include an observation or omit it".into(),
        ));
    }
    if executable_generation.is_some() && !observed {
        return Err(ModError::Other(
            "managed voice build requires an archive observation on every edit".into(),
        ));
    }
    let format = if executable_generation.is_some() {
        3
    } else if observed {
        2
    } else {
        1
    };

    let voice_limits = gore_vo::Limits::default();
    let mut edits = Vec::with_capacity(inputs.len());
    let mut retained_ogg_bytes = 0u64;
    for (i, edit) in inputs.into_iter().enumerate() {
        validate_voice_target(&edit.archive, &edit.archive_path)?;
        validate_voice_edit_observation(edit.op, edit.observation.as_ref())?;
        let max_bytes =
            voice_payload_byte_limit(retained_ogg_bytes, voice_limits.max_ogg_bytes as u64)?;
        let (ogg, source_label) = match edit.payload {
            VoiceBuildPayload::File(path) => {
                let ogg = read_regular_file_limited(Path::new(&path), "voice Ogg", max_bytes)?;
                (ogg, path)
            }
            VoiceBuildPayload::Bytes(ogg) => {
                if ogg.len() as u64 > max_bytes {
                    return Err(ModError::Other(format!(
                        "voice Ogg bytes for {:?} member {:?} exceed the {max_bytes}-byte limit: {} bytes",
                        edit.archive,
                        edit.archive_path,
                        ogg.len()
                    )));
                }
                let label = format!("{}:{}", edit.archive, edit.archive_path);
                (ogg, label)
            }
        };
        retained_ogg_bytes = retained_ogg_bytes
            .checked_add(ogg.len() as u64)
            .ok_or_else(|| ModError::Other("voice Ogg memory budget overflow".into()))?;
        gore_vo::validate_ogg(&ogg, &voice_limits)
            .map_err(|e| ModError::Voice(format!("{source_label}: {e}")))?;
        // Formats 1 and 2 are committed compatibility contracts. In particular format 2 carries
        // archive observations exactly as before; payload seals belong exclusively to format 3.
        let payload_seal = (format == 3).then(|| voice_payload_seal(&ogg));
        let payload = format!("voice/payload/{i}.ogg");
        files.insert(payload.clone(), ogg);
        edits.push(VoicePatchEntry {
            archive: edit.archive,
            op: edit.op,
            archive_path: edit.archive_path,
            ogg: payload,
            observation: edit.observation,
            payload_seal,
        });
    }
    let manifest = VoicePatchManifest {
        format,
        executable_generation,
        edits,
    };
    validate_voice_manifest(&manifest)?;
    files.insert(
        "voice/manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    components.push(Component::VoiceArchivePatch {
        path: "voice".into(),
    });
    Ok(())
}

/// Build a voice-only, format-3 bundle from owned Ogg bytes without accepting caller-controlled
/// source file paths.
///
/// Every edit is structurally a sealed replacement. Archive/member safety, exact `Present`
/// observations, per-Ogg limits, the aggregate voice memory budget, and Ogg validity are checked
/// before a [`Bundle`] is returned. This function only assembles an in-memory bundle; it performs
/// no deployment or game writes.
pub fn build_sealed_voice_bundle(
    meta: ModMeta,
    executable_generation: VoiceExecutableGenerationSeal,
    edits: Vec<SealedVoiceArchiveReplace>,
) -> Result<Bundle> {
    if !is_safe_mod_name(&meta.name) {
        return Err(ModError::Other(format!(
            "invalid mod name {:?}: must be a single path component with no separators, '..', or control characters",
            meta.name
        )));
    }
    let inputs = edits
        .into_iter()
        .map(|edit| VoiceBuildInput {
            archive: edit.archive,
            op: VoicePatchOp::Replace,
            archive_path: edit.archive_path,
            payload: VoiceBuildPayload::Bytes(edit.ogg),
            observation: Some(edit.observation),
        })
        .collect();
    let mut files = Files::new();
    let mut components = Vec::new();
    lower_voice_component(
        &mut files,
        &mut components,
        inputs,
        Some(executable_generation),
    )?;
    let manifest = ModManifest {
        format: mod_manifest_format_for_components(&components),
        mod_meta: meta,
        components,
    };
    validate_mod_manifest_format(&manifest)?;
    files.insert(
        "gore-mod.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    Ok(Bundle { files, manifest })
}

// ── Build ──────────────────────────────────────────────────────────────────────
/// Resolve one authored asset path against `base`: an absolute path is used as written, a relative
/// one is joined onto the base. Same rule as `gore audio replace --map`, which resolves its WAVs
/// against the map file's directory.
fn resolve_spec_path(base: &Path, rel: &str) -> PathBuf {
    let path = Path::new(rel);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Assemble the in-memory bundle (files + manifest) from a declarative spec, resolving every
/// relative asset path against the process's working directory.
///
/// This is the compatibility entry point for callers that have no spec FILE — the GUI builds its
/// spec in memory and fills every path from a file picker, so it is always absolute. A caller that
/// read the spec from disk should use [`build_bundle_relative_to`] with the spec's own directory:
/// a path written next to the spec must mean the file next to the spec, whatever directory the
/// process happens to be in.
pub fn build_bundle(spec: &BuildSpec) -> Result<Bundle> {
    build_bundle_relative_to(spec, Path::new("."))
}

/// Assemble the in-memory bundle (files + manifest) from a declarative spec, resolving every
/// relative asset path (`wav_path`, `ogg_path`, `image_path`, `mini_cache`, `source_path`) against
/// `base` — the directory the spec itself came from. Absolute paths are used as written.
pub fn build_bundle_relative_to(spec: &BuildSpec, base: &Path) -> Result<Bundle> {
    let mut files = Files::new();
    let mut components = Vec::new();
    let name = &spec.meta.name;
    if !is_safe_mod_name(name) {
        return Err(ModError::Other(format!(
            "invalid mod name {name:?}: must be a single path component with no \
             separators, '..', or control characters"
        )));
    }

    // overrides → UE4SS Lua mod
    // Runtime UE4SS content is emitted as exactly ONE component. Dialog topic registration shares
    // that component with generated CDO overrides; emitting two roots would otherwise reintroduce
    // ambiguous last-wins deployment behavior.
    let ue4ss_opaque = !spec.dialog_topics.is_empty();
    let dialog_runtime = if spec.dialog_topics.is_empty() {
        None
    } else {
        Some(
            dialog::render_dialog_runtime(name, &spec.dialog_topics)
                .map_err(|error| ModError::Other(format!("invalid dialog topics: {error}")))?,
        )
    };
    let mut ue4ss_lua = None;
    let mut ue4ss_targets = Vec::new();

    if !spec.overrides.is_empty() {
        let cfg = OverridesConfig {
            meta: MetaConfig {
                name: name.clone(),
                delay_ms: spec.delay_ms,
            },
            overrides: spec.overrides.clone(),
        };
        ue4ss_lua = Some(gen_lua(&cfg));
        // The `Class.Field` CDO targets this mod sets, for the manager's conflict detection.
        ue4ss_targets = spec
            .overrides
            .iter()
            .map(|o| format!("{}.{}", o.class, o.field))
            .collect();
        ue4ss_targets.sort();
        ue4ss_targets.dedup();
    }

    if let Some(runtime) = dialog_runtime {
        let lua = ue4ss_lua.get_or_insert_with(String::new);
        if !lua.is_empty() && !lua.ends_with('\n') {
            lua.push('\n');
        }
        lua.push_str(&runtime);
        // Dialog registration also mutates transient topic sets. The component is marked opaque
        // below, while its exact generated CDO-override targets remain useful partial metadata.
    }

    if let Some(lua) = ue4ss_lua {
        files.insert(format!("ue4ss/{name}/enabled.txt"), Vec::new());
        files.insert(format!("ue4ss/{name}/Scripts/main.lua"), lua.into_bytes());
        components.push(Component::Ue4ssLua {
            name: name.clone(),
            path: format!("ue4ss/{name}"),
            targets: ue4ss_targets,
            opaque: ue4ss_opaque,
        });
    }

    // loc edits → declarative patch
    if !spec.loc_edits.is_empty() {
        files.insert(
            "loc/edits.json".into(),
            serde_json::to_vec_pretty(&spec.loc_edits)?,
        );
        components.push(Component::LocPatch {
            path: "loc/edits.json".into(),
        });
    }

    // audio → manifest + wavs (no game audio, just the replacements)
    if !spec.audio.is_empty() {
        let mut map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (i, a) in spec.audio.iter().enumerate() {
            validate_bank_name(&a.bank)?;
            let source = resolve_spec_path(base, &a.wav_path);
            let wav = std::fs::read(&source)
                .map_err(io(&format!("reading audio[{i}] wav {}", source.display())))?;
            // Prefix with the index so distinct samples that sanitize to the same name can't
            // collide and overwrite each other.
            let fname = format!("{i}_{}__{}.wav", sanitize(&a.bank), sanitize(&a.sample));
            files.insert(format!("audio/{fname}"), wav);
            map.entry(a.bank.clone())
                .or_default()
                .insert(a.sample.clone(), format!("audio/{fname}"));
        }
        let banks: Vec<String> = map.keys().cloned().collect();
        files.insert(
            "audio/manifest.json".into(),
            serde_json::to_vec_pretty(&map)?,
        );
        components.push(Component::AudioPatch {
            path: "audio".into(),
            banks,
        });
    }

    // textures → manifest + pngs (source images; cooked+packed at deploy)
    // Voice ZIP edits -> versioned manifest + validated Ogg payloads. The manifest keeps input
    // order; deploy/manager composition applies case-insensitive later-wins before asking gore-vo
    // for one verified rewrite per target archive.
    if !spec.voice.is_empty() {
        let inputs = spec
            .voice
            .iter()
            .map(|edit| VoiceBuildInput {
                archive: edit.archive.clone(),
                op: edit.op,
                archive_path: edit.archive_path.clone(),
                payload: VoiceBuildPayload::File(
                    resolve_spec_path(base, &edit.ogg_path)
                        .display()
                        .to_string(),
                ),
                observation: edit.observation.clone(),
            })
            .collect();
        lower_voice_component(&mut files, &mut components, inputs, None)?;
    }

    if !spec.texture.is_empty() {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        for (i, t) in spec.texture.iter().enumerate() {
            let source = resolve_spec_path(base, &t.image_path);
            let png = std::fs::read(&source).map_err(io(&format!(
                "reading texture[{i}] png {}",
                source.display()
            )))?;
            let fname = format!("{i}_{}.png", sanitize(&t.asset));
            files.insert(format!("texture/{fname}"), png);
            map.insert(t.asset.clone(), format!("texture/{fname}"));
        }
        let assets: Vec<String> = map.keys().cloned().collect();
        files.insert(
            "texture/manifest.json".into(),
            serde_json::to_vec_pretty(&map)?,
        );
        components.push(Component::TexturePatch {
            path: "texture".into(),
            assets,
        });
    }

    // scripts → manifest + compiled mini-caches (spliced/replaced at deploy)
    if !spec.scripts.is_empty() {
        let mut entries: Vec<ScriptEntry> = Vec::new();
        for (i, s) in spec.scripts.iter().enumerate() {
            if s.op != "add" && s.op != "edit" {
                return Err(ModError::Other(format!(
                    "invalid script op {:?} for module {:?} (want \"add\" or \"edit\")",
                    s.op, s.module_name
                )));
            }
            let source = resolve_spec_path(base, &s.mini_cache);
            let mini = std::fs::read(&source).map_err(io(&format!(
                "reading scripts[{i}] mini-cache {}",
                source.display()
            )))?;
            let mini_rel = format!("scripts/{i}_{}.cache", sanitize(&s.module_name));
            files.insert(mini_rel.clone(), mini);
            entries.push(ScriptEntry {
                op: s.op.clone(),
                module: s.module_name.clone(),
                mini: mini_rel,
            });
        }
        files.insert(
            "scripts/manifest.json".into(),
            serde_json::to_vec_pretty(&entries)?,
        );
        components.push(Component::AngelScriptPatch {
            path: "scripts".into(),
        });
    }

    // loose files → manifest + opaque payloads (replaced in place at deploy, with a backup)
    if !spec.files.is_empty() {
        let map = lower_loose_section(base, "files", &spec.files, &mut files)?;
        components.push(Component::FilePatch {
            path: "files".into(),
            targets: map.into_keys().collect(),
        });
    }

    // pak files → the same manifest + payloads, packed into an additive `~mods` pak at deploy
    if !spec.pak_files.is_empty() {
        let map = lower_loose_section(base, "pak_files", &spec.pak_files, &mut files)?;
        components.push(Component::PakFilePatch {
            path: "pak_files".into(),
            targets: map.into_keys().collect(),
        });
    }

    let manifest = ModManifest {
        format: mod_manifest_format_for_components(&components),
        mod_meta: spec.meta.clone(),
        components,
    };
    validate_mod_manifest_format(&manifest)?;
    files.insert(
        "gore-mod.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    Ok(Bundle { files, manifest })
}

/// Stage one loose-file section's payloads into the bundle under `section/` and return its
/// `{ game_path: bundle-relative payload }` manifest map.
///
/// `files` and `pak_files` name destinations in exactly the same spelling and differ only in the
/// mechanism deploy uses to reach them, so the authoring questions — is this destination allowed,
/// and was it named twice — are asked here once. Build stays game-free: whether the destination is
/// reachable in place on THIS install is a deploy-time question, not a bundle property.
fn lower_loose_section(
    base: &Path,
    section: &str,
    entries: &[LooseFileReplacement],
    files: &mut Files,
) -> Result<BTreeMap<String, String>> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (i, f) in entries.iter().enumerate() {
        validate_loose_game_path(&f.game_path)?;
        // Windows path identity is case-insensitive, so two spellings of one destination are
        // one destination. Silently letting the later entry win would leave the loser's bytes
        // in the bundle with nothing pointing at them; say so instead.
        if !seen.insert(f.game_path.to_ascii_lowercase()) {
            return Err(ModError::Other(format!(
                "duplicate {section} entry for game path {:?}: one bundle must not name the \
                 same loose file twice in one section",
                f.game_path
            )));
        }
        let source = resolve_spec_path(base, &f.source_path);
        let bytes = read_regular_file_limited(
            &source,
            &format!("{section}[{i}] source"),
            MAX_LOOSE_FILE_BYTES,
        )?;
        // Prefix with the index so distinct destinations that sanitize to the same name can't
        // collide and overwrite each other.
        let fname = format!("{i}_{}", sanitize(&f.game_path));
        files.insert(format!("{section}/{fname}"), bytes);
        map.insert(f.game_path.clone(), format!("{section}/{fname}"));
    }
    files.insert(
        format!("{section}/manifest.json"),
        serde_json::to_vec_pretty(&map)?,
    );
    Ok(map)
}

/// Write a built bundle's files under `dir` (creating parent dirs).
pub fn write_bundle(dir: &Path, bundle: &Bundle) -> Result<()> {
    // Validate the typed manifest and the exact bytes that will be written before clearing an
    // existing output tree. `Bundle` fields are public, so callers can otherwise forge a mismatch
    // after build and turn an invalid contract into a destructive rebuild side effect.
    validate_mod_manifest_format(&bundle.manifest)?;
    let manifest_bytes = bundle
        .files
        .get("gore-mod.json")
        .ok_or_else(|| ModError::Other("bundle is missing gore-mod.json".into()))?;
    let serialized_manifest: ModManifest = serde_json::from_slice(manifest_bytes)?;
    validate_mod_manifest_format(&serialized_manifest)?;
    if manifest_bytes != &serde_json::to_vec_pretty(&bundle.manifest)? {
        return Err(ModError::Other(
            "bundle's gore-mod.json disagrees with its typed manifest".into(),
        ));
    }
    // Rebuild into a clean directory: a prior build of the same bundle may have left files that are
    // no longer part of it (e.g. a component the user removed). Deploy copies some component dirs
    // (like `ue4ss/<name>`) wholesale, so stale leftovers would still be shipped/deployed. Clear the
    // destination first so it holds exactly this bundle.
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .map_err(io(&format!("clearing bundle dir {}", dir.display())))?;
    }
    for (rel, bytes) in &bundle.files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io("create dir"))?;
        }
        std::fs::write(&path, bytes).map_err(io(&format!("writing {}", path.display())))?;
    }
    Ok(())
}

fn validate_sealed_voice_contract(
    manifest: &ModManifest,
    voice_manifest: &VoicePatchManifest,
) -> Result<BTreeSet<String>> {
    validate_mod_manifest_format(manifest)?;
    if !is_safe_mod_name(&manifest.mod_meta.name) {
        return Err(ModError::Other(format!(
            "invalid mod name {:?} in sealed voice bundle",
            manifest.mod_meta.name
        )));
    }
    if !matches!(
        manifest.components.as_slice(),
        [Component::VoiceArchivePatch { path }] if path == "voice"
    ) {
        return Err(ModError::Other(
            "sealed voice bundle must contain exactly one voice component at path \"voice\"".into(),
        ));
    }
    validate_voice_manifest(voice_manifest)?;
    if voice_manifest.format != 3 {
        return Err(ModError::Other(
            "sealed voice bundle requires voice manifest format 3".into(),
        ));
    }

    let mut expected_files = BTreeSet::from([
        "gore-mod.json".to_string(),
        "voice/manifest.json".to_string(),
    ]);
    for (index, edit) in voice_manifest.edits.iter().enumerate() {
        if edit.op != VoicePatchOp::Replace
            || !matches!(
                edit.observation.as_ref().map(|value| &value.member_proof),
                Some(VoiceMemberProof::Present { .. })
            )
        {
            return Err(ModError::Other(format!(
                "sealed voice bundle edit {index} must replace an observed existing member"
            )));
        }
        let expected_payload = format!("voice/payload/{index}.ogg");
        if edit.ogg != expected_payload {
            return Err(ModError::Other(format!(
                "sealed voice bundle edit {index} has non-canonical payload path {:?} (want {expected_payload:?})",
                edit.ogg
            )));
        }
        if edit.payload_seal.is_none() {
            return Err(ModError::Other(format!(
                "sealed voice bundle edit {index} lacks a replacement payload seal"
            )));
        }
        expected_files.insert(expected_payload);
    }
    Ok(expected_files)
}

fn validate_sealed_voice_bundle_memory(bundle: &Bundle) -> Result<()> {
    let manifest_bytes = bundle
        .files
        .get("gore-mod.json")
        .ok_or_else(|| ModError::Other("sealed voice bundle is missing gore-mod.json".into()))?;
    if manifest_bytes != &serde_json::to_vec_pretty(&bundle.manifest)? {
        return Err(ModError::Other(
            "sealed voice bundle's gore-mod.json disagrees with its typed manifest".into(),
        ));
    }
    let voice_manifest_bytes = bundle.files.get("voice/manifest.json").ok_or_else(|| {
        ModError::Other("sealed voice bundle is missing voice/manifest.json".into())
    })?;
    let voice_manifest: VoicePatchManifest = serde_json::from_slice(voice_manifest_bytes)?;
    if voice_manifest_bytes != &serde_json::to_vec_pretty(&voice_manifest)? {
        return Err(ModError::Other(
            "sealed voice bundle has a non-canonical voice manifest encoding".into(),
        ));
    }
    let expected_files = validate_sealed_voice_contract(&bundle.manifest, &voice_manifest)?;
    let actual_files: BTreeSet<_> = bundle.files.keys().cloned().collect();
    if actual_files != expected_files {
        let missing: Vec<_> = expected_files.difference(&actual_files).cloned().collect();
        let extra: Vec<_> = actual_files.difference(&expected_files).cloned().collect();
        return Err(ModError::Other(format!(
            "sealed voice bundle file layout mismatch (missing: {missing:?}, extra: {extra:?})"
        )));
    }

    let voice_limits = gore_vo::Limits::default();
    let mut retained_ogg_bytes = 0u64;
    for (index, edit) in voice_manifest.edits.iter().enumerate() {
        let payload = format!("voice/payload/{index}.ogg");
        let ogg = bundle
            .files
            .get(&payload)
            .expect("expected file set was checked");
        let max_bytes =
            voice_payload_byte_limit(retained_ogg_bytes, voice_limits.max_ogg_bytes as u64)?;
        if ogg.len() as u64 > max_bytes {
            return Err(ModError::Other(format!(
                "voice Ogg payload {payload:?} exceeds the {max_bytes}-byte limit: {} bytes",
                ogg.len()
            )));
        }
        retained_ogg_bytes = retained_ogg_bytes
            .checked_add(ogg.len() as u64)
            .ok_or_else(|| ModError::Other("voice Ogg memory budget overflow".into()))?;
        gore_vo::validate_ogg(ogg, &voice_limits)
            .map_err(|e| ModError::Voice(format!("{payload}: {e}")))?;
        require_voice_payload_seal(edit, ogg)?;
    }
    Ok(())
}

fn open_voice_bundle_parent(parent: &Path) -> Result<(PathBuf, mgr::model::SecureDirectory)> {
    if parent
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ModError::Other(format!(
            "voice bundle parent must not contain '..': {}",
            parent.display()
        )));
    }
    let absolute = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(io("reading current directory for voice bundle output"))?
            .join(parent)
    };
    let anchor =
        mgr::model::open_directory_chain_nofollow(&absolute, "voice bundle output parent")?;
    let canonical = anchor.path().to_path_buf();
    Ok((canonical, anchor))
}

#[derive(Debug, Clone, Copy)]
enum VoiceBundleOwnedFileParent {
    Target,
    Voice,
    Payload,
}

#[derive(Debug)]
struct VoiceBundleOwnedFile {
    parent: VoiceBundleOwnedFileParent,
    name: std::ffi::OsString,
    identity: mgr::model::FileIdentity,
}

#[derive(Debug)]
struct VoiceBundleTreeOwnership {
    parent: Option<mgr::model::SecureDirectory>,
    target_name: std::ffi::OsString,
    target_dir: PathBuf,
    target: Option<mgr::model::SecureDirectory>,
    voice: Option<mgr::model::SecureDirectory>,
    payload: Option<mgr::model::SecureDirectory>,
    files: Vec<VoiceBundleOwnedFile>,
    armed: bool,
}

#[derive(Debug)]
struct VoiceBundleCleanupOutcome {
    confirmed: bool,
    errors: Vec<String>,
}

impl VoiceBundleTreeOwnership {
    fn parent(&self) -> &mgr::model::SecureDirectory {
        self.parent
            .as_ref()
            .expect("owned Voice bundle retains its parent anchor")
    }

    fn target_identity(&self) -> mgr::model::FileIdentity {
        self.target
            .as_ref()
            .expect("owned Voice bundle retains its target anchor")
            .identity()
    }

    fn voice_identity(&self) -> mgr::model::FileIdentity {
        self.voice
            .as_ref()
            .expect("owned Voice bundle retains its voice anchor")
            .identity()
    }

    fn payload_identity(&self) -> mgr::model::FileIdentity {
        self.payload
            .as_ref()
            .expect("owned Voice bundle retains its payload anchor")
            .identity()
    }

    fn directory(
        &self,
        parent: VoiceBundleOwnedFileParent,
    ) -> Option<&mgr::model::SecureDirectory> {
        match parent {
            VoiceBundleOwnedFileParent::Target => self.target.as_ref(),
            VoiceBundleOwnedFileParent::Voice => self.voice.as_ref(),
            VoiceBundleOwnedFileParent::Payload => self.payload.as_ref(),
        }
    }

    fn verify_complete_binding(&self) -> Result<()> {
        let target_identity = self.target_identity();
        let voice_identity = self.voice_identity();
        let payload_identity = self.payload_identity();
        let target = expect_secure_directory(
            self.parent()
                .open_child(&self.target_name, "owned Voice bundle root")?,
            "owned Voice bundle root",
        )?;
        let voice = expect_secure_directory(
            target.open_child(
                std::ffi::OsStr::new("voice"),
                "owned Voice component directory",
            )?,
            "owned Voice component directory",
        )?;
        let payload = expect_secure_directory(
            voice.open_child(
                std::ffi::OsStr::new("payload"),
                "owned Voice payload directory",
            )?,
            "owned Voice payload directory",
        )?;
        if target.identity() != target_identity
            || voice.identity() != voice_identity
            || payload.identity() != payload_identity
        {
            return Err(ModError::Other(
                "owned Voice bundle directory identity changed".into(),
            ));
        }
        Ok(())
    }

    fn cleanup_owned(&mut self) -> VoiceBundleCleanupOutcome {
        if !self.armed {
            return VoiceBundleCleanupOutcome {
                confirmed: true,
                errors: Vec::new(),
            };
        }
        if self.parent.is_none() {
            self.armed = false;
            self.files.clear();
            self.payload.take();
            self.voice.take();
            self.target.take();
            return VoiceBundleCleanupOutcome {
                confirmed: false,
                errors: vec![
                    "owned Voice bundle parent could not be rebound; cleanup refused".into(),
                ],
            };
        }
        let target_identity = self.target.as_ref().map(|value| value.identity());
        let voice_identity = self.voice.as_ref().map(|value| value.identity());
        let payload_identity = self.payload.as_ref().map(|value| value.identity());
        let mut errors = Vec::new();
        let rebound = self
            .parent()
            .open_child(&self.target_name, "owned Voice bundle root")
            .and_then(|node| expect_secure_directory(node, "owned Voice bundle root"));
        if !matches!(
            (&rebound, target_identity),
            (Ok(directory), Some(expected)) if directory.identity() == expected
        ) {
            errors.push(match rebound {
                Ok(_) => "owned Voice bundle root was replaced; cleanup refused".into(),
                Err(error) => format!(
                    "owned Voice bundle root could not be rebound; cleanup refused: {error}"
                ),
            });
            self.armed = false;
            self.files.clear();
            self.payload.take();
            self.voice.take();
            self.target.take();
            return VoiceBundleCleanupOutcome {
                confirmed: false,
                errors,
            };
        }
        drop(rebound);

        for file in self.files.iter().rev() {
            let Some(directory) = self.directory(file.parent) else {
                errors.push(format!(
                    "owned parent anchor unavailable for staged file {:?}",
                    file.name
                ));
                continue;
            };
            if let Err(error) = directory.remove_child_file_if_identity(
                &file.name,
                file.identity,
                "owned Voice bundle file",
            ) {
                errors.push(format!("{:?}: {error}", file.name));
            }
        }
        self.files.clear();
        self.payload.take();
        if let (Some(voice), Some(expected)) = (&self.voice, payload_identity) {
            if let Err(error) = voice.remove_child_directory_if_identity(
                std::ffi::OsStr::new("payload"),
                expected,
                "owned Voice payload directory",
            ) {
                errors.push(format!("payload: {error}"));
            }
        }
        self.voice.take();
        if let (Some(target), Some(expected)) = (&self.target, voice_identity) {
            if let Err(error) = target.remove_child_directory_if_identity(
                std::ffi::OsStr::new("voice"),
                expected,
                "owned Voice component directory",
            ) {
                errors.push(format!("voice: {error}"));
            }
        }
        self.target.take();
        if let Some(expected) = target_identity {
            if let Err(error) = self.parent().remove_child_directory_if_identity(
                &self.target_name,
                expected,
                "owned Voice bundle root",
            ) {
                errors.push(format!("{}: {error}", self.target_dir.display()));
            }
        }
        let absent = match self
            .parent()
            .contains_child(&self.target_name, "owned Voice bundle parent")
        {
            Ok(present) => !present,
            Err(error) => {
                errors.push(format!("confirming owned Voice cleanup: {error}"));
                false
            }
        };
        self.armed = false;
        VoiceBundleCleanupOutcome {
            confirmed: absent && errors.is_empty(),
            errors,
        }
    }

    fn close_for_promotion(&mut self) -> Result<mgr::model::RenameDirectoryGuard> {
        let rename_guard = self
            .parent()
            .clone()
            .into_rename_guard("Voice bundle staging parent")?;
        self.payload.take();
        self.voice.take();
        self.target.take();
        self.parent.take();
        Ok(rename_guard)
    }

    fn reanchor_after_failed_promotion(
        &mut self,
        parent_guard: &mgr::model::RenameDirectoryGuard,
        target_identity: mgr::model::FileIdentity,
        voice_identity: mgr::model::FileIdentity,
        payload_identity: mgr::model::FileIdentity,
    ) -> Result<()> {
        let parent = mgr::model::open_directory_chain_nofollow(
            parent_guard.path(),
            "failed Voice staging parent",
        )?;
        if parent.identity() != parent_guard.identity() {
            return Err(ModError::Other(
                "failed Voice staging parent changed filesystem identity".into(),
            ));
        }
        self.parent = Some(parent);
        let target = expect_secure_directory(
            self.parent()
                .open_child(&self.target_name, "failed Voice staging root")?,
            "failed Voice staging root",
        )?;
        if target.identity() != target_identity {
            return Err(ModError::Other(
                "failed Voice staging root no longer has its owned identity".into(),
            ));
        }
        let voice = expect_secure_directory(
            target.open_child(
                std::ffi::OsStr::new("voice"),
                "failed Voice staging component",
            )?,
            "failed Voice staging component",
        )?;
        if voice.identity() != voice_identity {
            return Err(ModError::Other(
                "failed Voice staging component no longer has its owned identity".into(),
            ));
        }
        let payload = expect_secure_directory(
            voice.open_child(
                std::ffi::OsStr::new("payload"),
                "failed Voice staging payload",
            )?,
            "failed Voice staging payload",
        )?;
        if payload.identity() != payload_identity {
            return Err(ModError::Other(
                "failed Voice staging payload no longer has its owned identity".into(),
            ));
        }
        self.target = Some(target);
        self.voice = Some(voice);
        self.payload = Some(payload);
        Ok(())
    }

    fn release(mut self) {
        self.armed = false;
    }
}

impl Drop for VoiceBundleTreeOwnership {
    fn drop(&mut self) {
        let _ = self.cleanup_owned();
    }
}

#[derive(Debug)]
enum RetainedVoiceBundleWriteError {
    Collision,
    Failed(ModError),
}

impl From<ModError> for RetainedVoiceBundleWriteError {
    fn from(value: ModError) -> Self {
        Self::Failed(value)
    }
}

impl RetainedVoiceBundleWriteError {
    fn into_mod_error(self, target: &Path) -> ModError {
        match self {
            Self::Collision => ModError::Other(format!(
                "voice bundle target already exists: {}",
                target.display()
            )),
            Self::Failed(error) => error,
        }
    }
}

/// Write a hardened voice-only bundle into a brand-new target directory and verify the exact
/// result before returning.
///
/// The direct parent must already be a real, non-link directory. The target itself must not exist;
/// this function creates it atomically, uses create-new semantics for every file, and never calls
/// the compatibility writer that clears an existing tree. If a later write or verification fails,
/// only the target root successfully created by this call is removed on a best-effort basis.
fn write_voice_bundle_new_retained(
    dir: &Path,
    bundle: &Bundle,
) -> std::result::Result<VoiceBundleTreeOwnership, RetainedVoiceBundleWriteError> {
    validate_sealed_voice_bundle_memory(bundle)?;
    let target_name = dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            ModError::Other(format!(
                "voice bundle target must end in one portable directory name: {}",
                dir.display()
            ))
        })?;
    if !is_safe_filename(target_name) {
        return Err(ModError::Other(format!(
            "unsafe voice bundle target directory name {target_name:?}"
        ))
        .into());
    }
    let parent = dir
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (canonical_parent, parent_anchor) = open_voice_bundle_parent(parent)?;
    let target_dir = canonical_parent.join(target_name);
    match std::fs::symlink_metadata(&target_dir) {
        Ok(_) => {
            return Err(RetainedVoiceBundleWriteError::Collision);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io("checking voice bundle target")(error).into()),
    }
    let target_name_os = std::ffi::OsString::from(target_name);
    let mut voice_anchor = None;
    let mut payload_anchor = None;
    let mut created_files: Vec<(
        mgr::model::SecureDirectory,
        std::ffi::OsString,
        mgr::model::FileIdentity,
    )> = Vec::new();
    let Some(initial_target) =
        parent_anchor.try_create_child_directory_new(&target_name_os, "new Voice bundle root")?
    else {
        return Err(RetainedVoiceBundleWriteError::Collision);
    };
    let mut target_anchor = Some(initial_target.clone());
    let result = (|| -> Result<()> {
        let target = initial_target;
        let voice = target.create_child_directory_new(
            std::ffi::OsStr::new("voice"),
            "Voice bundle component directory",
        )?;
        voice_anchor = Some(voice.clone());
        let payload = voice.create_child_directory_new(
            std::ffi::OsStr::new("payload"),
            "Voice bundle payload directory",
        )?;
        payload_anchor = Some(payload.clone());

        for (relative, bytes) in &bundle.files {
            let (directory, name) = match relative.as_str() {
                "gore-mod.json" => (&target, std::ffi::OsStr::new("gore-mod.json")),
                "voice/manifest.json" => (&voice, std::ffi::OsStr::new("manifest.json")),
                value if value.starts_with("voice/payload/") => {
                    let name = value
                        .strip_prefix("voice/payload/")
                        .expect("prefix was checked");
                    if name.is_empty() || name.contains('/') || name.contains('\\') {
                        return Err(ModError::Other(format!(
                            "sealed Voice bundle contains a non-flat payload path: {relative:?}"
                        )));
                    }
                    (&payload, std::ffi::OsStr::new(name))
                }
                _ => {
                    return Err(ModError::Other(format!(
                        "sealed Voice bundle contains an unexpected output path: {relative:?}"
                    )))
                }
            };
            let name = name.to_os_string();
            let (mut file, identity) =
                directory.create_child_file_new(&name, "Voice bundle file")?;
            created_files.push((directory.clone(), name, identity));
            file.write_all(bytes)
                .map_err(io(&format!("writing Voice bundle file {relative}")))?;
            file.sync_all()
                .map_err(io(&format!("syncing Voice bundle file {relative}")))?;
            directory.sync_after_mutation("Voice bundle file parent")?;
            #[cfg(test)]
            if take_injected_voice_bundle_write_failure(&target_dir) {
                return Err(ModError::Other(
                    "injected new voice bundle write failure".into(),
                ));
            }
        }

        let rebound_target = expect_secure_directory(
            parent_anchor.open_child(&target_name_os, "completed Voice bundle root")?,
            "completed Voice bundle root",
        )?;
        let rebound_voice = expect_secure_directory(
            target.open_child(
                std::ffi::OsStr::new("voice"),
                "completed Voice component directory",
            )?,
            "completed Voice component directory",
        )?;
        let rebound_payload = expect_secure_directory(
            voice.open_child(
                std::ffi::OsStr::new("payload"),
                "completed Voice payload directory",
            )?,
            "completed Voice payload directory",
        )?;
        if rebound_target.identity() != target.identity()
            || rebound_voice.identity() != voice.identity()
            || rebound_payload.identity() != payload.identity()
        {
            return Err(ModError::Other(
                "Voice bundle directory identity changed while being written".into(),
            ));
        }
        verify_sealed_voice_bundle(target.path())?;
        let final_target = expect_secure_directory(
            parent_anchor.open_child(&target_name_os, "verified Voice bundle root")?,
            "verified Voice bundle root",
        )?;
        if final_target.identity() != target.identity() {
            return Err(ModError::Other(
                "Voice bundle root changed identity during final verification".into(),
            ));
        }
        Ok(())
    })();

    if let Err(error) = result {
        let mut cleanup_errors = Vec::new();
        for (directory, name, identity) in created_files.iter().rev() {
            if let Err(cleanup) =
                directory.remove_child_file_if_identity(name, *identity, "failed Voice bundle file")
            {
                cleanup_errors.push(format!("{name:?}: {cleanup}"));
            }
        }
        // Windows deliberately opens anchors without DELETE sharing. Close each child anchor
        // after cleaning its contents and before asking its still-retained parent to remove that
        // one direct name. Removal is non-recursive, so even a hostile late replacement cannot
        // make cleanup traverse or delete anything beneath an external link target.
        created_files.clear();
        let payload_identity = payload_anchor.as_ref().map(|anchor| anchor.identity());
        payload_anchor.take();
        if let (Some(voice), Some(identity)) = (&voice_anchor, payload_identity) {
            if let Err(cleanup) = voice.remove_child_directory_if_identity(
                std::ffi::OsStr::new("payload"),
                identity,
                "failed Voice payload directory",
            ) {
                cleanup_errors.push(format!("payload: {cleanup}"));
            }
        }
        let voice_identity = voice_anchor.as_ref().map(|anchor| anchor.identity());
        voice_anchor.take();
        if let (Some(target), Some(identity)) = (&target_anchor, voice_identity) {
            if let Err(cleanup) = target.remove_child_directory_if_identity(
                std::ffi::OsStr::new("voice"),
                identity,
                "failed Voice component directory",
            ) {
                cleanup_errors.push(format!("voice: {cleanup}"));
            }
        }
        let target_identity = target_anchor.as_ref().map(|anchor| anchor.identity());
        target_anchor.take();
        if let Some(identity) = target_identity {
            if let Err(cleanup) = parent_anchor.remove_child_directory_if_identity(
                &target_name_os,
                identity,
                "failed Voice bundle root",
            ) {
                cleanup_errors.push(format!("{}: {cleanup}", target_dir.display()));
            }
        }
        if cleanup_errors.is_empty() {
            return Err(error.into());
        }
        return Err(ModError::Other(format!(
            "{error}; additionally failed to clean parts of newly-created voice bundle: {}",
            cleanup_errors.join("; ")
        ))
        .into());
    }
    let target = target_anchor.expect("successful Voice write retained its target anchor");
    let voice = voice_anchor.expect("successful Voice write retained its voice anchor");
    let payload = payload_anchor.expect("successful Voice write retained its payload anchor");
    let target_identity = target.identity();
    let voice_identity = voice.identity();
    let payload_identity = payload.identity();
    let files = created_files
        .into_iter()
        .map(|(directory, name, identity)| {
            let parent = if directory.identity() == target_identity {
                VoiceBundleOwnedFileParent::Target
            } else if directory.identity() == voice_identity {
                VoiceBundleOwnedFileParent::Voice
            } else if directory.identity() == payload_identity {
                VoiceBundleOwnedFileParent::Payload
            } else {
                unreachable!("every created Voice file belongs to one retained bundle directory")
            };
            VoiceBundleOwnedFile {
                parent,
                name,
                identity,
            }
        })
        .collect();
    Ok(VoiceBundleTreeOwnership {
        parent: Some(parent_anchor),
        target_name: target_name_os,
        target_dir,
        target: Some(target),
        voice: Some(voice),
        payload: Some(payload),
        files,
        armed: true,
    })
}

/// Write a hardened voice-only bundle into a brand-new final target.
pub fn write_voice_bundle_new(dir: &Path, bundle: &Bundle) -> Result<()> {
    write_voice_bundle_new_retained(dir, bundle)
        .map_err(|error| error.into_mod_error(dir))?
        .release();
    Ok(())
}

/// Machine-readable outcome class for a staged Voice bundle operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceBundleStagingErrorKind {
    /// The requested operation failed and owned staging absence was confirmed.
    OperationFailed,
    /// Owned staging absence could not be confirmed, so callers must preserve/report the path.
    CleanupFailed,
    /// The atomic promotion succeeded, but its final durability/identity check failed.
    PublishedButUnconfirmed,
}

/// Structured failure from staged promotion or explicit abort.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct VoiceBundleStagingError {
    kind: VoiceBundleStagingErrorKind,
    cleanup_confirmed: bool,
    message: String,
}

impl VoiceBundleStagingError {
    pub fn kind(&self) -> VoiceBundleStagingErrorKind {
        self.kind
    }

    /// True only after the retained parent observed the owned staging name absent following
    /// identity-checked, direct-child cleanup.
    pub fn cleanup_confirmed(&self) -> bool {
        self.cleanup_confirmed
    }
}

/// A verified Voice bundle materialized in a unique sibling directory with retained filesystem
/// ownership. Dropping it attempts the same identity-checked cleanup as [`Self::abort`], without
/// deleting recursively or following a link/reparse replacement.
#[derive(Debug)]
pub struct StagedVoiceBundle {
    ownership: Option<VoiceBundleTreeOwnership>,
    final_name: std::ffi::OsString,
    final_path: PathBuf,
    expected_seal: VoiceBundleTreeSeal,
}

impl StagedVoiceBundle {
    pub fn path(&self) -> &Path {
        &self
            .ownership
            .as_ref()
            .expect("live staged Voice bundle retains ownership")
            .target_dir
    }

    /// Atomically publish the owned sibling without replacing an existing final target.
    pub fn promote_new(mut self) -> std::result::Result<(), VoiceBundleStagingError> {
        let mut owned = self
            .ownership
            .take()
            .expect("live staged Voice bundle retains ownership");
        let preflight = (|| -> Result<()> {
            owned.verify_complete_binding()?;
            let seal = seal_voice_bundle_disk_tree(&owned.target_dir)?;
            if seal != self.expected_seal {
                return Err(ModError::Other(
                    "owned staged Voice bundle differs from the requested bundle".into(),
                ));
            }
            owned.verify_complete_binding()
        })();
        if let Err(error) = preflight {
            return Err(staged_voice_operation_failure(error, &mut owned));
        }

        let target_identity = owned.target_identity();
        let voice_identity = owned.voice_identity();
        let payload_identity = owned.payload_identity();
        let parent_guard = match owned.close_for_promotion() {
            Ok(guard) => guard,
            Err(error) => return Err(staged_voice_operation_failure(error, &mut owned)),
        };
        if let Err(error) = promote_directory_noclobber(&owned.target_dir, &self.final_path) {
            let primary = ModError::Other(format!(
                "atomically promoting staged Voice bundle without replacement: {error}"
            ));
            if let Err(reanchor) = owned.reanchor_after_failed_promotion(
                &parent_guard,
                target_identity,
                voice_identity,
                payload_identity,
            ) {
                return Err(staged_voice_operation_failure(
                    ModError::Other(format!(
                        "{primary}; owned staging rebind also failed: {reanchor}"
                    )),
                    &mut owned,
                ));
            }
            return Err(staged_voice_operation_failure(primary, &mut owned));
        }

        // The owned identity is now intentionally live at the final name. Never run staging
        // cleanup after this boundary, including when a durability query itself fails.
        owned.armed = false;
        let publication_check = (|| -> Result<()> {
            let published_parent = mgr::model::open_directory_chain_nofollow(
                parent_guard.path(),
                "published Voice bundle parent",
            )?;
            if published_parent.identity() != parent_guard.identity() {
                return Err(ModError::Other(
                    "published Voice bundle parent changed filesystem identity".into(),
                ));
            }
            published_parent.sync_after_mutation("published Voice bundle parent")?;
            let final_root = expect_secure_directory(
                published_parent.open_child(&self.final_name, "published Voice bundle root")?,
                "published Voice bundle root",
            )?;
            if final_root.identity() != target_identity {
                return Err(ModError::Other(
                    "published Voice bundle has an unexpected filesystem identity".into(),
                ));
            }
            Ok(())
        })();
        publication_check.map_err(|error| VoiceBundleStagingError {
            kind: VoiceBundleStagingErrorKind::PublishedButUnconfirmed,
            cleanup_confirmed: false,
            message: error.to_string(),
        })
    }

    /// Remove this exact owned staging tree and report any inability to confirm absence.
    pub fn abort(mut self) -> std::result::Result<(), VoiceBundleStagingError> {
        let mut owned = self
            .ownership
            .take()
            .expect("live staged Voice bundle retains ownership");
        let cleanup = owned.cleanup_owned();
        if cleanup.confirmed {
            Ok(())
        } else {
            Err(VoiceBundleStagingError {
                kind: VoiceBundleStagingErrorKind::CleanupFailed,
                cleanup_confirmed: false,
                message: voice_cleanup_message(&cleanup.errors),
            })
        }
    }
}

fn voice_cleanup_message(errors: &[String]) -> String {
    if errors.is_empty() {
        "owned Voice staging cleanup could not be confirmed".into()
    } else {
        format!(
            "owned Voice staging cleanup could not be confirmed: {}",
            errors.join("; ")
        )
    }
}

fn staged_voice_operation_failure(
    primary: ModError,
    owned: &mut VoiceBundleTreeOwnership,
) -> VoiceBundleStagingError {
    let cleanup = owned.cleanup_owned();
    if cleanup.confirmed {
        VoiceBundleStagingError {
            kind: VoiceBundleStagingErrorKind::OperationFailed,
            cleanup_confirmed: true,
            message: primary.to_string(),
        }
    } else {
        VoiceBundleStagingError {
            kind: VoiceBundleStagingErrorKind::CleanupFailed,
            cleanup_confirmed: false,
            message: format!("{primary}; {}", voice_cleanup_message(&cleanup.errors)),
        }
    }
}

/// Write a verified Voice bundle into a unique create-new sibling of `final_target` and retain its
/// exact identities until the caller promotes, aborts, or drops the returned handle.
pub fn write_voice_bundle_staged_new(
    final_target: &Path,
    bundle: &Bundle,
) -> Result<StagedVoiceBundle> {
    static NEXT_STAGE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    validate_sealed_voice_bundle_memory(bundle)?;
    let final_name = final_target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            ModError::Other(format!(
                "final Voice bundle target must end in one portable directory name: {}",
                final_target.display()
            ))
        })?;
    if !is_safe_filename(final_name) {
        return Err(ModError::Other(format!(
            "unsafe final Voice bundle directory name {final_name:?}"
        )));
    }
    let parent = final_target
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (canonical_parent, _) = open_voice_bundle_parent(parent)?;
    let final_path = canonical_parent.join(final_name);
    let expected_seal = canonical_voice_bundle_tree_seal(
        bundle
            .files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice())),
    )?;

    for _ in 0..128 {
        let serial = NEXT_STAGE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let stage_name = format!("gore-voice-stage-{}-{serial}", std::process::id());
        let stage_path = canonical_parent.join(stage_name);
        match std::fs::symlink_metadata(&stage_path) {
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io("checking unique Voice staging sibling")(error)),
        }
        match write_voice_bundle_new_retained(&stage_path, bundle) {
            Ok(ownership) => {
                return Ok(StagedVoiceBundle {
                    ownership: Some(ownership),
                    final_name: std::ffi::OsString::from(final_name),
                    final_path,
                    expected_seal,
                })
            }
            Err(RetainedVoiceBundleWriteError::Collision) => continue,
            Err(RetainedVoiceBundleWriteError::Failed(error)) => return Err(error),
        }
    }
    Err(ModError::Other(
        "could not claim a unique Voice staging sibling after 128 attempts".into(),
    ))
}

fn expect_secure_directory(
    node: mgr::model::SecureNode,
    label: &str,
) -> Result<mgr::model::SecureDirectory> {
    match node {
        mgr::model::SecureNode::Directory(directory) => Ok(directory),
        mgr::model::SecureNode::File(file) => Err(ModError::Other(format!(
            "{label} became a regular file: {}",
            file.path().display()
        ))),
    }
}

/// Reopen and validate an exact voice-only bundle produced by [`build_sealed_voice_bundle`].
///
/// The verifier is deliberately narrower than deployment: it accepts one format-1 gore-mod
/// manifest containing exactly one `voice` component, one format-3 voice manifest containing only
/// sealed existing-member replacements, and the canonical indexed Ogg payload layout emitted by
/// the builder. Every file is reopened through the bundle path/link defenses and all payloads are
/// checked against the per-Ogg and aggregate byte limits. Missing files, extra files/directories,
/// extra components, non-canonical payload paths, and invalid observations/Ogg data are rejected.
/// No game path is resolved and no file is written.
pub fn verify_sealed_voice_bundle(dir: &Path) -> Result<()> {
    resolve_safe_bundle_root(dir)?;
    let manifest_bytes = read_safe_bundle_file(
        dir,
        Path::new("gore-mod.json"),
        "gore-mod.json",
        MAX_BUNDLE_MANIFEST_BYTES,
    )?;
    let manifest: ModManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest_bytes != serde_json::to_vec_pretty(&manifest)? {
        return Err(ModError::Other(
            "sealed voice bundle has a non-canonical gore-mod manifest encoding".into(),
        ));
    }
    let voice_manifest_bytes = read_safe_bundle_file(
        dir,
        Path::new("voice/manifest.json"),
        "voice manifest",
        MAX_VOICE_MANIFEST_BYTES,
    )?;
    let voice_manifest: VoicePatchManifest = serde_json::from_slice(&voice_manifest_bytes)?;
    if voice_manifest_bytes != serde_json::to_vec_pretty(&voice_manifest)? {
        return Err(ModError::Other(
            "sealed voice bundle has a non-canonical voice manifest encoding".into(),
        ));
    }
    let expected_files = validate_sealed_voice_contract(&manifest, &voice_manifest)?;
    let expected_dirs = BTreeSet::from(["voice".to_string(), "voice/payload".to_string()]);
    let voice_limits = gore_vo::Limits::default();
    let mut retained_ogg_bytes = 0u64;
    for (index, edit) in voice_manifest.edits.iter().enumerate() {
        let expected_payload = format!("voice/payload/{index}.ogg");
        let max_bytes =
            voice_payload_byte_limit(retained_ogg_bytes, voice_limits.max_ogg_bytes as u64)?;
        let ogg = read_safe_bundle_file(
            dir,
            Path::new(&expected_payload),
            "voice Ogg payload",
            max_bytes,
        )?;
        retained_ogg_bytes = retained_ogg_bytes
            .checked_add(ogg.len() as u64)
            .ok_or_else(|| ModError::Other("voice Ogg memory budget overflow".into()))?;
        gore_vo::validate_ogg(&ogg, &voice_limits)
            .map_err(|e| ModError::Voice(format!("{expected_payload}: {e}")))?;
        require_voice_payload_seal(edit, &ogg)?;
    }

    let (actual_files, actual_dirs) = collect_exact_bundle_layout(dir)?;
    if actual_files != expected_files {
        let missing: Vec<_> = expected_files.difference(&actual_files).cloned().collect();
        let extra: Vec<_> = actual_files.difference(&expected_files).cloned().collect();
        return Err(ModError::Other(format!(
            "sealed voice bundle file layout mismatch (missing: {missing:?}, extra: {extra:?})"
        )));
    }
    if actual_dirs != expected_dirs {
        let missing: Vec<_> = expected_dirs.difference(&actual_dirs).cloned().collect();
        let extra: Vec<_> = actual_dirs.difference(&expected_dirs).cloned().collect();
        return Err(ModError::Other(format!(
            "sealed voice bundle directory layout mismatch (missing: {missing:?}, extra: {extra:?})"
        )));
    }
    Ok(())
}

fn canonical_voice_bundle_tree_seal<P, B>(
    files: impl IntoIterator<Item = (P, B)>,
) -> Result<VoiceBundleTreeSeal>
where
    P: AsRef<str>,
    B: AsRef<[u8]>,
{
    let mut byte_len = 0u64;
    let mut digest = Sha256::new();
    digest.update(b"gore-mod.voice-bundle-tree.v1\0");
    for (path, bytes) in files {
        let path = path.as_ref();
        let bytes = bytes.as_ref();
        let path_len = u64::try_from(path.len())
            .map_err(|_| ModError::Other("voice bundle path length overflow".into()))?;
        let file_len = u64::try_from(bytes.len())
            .map_err(|_| ModError::Other("voice bundle file length overflow".into()))?;
        byte_len = byte_len
            .checked_add(file_len)
            .ok_or_else(|| ModError::Other("voice bundle tree byte total overflow".into()))?;
        digest.update(path_len.to_be_bytes());
        digest.update(path.as_bytes());
        digest.update(file_len.to_be_bytes());
        digest.update(bytes);
    }
    Ok(VoiceBundleTreeSeal {
        byte_len,
        sha256: format!("{:x}", digest.finalize()),
    })
}

fn read_voice_bundle_tree_for_seal(dir: &Path) -> Result<VoiceBundleTreeSeal> {
    let (files, _) = collect_exact_bundle_layout(dir)?;
    let voice_limits = gore_vo::Limits::default();
    let mut payload_total = 0u64;
    let mut loaded = Vec::with_capacity(files.len());
    for relative in files {
        let max_bytes = match relative.as_str() {
            "gore-mod.json" => MAX_BUNDLE_MANIFEST_BYTES,
            "voice/manifest.json" => MAX_VOICE_MANIFEST_BYTES,
            _ if relative.starts_with("voice/payload/") => {
                let remaining = MAX_PENDING_VOICE_OGG_BYTES
                    .checked_sub(payload_total)
                    .ok_or_else(|| ModError::Other("voice Ogg memory budget underflow".into()))?;
                remaining.min(voice_limits.max_ogg_bytes as u64)
            }
            _ => {
                return Err(ModError::Other(format!(
                    "unexpected file in exact Voice bundle tree: {relative:?}"
                )));
            }
        };
        let bytes = read_safe_bundle_file(
            dir,
            Path::new(&relative),
            "sealed Voice bundle tree file",
            max_bytes,
        )?;
        if relative.starts_with("voice/payload/") {
            payload_total = payload_total
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| ModError::Other("voice Ogg memory budget overflow".into()))?;
        }
        loaded.push((relative, bytes));
    }
    canonical_voice_bundle_tree_seal(loaded)
}

/// Reopen an exact verified managed Voice bundle and return its canonical disk-tree content seal.
///
/// The tree is verified before and after each bounded no-follow sealing pass. Two independently
/// reopened passes must agree, so a valid Ogg substitution, manifest swap, added path, or
/// concurrent tree-generation change cannot be reported as the originally verified output.
pub fn seal_voice_bundle_disk_tree(dir: &Path) -> Result<VoiceBundleTreeSeal> {
    verify_sealed_voice_bundle(dir)?;
    let first = read_voice_bundle_tree_for_seal(dir)?;
    verify_sealed_voice_bundle(dir)?;
    let second = read_voice_bundle_tree_for_seal(dir)?;
    if first != second {
        return Err(ModError::Other(
            "sealed Voice bundle tree changed while its disk seal was being computed".into(),
        ));
    }
    verify_sealed_voice_bundle(dir)?;
    Ok(second)
}

fn collect_exact_bundle_layout(bundle_root: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>)> {
    let root = resolve_safe_bundle_root(bundle_root)?;
    let mut pending = vec![root.clone()];
    let mut files = BTreeSet::new();
    let mut dirs = BTreeSet::new();
    let mut entry_count = 0u64;
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).map_err(io(&format!(
            "reading sealed voice bundle directory {}",
            dir.display()
        )))? {
            let entry = entry.map_err(io("reading sealed voice bundle directory entry"))?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(io(&format!(
                "reading sealed voice bundle metadata {}",
                path.display()
            )))?;
            if metadata_is_link(&metadata) {
                return Err(ModError::Other(format!(
                    "sealed voice bundle contains a symbolic link or reparse point: {}",
                    path.display()
                )));
            }
            let relative = path.strip_prefix(&root).map_err(|_| {
                ModError::Other(format!(
                    "sealed voice bundle path escaped its root: {}",
                    path.display()
                ))
            })?;
            let portable = relative
                .to_str()
                .ok_or_else(|| {
                    ModError::Other(format!(
                        "sealed voice bundle path is not UTF-8: {}",
                        path.display()
                    ))
                })?
                .replace('\\', "/");
            if !is_safe_rel_path(&portable) {
                return Err(ModError::Other(format!(
                    "sealed voice bundle contains a non-portable path: {}",
                    path.display()
                )));
            }
            entry_count = entry_count.checked_add(1).ok_or_else(|| {
                ModError::Other("sealed voice bundle entry count overflow".into())
            })?;
            if entry_count > MAX_UE4SS_TREE_ENTRIES {
                return Err(ModError::Other(format!(
                    "sealed voice bundle exceeds the {MAX_UE4SS_TREE_ENTRIES}-entry limit"
                )));
            }
            if metadata.is_dir() {
                dirs.insert(portable);
                pending.push(path);
            } else if metadata.is_file() {
                files.insert(portable);
            } else {
                return Err(ModError::Other(format!(
                    "sealed voice bundle contains a non-file payload: {}",
                    path.display()
                )));
            }
        }
    }
    Ok((files, dirs))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// A safe mod name is a single normal path component: non-empty, no path separators, no `..`,
/// no control characters — so it can't escape the bundle/UE4SS Mods directory.
fn is_safe_mod_name(name: &str) -> bool {
    !name.contains('/')
        && !name.contains('\\')
        && gore_vo::validate_archive_entry_path(name, &gore_vo::Limits::default()).is_ok()
}

/// A safe single filename: non-empty, no separators, no `..`, no control chars.
fn is_safe_filename(name: &str) -> bool {
    is_safe_mod_name(name)
}

/// A safe relative path inside the bundle: non-empty, not absolute, every component a normal
/// name (no `..`, no root/prefix), no control characters — so it can't escape the bundle dir.
fn is_safe_rel_path(p: &str) -> bool {
    gore_vo::validate_archive_entry_path(p, &gore_vo::Limits::default()).is_ok()
}

/// The native path a loose-file destination names below the install root. Built component by
/// component so the joined result uses this platform's separator: a stored record path with mixed
/// separators would still validate, but every later comparison would have to normalize it first.
pub(crate) fn loose_relative_os_path(game_path: &str) -> PathBuf {
    game_path.split('/').collect()
}

/// Accept one authored loose-file destination, or say why it is refused.
///
/// Two independent gates: the spelling must be a portable relative path (no `..`, no backslash, no
/// drive letter or ADS colon, no reserved device name), and the destination must be inside the set
/// of files a bundle is allowed to replace. The second gate is [`loose_target_allowed`], the same
/// one every deploy-record write goes through, so a bundle can never author a destination the
/// record layer would later reject.
pub(crate) fn validate_loose_game_path(game_path: &str) -> Result<()> {
    if !is_safe_rel_path(game_path) {
        return Err(ModError::Other(format!(
            "unsafe loose file game path {game_path:?}: expected a forward-slash path relative to \
             the game install root, e.g. \"G1R/Content/Slate/Cursors/Normal/Normal.PNG\""
        )));
    }
    if !loose_target_allowed(&loose_relative_os_path(game_path)) {
        return Err(ModError::Other(format!(
            "loose file game path {game_path:?} is not a replaceable game file; only files under \
             G1R/Content or G1R/Config qualify, and not the pak containers, a *.gore-bak backup, \
             or a file that already has its own deploy mechanism (.lcache, FMOD bank, script \
             cache, voice ZIP)"
        )));
    }
    Ok(())
}

/// Accept one authored audio `bank` field, or say why it is refused.
///
/// Deploy joins this straight onto the install's `G1R/Content/FMOD/Desktop`, so the only spelling
/// that can ever work is a bare file name. The check runs twice on purpose: at build time, where
/// the rest of the spec is validated, so a spec that can never deploy does not first print
/// "built bundle"; and again in the deploy planner, which also accepts bundles this process did
/// not build.
pub(crate) fn validate_bank_name(bank: &str) -> Result<()> {
    if !is_safe_filename(bank) {
        return Err(ModError::Other(format!(
            "invalid audio bank name {bank:?}: must be the bare file name of a bank in the \
             install's G1R/Content/FMOD/Desktop, not a path — e.g. \"SFX.bank\""
        )));
    }
    Ok(())
}

/// Which mounted pak entries this install already carries — the oracle that says whether an
/// in-place loose-file write can be observed at all.
///
/// Unreal consults a mounted pak before the physical filesystem, so a destination the base
/// containers already carry is inert on disk: the write succeeds, the backup is taken, and the game
/// keeps reading the packed copy. This is deliberately a DEPLOY-time question. Whether a path is
/// shadowed is a property of the installation, not of the bundle, and a bundle that decided its own
/// footprint per machine would owe a different undeploy on every machine.
///
/// Built at most once per deploy and only when something actually asks: a texture-only bundle must
/// not pay for parsing ~110 KB of pak directory it will never consult.
pub(crate) struct PakShadowIndex {
    game_root: PathBuf,
    entries: Option<BTreeSet<String>>,
}

impl PakShadowIndex {
    pub(crate) fn new(game_root: &Path) -> Self {
        Self {
            game_root: game_root.to_path_buf(),
            entries: None,
        }
    }

    /// The filename of the pak already carrying `game_path`, or `None` when the destination is
    /// reachable on disk.
    pub(crate) fn owning_pak(&mut self, game_path: &str) -> Result<Option<String>> {
        let paks = self.game_root.join("G1R").join("Content").join("Paks");
        // An install with no `Paks` directory carries nothing, and neither does a synthetic test
        // root. Asking gore-tex to scan a directory that is not there would turn "no paks" into a
        // deploy failure, which is the opposite of what this gate is for.
        if !paks.is_dir() {
            return Ok(None);
        }
        if self.entries.is_none() {
            self.entries = Some(
                gore_tex::container::pak_shadow_index(&self.game_root).map_err(|error| {
                    ModError::Other(format!(
                        "reading the installed pak index below {}: {error}",
                        paks.display()
                    ))
                })?,
            );
        }
        let key = shadow_key(game_path);
        if !self
            .entries
            .as_ref()
            .expect("the index was just built")
            .contains(&key)
        {
            return Ok(None);
        }
        // Name the container, not just the fact. The index is a flat set of entry paths, so the
        // provenance is recovered here — on the refusal path only, where re-reading a few
        // indexes costs nothing and every ordinary deploy pays nothing for it.
        Ok(Some(
            shadowing_pak_name(&self.game_root, &key).unwrap_or_else(|| "a mounted pak".into()),
        ))
    }
}

/// The one spelling both sides of a shadow comparison are folded to: forward slashes, lowercase.
/// Deliberately `to_lowercase`, not the ASCII-only fold used for authoring dedup — this string is
/// looked up in a set gore-tex built with `to_lowercase`, and the two must agree exactly.
fn shadow_key(path: &str) -> String {
    path.trim().replace('\\', "/").to_lowercase()
}

/// The filename of the shipped container whose index carries `key` (an already-folded game-root-
/// relative path). Entries are re-anchored through the same mount-prefix rule the shadow index
/// itself uses, so the name this reports can never disagree with the refusal it decorates.
fn shadowing_pak_name(game_root: &Path, key: &str) -> Option<String> {
    for listing in gore_tex::container::list_game_paks(game_root).ok()? {
        let Some(prefix) = gore_tex::container::mount_prefix_from_game_root(&listing.mount_point)
        else {
            continue;
        };
        if listing
            .files
            .iter()
            .any(|file| shadow_key(&format!("{prefix}{file}")) == key)
        {
            return listing
                .pak
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
        }
    }
    None
}

/// Voice archives are single `.zip` filenames under the fixed `Story/VoiceOver` directory.
pub(crate) fn is_safe_voice_archive(name: &str) -> bool {
    is_safe_filename(name)
        && name.to_ascii_lowercase().ends_with(".zip")
        && gore_vo::validate_archive_entry_path(name, &gore_vo::Limits::default()).is_ok()
}

/// ZIP member paths use one canonical spelling: forward-slash-separated relative `.ogg` paths.
pub(crate) fn is_safe_voice_member(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".ogg")
        && gore_vo::validate_archive_entry_path(path, &gore_vo::Limits::default()).is_ok()
}

pub(crate) fn validate_voice_target(archive: &str, archive_path: &str) -> Result<()> {
    if !is_safe_voice_archive(archive) {
        return Err(ModError::Other(format!(
            "unsafe voice archive name {archive:?}: expected one .zip filename"
        )));
    }
    if !is_safe_voice_member(archive_path) {
        return Err(ModError::Other(format!(
            "unsafe voice archive member {archive_path:?}: expected a relative forward-slash .ogg path"
        )));
    }
    Ok(())
}

fn parse_voice_archive_sha256(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ModError::Other(format!(
            "invalid voice archive SHA-256 {value:?}: expected exactly 64 lowercase hexadecimal characters"
        )));
    }
    let mut decoded = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |byte: u8| -> u8 {
            if byte.is_ascii_digit() {
                byte - b'0'
            } else {
                byte - b'a' + 10
            }
        };
        decoded[index] = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    Ok(decoded)
}

fn voice_payload_seal(bytes: &[u8]) -> VoicePayloadSeal {
    VoicePayloadSeal {
        byte_len: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn require_voice_payload_seal(edit: &VoicePatchEntry, bytes: &[u8]) -> Result<()> {
    let Some(expected) = &edit.payload_seal else {
        return Err(ModError::Other(format!(
            "sealed voice payload {:?} has no content seal",
            edit.ogg
        )));
    };
    let actual = voice_payload_seal(bytes);
    if &actual != expected {
        return Err(ModError::Other(format!(
            "sealed voice payload {:?} disagrees with its content seal",
            edit.ogg
        )));
    }
    Ok(())
}

fn digest_regular_file_nofollow(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<VoiceExecutableGenerationSeal> {
    let mut source = mgr::model::open_file_nofollow(path, label)?;
    let advertised = source.len();
    if advertised == 0 {
        return Err(ModError::Other(format!(
            "{label} must be non-empty: {}",
            source.path().display()
        )));
    }
    if advertised > max_bytes {
        return Err(ModError::Other(format!(
            "{label} exceeds the {max_bytes}-byte limit: {advertised} bytes at {}",
            source.path().display()
        )));
    }
    let mut digest = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = source
            .file
            .read(&mut buffer)
            .map_err(io(&format!("reading {label} {}", source.path().display())))?;
        if count == 0 {
            break;
        }
        length = length
            .checked_add(count as u64)
            .ok_or_else(|| ModError::Other(format!("{label} length overflow")))?;
        if length > max_bytes {
            return Err(ModError::Other(format!(
                "{label} grew beyond the {max_bytes}-byte limit while being read: {}",
                source.path().display()
            )));
        }
        digest.update(&buffer[..count]);
    }
    if length != advertised {
        return Err(ModError::Other(format!(
            "{label} changed length while being read: {}",
            source.path().display()
        )));
    }
    source.verify_len(advertised, label)?;
    Ok(VoiceExecutableGenerationSeal {
        byte_len: length,
        sha256: format!("{:x}", digest.finalize()),
    })
}

fn live_voice_executable_generation(gp: &GamePaths) -> Result<VoiceExecutableGenerationSeal> {
    let first = digest_regular_file_nofollow(
        &gp.executable,
        "game executable generation",
        MAX_GAME_EXECUTABLE_BYTES,
    )?;
    // A second no-follow reopen binds the digest back to the current fixed path after the first
    // streaming pass. This catches Unix rename/substitution races that an opened handle alone
    // cannot prevent (Windows keeps the first identity non-deletable while open).
    let second = digest_regular_file_nofollow(
        &gp.executable,
        "game executable generation",
        MAX_GAME_EXECUTABLE_BYTES,
    )?;
    if first != second {
        return Err(ModError::Other(format!(
            "game executable generation changed while being authenticated: {}",
            gp.executable.display()
        )));
    }
    Ok(second)
}

fn require_live_voice_executable_generation(
    gp: &GamePaths,
    expected: &VoiceExecutableGenerationSeal,
) -> Result<()> {
    if expected.byte_len == 0 {
        return Err(ModError::Other(
            "voice executable generation length must be non-zero".into(),
        ));
    }
    parse_voice_archive_sha256(&expected.sha256)?;
    let actual = live_voice_executable_generation(gp)?;
    if &actual != expected {
        return Err(ModError::Other(format!(
            "installed game executable does not match the format-3 Voice bundle generation: {}",
            gp.executable.display()
        )));
    }
    Ok(())
}

fn validate_voice_edit_observation(
    op: VoicePatchOp,
    observation: Option<&VoiceArchiveObservation>,
) -> Result<()> {
    let Some(observation) = observation else {
        return Ok(());
    };
    if observation.archive_size == 0 {
        return Err(ModError::Other(
            "invalid voice archive observation: archive size must be non-zero".into(),
        ));
    }
    parse_voice_archive_sha256(&observation.archive_sha256)?;
    match (op, &observation.member_proof) {
        (VoicePatchOp::Add, VoiceMemberProof::Absent) => Ok(()),
        (
            VoicePatchOp::Replace,
            VoiceMemberProof::Present {
                uncompressed_size,
                ..
            },
        ) if *uncompressed_size > 0 => Ok(()),
        (VoicePatchOp::Replace, VoiceMemberProof::Present { .. }) => Err(ModError::Other(
            "invalid sealed voice replace: present member proof must have a non-zero uncompressed size"
                .into(),
        )),
        (VoicePatchOp::Add, VoiceMemberProof::Present { .. }) => Err(ModError::Other(
            "invalid sealed voice add: member proof must be absent".into(),
        )),
        (VoicePatchOp::Replace, VoiceMemberProof::Absent) => Err(ModError::Other(
            "invalid sealed voice replace: member proof must be present".into(),
        )),
    }
}

pub(crate) fn validate_voice_manifest(manifest: &VoicePatchManifest) -> Result<()> {
    if manifest.edits.is_empty() {
        return Err(ModError::Other(
            "voice patch manifest contains no edits".into(),
        ));
    }
    match manifest.format {
        1 if manifest.executable_generation.is_some()
            || manifest
                .edits
                .iter()
                .any(|edit| edit.observation.is_some() || edit.payload_seal.is_some()) =>
        {
            return Err(ModError::Other(
                "voice patch manifest format 1 must not contain a generation seal, archive observations, or payload seals"
                    .into(),
            ));
        }
        2 if manifest.executable_generation.is_some()
            || manifest
                .edits
                .iter()
                .any(|edit| edit.observation.is_none() || edit.payload_seal.is_some()) =>
        {
            return Err(ModError::Other(
                "voice patch manifest format 2 requires archive observations and must not contain generation or payload seals"
                    .into(),
            ));
        }
        3 if manifest.executable_generation.is_none()
            || manifest
                .edits
                .iter()
                .any(|edit| edit.observation.is_none() || edit.payload_seal.is_none()) =>
        {
            return Err(ModError::Other(
                "voice patch manifest format 3 requires an executable generation, archive observation, and payload seal on every edit"
                    .into(),
            ));
        }
        1..=3 => {}
        format => {
            return Err(ModError::Other(format!(
                "unsupported voice patch manifest format {format} (want 1, 2, or 3)"
            )));
        }
    }
    if let Some(generation) = &manifest.executable_generation {
        if generation.byte_len == 0 {
            return Err(ModError::Other(
                "voice executable generation length must be non-zero".into(),
            ));
        }
        parse_voice_archive_sha256(&generation.sha256)?;
    }
    let mut archive_seals: BTreeMap<String, (u64, [u8; 32])> = BTreeMap::new();
    let mut sealed_targets = BTreeSet::new();
    for edit in &manifest.edits {
        validate_voice_target(&edit.archive, &edit.archive_path)?;
        validate_voice_edit_observation(edit.op, edit.observation.as_ref())?;
        if let Some(observation) = &edit.observation {
            let seal = (
                observation.archive_size,
                parse_voice_archive_sha256(&observation.archive_sha256)?,
            );
            match archive_seals.entry(voice_key(&edit.archive)) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(seal);
                }
                std::collections::btree_map::Entry::Occupied(entry) if entry.get() != &seal => {
                    return Err(ModError::Other(format!(
                        "sealed voice edits for archive {:?} disagree on the pristine archive identity",
                        edit.archive
                    )));
                }
                std::collections::btree_map::Entry::Occupied(_) => {}
            }
        }
        if let Some(payload_seal) = &edit.payload_seal {
            if payload_seal.byte_len == 0 {
                return Err(ModError::Other(
                    "sealed voice payload length must be non-zero".into(),
                ));
            }
            parse_voice_archive_sha256(&payload_seal.sha256)?;
        }
        if manifest.format == 3
            && !sealed_targets.insert((voice_key(&edit.archive), voice_key(&edit.archive_path)))
        {
            return Err(ModError::Other(format!(
                "sealed voice manifest contains duplicate deployment target {:?}:{:?}",
                edit.archive, edit.archive_path
            )));
        }
        if manifest.format == 3
            && (edit.op != VoicePatchOp::Replace
                || !matches!(
                    edit.observation.as_ref().map(|value| &value.member_proof),
                    Some(VoiceMemberProof::Present { .. })
                ))
        {
            return Err(ModError::Other(
                "voice patch manifest format 3 may only replace observed existing members".into(),
            ));
        }
        if edit.ogg.contains('\\')
            || !is_safe_rel_path(&edit.ogg)
            || !edit.ogg.to_ascii_lowercase().ends_with(".ogg")
        {
            return Err(ModError::Other(format!(
                "unsafe voice Ogg payload path {:?}",
                edit.ogg
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct PendingVoiceEdit {
    archive: String,
    op: VoicePatchOp,
    archive_path: String,
    ogg: Vec<u8>,
    observation: Option<VoiceArchiveObservation>,
    order: usize,
}

#[derive(Debug, Default)]
pub(crate) struct PendingVoiceEdits {
    edits: BTreeMap<(String, String), PendingVoiceEdit>,
    retained_ogg_bytes: u64,
    executable_generation: Option<VoiceExecutableGenerationSeal>,
}

impl PendingVoiceEdits {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    fn remaining_allocation(&self) -> Result<u64> {
        MAX_PENDING_VOICE_OGG_BYTES
            .checked_sub(self.retained_ogg_bytes)
            .ok_or_else(|| ModError::Other("voice Ogg memory budget underflow".into()))
    }

    fn insert(&mut self, key: (String, String), edit: PendingVoiceEdit) -> Result<()> {
        let new_len = edit.ogg.len() as u64;
        let previous_len = self
            .edits
            .get(&key)
            .map_or(0, |previous| previous.ogg.len() as u64);
        let retained_ogg_bytes = self
            .retained_ogg_bytes
            .checked_sub(previous_len)
            .and_then(|total| total.checked_add(new_len))
            .ok_or_else(|| ModError::Other("voice Ogg memory accounting overflow".into()))?;
        if retained_ogg_bytes > MAX_PENDING_VOICE_OGG_BYTES {
            return Err(ModError::Other(format!(
                "voice Ogg payloads exceed the {MAX_PENDING_VOICE_OGG_BYTES}-byte deployment memory budget"
            )));
        }
        self.edits.insert(key, edit);
        self.retained_ogg_bytes = retained_ogg_bytes;
        Ok(())
    }

    fn merge_executable_generation(
        &mut self,
        generation: Option<&VoiceExecutableGenerationSeal>,
    ) -> Result<()> {
        let Some(generation) = generation else {
            return Ok(());
        };
        match &self.executable_generation {
            None => self.executable_generation = Some(generation.clone()),
            Some(existing) if existing == generation => {}
            Some(_) => {
                return Err(ModError::Other(
                    "format-3 voice components in one loadout target conflicting executable generations"
                        .into(),
                ));
            }
        }
        Ok(())
    }
}

fn voice_key(value: &str) -> String {
    value.replace('\\', "/").to_lowercase()
}

/// Read and fully validate one voice component, merging it into `pending`. Collisions use the
/// manager/direct-deploy contract: case-insensitive `(archive, member)`, later entry wins while
/// retaining that winning entry's spelling and operation.
pub(crate) fn merge_voice_component(
    bundle_root: &Path,
    component_path: &str,
    pending: &mut PendingVoiceEdits,
    next_order: &mut usize,
) -> Result<()> {
    if !is_safe_rel_path(component_path) {
        return Err(ModError::Other(format!(
            "unsafe voice patch path: {component_path:?}"
        )));
    }
    let manifest_rel = PathBuf::from(format!("{component_path}/manifest.json"));
    let manifest_bytes = read_safe_bundle_file(
        bundle_root,
        &manifest_rel,
        "voice manifest",
        MAX_VOICE_MANIFEST_BYTES,
    )?;
    let manifest: VoicePatchManifest = serde_json::from_slice(&manifest_bytes)?;
    validate_voice_manifest(&manifest)?;
    pending.merge_executable_generation(manifest.executable_generation.as_ref())?;
    let payload_sealed = manifest.format == 3;
    let voice_limits = gore_vo::Limits::default();
    for edit in manifest.edits {
        let key = (voice_key(&edit.archive), voice_key(&edit.archive_path));
        // The previous winning payload remains resident until the new one has been fully validated,
        // so replacements also reserve their complete new length before allocation.
        let remaining = pending.remaining_allocation()?;
        let (ogg_path, _) =
            resolve_safe_bundle_file(bundle_root, Path::new(&edit.ogg), "voice Ogg payload")?;
        let ogg = read_regular_file_limited(
            &ogg_path,
            "voice Ogg payload",
            (voice_limits.max_ogg_bytes as u64).min(remaining),
        )?;
        gore_vo::validate_ogg(&ogg, &voice_limits)
            .map_err(|e| ModError::Voice(format!("{}: {e}", edit.ogg)))?;
        if payload_sealed {
            require_voice_payload_seal(&edit, &ogg)?;
        }
        pending.insert(
            key,
            PendingVoiceEdit {
                archive: edit.archive,
                op: edit.op,
                archive_path: edit.archive_path,
                ogg,
                observation: edit.observation,
                order: *next_order,
            },
        )?;
        *next_order = next_order.checked_add(1).ok_or_else(|| {
            ModError::Other("too many voice patch entries to order safely".into())
        })?;
    }
    Ok(())
}

/// Read a regular, non-link file whose canonical location stays under `bundle_root`.
pub(crate) fn read_safe_bundle_file(
    bundle_root: &Path,
    rel: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    let (canonical, _) = resolve_safe_bundle_file(bundle_root, rel, label)?;
    read_regular_file_limited(&canonical, label, max_bytes)
}

fn resolve_safe_bundle_file(bundle_root: &Path, rel: &Path, label: &str) -> Result<(PathBuf, u64)> {
    let rel_text = rel.to_string_lossy();
    if !is_safe_rel_path(&rel_text) {
        return Err(ModError::Other(format!("unsafe {label} path {rel_text:?}")));
    }
    let root = resolve_safe_bundle_root(bundle_root)?;
    let mut path = bundle_root.to_path_buf();
    let components: Vec<_> = rel.components().collect();
    let mut final_metadata = None;
    for (index, component) in components.iter().enumerate() {
        path.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(io(&format!("reading {label} metadata {}", path.display())))?;
        if metadata_is_link(&metadata) {
            return Err(ModError::Other(format!(
                "{label} path contains a symbolic link or reparse point: {}",
                path.display()
            )));
        }
        if index + 1 == components.len() {
            final_metadata = Some(metadata);
        } else if !metadata.is_dir() {
            return Err(ModError::Other(format!(
                "{label} ancestor is not a real directory: {}",
                path.display()
            )));
        }
    }
    let metadata = final_metadata.expect("safe relative paths contain a component");
    if !metadata.is_file() {
        return Err(ModError::Other(format!(
            "{label} must be a regular file: {}",
            path.display()
        )));
    }
    let canonical = std::fs::canonicalize(&path)
        .map_err(io(&format!("resolving {label} {}", path.display())))?;
    if !canonical.starts_with(&root) {
        return Err(ModError::Other(format!(
            "{label} resolves outside the bundle: {}",
            path.display()
        )));
    }
    Ok((canonical, metadata.len()))
}

fn resolve_safe_bundle_root(bundle_root: &Path) -> Result<PathBuf> {
    let metadata = std::fs::symlink_metadata(bundle_root).map_err(io(&format!(
        "reading bundle root metadata {}",
        bundle_root.display()
    )))?;
    if metadata_is_link(&metadata) || !metadata.is_dir() {
        return Err(ModError::Other(format!(
            "bundle root must be a real non-link directory: {}",
            bundle_root.display()
        )));
    }
    std::fs::canonicalize(bundle_root).map_err(io(&format!(
        "resolving bundle root {}",
        bundle_root.display()
    )))
}

fn resolve_safe_bundle_tree(bundle_root: &Path, rel: &Path, label: &str) -> Result<PathBuf> {
    let rel_text = rel.to_string_lossy();
    if !is_safe_rel_path(&rel_text) {
        return Err(ModError::Other(format!("unsafe {label} path {rel_text:?}")));
    }
    let root = resolve_safe_bundle_root(bundle_root)?;
    let mut candidate = bundle_root.to_path_buf();
    for component in rel.components() {
        candidate.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&candidate).map_err(io(&format!(
            "reading {label} metadata {}",
            candidate.display()
        )))?;
        if metadata_is_link(&metadata) || !metadata.is_dir() {
            return Err(ModError::Other(format!(
                "{label} path must contain only real directories: {}",
                candidate.display()
            )));
        }
    }
    let canonical = std::fs::canonicalize(&candidate)
        .map_err(io(&format!("resolving {label} {}", candidate.display())))?;
    if !canonical.starts_with(&root) {
        return Err(ModError::Other(format!(
            "{label} resolves outside the bundle: {}",
            candidate.display()
        )));
    }
    validate_bundle_tree_contents(&canonical, label)?;
    Ok(canonical)
}

fn validate_bundle_tree_contents(root: &Path, label: &str) -> Result<()> {
    let mut pending = vec![root.to_path_buf()];
    let mut entries = 0u64;
    let mut total_bytes = 0u64;
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)
            .map_err(io(&format!("reading {label} directory {}", dir.display())))?
        {
            let entry = entry.map_err(io(&format!("reading {label} directory entry")))?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(io(&format!(
                "reading {label} payload metadata {}",
                path.display()
            )))?;
            if metadata_is_link(&metadata) {
                return Err(ModError::Other(format!(
                    "{label} contains a symbolic link or reparse point: {}",
                    path.display()
                )));
            }
            let relative = path.strip_prefix(root).map_err(|_| {
                ModError::Other(format!("{label} path escaped its root: {}", path.display()))
            })?;
            let portable = relative
                .to_str()
                .ok_or_else(|| {
                    ModError::Other(format!("{label} path is not UTF-8: {}", path.display()))
                })?
                .replace('\\', "/");
            if !is_safe_rel_path(&portable) {
                return Err(ModError::Other(format!(
                    "{label} contains a non-portable path: {}",
                    path.display()
                )));
            }
            entries = entries
                .checked_add(1)
                .ok_or_else(|| ModError::Other(format!("{label} entry count overflow")))?;
            if entries > MAX_UE4SS_TREE_ENTRIES {
                return Err(ModError::Other(format!(
                    "{label} exceeds the {MAX_UE4SS_TREE_ENTRIES}-entry limit"
                )));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                if metadata.len() > MAX_UE4SS_FILE_BYTES {
                    return Err(ModError::Other(format!(
                        "{label} file exceeds the {MAX_UE4SS_FILE_BYTES}-byte limit: {}",
                        path.display()
                    )));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or_else(|| ModError::Other(format!("{label} byte total overflow")))?;
                if total_bytes > MAX_UE4SS_TREE_BYTES {
                    return Err(ModError::Other(format!(
                        "{label} exceeds the {MAX_UE4SS_TREE_BYTES}-byte total limit"
                    )));
                }
            } else {
                return Err(ModError::Other(format!(
                    "{label} contains a non-file payload: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

/// Read a regular, non-link file into memory without ever allocating or consuming more than the
/// declared limit. Metadata rejects an already-oversized file before opening it; `take(limit+1)`
/// closes the grow-after-metadata race and makes a one-byte-over-limit result distinguishable.
fn read_regular_file_limited(path: &Path, label: &str, max_bytes: u64) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(io(&format!("reading {label} metadata {}", path.display())))?;
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(ModError::Other(format!(
            "{label} must be a regular non-link file: {}",
            path.display()
        )));
    }
    if metadata.len() > max_bytes {
        return Err(ModError::Other(format!(
            "{label} exceeds the {max_bytes}-byte limit: {} bytes at {}",
            metadata.len(),
            path.display()
        )));
    }
    let mut file =
        std::fs::File::open(path).map_err(io(&format!("opening {label} {}", path.display())))?;
    let opened = file
        .metadata()
        .map_err(io(&format!("reading opened {label} metadata")))?;
    let rechecked = std::fs::symlink_metadata(path).map_err(io(&format!(
        "rechecking {label} metadata {}",
        path.display()
    )))?;
    if !opened.is_file()
        || opened.len() != metadata.len()
        || metadata_is_link(&rechecked)
        || !rechecked.is_file()
        || rechecked.len() != opened.len()
    {
        return Err(ModError::Other(format!(
            "{label} changed while being opened: {}",
            path.display()
        )));
    }
    let capacity = usize::try_from(opened.len()).map_err(|_| {
        ModError::Other(format!(
            "{label} is too large for this platform: {}",
            path.display()
        ))
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_| {
        ModError::Other(format!(
            "could not reserve {} bytes for {label}: {}",
            opened.len(),
            path.display()
        ))
    })?;
    let read_limit = max_bytes.saturating_add(1);
    std::io::Read::by_ref(&mut file)
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(io(&format!("reading {label} {}", path.display())))?;
    if bytes.len() as u64 > max_bytes {
        return Err(ModError::Other(format!(
            "{label} grew beyond the {max_bytes}-byte limit while being read: {}",
            path.display()
        )));
    }
    let after_handle = file
        .metadata()
        .map_err(io(&format!("rechecking opened {label} metadata")))?;
    let after_path = std::fs::symlink_metadata(path)
        .map_err(io(&format!("rechecking {label} path {}", path.display())))?;
    if bytes.len() as u64 != opened.len()
        || after_handle.len() != opened.len()
        || metadata_is_link(&after_path)
        || !after_path.is_file()
        || after_path.len() != opened.len()
    {
        return Err(ModError::Other(format!(
            "{label} changed while being read: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

fn metadata_is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

// ── Game paths ──────────────────────────────────────────────────────────────────
/// Resolved game-install locations. `root` is the game folder that contains `G1R/`.
pub struct GamePaths {
    /// The semantic install root — the directory that CONTAINS `G1R`, whether the caller passed
    /// that directory or its `G1R` child. Everything game-root-relative (loose files, `~mods`)
    /// resolves against this instead of counting ancestors off another field.
    pub root: PathBuf,
    pub executable: PathBuf,
    pub ue4ss_mods: PathBuf,
    pub fmod_desktop: PathBuf,
    pub voice_over: PathBuf,
    pub lcache: Option<PathBuf>,
    pub script_cache: PathBuf,
}

/// Normalize either an install directory or its direct `G1R` child to the semantic install root.
///
/// Windows path identity is case-insensitive, so `G1R`, `g1r`, and mixed-case spellings must all
/// mean the same direct child. This helper is lexical and intentionally does not dereference a
/// caller-configured install-root alias.
pub fn semantic_install_root(root: &Path) -> PathBuf {
    if root
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("G1R"))
    {
        root.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf())
    } else {
        root.to_path_buf()
    }
}

/// Bind the fixed VoiceOver directory without following a symlink/reparse component. A missing
/// suffix is represented as None so target discovery can keep its established unresolved result;
/// an existing non-directory or link/reparse component is always an error.
pub fn bind_voice_over_root(game_root: &Path) -> Result<Option<VoiceOverPathGuard>> {
    let semantic = semantic_install_root(game_root);
    let absolute = if semantic.is_absolute() {
        semantic
    } else {
        std::env::current_dir()
            .map_err(io("reading current directory for VoiceOver binding"))?
            .join(semantic)
    };
    let install =
        mgr::model::open_directory_chain_nofollow(&absolute, "configured game installation")?;
    let mut directory = install.clone();
    for component in ["G1R", "Story", "VoiceOver"] {
        let Some(child) = directory.open_optional_child_directory(
            std::ffi::OsStr::new(component),
            "installed VoiceOver directory",
        )?
        else {
            return Ok(None);
        };
        directory = child;
    }
    let install_root = install.path().to_path_buf();
    let directory = directory.into_rename_guard("installed VoiceOver directory")?;
    Ok(Some(VoiceOverPathGuard {
        install_root,
        directory: std::sync::Arc::new(directory),
    }))
}

pub fn resolve_game_paths(root: &Path) -> GamePaths {
    let g1r = if root
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("G1R"))
    {
        root.to_path_buf()
    } else {
        root.join("G1R")
    };
    let lcache = {
        let cache = g1r.join("Story").join("Cache");
        std::fs::read_dir(&cache).ok().and_then(|rd| {
            let mut matches: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.starts_with("AlkimiaLocalization") && n.ends_with(".lcache")
                    })
                })
                .collect();
            // Deterministic when several caches exist: pick the most recently modified (the active
            // one). Sort by PATH first, then stably by mtime, then take the last — identical to
            // gore-loc's locator, so deploy patches/backs up the SAME cache the catalog was
            // extracted from even when mtimes tie or metadata can't be read.
            matches.sort();
            matches.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
            matches.pop()
        })
    };
    GamePaths {
        root: semantic_install_root(root),
        executable: g1r
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe"),
        ue4ss_mods: g1r
            .join("Binaries")
            .join("Win64")
            .join("ue4ss")
            .join("Mods"),
        fmod_desktop: g1r.join("Content").join("FMOD").join("Desktop"),
        voice_over: g1r.join("Story").join("VoiceOver"),
        lcache,
        script_cache: g1r.join("Script").join("PrecompiledScript_Shipping.Cache"),
    }
}

// ── Deploy / undeploy ────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployPhase {
    /// Every planned write and the final deploy record were durably published.
    #[default]
    Applied,
    /// A pre-write recovery record is on disk. The process may have stopped anywhere during apply;
    /// callers must recover with undeploy instead of treating this as a completed deployment.
    RecoveryRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCleanupClaim {
    /// Unique direct child of the install root, outside every game/mod scan directory.
    pub holder: String,
    /// Exact identities accepted for the object atomically moved to `holder/claimed`.
    pub expected_hashes: Vec<String>,
    /// Authenticated backup to publish no-clobber at the original name; `None` means delete-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_hash: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeployRecord {
    /// Localization edits this deploy could not write, each naming the id and language. Carried
    /// back to the caller so it can be shown, never written to the on-disk record: it describes
    /// one run, not the deployment's state, and the record is read back by undeploy and status.
    #[serde(skip)]
    pub loc_skipped: Vec<String>,
    /// Localization edits this deploy wrote that the game will not display, because the id also
    /// carries a newer generation of the same language. Written, not skipped — the distinction
    /// decides whether a reader should fix their spec or undo their deployment.
    #[serde(skip)]
    pub loc_shadowed: Vec<String>,
    pub mod_name: String,
    /// deployed UE4SS mod dir (absolute), if any
    pub ue4ss_mod_dir: Option<String>,
    /// (live_path, backup_path, created_by_this_deploy) to restore on undeploy. `created` is
    /// false when the `*.gore-bak` already existed (it belongs to a previous deployment), so a
    /// rollback restores from but does not delete it.
    pub backups: Vec<(String, String, bool)>,
    /// Previous-deployment UE4SS mod dirs (different name from the new one) that couldn't be
    /// removed at deploy time (locked/permissions). Tracked so undeploy still cleans them up;
    /// otherwise their enabled scripts would linger. `#[serde(default)]` keeps old records loadable.
    #[serde(default)]
    pub stale_ue4ss_dirs: Vec<String>,
    /// live/additive path → hash of the bytes this deploy wrote there. In-place legacy entries use
    /// stable FNV-1a; additive files use a `sha256:`-prefixed digest for ownership. On undeploy, if
    /// current live file no longer matches, the game was updated/verified externally (e.g. Steam),
    /// so the recorded `*.gore-bak` is stale and restoring it would downgrade the newer asset —
    /// the restore is skipped instead. `#[serde(default)]` keeps old records loadable.
    #[serde(default)]
    pub deployed_hashes: BTreeMap<String, String>,
    /// Absolute `*.gore-bak` path -> SHA-256 of the pristine bytes captured there.  A backup is a
    /// restore source, not merely bookkeeping: its exact content must still be the snapshot this
    /// deployment recorded before it may be copied over a game file or removed.  Legacy records
    /// deserialize with an empty map and are handled fail-closed unless live and backup are already
    /// byte-identical.
    #[serde(default)]
    pub backup_hashes: BTreeMap<String, String>,
    /// Absolute UE4SS directory path -> deterministic SHA-256 tree fingerprint. Unlike the legacy
    /// path-only fields, this proves that cleanup still addresses the exact tree we installed.
    /// Old records deserialize with an empty map and are treated as unverifiable (never adopted).
    #[serde(default)]
    pub ue4ss_tree_fingerprints: BTreeMap<String, String>,
    /// Durable two-step directory cleanup.  `original -> holder` is written after reserving a
    /// unique absent holder, before the atomic move to that holder. Once the moved tree is
    /// verified, the holder is added to `trusted_ue4ss_tombstones`; recursive deletion may then be
    /// retried after a crash without requiring the now-partial tree to retain its original digest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ue4ss_cleanup_claims: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_ue4ss_tombstones: Vec<String>,
    /// File counterpart to the UE4SS claim state.  The record is persisted with an empty unique
    /// holder before moving a live/additive/backup file into `holder/claimed`.  A trusted holder is
    /// outside scan directories and may be deleted after crashes without touching a same-path
    /// replacement that appeared at the original name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_cleanup_claims: BTreeMap<String, FileCleanupClaim>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_file_tombstones: Vec<String>,
    /// During a recovery-required transition an existing owned target may still contain either
    /// the previous deployment or the newly intended deployment. These alternate exact identities
    /// preserve crash recovery without allowing a same-path external replacement to be removed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub recovery_file_hashes: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub recovery_tree_fingerprints: BTreeMap<String, Vec<String>>,
    /// Absolute dst paths of additive texture-override Zen triplet files written into `~mods`.
    /// These are pure additions (no in-place game-file patch, no `*.gore-bak`); undeploy simply
    /// deletes them. `#[serde(default)]` keeps old records loadable.
    #[serde(default)]
    pub texture_triplets: Vec<String>,
    /// Who owns this deployment: `""` = legacy/studio single-mod deploy, `"manager"` = the
    /// multi-mod manager. `#[serde(default)]` keeps old records loadable (as `""`).
    #[serde(default)]
    pub owner: String,
    /// Manager only: the loadout snapshot this deployment realized (library ids + enabled),
    /// so status/apply can diff the deployed state against the current loadout.
    #[serde(default)]
    pub loadout: Vec<crate::mgr::LoadoutEntry>,
    /// Manager only: ALL deployed UE4SS mod dirs (absolute). A manager deployment
    /// (`owner == "manager"`) records EVERY dir here and leaves `ue4ss_mod_dir` = None; a studio
    /// deployment (`owner == ""`) instead puts its single dir in the legacy `ue4ss_mod_dir` and
    /// leaves this empty. Undeploy removes each entry with the same semantics as `ue4ss_mod_dir`.
    #[serde(default)]
    pub ue4ss_mod_dirs: Vec<String>,
    /// Absolute dst paths of manager-installed pak/triplet files (loose paks, foreign
    /// triplets) in `~mods`. Pure additions like `texture_triplets`; undeploy deletes them.
    #[serde(default)]
    pub managed_paks: Vec<String>,
    /// Manager only: mod id → the library entry's content [`fingerprint`] at deploy time. Lets
    /// status detect a same-id UPDATE (a re-import that changed a mod's components/bytes but not
    /// its id): if a library mod's current fingerprint differs from the one recorded here, the
    /// deployed bytes are stale even though the loadout id set still matches. Old records parse as
    /// empty (`#[serde(default)]`), where a missing fingerprint reads as changed (re-apply needed).
    ///
    /// [`fingerprint`]: crate::mgr::model::ModEntryMeta::fingerprint
    #[serde(default)]
    pub deployed_fingerprints: std::collections::BTreeMap<String, String>,
    /// Durable transaction phase. Old records predate this field and therefore deserialize as the
    /// completed [`DeployPhase::Applied`] state. New commits persist `recovery_required` before any
    /// live write and clear it only in the final durable record write.
    #[serde(default)]
    pub phase: DeployPhase,
}

/// One successfully parsed on-disk record together with the exact bytes that produced it. Keeping
/// the raw representation avoids a second, racy read and lets rollback restore byte-for-byte state
/// (including whitespace and unknown forward-compatible fields) rather than reserializing it.
#[derive(Debug)]
struct StoredDeployRecord {
    record: DeployRecord,
    raw: Vec<u8>,
}

/// Stable content fingerprint for drift detection (not cryptographic — only distinguishes our own
/// deployed bytes from a later external overwrite). FNV-1a 64-bit: a fixed algorithm whose output
/// never changes across Rust/tool versions, unlike `DefaultHasher` (SipHash), so a record written
/// by one build is read back consistently by a later one.
pub(crate) fn content_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    update_content_hash(&mut h, bytes);
    format!("{h:016x}")
}

fn update_content_hash(hash: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *hash ^= byte as u64;
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
    }
}

/// Hash a file with a fixed-size buffer. Voice archives can be many GiB, so drift checks must not
/// materialize the live ZIP merely to compare it with the hash persisted in the deploy record.
fn content_hash_file(path: &Path) -> std::io::Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("not a regular non-link file: {}", path.display()),
        ));
    }
    let mut file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    // Keep the streaming buffer off the caller's stack. Windows console binaries reserve only
    // 1 MiB by default, so an equally-sized local array can overflow before the first read.
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut read_total = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        update_content_hash(&mut hash, &buffer[..read]);
        read_total = read_total.checked_add(read as u64).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "file length overflow")
        })?;
    }
    // If a Steam update races the hash, do not accept a digest of a partial/mixed generation.
    if read_total != metadata.len() || std::fs::metadata(path)?.len() != metadata.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file changed while hashing: {}", path.display()),
        ));
    }
    Ok(format!("{hash:016x}"))
}

fn sha256_file(path: &Path) -> Result<String> {
    let metadata = std::fs::symlink_metadata(path).map_err(io(&format!(
        "reading SHA-256 source metadata {}",
        path.display()
    )))?;
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(ModError::Other(format!(
            "SHA-256 source is not a regular non-link file: {}",
            path.display()
        )));
    }
    let mut file = std::fs::File::open(path)
        .map_err(io(&format!("opening SHA-256 source {}", path.display())))?;
    let opened = file
        .metadata()
        .map_err(io("reading opened SHA-256 source metadata"))?;
    let mut hasher = Sha256::new();
    // This helper is used during deploy preflight on the CLI's 1 MiB Windows main stack.
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(io("hashing SHA-256 source"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| ModError::Other("SHA-256 source length overflow".into()))?;
    }
    let after = file
        .metadata()
        .map_err(io("re-reading opened SHA-256 source metadata"))?;
    if total != opened.len() || after.len() != opened.len() || metadata.len() != opened.len() {
        return Err(ModError::Other(format!(
            "file changed while computing SHA-256: {}",
            path.display()
        )));
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub(crate) fn file_matches_recorded_hash(path: &Path, expected: &str) -> bool {
    file_matches_recorded_hash_result(path, expected).unwrap_or(false)
}

fn file_matches_recorded_hash_result(path: &Path, expected: &str) -> Result<bool> {
    if let Some(hex) = expected.strip_prefix("sha256:") {
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ModError::Other(format!(
                "invalid recorded SHA-256 identity for {}",
                path.display()
            )));
        }
        sha256_file(path).map(|current| current == expected)
    } else {
        if expected.len() != 16 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ModError::Other(format!(
                "invalid recorded legacy content hash for {}",
                path.display()
            )));
        }
        content_hash_file(path)
            .map(|current| current == expected)
            .map_err(io(&format!("hashing deployed file {}", path.display())))
    }
}

fn tree_fingerprint(root: &Path) -> Result<String> {
    tree_fingerprint_with_prefix(root, None)
}

fn tree_fingerprint_with_prefix(root: &Path, prefix: Option<&str>) -> Result<String> {
    let root_metadata = std::fs::symlink_metadata(root).map_err(io(&format!(
        "reading UE4SS tree metadata {}",
        root.display()
    )))?;
    if metadata_is_link(&root_metadata) || !root_metadata.is_dir() {
        return Err(ModError::Other(format!(
            "UE4SS identity source is not a real directory: {}",
            root.display()
        )));
    }

    #[derive(Clone)]
    struct Entry {
        relative: String,
        path: PathBuf,
        is_dir: bool,
        len: u64,
    }

    let mut entries = Vec::new();
    if let Some(prefix) = prefix {
        entries.push(Entry {
            relative: prefix.to_string(),
            path: root.to_path_buf(),
            is_dir: true,
            len: 0,
        });
    }
    let mut pending = vec![root.to_path_buf()];
    let mut total_bytes = 0u64;
    while let Some(dir) = pending.pop() {
        let read_dir = std::fs::read_dir(&dir).map_err(io(&format!(
            "reading UE4SS identity tree {}",
            dir.display()
        )))?;
        for entry in read_dir {
            let entry = entry.map_err(io("reading UE4SS identity entry"))?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(io("reading UE4SS identity entry metadata"))?;
            if metadata_is_link(&metadata) {
                return Err(ModError::Other(format!(
                    "UE4SS identity tree contains a link or reparse point: {}",
                    path.display()
                )));
            }
            let raw_relative = path.strip_prefix(root).map_err(|_| {
                ModError::Other(format!(
                    "UE4SS identity entry escaped its root: {}",
                    path.display()
                ))
            })?;
            let raw_relative = raw_relative.to_str().ok_or_else(|| {
                ModError::Other(format!(
                    "UE4SS identity entry is not UTF-8: {}",
                    path.display()
                ))
            })?;
            let relative = match prefix {
                Some(prefix) => format!("{prefix}/{}", raw_relative.replace('\\', "/")),
                None => raw_relative.replace('\\', "/"),
            };
            if !is_safe_rel_path(&relative) {
                return Err(ModError::Other(format!(
                    "UE4SS identity tree contains a non-portable path: {}",
                    path.display()
                )));
            }
            if entries.len() as u64 >= MAX_UE4SS_TREE_ENTRIES {
                return Err(ModError::Other(format!(
                    "UE4SS identity tree exceeds the {MAX_UE4SS_TREE_ENTRIES}-entry limit"
                )));
            }
            if metadata.is_dir() {
                pending.push(path.clone());
                entries.push(Entry {
                    relative,
                    path,
                    is_dir: true,
                    len: 0,
                });
            } else if metadata.is_file() {
                if metadata.len() > MAX_UE4SS_FILE_BYTES {
                    return Err(ModError::Other(format!(
                        "UE4SS identity file exceeds the {MAX_UE4SS_FILE_BYTES}-byte limit: {}",
                        path.display()
                    )));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or_else(|| ModError::Other("UE4SS identity byte total overflow".into()))?;
                if total_bytes > MAX_UE4SS_TREE_BYTES {
                    return Err(ModError::Other(format!(
                        "UE4SS identity tree exceeds the {MAX_UE4SS_TREE_BYTES}-byte total limit"
                    )));
                }
                entries.push(Entry {
                    relative,
                    path,
                    is_dir: false,
                    len: metadata.len(),
                });
            } else {
                return Err(ModError::Other(format!(
                    "UE4SS identity tree contains a non-file entry: {}",
                    path.display()
                )));
            }
        }
    }
    entries.sort_by(|left, right| left.relative.cmp(&right.relative));

    let mut hasher = Sha256::new();
    hasher.update(b"gore-ue4ss-tree-v1\0");
    for entry in entries {
        hasher.update(if entry.is_dir { b"d" } else { b"f" });
        hasher.update((entry.relative.len() as u64).to_le_bytes());
        hasher.update(entry.relative.as_bytes());
        hasher.update(entry.len.to_le_bytes());
        if !entry.is_dir {
            let expected = sha256_file(&entry.path)?;
            hasher.update(expected.as_bytes());
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Short stable hash of an arbitrary string for disambiguating filenames. FNV-1a 64-bit (same
/// fixed algorithm as [`content_hash`]) truncated to 8 hex chars: distinct mod names that sanitize
/// to the same stem (e.g. `A+B` vs `A B` -> `A_B`) get distinct triplet names because this hashes
/// the ORIGINAL (unsanitized) name.
fn name_hash(s: &str) -> String {
    content_hash(s.as_bytes())[..8].to_string()
}

/// Whether `live` should be restored from its backup: true unless we recorded what we deployed
/// there and the current file no longer matches it (external update — restoring would downgrade).
fn deployed_hash_for_path<'a>(
    live: &str,
    deployed_hashes: &'a BTreeMap<String, String>,
) -> Option<&'a String> {
    deployed_hashes.get(live).or_else(|| {
        deployed_hashes
            .iter()
            .find_map(|(stored, hash)| same_path_s(stored, live).then_some(hash))
    })
}

fn backup_hash_for_path<'a>(
    backup: &Path,
    backup_hashes: &'a BTreeMap<String, String>,
) -> Option<&'a String> {
    backup_hashes
        .iter()
        .find_map(|(stored, hash)| same_path(backup, stored).then_some(hash))
}

fn remove_backup_hash_for_path(
    backup: &Path,
    backup_hashes: &mut BTreeMap<String, String>,
) -> Option<(String, String)> {
    let key = backup_hashes
        .keys()
        .find(|stored| same_path(backup, stored))
        .cloned()?;
    backup_hashes.remove_entry(&key)
}

fn backup_matches_recorded_hash(record: &DeployRecord, backup: &Path) -> Result<bool> {
    let Some(expected) = backup_hash_for_path(backup, &record.backup_hashes) else {
        return Ok(false);
    };
    if !expected.starts_with("sha256:") {
        return Err(ModError::Other(format!(
            "invalid non-SHA-256 backup identity for {}",
            backup.display()
        )));
    }
    file_matches_recorded_hash_result(backup, expected)
}

/// Remove the drift hash belonging to `live`, preserving the exact stored key so a failed
/// cleanup can re-track it without changing path spelling (for example a Windows `\\?\` prefix).
fn remove_deployed_hash_for_path(
    live: &str,
    deployed_hashes: &mut BTreeMap<String, String>,
) -> Option<(String, String)> {
    let key = deployed_hashes
        .keys()
        .find(|stored| same_path_s(stored, live))
        .cloned()?;
    deployed_hashes.remove_entry(&key)
}

fn safe_to_restore(live: &str, record: &DeployRecord) -> Result<bool> {
    let live_path = Path::new(live);
    let live_exists = match std::fs::symlink_metadata(live_path) {
        // A game update may intentionally remove a target. Treat absence as external drift: never
        // recreate a stale file from an older backup merely because our prior deploy touched it.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(io("reading live restore metadata")(error)),
        Ok(metadata) if metadata_is_link(&metadata) || !metadata.is_file() => {
            return Err(ModError::Other(format!(
                "live restore target is not a regular non-link file: {}",
                live_path.display()
            )))
        }
        Ok(_) => true,
    };
    let backup = bak_path(live_path);
    match std::fs::symlink_metadata(&backup) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io("reading restore-backup metadata")(error)),
        Ok(metadata) if metadata_is_link(&metadata) || !metadata.is_file() => {
            return Err(ModError::Other(format!(
                "restore backup is not a regular non-link file: {}",
                backup.display()
            )))
        }
        Ok(_) => {
            if backup_hash_for_path(&backup, &record.backup_hashes).is_some() {
                if !backup_matches_recorded_hash(record, &backup)? {
                    return Err(ModError::Other(format!(
                        "restore backup content no longer matches its recorded identity: {}",
                        backup.display()
                    )));
                }
            } else if !live_exists || sha256_file(live_path)? != sha256_file(&backup)? {
                return Err(ModError::Other(format!(
                    "unverifiable legacy restore backup for '{}': no backup SHA-256 is recorded",
                    backup.display()
                )));
            }
        }
    }
    if !live_exists {
        return Ok(false);
    }
    let mut had_identity = false;
    let live_key = live_path.display().to_string();
    for expected in deployed_hash_for_path(&live_key, &record.deployed_hashes)
        .into_iter()
        .chain(
            values_for_path(live_path, &record.recovery_file_hashes)
                .into_iter()
                .flatten(),
        )
    {
        had_identity = true;
        if file_matches_recorded_hash_result(live_path, expected)? {
            return Ok(true);
        }
    }
    if had_identity {
        return Ok(false);
    }

    if sha256_file(live_path)? == sha256_file(&backup)? {
        return Ok(true);
    }
    Err(ModError::Other(format!(
        "unverifiable legacy in-place deployment for '{}': no deployed hash is recorded and live differs from its backup; refusing path-only restore",
        live_path.display()
    )))
}

fn values_for_path<'a>(
    path: &Path,
    values: &'a BTreeMap<String, Vec<String>>,
) -> Option<&'a Vec<String>> {
    values
        .iter()
        .find_map(|(stored, value)| same_path(path, stored).then_some(value))
}

fn file_cleanup_is_owned(record: &DeployRecord, path: &Path) -> bool {
    if !path_exists_no_follow(path) {
        return true;
    }
    let key = path.display().to_string();
    deployed_hash_for_path(&key, &record.deployed_hashes)
        .into_iter()
        .chain(
            values_for_path(path, &record.recovery_file_hashes)
                .into_iter()
                .flatten(),
        )
        .any(|expected| {
            expected.starts_with("sha256:") && file_matches_recorded_hash(path, expected)
        })
}

fn tree_cleanup_is_owned(record: &DeployRecord, path: &Path) -> bool {
    if !path_exists_no_follow(path) {
        return true;
    }
    tree_fingerprint_for_path(path, &record.ue4ss_tree_fingerprints)
        .into_iter()
        .chain(
            values_for_path(path, &record.recovery_tree_fingerprints)
                .into_iter()
                .flatten(),
        )
        .any(|expected| tree_matches_recorded_fingerprint(path, expected))
}

fn take_vec_map_path(
    path: &Path,
    values: &mut BTreeMap<String, Vec<String>>,
) -> Option<(String, Vec<String>)> {
    if let Some(key) = values
        .keys()
        .find(|stored| same_path(path, stored))
        .cloned()
    {
        return values.remove_entry(&key);
    }
    None
}

fn remove_vec_map_path(path: &Path, values: &mut BTreeMap<String, Vec<String>>) {
    let _ = take_vec_map_path(path, values);
}

fn remove_string_map_path(path: &Path, values: &mut BTreeMap<String, String>) {
    if let Some(key) = values
        .keys()
        .find(|stored| same_path(path, stored))
        .cloned()
    {
        values.remove(&key);
    }
}

fn prune_file_identity(record: &mut DeployRecord, path: &Path) {
    remove_string_map_path(path, &mut record.deployed_hashes);
    remove_vec_map_path(path, &mut record.recovery_file_hashes);
}

fn prune_tree_identity(record: &mut DeployRecord, path: &Path) {
    remove_string_map_path(path, &mut record.ue4ss_tree_fingerprints);
    remove_vec_map_path(path, &mut record.recovery_tree_fingerprints);
}

const RECORD_NAME: &str = "gore-mod.deployed.json";

/// This install's FMOD bank encryption key: the one gore-dump recovered into `gore_fmod_key.json`
/// (written to `Binaries/Win64`) if present and valid, else the known [`gore_fmod::GOTHIC_STUDIO_KEY`]
/// constant. The key stays constant until a game patch changes it; a user who re-dumps after such a
/// patch can then deploy audio without the build/deploy path being stuck on the old constant.
pub(crate) fn resolve_fmod_key(gp: &GamePaths) -> Vec<u8> {
    #[derive(Deserialize)]
    struct FmodKeyFile {
        #[serde(default)]
        found: bool,
        #[serde(default)]
        encryption_key: String,
    }
    // gp.ue4ss_mods == <...>/Binaries/Win64/ue4ss/Mods, so its grandparent is Binaries/Win64.
    if let Some(win64) = gp.ue4ss_mods.parent().and_then(Path::parent) {
        let key_file = win64.join("gore_fmod_key.json");
        if let Ok(bytes) = std::fs::read(&key_file) {
            if let Ok(k) = serde_json::from_slice::<FmodKeyFile>(&bytes) {
                if k.found && !k.encryption_key.is_empty() {
                    return k.encryption_key.into_bytes();
                }
            }
        }
    }
    gore_fmod::GOTHIC_STUDIO_KEY.to_vec()
}

/// Absolutize the game root so every path derived from it (live files, `*.gore-bak`, UE4SS dirs)
/// and persisted in the deploy record is absolute. Otherwise a deploy from the install dir with a
/// relative root (e.g. `--game .`) would serialize relative paths, and a later undeploy from a
/// different working directory would resolve them against the wrong tree.
fn abs_root(root: &Path) -> PathBuf {
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
}

/// Canonical install root for the deploy record. `resolve_game_paths` accepts both the install
/// dir and its `G1R` child, so normalize to the install dir (the parent of `G1R`) — otherwise a
/// deploy via `.../G1R` and an undeploy via the Steam-detected parent would use different record
/// paths, leaving the mod silently un-undeployable.
fn record_root(root: &Path) -> PathBuf {
    semantic_install_root(root)
}

fn record_path(root: &Path) -> PathBuf {
    record_root(root).join(RECORD_NAME)
}

/// Where this install's deploy record lives, for a read-only diagnosis that has to name the file
/// it is reporting on (`gore doctor`).
///
/// Exposed rather than reconstructed by the caller: the record's name and the normalization of the
/// root are the two things a second spelling would get wrong, and a diagnosis that looked for the
/// record somewhere other than where deploy writes it would report "nothing is deployed" about a
/// live deployment. Reading the record itself stays private — [`mgr::status::status`] is the
/// supported way to ask what state it describes.
pub fn deploy_record_path(root: &Path) -> PathBuf {
    record_path(root)
}

fn install_compile_state_detail(state: &gore_as::compile::InstallCompileStateProbe) -> String {
    use gore_as::compile::InstallCompileStateDisposition;

    match state.disposition {
        InstallCompileStateDisposition::SafeToCompile => "safe".into(),
        InstallCompileStateDisposition::GameProcessRunning => {
            "G1R-Win64-Shipping.exe is running; close the game before changing its installation"
                .into()
        }
        InstallCompileStateDisposition::RecoveryArtifactsPresent => state
            .artifacts
            .first()
            .map(|artifact| format!("{:?} at {}", artifact.kind, artifact.path))
            .unwrap_or_else(|| "an AngelScript compile/recovery artifact is present".into()),
        InstallCompileStateDisposition::InspectionFailed => state
            .issues
            .first()
            .map(|issue| match &issue.path {
                Some(path) => format!("{:?} for {path}: {}", issue.kind, issue.message),
                None => format!("{:?}: {}", issue.kind, issue.message),
            })
            .unwrap_or_else(|| "the install state could not be inspected safely".into()),
    }
}

/// Acquire cross-tool ownership only after the read-only gore-as process/recovery probe says the
/// install is safe. Both functions normalize `G1R/` callers to the same semantic parent so gore-as
/// compile, single-mod deploy, manager apply, and undeploy contend on exactly one lock path.
/// Whether this build asks the operating system whether the game is running.
///
/// Production does. A test build answers "closed" unless a test says otherwise, because these
/// fixtures are temporary directories: the installed game cannot be running *in* one of them, so
/// "nothing has this tree open" is the honest answer rather than a convenient one. Asking the real
/// process list here would make every deploy and apply test depend on whether a developer happens
/// to have Gothic open while the suite runs — a fact about the desktop, not about the transaction
/// being tested. A test that is *about* the refusal states it, with [`StatedGameProcess`].
#[cfg(not(test))]
fn probe_install_state(install_root: &Path) -> gore_as::compile::InstallCompileStateProbe {
    gore_as::compile::probe_install_compile_state(install_root)
}

#[cfg(test)]
thread_local! {
    static STATED_GAME_PROCESS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn probe_install_state(install_root: &Path) -> gore_as::compile::InstallCompileStateProbe {
    let running = STATED_GAME_PROCESS.with(|stated| stated.get());
    gore_as::compile::probe_install_compile_state_with_stated_game_process(install_root, move || {
        Ok(running)
    })
}

/// States, for the rest of this test, that the game is running. Restores the previous answer on
/// drop so one test cannot leak its premise into the next.
#[cfg(test)]
struct StatedGameProcess(bool);

#[cfg(test)]
impl StatedGameProcess {
    fn running() -> Self {
        Self(STATED_GAME_PROCESS.with(|stated| stated.replace(true)))
    }
}

#[cfg(test)]
impl Drop for StatedGameProcess {
    fn drop(&mut self) {
        STATED_GAME_PROCESS.with(|stated| stated.set(self.0));
    }
}

fn acquire_live_install_mutation(
    game_root: &Path,
    owner: &str,
) -> Result<gore_as::compile::InstallMutationGuard> {
    let install_root = record_root(game_root);
    let state = probe_install_state(&install_root);
    if !state.safe_to_compile {
        return Err(ModError::Other(format!(
            "INSTALL_MUTATION_BLOCKED: {}",
            install_compile_state_detail(&state)
        )));
    }
    gore_as::compile::InstallMutationGuard::acquire(&install_root, owner).map_err(|error| {
        ModError::Other(format!(
            "INSTALL_MUTATION_BLOCKED: acquiring cross-tool install ownership: {error}"
        ))
    })
}

fn finish_live_install_mutation<T>(
    result: Result<T>,
    mut guard: gore_as::compile::InstallMutationGuard,
) -> Result<T> {
    let release = guard.release();
    match (result, release) {
        (result, Ok(())) => result,
        (primary, Err(release)) => {
            let blocker = guard.path().to_path_buf();
            let blocker_state = match std::fs::symlink_metadata(&blocker) {
                Ok(_) => Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            };
            // Do not let Drop silently retry and erase the only durable evidence behind a
            // recovery-required response. A missing/replaced pathname is classified separately
            // below and the exact owned handle is still closed without deleting another object.
            guard.preserve_for_manual_recovery();
            let release = match blocker_state {
                Ok(true) => format!(
                    "INSTALL_MUTATION_RECOVERY_REQUIRED: releasing cross-tool ownership failed; \
                     the persistent blocker remains at {}: {release}",
                    blocker.display()
                ),
                Ok(false) => format!(
                    "releasing cross-tool ownership failed, but its pathname is absent; no \
                     recovery blocker was retained: {release}"
                ),
                Err(inspect) => format!(
                    "INSTALL_MUTATION_RECOVERY_REQUIRED: releasing cross-tool ownership failed \
                     and the blocker state at {} could not be inspected: {release}; {inspect}",
                    blocker.display()
                ),
            };
            match primary {
                Ok(_) => Err(ModError::Other(format!(
                    "installation mutation completed, but {release}"
                ))),
                Err(primary) => Err(ModError::Other(format!(
                    "{primary}; additionally, {release}"
                ))),
            }
        }
    }
}

/// A fully-prepared deployment: everything to write, computed in memory so the failure-prone
/// work happens BEFORE the game is touched. The single-bundle [`deploy`] fills 0/1 `ue4ss_dirs`
/// entries and no `managed_paks`; the multi-mod manager composes several of each.
#[derive(Debug, Default)]
pub(crate) struct DeployPlan {
    /// (source dir on disk, dest under ue4ss/Mods) — each installed via staged swap.
    pub(crate) ue4ss_dirs: Vec<(PathBuf, PathBuf)>,
    /// (live game file, new contents)
    pub(crate) writes: Vec<(PathBuf, Vec<u8>)>,
    /// Large verified candidates kept on disk rather than retained as archive-sized `Vec`s.
    file_writes: Vec<DiskWrite>,
    /// Live files whose preserved `*.gore-bak` is stale because the file drifted (game updated)
    /// since we deployed: stage must drop that backup so it re-snapshots the current pristine.
    pub(crate) refresh_baks: Vec<PathBuf>,
    /// Additive texture-override Zen triplet files to copy: (src triplet file in temp, dst in
    /// `~mods`). No backup — undeploy deletes the dst.
    pub(crate) texture_triplets: Vec<(PathBuf, PathBuf)>,
    /// Manager-installed pak/triplet files to copy: (src in the mod library, dst in `~mods`).
    /// Pure additions like `texture_triplets` (no backup; undeploy deletes the dst), but the
    /// srcs are durable library files, never temp dirs to clean up.
    pub(crate) managed_paks: Vec<(PathBuf, PathBuf)>,
    /// Unique temporary roots backing prepared texture/PakFilePatch sources. Retaining their
    /// guards in the plan keeps every source alive through commit while preventing concurrent
    /// prepares in this process from deleting one another. Drop cleans every root on all exits.
    pub(crate) temporary_roots: Vec<tempfile::TempDir>,
    /// Exact commit-time identities prepared from the source and, for a replacement, verified
    /// against the prior record. Filled by `commit_plan`, never by bundle parsing.
    additive_identities: BTreeMap<PathBuf, PlannedIdentity>,
    ue4ss_identities: BTreeMap<PathBuf, PlannedIdentity>,
    /// Format-3 Voice generation retained across prepare for the final pre-mutation check.
    voice_executable_generation: Option<VoiceExecutableGenerationSeal>,
    /// Retained component-wise no-follow binding of the fixed VoiceOver directory. This survives
    /// prepare so commit can reject an identity replacement before any game mutation.
    voice_over_guard: Option<VoiceOverPathGuard>,
    /// Localization edits that could not be written: the id carries no slot for that language.
    /// Reported after a successful deploy rather than refused, so one unusable edit in a large
    /// bundle does not block the rest.
    pub(crate) loc_skipped: Vec<String>,
    /// Localization edits that WERE written and will not be seen, because the id also carries a
    /// newer generation of the same language and the game reads that one.
    ///
    /// Kept apart from [`Self::loc_skipped`] deliberately. Counting them together reported a
    /// landed edit as one that "did not apply", which invites somebody to undo a deployment that
    /// worked — and one edit can raise both, so even the count was wrong.
    pub(crate) loc_shadowed: Vec<String>,
}

#[derive(Debug, Clone)]
struct PlannedIdentity {
    intended: String,
    previous: Option<String>,
}

#[derive(Debug)]
struct DiskWrite {
    live: PathBuf,
    candidate: tempfile::TempPath,
    hash: String,
}

impl DeployPlan {
    /// Dst paths of every UE4SS mod dir this plan installs.
    fn ue4ss_dsts(&self) -> Vec<String> {
        self.ue4ss_dirs
            .iter()
            .map(|(_, dst)| dst.display().to_string())
            .collect()
    }
    /// Dst paths of every ADDITIVE `~mods` file this plan installs — texture triplets AND manager
    /// paks together. Prev-vs-new reconciliation (pre-seed + retire) must treat these as one set:
    /// a manager deploy mirrors its `managed_paks` into the legacy `texture_triplets` record field,
    /// so a prev entry recorded under either field must be considered "still installed" if THIS
    /// plan re-creates that path under either kind — otherwise retire would delete a file this
    /// deploy just wrote.
    fn additive_dsts(&self) -> Vec<String> {
        self.texture_triplets
            .iter()
            .chain(self.managed_paks.iter())
            .map(|(_, dst)| dst.display().to_string())
            .collect()
    }
}

/// Select the pristine source for a voice ZIP without loading it into memory. Unlike FMOD banks,
/// ZIPs have no cheap `is_pristine_bank` signal: with an existing backup and no recorded hash the
/// caller has already required an untracked backup to be byte-identical, so the backup is safe.
/// A recorded hash is checked by streaming the live archive; mismatch means a game update and the
/// updated live ZIP becomes the new pristine source.
fn pristine_voice_source(live: &Path, prev: Option<&DeployRecord>) -> Result<(PathBuf, bool)> {
    ensure_pristine_sources_bounded(live, gore_vo::Limits::default().max_archive_bytes)?;
    let source = select_pristine_source(live, prev)?;
    Ok((source.path, source.drifted))
}

/// Resolve one Voice archive to the same authenticated pristine source used by deployment.
///
/// This is deliberately read-only: it never creates a backup or changes the active record. An
/// interrupted deployment, unsafe archive spelling, untrusted/differing legacy backup, missing
/// authenticated active backup, link/reparse object, or oversized source fails closed. The
/// returned path is suitable for immediate inspection; consumers should retain their own
/// no-follow/path-identity guard while opening it.
pub fn resolve_pristine_voice_archive_for_inspection(
    game_root: &Path,
    archive: &str,
) -> Result<VoiceArchiveInspectionSource> {
    let guard = bind_voice_over_root(game_root)?.ok_or_else(|| {
        ModError::Other(format!(
            "installed VoiceOver directory is missing below {}",
            semantic_install_root(game_root).display()
        ))
    })?;
    guard.resolve_pristine_archive(archive)
}

fn resolve_pristine_voice_archive_with_guard(
    guard: &VoiceOverPathGuard,
    archive: &str,
) -> Result<VoiceArchiveInspectionSource> {
    if !is_safe_voice_archive(archive) {
        return Err(ModError::Other(format!(
            "unsafe voice archive name {archive:?}: expected one .zip filename"
        )));
    }
    let stored = read_record(&guard.install_root)?;
    let prior = stored.as_ref().map(|value| &value.record);
    if prior.is_some_and(|record| record.phase == DeployPhase::RecoveryRequired) {
        return Err(recovery_required_error());
    }
    let live = guard.path().join(archive);
    let (path, drifted) = pristine_voice_source(&live, prior)?;
    let metadata = std::fs::symlink_metadata(&path).map_err(io(&format!(
        "reading selected pristine Voice archive metadata {}",
        path.display()
    )))?;
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(ModError::Other(format!(
            "selected pristine Voice archive is not a regular non-link file: {}",
            path.display()
        )));
    }
    let rebound = bind_voice_over_root(&guard.install_root)?
        .ok_or_else(|| ModError::Other("installed VoiceOver directory disappeared".into()))?;
    if !guard.same_identity(&rebound) {
        return Err(ModError::Other(
            "installed VoiceOver directory changed identity during pristine selection".into(),
        ));
    }
    Ok(VoiceArchiveInspectionSource { path, drifted })
}

fn sealed_voice_archive_identity(
    archive_name: &str,
    edits: &[&PendingVoiceEdit],
) -> Result<Option<gore_vo::ArchiveSeal>> {
    let sealed_count = edits
        .iter()
        .filter(|edit| edit.observation.is_some())
        .count();
    if sealed_count == 0 {
        return Ok(None);
    }
    if sealed_count != edits.len() {
        return Err(ModError::Other(format!(
            "voice edits for archive {archive_name:?} mix sealed and legacy observations"
        )));
    }

    let first = edits[0]
        .observation
        .as_ref()
        .expect("a fully sealed edit group has an observation");
    let expected = gore_vo::ArchiveSeal {
        size: first.archive_size,
        sha256: parse_voice_archive_sha256(&first.archive_sha256)?,
    };
    for edit in &edits[1..] {
        let observation = edit
            .observation
            .as_ref()
            .expect("a fully sealed edit group has an observation");
        let seal = gore_vo::ArchiveSeal {
            size: observation.archive_size,
            sha256: parse_voice_archive_sha256(&observation.archive_sha256)?,
        };
        if seal != expected {
            return Err(ModError::Other(format!(
                "sealed voice edits for archive {archive_name:?} disagree on the pristine archive identity"
            )));
        }
    }
    Ok(Some(expected))
}

fn enforce_voice_member_proofs(
    archive_name: &str,
    archive: &gore_vo::ArchiveIndex,
    edits: &[&PendingVoiceEdit],
) -> Result<()> {
    for edit in edits {
        let Some(observation) = &edit.observation else {
            continue;
        };
        match &observation.member_proof {
            VoiceMemberProof::Absent => {
                if archive
                    .entries()
                    .iter()
                    .any(|entry| voice_key(&entry.path) == voice_key(&edit.archive_path))
                {
                    return Err(ModError::Other(format!(
                        "sealed voice member proof mismatch in archive {archive_name:?}: {:?} was expected to be absent",
                        edit.archive_path
                    )));
                }
            }
            VoiceMemberProof::Present {
                uncompressed_size,
                crc32,
            } => {
                let mut matches = archive
                    .entries()
                    .iter()
                    .filter(|entry| entry.path == edit.archive_path);
                let Some(entry) = matches.next() else {
                    return Err(ModError::Other(format!(
                        "sealed voice member proof mismatch in archive {archive_name:?}: exact member {:?} is missing",
                        edit.archive_path
                    )));
                };
                if entry.is_directory
                    || entry.is_symlink
                    || entry.encrypted
                    || !matches!(
                        entry.compression,
                        zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
                    )
                {
                    return Err(ModError::Other(format!(
                        "sealed voice member proof mismatch in archive {archive_name:?}: exact member {:?} is not an eligible regular Ogg entry",
                        edit.archive_path
                    )));
                }
                if matches.next().is_some()
                    || entry.uncompressed_size != *uncompressed_size
                    || entry.crc32 != *crc32
                {
                    return Err(ModError::Other(format!(
                        "sealed voice member proof mismatch in archive {archive_name:?}: exact member {:?} metadata changed",
                        edit.archive_path
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Materialize every effective voice edit into exactly one verified disk-backed rewrite per archive.
/// Missing archives are a hard error: silently skipping one would create a partial voice patch.
/// The source is always the same base selected by [`select_pristine_source`].
pub(crate) fn prepare_voice_archive_writes(
    pending: &PendingVoiceEdits,
    gp: &GamePaths,
    prev: Option<&DeployRecord>,
    plan: &mut DeployPlan,
) -> Result<()> {
    if pending.edits.is_empty() {
        return Ok(());
    }
    let install_root =
        gp.voice_over.ancestors().nth(3).ok_or_else(|| {
            ModError::Other("installed VoiceOver path has no install root".into())
        })?;
    let voice_over_guard = bind_voice_over_root(install_root)?.ok_or_else(|| {
        ModError::Other(format!(
            "installed VoiceOver directory is missing: {}",
            gp.voice_over.display()
        ))
    })?;
    match &plan.voice_over_guard {
        None => plan.voice_over_guard = Some(voice_over_guard.clone()),
        Some(existing) if existing.same_identity(&voice_over_guard) => {}
        Some(_) => {
            return Err(ModError::Other(
                "deploy plan contains conflicting VoiceOver directory identities".into(),
            ))
        }
    }
    if let Some(generation) = &pending.executable_generation {
        require_live_voice_executable_generation(gp, generation)?;
        match &plan.voice_executable_generation {
            None => plan.voice_executable_generation = Some(generation.clone()),
            Some(existing) if existing == generation => {}
            Some(_) => {
                return Err(ModError::Other(
                    "deploy plan contains conflicting format-3 Voice executable generations".into(),
                ));
            }
        }
    }
    let mut by_archive: BTreeMap<String, Vec<&PendingVoiceEdit>> = BTreeMap::new();
    for ((archive_key, _), edit) in &pending.edits {
        by_archive
            .entry(archive_key.clone())
            .or_default()
            .push(edit);
    }

    for (_archive_key, mut edits) in by_archive {
        edits.sort_by_key(|edit| edit.order);
        let archive_name = &edits
            .iter()
            .max_by_key(|edit| edit.order)
            .expect("a grouped archive has at least one edit")
            .archive;
        let live = voice_over_guard.path().join(archive_name);
        let live_metadata = std::fs::symlink_metadata(&live).map_err(|_| {
            ModError::Other(format!(
                "voice archive target is missing; refusing a partial voice patch: {}",
                live.display()
            ))
        })?;
        if metadata_is_link(&live_metadata) || !live_metadata.is_file() {
            return Err(ModError::Other(format!(
                "voice archive target is not a safe regular file; refusing a partial voice patch: {}",
                live.display()
            )));
        }
        ensure_pristine_sources_bounded(&live, gore_vo::Limits::default().max_archive_bytes)?;

        let backup = bak_path(&live);
        match std::fs::symlink_metadata(&backup) {
            Ok(backup_metadata) => {
                if metadata_is_link(&backup_metadata) || !backup_metadata.is_file() {
                    return Err(ModError::Other(format!(
                        "voice archive backup is not a safe regular file: {}",
                        backup.display()
                    )));
                }
                let tracked = prev.is_some_and(|record| {
                    record
                        .backups
                        .iter()
                        .any(|(stored_live, stored_backup, _)| {
                            same_path(&live, stored_live) && same_path(&backup, stored_backup)
                        })
                });
                // A differing, untracked backup has no trustworthy freshness relation to the live
                // archive (typical after a lost record or Steam hotfix). Never guess that it is
                // pristine and risk downgrading the current game. An identical orphan is harmless
                // and may be adopted by the new record.
                if !tracked && !files_equal(&live, &backup)? {
                    return Err(ModError::Other(format!(
                        "voice archive has an untracked backup that differs from the live file; \
                         refusing to guess which is pristine: {}",
                        backup.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io("reading voice backup metadata")(error)),
        }

        let (source, drifted) = pristine_voice_source(&live, prev)?;
        let source_metadata = std::fs::symlink_metadata(&source).map_err(io(&format!(
            "reading voice archive base metadata {}",
            source.display()
        )))?;
        if metadata_is_link(&source_metadata) || !source_metadata.is_file() {
            return Err(ModError::Other(format!(
                "voice archive base must be a regular non-link file: {}",
                source.display()
            )));
        }
        // Conservative disk preflight: the existing ZIP already contains all untouched bytes;
        // adding every edited Ogg length again (even replacements) plus 1 KiB of metadata per edit
        // overestimates the candidate. The actual candidate is streamed to a private temp file.
        let edit_bytes = edits.iter().try_fold(0u64, |total, edit| {
            total
                .checked_add(edit.ogg.len() as u64)
                .and_then(|value| value.checked_add(1024))
                .ok_or_else(|| ModError::Other("voice rewrite size overflow".into()))
        })?;
        let projected = source_metadata
            .len()
            .checked_add(edit_bytes)
            .ok_or_else(|| ModError::Other("voice rewrite size overflow".into()))?;
        let voice_limits = gore_vo::Limits::default();
        if projected > voice_limits.max_archive_bytes {
            return Err(ModError::Other(format!(
                "voice rewrite could exceed the {}-byte archive limit before composition",
                voice_limits.max_archive_bytes
            )));
        }
        // gore-vo hashes the selected source before and after composition, so subsequent drift is
        // rejected without first retaining a second full archive buffer here. Sealed format-2
        // edits additionally bind the first open to the exact authoring-time archive snapshot.
        let expected_seal = sealed_voice_archive_identity(archive_name, &edits)?;
        let archive = if let Some(expected_seal) = expected_seal {
            gore_vo::ArchiveIndex::open_with_expected_seal(&source, voice_limits, expected_seal)
        } else {
            gore_vo::ArchiveIndex::open(&source, voice_limits)
        }
        .map_err(|e| ModError::Voice(format!("{}: {e}", source.display())))?;
        enforce_voice_member_proofs(archive_name, &archive, &edits)?;
        let archive_edits: Vec<gore_vo::ArchiveEdit<'_>> = edits
            .iter()
            .map(|edit| match edit.op {
                VoicePatchOp::Add => gore_vo::ArchiveEdit::Add {
                    path: &edit.archive_path,
                    ogg: &edit.ogg,
                },
                VoicePatchOp::Replace => gore_vo::ArchiveEdit::Replace {
                    basename: &edit.archive_path,
                    exact_path: Some(&edit.archive_path),
                    ogg: &edit.ogg,
                },
            })
            .collect();
        let (candidate, _) = archive
            .rewrite_edits_to_temp(archive_edits)
            .map_err(|e| ModError::Voice(format!("{}: {e}", live.display())))?;
        let hash = content_hash_file(&candidate).map_err(io(&format!(
            "hashing voice candidate for {}",
            live.display()
        )))?;
        if drifted {
            plan.refresh_baks.push(live.clone());
        }
        plan.file_writes.push(DiskWrite {
            live,
            candidate,
            hash,
        });
    }
    Ok(())
}

/// First dst path that two entries of `plan` share (in-place writes, UE4SS dirs, texture triplets,
/// or manager paks), compared with `same_path` semantics; `None` if all dsts are distinct. Used to
/// reject a self-colliding plan before any game write.
pub(crate) fn first_duplicate_dst(plan: &DeployPlan) -> Option<String> {
    let mut dsts: Vec<String> = plan
        .writes
        .iter()
        .map(|(dst, _)| dst.display().to_string())
        .collect();
    dsts.extend(
        plan.file_writes
            .iter()
            .map(|write| write.live.display().to_string()),
    );
    dsts.extend(
        plan.ue4ss_dirs
            .iter()
            .chain(plan.texture_triplets.iter())
            .chain(plan.managed_paks.iter())
            .map(|(_, dst)| dst.display().to_string()),
    );
    for (i, d) in dsts.iter().enumerate() {
        if dsts[..i].iter().any(|prev| same_path_s(prev, d)) {
            return Some(d.clone());
        }
    }
    None
}

fn tree_fingerprint_for_path<'a>(
    path: &Path,
    fingerprints: &'a BTreeMap<String, String>,
) -> Option<&'a String> {
    fingerprints
        .iter()
        .find_map(|(stored, fingerprint)| same_path(path, stored).then_some(fingerprint))
}

/// Resolve exact source identities and prove every existing destination still contains the prior
/// bytes/tree. Path membership alone is not ownership: an external same-path replacement after a
/// deploy must be treated as unowned and left untouched.
fn prepare_target_identities(plan: &mut DeployPlan, prior: Option<&DeployRecord>) -> Result<()> {
    for (src, dst) in &plan.ue4ss_dirs {
        let intended = tree_fingerprint(src)?;
        let mut previous = None;
        match std::fs::symlink_metadata(dst) {
            Ok(metadata) => {
                if metadata_is_link(&metadata) || !metadata.is_dir() {
                    return Err(ModError::Other(format!(
                        "existing UE4SS target is not a safe real directory: {}",
                        dst.display()
                    )));
                }
                let expected = prior
                    .and_then(|record| {
                        tree_fingerprint_for_path(dst, &record.ue4ss_tree_fingerprints)
                    })
                    .ok_or_else(|| {
                        ModError::Other(format!(
                            "refusing to overwrite unowned existing UE4SS target: {}",
                            dst.display()
                        ))
                    })?;
                let current = tree_fingerprint(dst)?;
                if &current != expected {
                    return Err(ModError::Other(format!(
                        "refusing to overwrite externally changed UE4SS target: {}",
                        dst.display()
                    )));
                }
                previous = Some(expected.clone());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io("reading existing UE4SS target metadata")(error)),
        }
        plan.ue4ss_identities
            .insert(dst.clone(), PlannedIdentity { intended, previous });
    }
    for (src, dst) in plan.texture_triplets.iter().chain(plan.managed_paks.iter()) {
        let intended = sha256_file(src)?;
        let mut previous = None;
        match std::fs::symlink_metadata(dst) {
            Ok(metadata) => {
                if metadata_is_link(&metadata) || !metadata.is_file() {
                    return Err(ModError::Other(format!(
                        "existing additive target is not a safe regular file: {}",
                        dst.display()
                    )));
                }
                let key = dst.display().to_string();
                let expected = prior
                    .and_then(|record| deployed_hash_for_path(&key, &record.deployed_hashes))
                    .filter(|hash| hash.starts_with("sha256:"))
                    .ok_or_else(|| {
                        ModError::Other(format!(
                            "refusing to overwrite unowned existing additive target: {}",
                            dst.display()
                        ))
                    })?;
                if !file_matches_recorded_hash(dst, expected) {
                    return Err(ModError::Other(format!(
                        "refusing to overwrite externally changed additive target: {}",
                        dst.display()
                    )));
                }
                previous = Some(expected.clone());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io("reading existing additive target metadata")(error)),
        }
        plan.additive_identities
            .insert(dst.clone(), PlannedIdentity { intended, previous });
    }
    Ok(())
}

/// Deploy a built bundle dir into the game at `game_root`. Two phases so a previous working
/// deployment is never lost to a failed new one:
/// 1. **prepare** — decode/inject/encode every change in memory; on any error the game is
///    untouched and the previous mod stays active.
/// 2. **commit** — revert the previous mod's footprint this deploy won't overwrite, then apply
///    (fs ops only); if a commit write fails, the partial deploy is rolled back to pristine.
///
/// Single active mod.
pub fn deploy(bundle_dir: &Path, game_root: &Path) -> Result<DeployRecord> {
    let manifest_bytes = read_safe_bundle_file(
        bundle_dir,
        Path::new("gore-mod.json"),
        "gore-mod.json",
        MAX_BUNDLE_MANIFEST_BYTES,
    )?;
    let manifest: ModManifest = serde_json::from_slice(&manifest_bytes)?;
    validate_mod_manifest_format(&manifest)?;
    // An empty bundle has nothing to apply; deploying it would only retire the active mod.
    if manifest.components.is_empty() {
        return Err(ModError::Other("bundle has no components to deploy".into()));
    }
    // Absolutize up front so every persisted path (record location, backups, UE4SS dirs) is
    // absolute and resolvable from any later working directory.
    let game_root = &abs_root(game_root);
    let gp = resolve_game_paths(game_root);

    // The previous deployment's record — used both to detect externally-updated (drifted) files
    // during prepare and to fold its leftovers during commit.
    let prev = read_record(game_root)?;
    let prior = prev.as_ref().map(|stored| &stored.record);

    // A manager-owned deployment composes MULTIPLE mods; letting the single-mod deploy fold it
    // away would silently retire the whole loadout. Refuse before any prepare/commit work.
    if prior.is_some_and(|p| p.phase == DeployPhase::RecoveryRequired) {
        return Err(recovery_required_error());
    }
    if prior.is_some_and(|p| p.owner == "manager") {
        return Err(ModError::Other(
            "manager loadout active — use gore mod-manager to change deployments \
             (undeploy there first)"
                .into(),
        ));
    }

    // PHASE 1 — prepare (no game writes). The previous deployment is left intact if this fails.
    let plan = prepare(bundle_dir, &manifest, &gp, prior)?;

    // PHASE 2 — commit.
    let record = DeployRecord {
        mod_name: manifest.mod_meta.name.clone(),
        ..Default::default()
    };
    // Carried across the commit because the plan is consumed there, and only reattached on
    // success: a deploy that failed has nothing to report about edits it never applied.
    let loc_skipped = plan.loc_skipped.clone();
    let loc_shadowed = plan.loc_shadowed.clone();
    let mut committed = commit_plan(&gp, game_root, plan, record, prev)?;
    committed.loc_skipped = loc_skipped;
    committed.loc_shadowed = loc_shadowed;
    Ok(committed)
}

/// Commit a prepared [`DeployPlan`]: stage backups, atomically persist a recovery record BEFORE
/// any live write, apply the writes, then retire the previous deployment's leftover footprint.
/// `record` is the caller-built record for THIS deployment (identity fields like `mod_name`/
/// `owner`/`loadout` already set); `prev` is the record the plan was prepared against, whose
/// not-overwritten leftovers/stale dirs/additive files are folded in here. A returned failure
/// attempts to restore the exact prior state and record; any failed inverse operation is reported
/// as `ROLLBACK_INCOMPLETE` and its disk snapshot is retained. This multi-target operation has no
/// write-ahead journal: an abrupt process/OS crash is recoverable to pristine via the record, but
/// is not promised to recreate the exact previously-active loadout automatically.
pub(crate) fn commit_plan(
    gp: &GamePaths,
    abs_root: &Path,
    plan: DeployPlan,
    record: DeployRecord,
    prev: Option<StoredDeployRecord>,
) -> Result<DeployRecord> {
    let mutation = acquire_live_install_mutation(abs_root, "gore-mod:deploy")?;
    let result = (|| {
        verify_deploy_record_basis(abs_root, prev.as_ref())?;
        if prev
            .as_ref()
            .is_some_and(|stored| stored.record.phase == DeployPhase::RecoveryRequired)
        {
            return Err(recovery_required_error());
        }
        commit_plan_guarded(gp, abs_root, plan, record, prev)
    })();
    finish_live_install_mutation(result, mutation)
}

/// Re-read the active record only after cross-tool ownership is held. Plans are intentionally built
/// outside the mutation lock; exact raw-byte comparison prevents a second deploy that completed
/// during that prepare window from being replaced using a stale `prev` basis.
fn verify_deploy_record_basis(
    game_root: &Path,
    expected: Option<&StoredDeployRecord>,
) -> Result<()> {
    let current = read_record(game_root)?;
    let unchanged = match (expected, current.as_ref()) {
        (None, None) => true,
        (Some(expected), Some(current)) => expected.raw == current.raw,
        _ => false,
    };
    if unchanged {
        Ok(())
    } else {
        Err(ModError::Other(
            "DEPLOY_BASIS_CHANGED: the active deploy record changed while the new deployment was \
             being prepared; rebuild the plan and retry"
                .into(),
        ))
    }
}

fn commit_plan_guarded(
    gp: &GamePaths,
    abs_root: &Path,
    mut plan: DeployPlan,
    mut record: DeployRecord,
    prev: Option<StoredDeployRecord>,
) -> Result<DeployRecord> {
    let game_root = abs_root;
    let prior = prev.as_ref().map(|stored| &stored.record);
    let prev_record_bytes = prev.as_ref().map(|stored| stored.raw.as_slice());

    // Reject a plan that would write the SAME destination twice (across in-place writes, UE4SS
    // dirs, texture triplets, and manager paks). Two components/mods targeting one path would race in
    // `apply_writes`, and the undo/retire bookkeeping (which keys off the dst) would double-track
    // or mis-restore it. Compare with `same_path` semantics so `\\?\C:\..` vs the plain form (and
    // Windows case differences) still count as the same target. This runs BEFORE `stage`, so on a
    // duplicate the game is untouched.
    if let Some(dup) = first_duplicate_dst(&plan) {
        return Err(ModError::Other(format!("duplicate deploy target: {dup}")));
    }
    prepare_target_identities(&mut plan, prior)?;

    // Recovery records are persisted before live writes. Record both the exact previous identity
    // (when replacing an active deployment) and the exact intended identity so a crash on either
    // side of an atomic publication can still restore from backup without falling back to unsafe
    // path-only ownership.
    for (live, bytes) in &plan.writes {
        let key = live.display().to_string();
        let accepted = record.recovery_file_hashes.entry(key.clone()).or_default();
        if let Some(previous) =
            prior.and_then(|prior| deployed_hash_for_path(&key, &prior.deployed_hashes).cloned())
        {
            accepted.push(previous);
        }
        accepted.push(content_hash(bytes));
        accepted.sort();
        accepted.dedup();
    }
    for write in &plan.file_writes {
        let key = write.live.display().to_string();
        let accepted = record.recovery_file_hashes.entry(key.clone()).or_default();
        if let Some(previous) =
            prior.and_then(|prior| deployed_hash_for_path(&key, &prior.deployed_hashes).cloned())
        {
            accepted.push(previous);
        }
        accepted.push(write.hash.clone());
        accepted.sort();
        accepted.dedup();
    }

    // `undo` captures the exact pre-deploy state for an in-process rollback;
    // the record is persisted BEFORE any live write so even a crash mid-write is recoverable.
    let mut undo = Undo::default();

    // (a) Stage: snapshot prior bytes + create every *.gore-bak, and note the intended UE4SS
    //     target — but do NOT write any live game file yet.
    if let Some(expected) = &plan.voice_over_guard {
        let actual = bind_voice_over_root(&expected.install_root)?
            .ok_or_else(|| ModError::Other("installed VoiceOver directory disappeared".into()))?;
        if !expected.same_identity(&actual) {
            return Err(ModError::Other(
                "installed VoiceOver directory changed identity before deployment".into(),
            ));
        }
    }
    if let Some(generation) = &plan.voice_executable_generation {
        require_live_voice_executable_generation(gp, generation)?;
    }
    // Re-enumerate after shared ownership and immediately before `stage` creates the first backup.
    // This narrows but cannot eliminate a later game-launch race because the game does not honor
    // gore's lock file; gore-as documents the same limitation on this shared native check.
    gore_as::compile::require_shipping_game_process_closed().map_err(|error| {
        ModError::Other(format!(
            "INSTALL_MUTATION_BLOCKED: final pre-write process check: {error}"
        ))
    })?;
    if let Err(e) = stage(&plan, &mut record, &mut undo) {
        return Err(with_rollback_failures(e, undo.rollback()));
    }

    // (b) Fold the previous mod's not-overwritten loose files into the record, then persist the
    //     record BEFORE touching live files. A crash after this point is recoverable via
    //     undeploy; a write failure here rolls back and restores the previous record.
    let leftovers: Vec<(String, String, bool)> = prior
        .map(|p| {
            p.backups
                .iter()
                .filter(|(live, _, _)| {
                    !plan.writes.iter().any(|(path, _)| same_path(path, live))
                        && !plan
                            .file_writes
                            .iter()
                            .any(|write| same_path(&write.live, live))
                })
                .map(|(l, b, _)| (l.clone(), b.clone(), false))
                .collect()
        })
        .unwrap_or_default();
    record.backups.extend(leftovers.iter().cloned());

    // Carry the previous deploy's drift hashes for the leftover (not-overwritten) files, so undeploy
    // can still detect an external update of those files and skip a stale-backup restore.
    if let Some(p) = prior {
        for (live, _, _) in &leftovers {
            if let Some(h) = deployed_hash_for_path(live, &p.deployed_hashes) {
                record.deployed_hashes.insert(live.clone(), h.clone());
            }
            let backup = bak_path(Path::new(live));
            if let Some(hash) = backup_hash_for_path(&backup, &p.backup_hashes) {
                record
                    .backup_hashes
                    .insert(backup.display().to_string(), hash.clone());
            }
        }
    }

    // Crash-safety: if the previous deployment used DIFFERENT-named UE4SS dir(s), record them as
    // stale BEFORE persisting. apply_writes/retire_leftovers haven't removed them yet, so a crash
    // in this window would otherwise leave them orphaned-and-active with no record to clean them
    // up. retire_leftovers prunes any it later removes successfully.
    if let Some(prev) = prior {
        let new_dirs = plan.ue4ss_dsts();
        for d in prev
            .ue4ss_mod_dir
            .iter()
            .chain(prev.stale_ue4ss_dirs.iter())
            .chain(prev.ue4ss_mod_dirs.iter())
        {
            // `same_path`: records hold canonicalized (`\\?\`-prefixed) paths but the plan may hold
            // the plain form of the same dir — a raw compare would wrongly mark a still-deployed dir
            // stale and retire could then delete a dir this deploy just installed.
            if !contains_same_path(&new_dirs, d) && !contains_same_path(&record.stale_ue4ss_dirs, d)
            {
                record.stale_ue4ss_dirs.push(d.clone());
                if let Some(fingerprint) =
                    tree_fingerprint_for_path(Path::new(d), &prev.ue4ss_tree_fingerprints)
                {
                    record
                        .ue4ss_tree_fingerprints
                        .insert(d.clone(), fingerprint.clone());
                }
            }
        }
    }

    // Same for the previous deploy's additive ~mods texture triplets: any not re-created by this
    // deploy must be retired. Pre-seed them into the record BEFORE persisting (crash-safety) so a
    // crash mid-retire still lets undeploy remove them; retire_leftovers deletes + prunes the ones
    // it cleans. Without this, redeploying (esp. a different mod name or a bundle with no texture
    // component) would leave the old triplet mounted in ~mods with no record to undeploy it.
    if let Some(prev) = prior {
        // Compare against the UNION of this plan's additive dsts (triplets + manager paks): a
        // manager deploy mirrors managed_paks into the legacy `texture_triplets` field, so a prev
        // entry re-created by THIS plan as EITHER kind must be kept, not retired. `same_path` so a
        // canonicalized prev path matches the plan's plain form of the same file.
        let new_additive = plan.additive_dsts();
        for t in &prev.texture_triplets {
            if !contains_same_path(&new_additive, t)
                && !contains_same_path(&record.texture_triplets, t)
            {
                record.texture_triplets.push(t.clone());
                if let Some(hash) = deployed_hash_for_path(t, &prev.deployed_hashes) {
                    record.deployed_hashes.insert(t.clone(), hash.clone());
                }
            }
        }
        for p in &prev.managed_paks {
            if !contains_same_path(&new_additive, p) && !contains_same_path(&record.managed_paks, p)
            {
                record.managed_paks.push(p.clone());
                if let Some(hash) = deployed_hash_for_path(p, &prev.deployed_hashes) {
                    record.deployed_hashes.insert(p.clone(), hash.clone());
                }
            }
        }
    }

    // Old-binary compatibility: a pre-v2 build deserializes a manager record but silently DROPS the
    // v2-only `managed_paks`/`ue4ss_mod_dirs` fields (serde unknown-field tolerance), so ITS
    // undeploy would never remove them — leaving manager paks/dirs mounted forever. Mirror the
    // manager footprint into the legacy fields an old binary DOES read, which have identical removal
    // semantics: manager paks → `texture_triplets` (delete file), manager UE4SS dirs →
    // `stale_ue4ss_dirs` (remove_dir_all). A v2 binary now sees each path in BOTH the new and the
    // legacy field; every removal site (`retire_leftovers`, `restore_record`) guards with `!exists()`
    // so the second pass is a harmless no-op, never a double-error. Studio deploys have no such
    // fields, so this is manager-only. Done BEFORE the record is persisted so even a pre-v2 undeploy
    // after a crash cleans up. (`record.managed_paks`/`ue4ss_mod_dirs` were filled by `stage`.)
    if record.owner == "manager" {
        for p in record.managed_paks.clone() {
            if !contains_same_path(&record.texture_triplets, &p) {
                record.texture_triplets.push(p);
            }
        }
        for d in record.ue4ss_mod_dirs.clone() {
            if !contains_same_path(&record.stale_ue4ss_dirs, &d) {
                record.stale_ue4ss_dirs.push(d);
            }
        }
    }

    record.phase = DeployPhase::RecoveryRequired;
    if let Err(e) = write_record_file(game_root, &record) {
        return Err(rollback_commit_error(e, undo, game_root, prev_record_bytes));
    }

    // (c) Apply: write the live files and install the UE4SS mod. On failure restore the exact
    //     prior state and the previous record.
    if let Err(e) = apply_writes(&plan, &mut undo) {
        return Err(rollback_commit_error(e, undo, game_root, prev_record_bytes));
    }

    // (c2) The live files now actually hold our content, so the drift hashes are valid — record
    //      them and persist. They were intentionally OMITTED from the pre-write record (b): had we
    //      stored them earlier, a crash between the record write and the live writes would leave
    //      the old content on disk with the new hashes recorded, and undeploy would mis-read that
    //      as an external update — skipping the restore and dropping the backup. With no hash, a
    //      crash in that window instead falls back to a plain pristine restore.
    for (live, bytes) in &plan.writes {
        record
            .deployed_hashes
            .insert(live.display().to_string(), content_hash(bytes));
    }
    for write in &plan.file_writes {
        record
            .deployed_hashes
            .insert(write.live.display().to_string(), write.hash.clone());
    }
    // This recovery-required write must be durable: without the hashes, a later Steam update
    // couldn't be detected and undeploy could restore a stale backup over an updated asset. The
    // undo is still live here, so on failure roll the whole deploy back rather than returning a
    // half-recorded success. The phase is cleared only after post-apply reconciliation succeeds.
    if let Err(e) = write_record_file(game_root, &record) {
        return Err(rollback_commit_error(e, undo, game_root, prev_record_bytes));
    }

    // (d) committed — drop the kept-aside previous UE4SS mod(s), then retire the previous mod's
    //     footprint now (best-effort), pruning retired leftovers from the record.
    let transaction_dirs = undo.ue4ss_transaction_dirs();
    let aside_remaining = undo.discard();
    let mut changed = retire_leftovers(game_root, &leftovers, prior, &plan, &mut record)?;
    for transaction_dir in transaction_dirs {
        if aside_remaining
            .iter()
            .any(|failed| same_path(failed, &transaction_dir.display().to_string()))
        {
            continue;
        }
        let before = record.stale_ue4ss_dirs.len();
        record
            .stale_ue4ss_dirs
            .retain(|stored| !same_path(&transaction_dir, stored));
        changed |= record.stale_ue4ss_dirs.len() != before;
        prune_tree_identity(&mut record, &transaction_dir);
    }
    for old in &aside_remaining {
        // Track every remaining transaction directory before durable cleanup claims it.
        let s = old.display().to_string();
        if !record.stale_ue4ss_dirs.contains(&s) {
            record.stale_ue4ss_dirs.push(s);
            changed = true;
        }
    }
    for old in aside_remaining {
        durable_ue4ss_cleanup(game_root, &mut record, &old)?;
        changed = true;
    }
    if changed {
        // Durable cleanup has already persisted each claim transition. Persist the fully pruned
        // footprint before clearing the recovery-required phase below.
        write_record_file(game_root, &record).map_err(|error| {
            ModError::Other(format!(
                "deployment applied, but persisting retired-footprint recovery state failed: {error}"
            ))
        })?;
    }
    record.recovery_file_hashes.clear();
    record.recovery_tree_fingerprints.clear();
    record.phase = DeployPhase::Applied;
    write_record_file(game_root, &record).map_err(|error| {
        ModError::Other(format!(
            "deployment applied, but clearing recovery-required state failed: {error}"
        ))
    })?;
    Ok(record)
}

fn write_record_file(game_root: &Path, record: &DeployRecord) -> Result<()> {
    validate_record(game_root, record)?;
    let bytes = serde_json::to_vec_pretty(record)?;
    if bytes.len() as u64 > MAX_DEPLOY_RECORD_BYTES {
        return Err(ModError::Other(format!(
            "deploy record exceeds the {MAX_DEPLOY_RECORD_BYTES}-byte limit"
        )));
    }
    // Write via temp + rename so a crash mid-write can't truncate an existing record (which
    // undeploy needs to parse to restore game files / clean up backups).
    atomic_write(&record_path(game_root), &bytes)
}

/// Restore the deploy record file to its pre-deploy contents on rollback (or remove it if there
/// was none), so the on-disk record matches the rolled-back game state.
fn restore_record_file(game_root: &Path, prev_bytes: Option<&[u8]>) -> Result<()> {
    match prev_bytes {
        Some(b) => atomic_write(&record_path(game_root), b),
        None => remove_file_durable(
            &record_path(game_root),
            "removing rolled-back deploy record",
        ),
    }
}

/// Preserve the primary deployment error while making every rollback failure visible. Returning
/// the primary error alone would falsely imply the old loadout is intact when a locked target or
/// disk error prevented restoration.
fn with_rollback_failures(primary: ModError, failures: Vec<String>) -> ModError {
    if failures.is_empty() {
        primary
    } else {
        ModError::Other(format!(
            "{primary}; ROLLBACK_INCOMPLETE: {}",
            failures.join(" | ")
        ))
    }
}

fn rollback_commit_error(
    primary: ModError,
    undo: Undo,
    game_root: &Path,
    prev_record_bytes: Option<&[u8]>,
) -> ModError {
    let mut failures = undo.rollback();
    if failures.is_empty() {
        if let Err(error) = restore_record_file(game_root, prev_record_bytes) {
            failures.push(format!("restoring deploy record: {error}"));
        }
    } else {
        // If the pre-write record landed, it describes enough of the attempted deployment for a
        // later undeploy to return to pristine. Replacing it with the old record while one live/
        // additive inverse failed could make that partial new footprint untracked. If it did not
        // land, live apply never began. In either case leave the on-disk record unchanged and say
        // so explicitly in the returned diagnostic.
        failures.push(
            "on-disk deploy record intentionally left unchanged because rollback was incomplete"
                .into(),
        );
    }
    with_rollback_failures(primary, failures)
}

/// Captures the exact pre-deploy state so a failed deploy can restore it precisely, rather than
/// only reverting to the game-pristine `*.gore-bak`.
#[derive(Default)]
struct Undo {
    /// Per-target live + backup state. These must stay grouped: if restoring a live snapshot fails,
    /// the attempted-deploy record remains authoritative and its current pristine backup must stay
    /// in place (in particular, it must not be replaced by a pre-update drift backup).
    files: Vec<LiveFileUndo>,
    /// Previous UE4SS mods moved into unique, non-loadable holders: restore on rollback, drop on
    /// success. Keeping the holder lets cleanup remove exactly the directory this transaction
    /// claimed, never a predictable path that could belong to another deploy.
    ue4ss_swaps: Vec<Ue4ssSwapUndo>,
    /// Additive `~mods` files copied by this deploy (texture triplets AND manager paks, in copy
    /// order), each with its PRE-OVERWRITE state: `None` if the file did not exist before this
    /// deploy (delete on rollback), `Some(snapshot)` if it did. Snapshots are verified, durable,
    /// same-directory temp files rather than `Vec<u8>`, so a large existing pak is never duplicated
    /// in process memory. Unwound in REVERSE (LIFO) so delete/restore pairs mirror copy order.
    texture_files: Vec<AdditiveFileUndo>,
}

struct LiveFileUndo {
    live: PathBuf,
    snapshot: tempfile::TempPath,
    /// Identity of the bytes this attempt may have published.  Set before publication so rollback
    /// also handles an error returned after the atomic rename (for example a parent-sync failure).
    published_hash: Option<String>,
    backup: Option<BackupUndo>,
}

struct AdditiveFileUndo {
    path: PathBuf,
    prior: Option<tempfile::TempPath>,
    published_hash: String,
}

struct BackupUndo {
    path: PathBuf,
    /// The current backup was created by this attempt and may be removed after live restoration.
    created: bool,
    /// Pre-update backup removed during drift refresh. Restore only if the matching live target was
    /// restored too; otherwise the recovery record needs the newly refreshed backup at `path`.
    stale_snapshot: Option<tempfile::TempPath>,
    published_hash: String,
}

struct Ue4ssSwapUndo {
    staging: PathBuf,
    holder: Option<PathBuf>,
    old: Option<PathBuf>,
    dst: PathBuf,
    intended_fingerprint: String,
    previous_fingerprint: Option<String>,
    state: Ue4ssSwapState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ue4ssSwapState {
    Prepared,
    OldMoved,
    PromotedFresh,
    PromotedReplacement,
}

impl Undo {
    /// Attempt every inverse operation and return all failures. A caller must surface a non-empty
    /// result: silently swallowing one of these errors can leave a partial mod mounted while the
    /// API reports only the original write failure.
    fn rollback(self) -> Vec<String> {
        let Undo {
            files,
            ue4ss_swaps,
            texture_files,
        } = self;
        let mut failures = Vec::new();
        let mut pending_backup_undos = Vec::new();
        for file in files {
            // Snapshot and destination share a directory, so this is one atomic replacement.
            let restored = rollback_restore_snapshot(
                &file.live,
                file.snapshot,
                file.published_hash.as_deref(),
                "live file",
                &mut failures,
            );
            if restored {
                if let Some(backup) = file.backup {
                    pending_backup_undos.push(backup);
                }
            }
            // On a failed live restore, `backup` drops only its obsolete stale snapshot. The current
            // backup remains untouched for the retained recovery-required record.
        }
        for swap in ue4ss_swaps.into_iter().rev() {
            rollback_remove_owned_tree(
                &swap.staging,
                &swap.intended_fingerprint,
                "UE4SS staging directory",
                &mut failures,
            );
            match swap.state {
                Ue4ssSwapState::Prepared => {
                    if let Some(holder) = swap.holder {
                        if let Err(error) = remove_empty_dir_durable(
                            &holder,
                            "removing unused empty UE4SS transaction holder",
                        ) {
                            failures.push(error.to_string());
                        }
                    }
                }
                Ue4ssSwapState::PromotedFresh => {
                    rollback_remove_owned_tree(
                        &swap.dst,
                        &swap.intended_fingerprint,
                        "fresh UE4SS directory",
                        &mut failures,
                    );
                }
                Ue4ssSwapState::OldMoved | Ue4ssSwapState::PromotedReplacement => {
                    let old = swap.old.expect("replacement swap has an old path");
                    let holder = swap.holder.expect("replacement swap has a holder");
                    let destination_ready = if swap.state == Ue4ssSwapState::PromotedReplacement {
                        rollback_remove_owned_tree(
                            &swap.dst,
                            &swap.intended_fingerprint,
                            "new UE4SS directory",
                            &mut failures,
                        )
                    } else if path_exists_no_follow(&swap.dst) {
                        failures.push(format!(
                            "refusing to overwrite path that appeared during UE4SS rollback: {}",
                            swap.dst.display()
                        ));
                        false
                    } else {
                        true
                    };
                    if destination_ready {
                        let previous_is_owned =
                            swap.previous_fingerprint.as_ref().is_some_and(|expected| {
                                tree_matches_recorded_fingerprint(&old, expected)
                            });
                        if !previous_is_owned {
                            failures.push(format!(
                                "refusing to restore changed UE4SS rollback tree retained at '{}'",
                                old.display()
                            ));
                        } else if let Err(error) = promote_path_noclobber(&old, &swap.dst) {
                            failures.push(format!(
                                "restoring UE4SS directory '{}' from '{}': {error}; previous \
                                 directory retained in transaction holder '{}'",
                                swap.dst.display(),
                                old.display(),
                                holder.display()
                            ));
                        } else if let Err(error) = remove_empty_dir_durable(
                            &holder,
                            "removing restored UE4SS transaction holder",
                        ) {
                            failures.push(error.to_string());
                        }
                    } else {
                        failures.push(format!(
                            "previous UE4SS directory retained at '{}'",
                            old.display()
                        ));
                    }
                }
            }
        }
        // Restore each additive `~mods` file we copied (triplets + manager paks): put back the
        // disk snapshot made before overwrite, or delete it if it's a fresh addition. Unwound in
        // REVERSE insertion order (LIFO) to mirror the copies.
        for file in texture_files.into_iter().rev() {
            match file.prior {
                Some(snapshot) => {
                    rollback_restore_snapshot(
                        &file.path,
                        snapshot,
                        Some(&file.published_hash),
                        "additive file",
                        &mut failures,
                    );
                }
                None => rollback_remove_owned_file(
                    &file.path,
                    &file.published_hash,
                    "fresh additive file",
                    &mut failures,
                ),
            }
        }
        // The attempted-deploy record is retained if ANY live/additive/UE4SS inverse failed. In
        // that case every current backup it references must remain available, including backups for
        // targets whose own live snapshot happened to restore successfully. Only undo backup
        // mutations after all other recovery work succeeded.
        if failures.is_empty() {
            for backup in pending_backup_undos {
                if let Some(stale) = backup.stale_snapshot {
                    // Replacing the current refresh atomically both restores the old backup and
                    // consumes the newly-created one at the same path.
                    rollback_restore_snapshot(
                        &backup.path,
                        stale,
                        Some(&backup.published_hash),
                        "stale backup",
                        &mut failures,
                    );
                } else if backup.created {
                    rollback_remove_owned_file(
                        &backup.path,
                        &backup.published_hash,
                        "new backup",
                        &mut failures,
                    );
                }
            }
        }
        failures
    }

    /// Return transaction directories that still exist after publication so the caller can move
    /// them through the durable UE4SS cleanup state machine.
    fn discard(self) -> Vec<PathBuf> {
        let mut failed = Vec::new();
        for swap in &self.ue4ss_swaps {
            if path_exists_no_follow(&swap.staging) {
                failed.push(swap.staging.clone());
            }
            if let Some(holder) = &swap.holder {
                if path_exists_no_follow(holder) {
                    failed.push(holder.clone());
                }
            }
        }
        failed.sort();
        failed.dedup();
        failed
    }

    fn ue4ss_transaction_dirs(&self) -> Vec<PathBuf> {
        self.ue4ss_swaps
            .iter()
            .flat_map(|swap| {
                std::iter::once(swap.staging.clone()).chain(swap.holder.iter().cloned())
            })
            .collect()
    }
}

fn rollback_restore_snapshot(
    dst: &Path,
    snapshot: tempfile::TempPath,
    published_hash: Option<&str>,
    kind: &str,
    failures: &mut Vec<String>,
) -> bool {
    let Some(published_hash) = published_hash else {
        return true;
    };
    let snapshot_path = snapshot.to_path_buf();
    let snapshot_hash = match sha256_file(&snapshot_path) {
        Ok(hash) => hash,
        Err(error) => {
            let retained = retain_temp_path(snapshot, "snapshot");
            failures.push(format!(
                "authenticating rollback snapshot for {kind} '{}': {error}; {retained}",
                dst.display()
            ));
            return false;
        }
    };
    if !path_exists_no_follow(dst) {
        let retained = retain_temp_path(snapshot, "snapshot");
        failures.push(format!(
            "refusing to recreate missing {kind} '{}'; {retained}",
            dst.display()
        ));
        return false;
    }
    let claimed = match claim_file_to_unique_holder(dst, ".gore-rollback-current-") {
        Ok(path) => path,
        Err(error) => {
            let retained = retain_temp_path(snapshot, "snapshot");
            failures.push(format!(
                "claiming current {kind} '{}' before rollback: {error}; {retained}",
                dst.display()
            ));
            return false;
        }
    };
    let current_is_ours = file_matches_recorded_hash_result(&claimed, published_hash);
    match current_is_ours {
        Ok(true) => {
            if let Err(error) = promote_path_noclobber(&snapshot_path, dst) {
                let retained = retain_temp_path(snapshot, "snapshot");
                failures.push(format!(
                    "restoring {kind} '{}' without clobber after claiming current bytes at '{}': \
                     {error}; {retained}",
                    dst.display(),
                    claimed.display()
                ));
                return false;
            }
            if let Err(error) = remove_file_durable(&claimed, "removing claimed rollback file") {
                failures.push(format!(
                    "restored {kind} '{}', but could not remove claimed prior publication '{}': \
                     {error}",
                    dst.display(),
                    claimed.display()
                ));
            } else {
                let _ = cleanup_untracked_holder(&claimed);
            }
            true
        }
        Ok(false) | Err(_) => {
            // The path changed after our publication (or cannot be authenticated). Put that exact
            // object back only if the original name is still free; never overwrite a racing winner.
            match promote_path_noclobber(&claimed, dst) {
                Ok(()) => {
                    let _ = cleanup_untracked_holder(&claimed);
                    if file_matches_recorded_hash_result(dst, &snapshot_hash).unwrap_or(false) {
                        // Publication failed before its rename; the original bytes never changed.
                        // The duplicate snapshot may be dropped and the prior record restored.
                        true
                    } else {
                        let retained = retain_temp_path(snapshot, "snapshot");
                        failures.push(format!(
                            "refusing to overwrite externally changed {kind} '{}'; {retained}",
                            dst.display()
                        ));
                        false
                    }
                }
                Err(error) => {
                    let retained = retain_temp_path(snapshot, "snapshot");
                    failures.push(format!(
                        "current {kind} '{}' changed externally and could not be returned from '{}': \
                         {error}; {retained}",
                        dst.display(),
                        claimed.display()
                    ));
                    false
                }
            }
        }
    }
}

fn rollback_remove_owned_file(
    path: &Path,
    published_hash: &str,
    kind: &str,
    failures: &mut Vec<String>,
) {
    if !path_exists_no_follow(path) {
        return;
    }
    let claimed = match claim_file_to_unique_holder(path, ".gore-rollback-delete-") {
        Ok(path) => path,
        Err(error) => {
            failures.push(format!(
                "claiming {kind} '{}' before rollback delete: {error}",
                path.display()
            ));
            return;
        }
    };
    match file_matches_recorded_hash_result(&claimed, published_hash) {
        Ok(true) => {
            if let Err(error) = remove_file_durable(&claimed, "removing claimed rollback file") {
                failures.push(format!(
                    "removing claimed {kind} '{}': {error}",
                    claimed.display()
                ));
            } else {
                let _ = cleanup_untracked_holder(&claimed);
            }
        }
        Ok(false) | Err(_) => {
            if let Err(error) = promote_path_noclobber(&claimed, path) {
                failures.push(format!(
                    "refusing to delete externally changed {kind} '{}'; claimed object retained at \
                     '{}': {error}",
                    path.display(),
                    claimed.display()
                ));
            } else {
                let _ = cleanup_untracked_holder(&claimed);
            }
        }
    }
}

fn rollback_remove_owned_tree(
    path: &Path,
    expected_fingerprint: &str,
    kind: &str,
    failures: &mut Vec<String>,
) -> bool {
    if !path_exists_no_follow(path) {
        return true;
    }
    let claimed = match claim_tree_to_unique_holder(path, ".gore-rollback-tree-") {
        Ok(path) => path,
        Err(error) => {
            failures.push(format!(
                "claiming {kind} '{}' before rollback delete: {error}",
                path.display()
            ));
            return false;
        }
    };
    if tree_matches_recorded_fingerprint(&claimed, expected_fingerprint) {
        match remove_dir_all_durable(&claimed, "removing claimed rollback tree") {
            Ok(()) => {
                let _ = cleanup_untracked_holder(&claimed);
                true
            }
            Err(error) => {
                failures.push(format!(
                    "removing claimed {kind} '{}': {error}",
                    claimed.display()
                ));
                false
            }
        }
    } else {
        if let Err(error) = promote_path_noclobber(&claimed, path) {
            failures.push(format!(
                "refusing to delete externally changed {kind} '{}'; claimed tree retained at '{}': \
                 {error}",
                path.display(),
                claimed.display()
            ));
        } else {
            let _ = cleanup_untracked_holder(&claimed);
        }
        false
    }
}

fn retain_temp_path(path: tempfile::TempPath, label: &str) -> String {
    match path.keep() {
        Ok(path) => format!("{label} retained at '{}'", path.display()),
        Err(error) => format!("{label} retention also failed: {error}"),
    }
}

fn claim_file_to_unique_holder(source: &Path, prefix: &str) -> Result<PathBuf> {
    let root = untracked_cleanup_root(source)?;
    let holder = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(&root)
        .map_err(io("allocating unique file-claim path"))?;
    let holder = holder.keep();
    let claimed = holder.join("claimed");
    match promote_path_noclobber(source, &claimed) {
        Ok(()) => Ok(claimed),
        // A parent-directory sync can fail after the atomic rename itself succeeded. Do not lose
        // the only path to the claimed object; the caller authenticates it before any delete.
        Err(_) if !path_exists_no_follow(source) && path_exists_no_follow(&claimed) => Ok(claimed),
        Err(error) => {
            let _ = remove_empty_dir_durable(&holder, "removing unused rollback file holder");
            Err(error)
        }
    }
}

fn claim_tree_to_unique_holder(source: &Path, prefix: &str) -> Result<PathBuf> {
    let root = untracked_cleanup_root(source)?;
    let holder = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(&root)
        .map_err(io("allocating unique tree-claim path"))?;
    let holder = holder.keep();
    let claimed = holder.join("claimed");
    match promote_path_noclobber(source, &claimed) {
        Ok(()) => Ok(claimed),
        // As above, retain ownership of a successful rename even if the durability barrier failed.
        Err(_) if !path_exists_no_follow(source) && path_exists_no_follow(&claimed) => Ok(claimed),
        Err(error) => {
            let _ = remove_empty_dir_durable(&holder, "removing unused rollback tree holder");
            Err(error)
        }
    }
}

fn untracked_cleanup_root(source: &Path) -> Result<PathBuf> {
    for ancestor in source.ancestors() {
        if ancestor
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("G1R"))
        {
            return ancestor.parent().map(Path::to_path_buf).ok_or_else(|| {
                ModError::Other(format!(
                    "G1R path has no install parent: {}",
                    source.display()
                ))
            });
        }
    }
    source.parent().map(Path::to_path_buf).ok_or_else(|| {
        ModError::Other(format!(
            "cleanup source has no parent: {}",
            source.display()
        ))
    })
}

fn cleanup_untracked_holder(claimed: &Path) -> Result<()> {
    let holder = claimed.parent().ok_or_else(|| {
        ModError::Other(format!(
            "untracked cleanup payload has no holder: {}",
            claimed.display()
        ))
    })?;
    remove_empty_dir_durable(holder, "removing untracked cleanup holder")
}

fn promote_path_noclobber(source: &Path, destination: &Path) -> Result<()> {
    promote_directory_noclobber(source, destination)
        .map_err(io("publishing no-clobber filesystem claim"))?;
    let source_parent = source.parent().ok_or_else(|| {
        ModError::Other(format!("claim source has no parent: {}", source.display()))
    })?;
    sync_parent_directory(source_parent)?;
    if destination.parent() != Some(source_parent) {
        let destination_parent = destination.parent().ok_or_else(|| {
            ModError::Other(format!(
                "claim destination has no parent: {}",
                destination.display()
            ))
        })?;
        sync_parent_directory(destination_parent)?;
    }
    Ok(())
}

/// Build everything to write, in memory. Any error here leaves the game untouched.
fn prepare(
    bundle_dir: &Path,
    manifest: &ModManifest,
    gp: &GamePaths,
    prev: Option<&DeployRecord>,
) -> Result<DeployPlan> {
    let mut plan = DeployPlan::default();
    let mut voice = PendingVoiceEdits::new();
    let mut voice_order = 0usize;
    // One oracle for the whole deploy, however many loose-file components the bundle carries.
    let mut shadow = PakShadowIndex::new(&gp.root);
    for (comp_idx, comp) in manifest.components.iter().enumerate() {
        match comp {
            Component::Ue4ssLua { name, path, .. } => {
                // The manifest may come from an untrusted bundle: reject names/paths that could
                // escape the bundle source or the UE4SS Mods directory.
                if !is_safe_mod_name(name) || !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!(
                        "unsafe ue4ss component in manifest: name={name:?} path={path:?}"
                    )));
                }
                // A single bundle installs at most one UE4SS mod. The guard below rejects a later
                // component instead of silently replacing the first one.
                if !plan.ue4ss_dirs.is_empty() {
                    return Err(ModError::Other(
                        "multiple UE4SS components in one bundle are unsupported".into(),
                    ));
                }
                let source =
                    resolve_safe_bundle_tree(bundle_dir, Path::new(path), "UE4SS component")?;
                plan.ue4ss_dirs.push((source, gp.ue4ss_mods.join(name)));
            }
            Component::LocPatch { path } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe loc patch path: {path:?}")));
                }
                let lcache = gp.lcache.clone().ok_or_else(|| {
                    ModError::Other("no AlkimiaLocalization .lcache found in game".into())
                })?;
                let (pristine, drifted) =
                    read_pristine_bounded(&lcache, prev, MAX_PRISTINE_PATCH_BYTES)?;
                if drifted {
                    plan.refresh_baks.push(lcache.clone());
                }
                let raw_edits: BTreeMap<String, BTreeMap<String, String>> =
                    serde_json::from_slice(&read_safe_bundle_file(
                        bundle_dir,
                        Path::new(path),
                        "localization edits manifest",
                        MAX_BUNDLE_MANIFEST_BYTES,
                    )?)?;
                // JSON object keys are case-sensitive, but lcache ids and language names are not.
                // Fold aliases before touching the cache so a newly-added id receives the union of
                // all its translations in one atomic add_key call. Otherwise the first spelling
                // creates a partial record and later spellings cannot add a missing language via
                // set_value. BTreeMap traversal makes the later alias deterministic.
                type FoldedLocEdits =
                    BTreeMap<String, (String, BTreeMap<String, (String, String)>)>;
                let mut edits: FoldedLocEdits = BTreeMap::new();
                for (id, langs) in raw_edits {
                    let pending = edits
                        .entry(id.to_ascii_lowercase())
                        .or_insert_with(|| (id.clone(), BTreeMap::new()));
                    pending.0 = id;
                    for (set, text) in langs {
                        pending.1.insert(set.to_ascii_lowercase(), (set, text));
                    }
                }
                /// Split a language tag into its stem and generation rank: `german` -> (german, 0),
                /// `german_new` -> (german, 1), `english_newer` -> (english, 2).
                ///
                /// Derived from the suffix rather than a fixed list of names, so a cache that
                /// grows another generation is covered without a code change. Only the ORDER is
                /// assumed — nothing in the file states that `_newer` outranks `_new`; that came
                /// from watching the game display the newer one. The stems it produces from the
                /// shipped header are exactly two ladders, german and english, and nothing else.
                fn generation(language: &str) -> (&str, u8) {
                    match language
                        .strip_suffix("_newer")
                        .map(|stem| (stem, 2))
                        .or_else(|| language.strip_suffix("_new").map(|stem| (stem, 1)))
                    {
                        Some(pair) => pair,
                        None => (language, 0),
                    }
                }

                let mut lc = gore_loc::loc::Lcache::decode(&pristine)?;
                let declared: BTreeMap<String, String> = lc
                    .languages()
                    .into_iter()
                    .map(|language| (language.to_ascii_lowercase(), language))
                    .collect();
                for (id, langs) in edits.values() {
                    if lc.has_key(id) {
                        // Owned so the shadow check can read the id's slots while set_value below
                        // holds the cache mutably.
                        let carried: Vec<String> = lc
                            .languages_for(id)
                            .into_iter()
                            .map(str::to_ascii_lowercase)
                            .collect();
                        for (folded_set, (set, text)) in langs {
                            // Best-effort: a language absent from THIS install's record (e.g. a
                            // shared mod built against a different game version) is skipped rather
                            // than aborting the entire deploy — but skipped is not the same as
                            // fine. Dropping it silently let the summary claim no edit was
                            // unwritable while this one had never been applied.
                            if !declared.contains_key(folded_set) {
                                plan.loc_skipped.push(format!(
                                    "'{id}': this install's cache declares no '{set}' language"
                                ));
                                continue;
                            }
                            // The write decides which of the two findings this edit can raise, so
                            // it happens first. `set_value` still fails when the language is
                            // declared by the cache but this particular id has no slot for it — the
                            // cache is sparse. That is a different miss from shadowing, and the
                            // standalone `gore loc import` reports it by name, so swallowing it
                            // here left the two paths disagreeing about the same edit.
                            if let Err(error) = lc.set_value(id, set, text) {
                                plan.loc_skipped.push(format!("'{id}' has no '{set}' text: {error}"));
                                continue;
                            }
                            // Only now: the edit is in the file, so the remaining question is
                            // whether anyone will see it. Asking before the write reported an edit
                            // that never landed under a note saying it had been written — and an id
                            // carrying `german_new` without `german` raised both findings for one
                            // edit, which is exactly the case a reader must not be confused about.
                            //
                            // Suppressed when this same bundle also writes the winning generation
                            // for this id, which is the practice the guide recommends and must not
                            // be nagged about.
                            let (stem, rank) = generation(folded_set);
                            if let Some(winner) = carried
                                .iter()
                                .filter(|carried_lang| {
                                    let (other_stem, other_rank) = generation(carried_lang);
                                    other_stem == stem && other_rank > rank
                                })
                                .max_by_key(|carried_lang| generation(carried_lang).1)
                            {
                                if !langs.contains_key(winner) {
                                    plan.loc_shadowed.push(format!(
                                        "'{id}' also carries '{winner}', which the game displays \
                                         instead of '{set}'"
                                    ));
                                }
                            }
                        }
                    } else {
                        // A loc mod may introduce dialog/quest text that has no vanilla id. Add
                        // all valid translations in one record so validation is atomic and pair
                        // order follows the lcache header. Unknown install languages remain the
                        // same best-effort skip as above.
                        let mut valid = BTreeMap::new();
                        for (folded_set, (set, text)) in langs {
                            match declared.get(folded_set) {
                                // Case aliases target one logical language. BTreeMap traversal is
                                // deterministic, and the later alias wins like other patch merges.
                                Some(canonical) => {
                                    valid.insert(canonical.clone(), text.clone());
                                }
                                // Same silent drop as the existing-id branch had, and the same
                                // reason to report it: the deploy succeeds and this translation is
                                // not in it.
                                None => plan.loc_skipped.push(format!(
                                    "'{id}': this install's cache declares no '{set}' language"
                                )),
                            }
                        }
                        if !valid.is_empty() {
                            // Unknown languages above are intentionally best-effort skips; any
                            // remaining add failure is structural and must abort prepare rather
                            // than silently deploying a bundle without its new localization id.
                            lc.add_key(id, &valid)?;
                        }
                    }
                }
                plan.writes.push((lcache, lc.encode()?));
            }
            Component::AudioPatch { path, banks: _ } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!(
                        "unsafe audio patch path: {path:?}"
                    )));
                }
                let map: BTreeMap<String, BTreeMap<String, String>> =
                    serde_json::from_slice(&read_safe_bundle_file(
                        bundle_dir,
                        Path::new(&format!("{path}/manifest.json")),
                        "audio manifest",
                        MAX_BUNDLE_MANIFEST_BYTES,
                    )?)?;
                // Use this install's recovered FMOD bank key if gore-dump left a gore_fmod_key.json,
                // so users whose key changed after a game patch can still deploy audio; else the
                // known constant.
                let fmod_key = resolve_fmod_key(gp);
                for (bank, samples) in &map {
                    validate_bank_name(bank)?;
                    let bank_path = gp.fmod_desktop.join(bank);
                    let (pristine, drifted) =
                        read_pristine_bounded(&bank_path, prev, MAX_PRISTINE_PATCH_BYTES)?;
                    if drifted {
                        plan.refresh_baks.push(bank_path.clone());
                    }
                    let mut repl = Vec::new();
                    for (sample, wav_rel) in samples {
                        if !is_safe_rel_path(wav_rel) {
                            return Err(ModError::Other(format!("unsafe wav path: {wav_rel:?}")));
                        }
                        let wav = read_safe_bundle_file(
                            bundle_dir,
                            Path::new(wav_rel),
                            "audio WAV payload",
                            MAX_AUDIO_WAV_BYTES,
                        )?;
                        let (rate, ch, pcm) =
                            gore_fmod::read_wav_pcm16(&wav).map_err(ModError::Fmod)?;
                        repl.push((
                            sample.clone(),
                            gore_fmod::Pcm16Sample {
                                name: sample.clone(),
                                freq: rate,
                                channels: ch,
                                pcm,
                            },
                        ));
                    }
                    let new_bank = gore_fmod::replace_samples(&pristine, &fmod_key, repl)
                        .map_err(ModError::Fmod)?;
                    plan.writes.push((bank_path, new_bank));
                }
            }
            Component::TexturePatch { path, assets: _ } => {
                let (triplets, temporary_root) = prepare_texture_component(
                    bundle_dir,
                    path,
                    &manifest.mod_meta.name,
                    comp_idx,
                    gp,
                )?;
                plan.texture_triplets.extend(triplets);
                plan.temporary_roots.push(temporary_root);
            }
            Component::AngelScriptPatch { path } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!(
                        "unsafe script patch path: {path:?}"
                    )));
                }
                let entries: Vec<ScriptEntry> = serde_json::from_slice(&read_safe_bundle_file(
                    bundle_dir,
                    Path::new(&format!("{path}/manifest.json")),
                    "script manifest",
                    MAX_BUNDLE_MANIFEST_BYTES,
                )?)?;
                // Validate manifest policy before parsing the install cache, preserving the
                // deterministic bad-op error even when the configured game fixture is incomplete.
                if let Some(e) = entries.iter().find(|e| e.op != "add" && e.op != "edit") {
                    return Err(ModError::Other(format!(
                        "invalid script op {:?} for module {:?}",
                        e.op, e.module
                    )));
                }
                let cache_path = gp.script_cache.clone();
                if !cache_path.exists() {
                    return Err(ModError::Other(format!(
                        "script cache not found: {} — is the game path correct?",
                        cache_path.display()
                    )));
                }
                let (pristine, drifted) =
                    read_pristine_bounded(&cache_path, prev, MAX_PRISTINE_PATCH_BYTES)?;
                if drifted {
                    plan.refresh_baks.push(cache_path.clone());
                }
                let mut script_merge_guard =
                    gore_as::cache::splice::SequentialMiniGuard::new(&pristine).map_err(|err| {
                        ModError::Other(format!("prepare script composition: {err}"))
                    })?;
                let mut running = pristine;
                for e in &entries {
                    if !is_safe_rel_path(&e.mini) {
                        return Err(ModError::Other(format!("unsafe mini path: {:?}", e.mini)));
                    }
                    let mini = read_safe_bundle_file(
                        bundle_dir,
                        Path::new(&e.mini),
                        "script mini-cache",
                        MAX_SCRIPT_MINI_BYTES,
                    )?;
                    if e.op != "add" && e.op != "edit" {
                        return Err(ModError::Other(format!(
                            "invalid script op {:?} for module {:?}",
                            e.op, e.module
                        )));
                    }
                    let prepared = script_merge_guard
                        .check_and_record(&mini)
                        .map_err(|err| ModError::Other(format!("compose {}: {err}", e.module)))?;
                    running = match e.op.as_str() {
                        "add" => gore_as::cache::splice::splice_auto(&running, &prepared).map_err(
                            |err| ModError::Other(format!("splice {}: {err}", e.module)),
                        )?,
                        "edit" => {
                            gore_as::cache::splice::replace_module(&running, &prepared, &e.module)
                                .map_err(|err| {
                                ModError::Other(format!("replace {}: {err}", e.module))
                            })?
                        }
                        other => {
                            return Err(ModError::Other(format!(
                                "invalid script op {other:?} for module {:?}",
                                e.module
                            )))
                        }
                    };
                }
                plan.writes.push((cache_path, running));
            }
            Component::FilePatch { path, targets: _ } => {
                prepare_file_component(bundle_dir, path, gp, prev, &mut shadow, &mut plan)?;
            }
            Component::PakFilePatch { path, targets: _ } => {
                let (paks, temporary_root) = prepare_pak_file_component(
                    bundle_dir,
                    path,
                    &manifest.mod_meta.name,
                    comp_idx,
                    gp,
                )?;
                plan.texture_triplets.extend(paks);
                plan.temporary_roots.push(temporary_root);
            }
            Component::VoiceArchivePatch { path } => {
                merge_voice_component(bundle_dir, path, &mut voice, &mut voice_order)?;
            }
        }
    }
    prepare_voice_archive_writes(&voice, gp, prev, &mut plan)?;
    Ok(plan)
}

/// Prepare ONE loose-file component: for every game-root-relative destination in its manifest,
/// take the same pristine/drift decision the .lcache and the FMOD banks take, and stage the
/// bundle payload as a verified disk-backed write. Nothing is written to the game here.
///
/// Replace-only. A missing, non-regular or link target is refused now, before `stage` creates a
/// single backup — an add-new loose file would need delete-on-undeploy semantics, and
/// `validate_record` refuses a delete-only cleanup claim against a live game file for good reason.
/// A destination a mounted pak already carries is refused for the opposite reason: the write would
/// succeed and change nothing, which is the one failure the toolkit cannot observe afterwards.
fn prepare_file_component(
    bundle_dir: &Path,
    path: &str,
    gp: &GamePaths,
    prev: Option<&DeployRecord>,
    shadow: &mut PakShadowIndex,
    plan: &mut DeployPlan,
) -> Result<()> {
    if !is_safe_rel_path(path) {
        return Err(ModError::Other(format!("unsafe file patch path: {path:?}")));
    }
    let map: BTreeMap<String, String> = serde_json::from_slice(&read_safe_bundle_file(
        bundle_dir,
        Path::new(&format!("{path}/manifest.json")),
        "loose file manifest",
        MAX_BUNDLE_MANIFEST_BYTES,
    )?)?;
    for (game_path, payload_rel) in &map {
        // The manifest may come from an untrusted bundle: re-ask the authoring-time question here
        // rather than trusting that whoever wrote the bundle asked it.
        validate_loose_game_path(game_path)?;
        if !is_safe_rel_path(payload_rel) {
            return Err(ModError::Other(format!(
                "unsafe loose file payload path: {payload_rel:?}"
            )));
        }
        let live = gp.root.join(loose_relative_os_path(game_path));
        match std::fs::symlink_metadata(&live) {
            Ok(metadata) if metadata_is_link(&metadata) || !metadata.is_file() => {
                return Err(ModError::Other(format!(
                    "loose file target is not a regular non-link file: {}",
                    live.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // The whole `G1R/Config` subtree is like this: shipped only inside the pak, absent
                // from disk. "Does not exist" would be true and misleading, so say which it is.
                if let Some(pak) = shadow.owning_pak(game_path)? {
                    return Err(shadowed_loose_target_error(game_path, &pak));
                }
                return Err(ModError::Other(format!(
                    "loose file target does not exist: {} — a bundle can only replace a file this \
                     install already ships",
                    live.display()
                )));
            }
            Err(error) => return Err(io("reading loose file target metadata")(error)),
        }
        if let Some(pak) = shadow.owning_pak(game_path)? {
            return Err(shadowed_loose_target_error(game_path, &pak));
        }
        // Inherit the game-update contract wholesale: if Steam replaced this file underneath a
        // deployed mod, the preserved backup is stale and `stage` must re-snapshot the newer file.
        if select_pristine_source(&live, prev)?.drifted {
            plan.refresh_baks.push(live.clone());
        }
        let (candidate, hash) = snapshot_bundle_payload(
            bundle_dir,
            payload_rel,
            "loose file payload",
            MAX_LOOSE_FILE_BYTES,
        )?;
        plan.file_writes.push(DiskWrite {
            live,
            candidate,
            hash,
        });
    }
    Ok(())
}

/// The refusal for an in-place destination a mounted pak already carries. It names the container
/// that wins and the section that would reach the same destination, because "this does nothing" is
/// only useful to an author who is told what does.
fn shadowed_loose_target_error(game_path: &str, pak: &str) -> ModError {
    ModError::Other(format!(
        "loose file target {game_path:?} is already packed in {pak}; a mounted pak is consulted \
         before the file on disk, so replacing it in place would change nothing — move this \
         destination to the bundle's \"pak_files\" section, which claims it from an additive \
         ~mods pak instead"
    ))
}

/// Stream one bundle payload into a private temp candidate and hash it, without ever holding the
/// payload in memory. The bundle-side resolver already refused links and anything resolving
/// outside the bundle; the copy then runs on that same resolved path and rejects a file that
/// changed size underneath it.
fn snapshot_bundle_payload(
    bundle_dir: &Path,
    rel: &str,
    label: &str,
    max_bytes: u64,
) -> Result<(tempfile::TempPath, String)> {
    let (canonical, len) = resolve_safe_bundle_file(bundle_dir, Path::new(rel), label)?;
    if len > max_bytes {
        return Err(ModError::Other(format!(
            "{label} exceeds the {max_bytes}-byte limit: {len} bytes at {}",
            canonical.display()
        )));
    }
    let mut source = std::fs::File::open(&canonical)
        .map_err(io(&format!("opening {label} {}", canonical.display())))?;
    let mut candidate = tempfile::Builder::new()
        .prefix(".gore-mod-loose-")
        .tempfile()
        .map_err(io(&format!("creating {label} candidate")))?;
    let copied = std::io::copy(
        &mut std::io::Read::by_ref(&mut source).take(max_bytes.saturating_add(1)),
        candidate.as_file_mut(),
    )
    .map_err(io(&format!("copying {label} {}", canonical.display())))?;
    if copied != len {
        return Err(ModError::Other(format!(
            "{label} changed while being copied: {}",
            canonical.display()
        )));
    }
    candidate
        .as_file()
        .sync_all()
        .map_err(io(&format!("syncing {label} candidate")))?;
    let candidate = candidate.into_temp_path();
    let hash = content_hash_file(&candidate).map_err(io(&format!("hashing {label} candidate")))?;
    Ok((candidate, hash))
}

/// Prepare ONE texture component: cook each PNG in the patch dir at `path` (bundle-relative)
/// over its original cooked asset and pack the result into a Zen triplet named
/// `zzz_{sanitize(mod_name)}_{name_hash(mod_name)}_{comp_idx}_tex_P`. Returns the (src, dst)
/// triplet file pairs to copy into `~mods` — purely in-memory/temp work, no game writes.
fn prepare_texture_component(
    bundle_dir: &Path,
    path: &str,
    mod_name: &str,
    comp_idx: usize,
    gp: &GamePaths,
) -> Result<(Vec<(PathBuf, PathBuf)>, tempfile::TempDir)> {
    if !is_safe_rel_path(path) {
        return Err(ModError::Other(format!(
            "unsafe texture patch path: {path:?}"
        )));
    }
    let map: BTreeMap<String, String> = serde_json::from_slice(&read_safe_bundle_file(
        bundle_dir,
        Path::new(&format!("{path}/manifest.json")),
        "texture manifest",
        MAX_BUNDLE_MANIFEST_BYTES,
    )?)?;
    let game_dir = gp.root.clone();
    let utoc = gore_tex::paths::main_container(&game_dir)
        .map_err(|e| ModError::Other(format!("container: {e}")))?;
    let usmap =
        gore_tex::paths::usmap(&game_dir).map_err(|e| ModError::Other(format!("usmap: {e}")))?;
    let usmap_bytes = std::fs::read(&usmap).map_err(io("reading usmap"))?;
    // Only use the cached index if it's current for this game build; a stale index
    // (game patched, .usmap/build_id changed) would map paths to outdated package
    // ids and cook the wrong texture. If stale/absent, fall back to a name scan.
    let build_id = gore_tex::index::build_id_for(&utoc, &usmap)
        .map_err(|e| ModError::Other(format!("texture source fingerprint: {e}")))?;
    let index = gore_tex::index::TextureIndex::load_current(
        &gore_tex::paths::texture_index_path(),
        &build_id,
    );
    // Every prepare owns a fresh root. PID + component-index paths race when two deployments or
    // parallel tests prepare the same component index: one removes the other's still-needed
    // source before commit. The retained pack guard returned below closes that lifetime gap.
    let cook_root = tempfile::Builder::new()
        .prefix("gore-mod-tex-cook-")
        .tempdir()
        .map_err(io("mkdir texture cook dir"))?;
    let cook_dir = cook_root.path();
    for (asset, png_rel) in &map {
        if !is_safe_rel_path(png_rel) {
            return Err(ModError::Other(format!("unsafe png path: {png_rel:?}")));
        }
        let leaf = asset.rsplit('/').next().unwrap_or(asset);
        // Map the UE mount root to its physical content path. Non-/Game
        // assets (e.g. /Engine/...) must NOT be forced under G1R/Content
        // or the override lands at the wrong virtual path and silently
        // does nothing; unknown roots (plugins) are rejected.
        let rel = gore_tex::paths::content_mount_rel(asset).ok_or_else(|| {
            ModError::Other(format!(
                "unsupported asset mount root (only /Game and /Engine): {asset}"
            ))
        })?;
        if !is_safe_rel_path(&rel) {
            return Err(ModError::Other(format!("unsafe asset path: {asset:?}")));
        }
        let dest_dir = cook_dir.join(
            std::path::Path::new(&rel)
                .parent()
                .ok_or_else(|| ModError::Other(format!("bad asset path {asset}")))?,
        );
        std::fs::create_dir_all(&dest_dir).map_err(io("mkdir cook dir"))?;
        // A guarded per-asset root prevents concurrent deploys from clobbering each other and
        // removes a potentially large unpack tree on every early-return path.
        let tmp_orig = tempfile::Builder::new()
            .prefix("gore-mod-tex-orig-")
            .tempdir()
            .map_err(io("mkdir orig"))?;
        let orig_uasset = match index.as_ref().and_then(|i| i.entries.get(asset)) {
            Some(&pid) => {
                gore_tex::container::unpack_asset_by_id(&utoc, &usmap, pid, leaf, tmp_orig.path())
            }
            None => gore_tex::container::unpack_asset(&utoc, &usmap, asset, tmp_orig.path()),
        }
        .map_err(|e| ModError::Other(format!("unpack {asset}: {e}")))?;
        let ua = std::fs::read(&orig_uasset).map_err(io("read uasset"))?;
        let ue = std::fs::read(orig_uasset.with_extension("uexp")).map_err(io("read uexp"))?;
        let ub = gore_tex::paths::read_optional(&orig_uasset.with_extension("ubulk"))
            .map_err(io("read ubulk"))?;
        let png = read_safe_bundle_file(
            bundle_dir,
            Path::new(png_rel),
            "texture image payload",
            MAX_TEXTURE_IMAGE_BYTES,
        )?;
        let mut image_limits = image::Limits::default();
        image_limits.max_image_width = Some(32_768);
        image_limits.max_image_height = Some(32_768);
        let mut reader = image::ImageReader::new(std::io::Cursor::new(png))
            .with_guessed_format()
            .map_err(|e| ModError::Other(format!("png {png_rel}: {e}")))?;
        reader.limits(image_limits);
        let img = reader
            .decode()
            .map_err(|e| ModError::Other(format!("png {png_rel}: {e}")))?
            .to_rgba8();
        let (w, h) = (img.width(), img.height());
        let info = gore_tex::decode::parse(&ua, &ue, &ub, &usmap_bytes)
            .map_err(|e| ModError::Other(format!("parse {asset}: {e}")))?;
        // Unified entry: encodes mips (regular) or re-tiles (virtual
        // texture) internally based on the original's shape.
        let (na, ne, nb) = gore_tex::texdata::replace_texture_image(
            &ua,
            &ue,
            &ub,
            img.as_raw(),
            w,
            h,
            &info.format,
        )
        .map_err(|e| ModError::Other(format!("replace {asset}: {e}")))?;
        std::fs::write(dest_dir.join(format!("{leaf}.uasset")), &na).map_err(io("write uasset"))?;
        std::fs::write(dest_dir.join(format!("{leaf}.uexp")), &ne).map_err(io("write uexp"))?;
        if !nb.is_empty() {
            std::fs::write(dest_dir.join(format!("{leaf}.ubulk")), &nb)
                .map_err(io("write ubulk"))?;
        }
        // The unpacked original has been rewritten into cook_dir; dropping its guard now keeps a
        // many-texture mod from retaining one multi-MB tree per asset.
        drop(tmp_orig);
    }
    // Triplet name must be unique across DISTINCT mods (so one mod's mounted pak can't
    // be clobbered by another whose name sanitizes to the same stem) AND across multiple
    // texture components WITHIN this bundle. Append a stable hash of the original
    // (unsanitized) mod name for the former and the component index for the latter.
    let triplet_name = format!(
        "zzz_{}_{}_{}_tex_P",
        sanitize(mod_name),
        name_hash(mod_name),
        comp_idx
    );
    let pack_out = tempfile::Builder::new()
        .prefix("gore-mod-tex-pack-")
        .tempdir()
        .map_err(io("mkdir texture pack dir"))?;
    let triplet = gore_tex::container::repack_to_zen(
        cook_dir,
        &triplet_name,
        pack_out.path(),
        &game_dir,
        false,
    )
    .map_err(|e| ModError::Other(format!("pack: {e}")))?;
    // The cooked tree is now packed into the triplet; its guard can clean it immediately. The
    // pack root remains retained by the returned guard until apply/commit finishes.
    drop(cook_root);
    let mods_dir = game_dir
        .join("G1R")
        .join("Content")
        .join("Paks")
        .join("~mods");
    let mut out = Vec::new();
    for src in triplet {
        let dst = mods_dir.join(
            src.file_name()
                .ok_or_else(|| ModError::Other("triplet file".into()))?,
        );
        out.push((src, dst));
    }
    Ok((out, pack_out))
}

/// Prepare ONE pak-file component: pack every payload in the manifest at `path` (bundle-relative)
/// into a single plain V11 `.pak` named
/// `zzz_{sanitize(mod_name)}_{name_hash(mod_name)}_{comp_idx}_files_P`, mirroring the `_tex_P`
/// convention. Returns the (src, dst) pair to copy into `~mods` — temp work only, no game writes.
///
/// The container shape is the one the shipped paks use: mount point `../../../` with entries
/// spelled `G1R/Content/…`, so a claimed destination lands in the same virtual namespace the base
/// container resolves. Entries are stored uncompressed; the base game's Oodle slot is never
/// referenced, so the method table stays empty and correct.
fn prepare_pak_file_component(
    bundle_dir: &Path,
    path: &str,
    mod_name: &str,
    comp_idx: usize,
    gp: &GamePaths,
) -> Result<(Vec<(PathBuf, PathBuf)>, tempfile::TempDir)> {
    if !is_safe_rel_path(path) {
        return Err(ModError::Other(format!(
            "unsafe pak file patch path: {path:?}"
        )));
    }
    let map: BTreeMap<String, String> = serde_json::from_slice(&read_safe_bundle_file(
        bundle_dir,
        Path::new(&format!("{path}/manifest.json")),
        "pak file manifest",
        MAX_BUNDLE_MANIFEST_BYTES,
    )?)?;
    if map.is_empty() {
        return Err(ModError::Other(
            "pak file component claims no destinations; an empty pak would mount and do nothing"
                .into(),
        ));
    }
    // The pak name must be unique across DISTINCT mods and across several pak-file components in
    // one bundle, for exactly the reasons the triplet name is. Keeping `files` non-numeric before
    // `_P` also keeps `container_priority_key`'s `_<n>_P` version branch from firing on it.
    let pak_name = format!(
        "zzz_{}_{}_{}_files_P",
        sanitize(mod_name),
        name_hash(mod_name),
        comp_idx
    );
    let pack_out = tempfile::Builder::new()
        .prefix("gore-mod-pak-files-")
        .tempdir()
        .map_err(io("mkdir pak-files pack dir"))?;
    let src = pack_out.path().join(format!("{pak_name}.pak"));
    {
        let file = std::fs::File::create(&src)
            .map_err(io(&format!("creating loose-file pak {}", src.display())))?;
        let mut buffered = std::io::BufWriter::new(file);
        {
            let mut writer = repak::PakBuilder::new().writer(
                &mut buffered,
                repak::Version::V11,
                "../../../".to_string(),
                None,
            );
            for (game_path, payload_rel) in &map {
                // The manifest may come from an untrusted bundle: ask the same authoring question
                // the `files` section asks, against the same allowlist. One allowlist, one answer
                // to "may a bundle claim this destination" — the mechanism does not widen it.
                validate_loose_game_path(game_path)?;
                if !is_safe_rel_path(payload_rel) {
                    return Err(ModError::Other(format!(
                        "unsafe pak file payload path: {payload_rel:?}"
                    )));
                }
                let bytes = read_safe_bundle_file(
                    bundle_dir,
                    Path::new(payload_rel),
                    "pak file payload",
                    MAX_LOOSE_FILE_BYTES,
                )?;
                writer
                    .write_file(game_path, false, &bytes)
                    .map_err(|error| {
                        ModError::Other(format!("writing pak entry {game_path:?}: {error}"))
                    })?;
            }
            writer
                .write_index()
                .map_err(|error| ModError::Other(format!("writing pak index: {error}")))?;
        }
        let file = buffered.into_inner().map_err(|error| {
            ModError::Io(format!(
                "flushing loose-file pak {}: {error}",
                src.display()
            ))
        })?;
        file.sync_all()
            .map_err(io(&format!("syncing loose-file pak {}", src.display())))?;
    }
    let dst = gp
        .root
        .join("G1R")
        .join("Content")
        .join("Paks")
        .join("~mods")
        .join(format!("{pak_name}.pak"));
    Ok((vec![(src, dst)], pack_out))
}

/// Stage a prepared plan WITHOUT touching any live game file: snapshot each target's current
/// (pre-deploy) bytes into the undo, create its `*.gore-bak` backup, and record the intended
/// UE4SS mod dir. This runs BEFORE the deploy record is persisted; the actual live writes happen
/// later in [`apply_writes`], so a crash between record-write and apply is still recoverable.
fn stage(plan: &DeployPlan, record: &mut DeployRecord, undo: &mut Undo) -> Result<()> {
    // Note the intended UE4SS target(s) now so the persisted record knows about them even if a
    // crash interrupts the swap in `apply_writes` — undeploy can then still clean them up.
    //   - manager (`owner == "manager"`): a deployment composes SEVERAL mods, so ALL dirs go into
    //     `ue4ss_mod_dirs` and the single-mod `ue4ss_mod_dir` stays None (an old binary that can't
    //     read the vec falls back to the legacy-field mirror set up by `commit_plan`).
    //   - studio (`owner == ""`): at most one mod, so the first (only) dir keeps the legacy
    //     single-field shape and the vec stays empty.
    let manager = record.owner == "manager";
    for (i, (src, dst)) in plan.ue4ss_dirs.iter().enumerate() {
        let identity = plan.ue4ss_identities.get(dst).ok_or_else(|| {
            ModError::Other(format!(
                "UE4SS target identity was not prepared: {}",
                dst.display()
            ))
        })?;
        let staging = unique_ue4ss_swap_dir(dst, ".gore-ue4ss-stage-")?;
        let (holder, old) = match std::fs::symlink_metadata(dst) {
            Ok(_) => match unique_ue4ss_swap_dir(dst, ".gore-ue4ss-old-") {
                Ok(holder) => {
                    let old = holder.join("previous");
                    (Some(holder), Some(old))
                }
                Err(error) => {
                    return Err(with_directory_cleanup(
                        error,
                        &staging,
                        "cleaning UE4SS staging after old-holder creation failure",
                    ));
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, None),
            Err(error) => {
                return Err(with_directory_cleanup(
                    io("reading UE4SS destination metadata")(error),
                    &staging,
                    "cleaning UE4SS staging after destination metadata failure",
                ));
            }
        };
        undo.ue4ss_swaps.push(Ue4ssSwapUndo {
            staging: staging.clone(),
            holder: holder.clone(),
            old,
            dst: dst.clone(),
            intended_fingerprint: identity.intended.clone(),
            previous_fingerprint: identity.previous.clone(),
            state: Ue4ssSwapState::Prepared,
        });
        // Fully materialize and verify the non-live staging tree before the recovery record is
        // published. Once that record exists, staging is therefore always either the exact
        // intended tree or has already been atomically moved to its final destination; undeploy
        // never has to guess ownership of a partially copied transaction tree after a crash.
        if let Err(error) = copy_dir(src, &staging) {
            return Err(with_directory_cleanup(
                error,
                &staging,
                "cleaning partial UE4SS staging directory",
            ));
        }
        if tree_fingerprint(&staging)? != identity.intended {
            return Err(with_directory_cleanup(
                ModError::Other(format!(
                    "UE4SS source changed while staging: {}",
                    src.display()
                )),
                &staging,
                "cleaning changed UE4SS staging directory",
            ));
        }
        for transaction_dir in std::iter::once(staging).chain(holder) {
            let transaction_dir = transaction_dir.display().to_string();
            if !contains_same_path(&record.stale_ue4ss_dirs, &transaction_dir) {
                record.stale_ue4ss_dirs.push(transaction_dir.clone());
            }
        }
        let s = dst.display().to_string();
        record
            .ue4ss_tree_fingerprints
            .insert(s.clone(), identity.intended.clone());
        if let Some(previous) = &identity.previous {
            record
                .recovery_tree_fingerprints
                .entry(s.clone())
                .or_default()
                .push(previous.clone());
        }
        let swap = undo
            .ue4ss_swaps
            .last()
            .expect("UE4SS swap was pushed immediately above");
        let empty_tree =
            tree_fingerprint(swap.holder.as_deref().unwrap_or(swap.staging.as_path()))?;
        let staging_key = swap.staging.display().to_string();
        record
            .ue4ss_tree_fingerprints
            .insert(staging_key.clone(), identity.intended.clone());
        // Staging was fully copied above, before this record can be persisted, so its intended
        // fingerprint is the only accepted identity. Holders remain empty until one atomic move.
        if let (Some(holder), Some(_)) = (&swap.holder, &identity.previous) {
            let holder_key = holder.display().to_string();
            let moved_tree = tree_fingerprint_with_prefix(dst, Some("previous"))?;
            record
                .ue4ss_tree_fingerprints
                .insert(holder_key.clone(), moved_tree);
            record
                .recovery_tree_fingerprints
                .entry(holder_key)
                .or_default()
                .push(empty_tree);
        }
        if !manager && i == 0 {
            record.ue4ss_mod_dir = Some(s);
        } else if !record.ue4ss_mod_dirs.contains(&s) {
            record.ue4ss_mod_dirs.push(s);
        }
    }
    // Record the additive texture triplet dsts so undeploy can delete them (no backup needed).
    for (_, dst) in &plan.texture_triplets {
        let key = dst.display().to_string();
        let identity = plan.additive_identities.get(dst).ok_or_else(|| {
            ModError::Other(format!(
                "additive target identity was not prepared: {}",
                dst.display()
            ))
        })?;
        record.texture_triplets.push(key.clone());
        record
            .deployed_hashes
            .insert(key.clone(), identity.intended.clone());
        if let Some(previous) = &identity.previous {
            record
                .recovery_file_hashes
                .entry(key)
                .or_default()
                .push(previous.clone());
        }
    }
    // Same for the manager-installed pak/triplet dsts (additive, no backup).
    for (_, dst) in &plan.managed_paks {
        let key = dst.display().to_string();
        let identity = plan.additive_identities.get(dst).ok_or_else(|| {
            ModError::Other(format!(
                "additive target identity was not prepared: {}",
                dst.display()
            ))
        })?;
        record.managed_paks.push(key.clone());
        record
            .deployed_hashes
            .insert(key.clone(), identity.intended.clone());
        if let Some(previous) = &identity.previous {
            record
                .recovery_file_hashes
                .entry(key)
                .or_default()
                .push(previous.clone());
        }
    }
    for live in plan
        .writes
        .iter()
        .map(|(live, _)| live)
        .chain(plan.file_writes.iter().map(|write| &write.live))
    {
        // Snapshot the current (pre-deploy) bytes so rollback restores the EXACT prior state —
        // the previous mod's content, not just the game-pristine backup. If this read fails we
        // abort BEFORE writing anything, rather than snapshot empty and risk an empty-file rollback.
        let prior = verified_temp_copy(live, ".gore-undo-live-")?.into_temp_path();
        undo.files.push(LiveFileUndo {
            live: live.clone(),
            snapshot: prior,
            published_hash: None,
            backup: None,
        });
        let file_undo = undo
            .files
            .last_mut()
            .expect("live undo was pushed immediately above");
        // If the live file drifted (game updated) since our last deploy, its preserved backup is
        // stale: drop it so backup() re-snapshots the current file as the new pristine, instead of
        // keeping a pre-update backup that a future undeploy would restore over the newer asset.
        // The removal MUST succeed — if it can't (read-only/locked), backup() would keep the stale
        // backup, so fail the deploy now (stage runs pre-write, so the caller rolls back cleanly).
        if plan.refresh_baks.iter().any(|p| p == live) {
            let bak = bak_path(live);
            if bak.exists() {
                // Snapshot the stale backup before deleting so rollback can put it back — otherwise
                // a later-step failure would restore the previous record while its backup is gone.
                // A failed snapshot must abort BEFORE deletion. Otherwise rollback could restore
                // the prior record while its only pristine backup has vanished.
                let prior_bak = verified_temp_copy(&bak, ".gore-undo-backup-")?.into_temp_path();
                if std::fs::remove_file(&bak).is_err() && bak.exists() {
                    return Err(ModError::Other(format!(
                        "stale backup '{}' could not be removed (read-only or locked); close the \
                         game and retry so the updated game file can be re-backed-up",
                        bak.display()
                    )));
                }
                file_undo.backup = Some(BackupUndo {
                    path: bak,
                    created: false,
                    stale_snapshot: Some(prior_bak),
                    published_hash: String::new(),
                });
            }
        }
        let (bak, created) = backup(live, record)?;
        if created || file_undo.backup.is_some() {
            let backup_undo = file_undo.backup.get_or_insert_with(|| BackupUndo {
                path: bak.clone(),
                created: false,
                stale_snapshot: None,
                published_hash: String::new(),
            });
            backup_undo.path = bak;
            backup_undo.created = created;
            backup_undo.published_hash =
                backup_hash_for_path(&backup_undo.path, &record.backup_hashes)
                    .expect("backup() records the authenticated backup identity")
                    .clone();
        }
    }
    Ok(())
}

/// Perform the live changes of a staged plan: install/swap the UE4SS mod and write each target
/// file. Backups and undo snapshots were already taken by [`stage`]; on error the caller calls
/// `undo.rollback()` to restore the exact prior state.
fn apply_writes(plan: &DeployPlan, undo: &mut Undo) -> Result<()> {
    if undo.ue4ss_swaps.len() != plan.ue4ss_dirs.len() {
        return Err(ModError::Other(
            "UE4SS writes were not staged before apply".into(),
        ));
    }
    for ((src, dst), swap) in plan.ue4ss_dirs.iter().zip(&mut undo.ue4ss_swaps) {
        let identity = plan.ue4ss_identities.get(dst).ok_or_else(|| {
            ModError::Other(format!(
                "UE4SS target identity was not prepared: {}",
                dst.display()
            ))
        })?;
        // Stage already built the complete tree before the recovery record was persisted. Recheck
        // it here immediately before the live swap so a post-stage mutation cannot be installed.
        let staged_fingerprint = tree_fingerprint(&swap.staging)?;
        if staged_fingerprint != identity.intended {
            return Err(with_directory_cleanup(
                ModError::Other(format!(
                    "UE4SS source changed after preflight: {}",
                    src.display()
                )),
                &swap.staging,
                "cleaning changed UE4SS staging directory",
            ));
        }
        #[cfg(test)]
        apply_injected_ue4ss_replacement(dst)?;
        if let Some(old) = &swap.old {
            let expected_previous = identity.previous.as_deref().ok_or_else(|| {
                ModError::Other(format!(
                    "existing UE4SS target has no prepared prior identity: {}",
                    dst.display()
                ))
            })?;
            if !tree_matches_recorded_fingerprint(dst, expected_previous) {
                return Err(with_directory_cleanup(
                    ModError::Other(format!(
                        "owned UE4SS destination changed before apply: {}",
                        dst.display()
                    )),
                    &swap.staging,
                    "cleaning UE4SS staging after destination disappeared",
                ));
            }
            if let Err(error) = std::fs::rename(dst, old) {
                return Err(with_directory_cleanup(
                    io("moving old ue4ss mod aside")(error),
                    &swap.staging,
                    "cleaning UE4SS staging after move-aside failure",
                ));
            }
            swap.state = Ue4ssSwapState::OldMoved;
            // `old` contains the former destination at this point. Validate it against the exact
            // prior tree before installing anything new; a concurrent content change is rolled
            // back rather than silently adopted.
            if !tree_matches_recorded_fingerprint(old, expected_previous) {
                return Err(ModError::Other(format!(
                    "UE4SS destination changed while being moved aside: {}",
                    dst.display()
                )));
            }
            if let Err(error) = promote_ue4ss_staging(&swap.staging, dst) {
                return Err(with_directory_cleanup(
                    io("installing ue4ss mod")(error),
                    &swap.staging,
                    "cleaning failed UE4SS staging promotion",
                ));
            }
            swap.state = Ue4ssSwapState::PromotedReplacement;
        } else {
            if path_exists_no_follow(dst) {
                return Err(with_directory_cleanup(
                    ModError::Other(format!(
                        "unowned UE4SS destination appeared after preflight: {}",
                        dst.display()
                    )),
                    &swap.staging,
                    "cleaning UE4SS staging after destination collision",
                ));
            }
            if let Err(error) = promote_ue4ss_staging(&swap.staging, dst) {
                return Err(with_directory_cleanup(
                    io("installing ue4ss mod")(error),
                    &swap.staging,
                    "cleaning failed UE4SS staging promotion",
                ));
            }
            swap.state = Ue4ssSwapState::PromotedFresh;
        }
        if tree_fingerprint(dst)? != identity.intended {
            return Err(ModError::Other(format!(
                "published UE4SS tree failed identity verification: {}",
                dst.display()
            )));
        }
    }
    // Copy each texture triplet file into `~mods`, tracking it for rollback. Snapshot any bytes
    // already at `dst` BEFORE overwriting (a same-named redeploy targets the same paths as the
    // currently-active deployment) so rollback restores the prior active triplet rather than
    // deleting it; `None` marks a fresh addition that rollback should delete.
    for (src, dst) in &plan.texture_triplets {
        let identity = plan.additive_identities.get(dst).ok_or_else(|| {
            ModError::Other(format!(
                "additive target identity was not prepared: {}",
                dst.display()
            ))
        })?;
        publish_additive(src, dst, identity, undo)?;
    }
    // Copy each manager-installed pak/triplet file into place, tracked for rollback exactly
    // like the texture triplets above (prior bytes restored, fresh additions deleted). Their
    // srcs are durable library files — no temp pack dirs to clean up afterwards.
    for (src, dst) in &plan.managed_paks {
        let identity = plan.additive_identities.get(dst).ok_or_else(|| {
            ModError::Other(format!(
                "additive target identity was not prepared: {}",
                dst.display()
            ))
        })?;
        publish_additive(src, dst, identity, undo)?;
    }
    // Prepared pack sources remain alive for the complete commit/rollback call. Their unique
    // `temporary_roots` guards clean them when the plan drops on either success or failure; no
    // fixed path is removed while another concurrent plan may still depend on it.
    for (live, bytes) in &plan.writes {
        let file_undo = undo
            .files
            .iter_mut()
            .find(|entry| same_path(&entry.live, &live.display().to_string()))
            .ok_or_else(|| {
                ModError::Other(format!(
                    "live target was not staged before publication: {}",
                    live.display()
                ))
            })?;
        file_undo.published_hash = Some(content_hash(bytes));
        atomic_write(live, bytes)?;
    }
    for write in &plan.file_writes {
        let file_undo = undo
            .files
            .iter_mut()
            .find(|entry| same_path(&entry.live, &write.live.display().to_string()))
            .ok_or_else(|| {
                ModError::Other(format!(
                    "disk-backed live target was not staged before publication: {}",
                    write.live.display()
                ))
            })?;
        file_undo.published_hash = Some(write.hash.clone());
        atomic_publish_copy(&write.candidate, &write.live)?;
    }
    Ok(())
}

fn tree_matches_recorded_fingerprint(path: &Path, expected: &str) -> bool {
    tree_fingerprint(path).is_ok_and(|current| current == expected)
}

fn publish_additive(
    src: &Path,
    dst: &Path,
    identity: &PlannedIdentity,
    undo: &mut Undo,
) -> Result<()> {
    let parent = dst.parent().ok_or_else(|| {
        ModError::Other(format!(
            "additive destination has no parent: {}",
            dst.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(io("creating additive destination directory"))?;
    let staged = verified_temp_copy_in(src, parent, ".gore-additive-stage-")?;
    if sha256_file(staged.path())? != identity.intended {
        return Err(ModError::Other(format!(
            "additive source changed after preflight: {}",
            src.display()
        )));
    }
    #[cfg(test)]
    if take_injected_atomic_write_failure(dst) {
        return Err(ModError::Other(format!(
            "injected atomic-write failure for additive target {}",
            dst.display()
        )));
    }
    #[cfg(test)]
    apply_injected_additive_replacement(dst)?;

    if let Some(previous) = &identity.previous {
        if !file_matches_recorded_hash(dst, previous) {
            return Err(ModError::Other(format!(
                "owned additive destination changed before apply: {}",
                dst.display()
            )));
        }
        let prior = snapshot_existing_additive(dst)?.ok_or_else(|| {
            ModError::Other(format!(
                "owned additive destination disappeared before apply: {}",
                dst.display()
            ))
        })?;
        // Revalidate immediately before the replacing publication. The snapshot read itself can
        // take time for a large pak, so the earlier check is intentionally not the final one.
        if !file_matches_recorded_hash(dst, previous) {
            return Err(ModError::Other(format!(
                "owned additive destination changed while being snapshotted: {}",
                dst.display()
            )));
        }
        undo.texture_files.push(AdditiveFileUndo {
            path: dst.to_path_buf(),
            prior: Some(prior),
            published_hash: identity.intended.clone(),
        });
        publish_atomic_temp(staged, dst)?;
        sync_parent_directory(parent)?;
    } else {
        match std::fs::symlink_metadata(dst) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => {
                return Err(ModError::Other(format!(
                    "unowned additive destination appeared after preflight: {}",
                    dst.display()
                )))
            }
            Err(error) => return Err(io("rechecking fresh additive destination")(error)),
        }
        // No replace flag: a racing creator wins, and its same-path file is never overwritten.
        undo.texture_files.push(AdditiveFileUndo {
            path: dst.to_path_buf(),
            prior: None,
            published_hash: identity.intended.clone(),
        });
        publish_noclobber_temp(staged, dst)?;
    }
    if !file_matches_recorded_hash(dst, &identity.intended) {
        return Err(ModError::Other(format!(
            "published additive file failed SHA-256 verification: {}",
            dst.display()
        )));
    }
    Ok(())
}

fn snapshot_existing_additive(dst: &Path) -> Result<Option<tempfile::TempPath>> {
    match std::fs::symlink_metadata(dst) {
        Ok(_) => Ok(Some(
            verified_temp_copy(dst, ".gore-undo-additive-")?.into_temp_path(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io(&format!(
            "reading existing additive file metadata {}",
            dst.display()
        ))(error)),
    }
}

/// Retire the previous mod's leftover footprint (already folded into `record` and persisted):
/// restore each leftover loose file to pristine now and, on success, drop the entry from the
/// record so undeploy won't later look for a deleted backup. Leftovers that can't be restored
/// yet stay tracked. Also removes a differently-named UE4SS mod. Returns whether `record` changed
/// and therefore needs a final reconciliation write. Each removal itself uses a durable claim.
fn retire_leftovers(
    game_root: &Path,
    leftovers: &[(String, String, bool)],
    prev: Option<&DeployRecord>,
    plan: &DeployPlan,
    record: &mut DeployRecord,
) -> Result<bool> {
    let mut changed = false;
    for (live_s, bak_s, _) in leftovers {
        let (live, bak) = (Path::new(live_s), Path::new(bak_s));
        let retired = match safe_to_restore(live_s, record) {
            Ok(false) => true,
            Ok(true) if !bak.exists() => false,
            Ok(true) => {
                let identities = file_identities_for_path(record, live);
                if identities.is_empty() {
                    files_equal(live, bak)?
                } else {
                    let backup_hash = backup_hash_for_path(bak, &record.backup_hashes)
                        .ok_or_else(|| {
                            ModError::Other(format!(
                                "authenticated backup identity is missing for {}",
                                bak.display()
                            ))
                        })?
                        .clone();
                    durable_file_cleanup(
                        game_root,
                        record,
                        live,
                        identities,
                        Some((bak, &backup_hash)),
                    )?;
                    true
                }
            }
            // An unreadable hash or legacy path-only ownership cannot authorize a write. Keep the
            // entry tracked so a later explicit recovery can retry after operator intervention.
            Err(_) => false,
        };
        if retired {
            if path_exists_no_follow(bak) {
                let backup_identity = match backup_hash_for_path(bak, &record.backup_hashes) {
                    Some(hash) => hash.clone(),
                    None if path_exists_no_follow(live) && files_equal(live, bak)? => {
                        let hash = sha256_file(bak)?;
                        record
                            .backup_hashes
                            .insert(bak.display().to_string(), hash.clone());
                        hash
                    }
                    None => {
                        return Err(ModError::Other(format!(
                            "refusing to retire legacy path-only backup: {}",
                            bak.display()
                        )))
                    }
                };
                durable_file_cleanup(game_root, record, bak, vec![backup_identity], None)?;
            }
            record.deployed_hashes.remove(live_s);
            remove_backup_hash_for_path(bak, &mut record.backup_hashes);
            record
                .backups
                .retain(|(l, b, _)| !(l == live_s && b == bak_s));
            changed = true;
        }
    }
    if let Some(prev) = prev {
        // ALL comparisons here use `same_path`: the record holds canonicalized (`\\?\`-prefixed)
        // paths while the plan holds the plain form of the same file, so a raw compare could both
        // (a) fail to recognize a still-installed target as "keep" — retiring a file/dir this very
        // deploy just wrote — and (b) fail to prune a cleaned entry from the record. `same_path`
        // resolves both to the same canonical file.
        let new_dirs = plan.ue4ss_dsts();
        // Retire the previous deploy's UE4SS dir(s) AND any dirs it had already failed to remove
        // (its own `stale_ue4ss_dirs`). These were pre-seeded into `record.stale_ue4ss_dirs` for
        // crash-safety; here we actually remove them and reconcile the list: drop the ones we
        // cleaned, keep (locked/permissions) ones so a later undeploy still cleans them up.
        let prev_dirs: Vec<String> = prev
            .ue4ss_mod_dir
            .iter()
            .chain(prev.stale_ue4ss_dirs.iter())
            .chain(prev.ue4ss_mod_dirs.iter())
            .cloned()
            .collect();
        for prev_dir in prev_dirs {
            if contains_same_path(&new_dirs, &prev_dir) {
                continue;
            }
            let prev_path = Path::new(&prev_dir);
            let identity_matches = tree_cleanup_is_owned(record, prev_path);
            if identity_matches {
                durable_ue4ss_cleanup(game_root, record, prev_path)?;
                changed = true;
            }
        }

        // Retire the previous deploy's additive ~mods files (texture triplets AND manager paks) not
        // re-created by this deploy. They have no backup — just delete the files. Membership is
        // tested against the UNION of the plan's additive dsts (see `DeployPlan::additive_dsts`):
        // the manager legacy-mirror means a prev entry recorded under either field may be re-created
        // by this plan under the OTHER kind, and must not be deleted. On success prune from
        // whichever record field held it (pre-seeded above); on failure (locked) keep it tracked.
        let new_additive = plan.additive_dsts();
        let prev_additive: Vec<String> = prev
            .texture_triplets
            .iter()
            .chain(prev.managed_paks.iter())
            .cloned()
            .collect();
        for t in prev_additive {
            if contains_same_path(&new_additive, &t) {
                continue; // this deploy re-creates it; it stays as the active deployment's file
            }
            let target = Path::new(&t);
            let removed = if !file_cleanup_is_owned(record, target) {
                false
            } else if !path_exists_no_follow(target) {
                true
            } else {
                let identities = file_identities_for_path(record, target);
                durable_file_cleanup(game_root, record, target, identities, None)?;
                true
            };
            if removed {
                prune_file_identity(record, target);
                if let Some(i) = record
                    .texture_triplets
                    .iter()
                    .position(|x| same_path_s(x, &t))
                {
                    record.texture_triplets.remove(i);
                    changed = true;
                }
                if let Some(i) = record.managed_paks.iter().position(|x| same_path_s(x, &t)) {
                    record.managed_paks.remove(i);
                    changed = true;
                }
            }
            // not removed (locked) -> leave it tracked in record for a later undeploy to retry
        }
    }
    Ok(changed)
}

/// Claim a unique create-new swap directory one level above `ue4ss/Mods`. It remains on the same
/// filesystem for atomic renames, but outside the scanned `Mods` directory so `enabled.txt` cannot
/// make staging/recovery artifacts loadable after a crash.
fn unique_ue4ss_swap_dir(dst: &Path, prefix: &str) -> Result<PathBuf> {
    let ue4ss_root = dst.parent().and_then(Path::parent).ok_or_else(|| {
        ModError::Other(format!(
            "UE4SS destination is not below a Mods directory: {}",
            dst.display()
        ))
    })?;
    std::fs::create_dir_all(ue4ss_root).map_err(io("creating UE4SS swap root"))?;
    Ok(tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(ue4ss_root)
        .map_err(io("creating unique UE4SS transaction directory"))?
        .keep())
}

fn with_directory_cleanup(primary: ModError, path: &Path, context: &'static str) -> ModError {
    match remove_dir_all_durable(path, context) {
        Ok(()) => primary,
        Err(cleanup) => ModError::Other(format!(
            "{primary}; cleanup also failed for '{}': {cleanup}",
            path.display()
        )),
    }
}

fn remove_dir_all_durable(path: &Path, context: &'static str) -> Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {
            let parent = path.parent().ok_or_else(|| {
                ModError::Other(format!(
                    "removed directory has no parent: {}",
                    path.display()
                ))
            })?;
            sync_parent_directory(parent)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(context)(error)),
    }
}

fn remove_empty_dir_durable(path: &Path, context: &'static str) -> Result<()> {
    match std::fs::remove_dir(path) {
        Ok(()) => {
            let parent = path.parent().ok_or_else(|| {
                ModError::Other(format!(
                    "removed directory has no parent: {}",
                    path.display()
                ))
            })?;
            sync_parent_directory(parent)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(context)(error)),
    }
}

fn promote_ue4ss_staging(staging: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if take_injected_ue4ss_promotion_failure(dst) {
        return Err(std::io::Error::other(format!(
            "injected UE4SS promotion failure for {}",
            dst.display()
        )));
    }
    promote_directory_noclobber(staging, dst)?;
    if let Some(parent) = staging.parent() {
        sync_parent_directory(parent).map_err(|error| std::io::Error::other(error.to_string()))?;
    }
    if dst.parent() != staging.parent() {
        if let Some(parent) = dst.parent() {
            sync_parent_directory(parent)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn promote_directory_noclobber(staging: &Path, dst: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source: Vec<u16> = staging
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = dst
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Deliberately omit REPLACE_EXISTING: a racing creator owns the destination and wins.
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn promote_directory_noclobber(staging: &Path, dst: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt as _;

    let source = CString::new(staging.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let destination = CString::new(dst.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn promote_directory_noclobber(staging: &Path, dst: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt as _;

    let source = CString::new(staging.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let destination = CString::new(dst.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn promote_directory_noclobber(staging: &Path, dst: &Path) -> std::io::Result<()> {
    // Platforms without an exclusive rename primitive still get a final no-follow collision
    // check. Supported desktop targets above use an atomic no-replace operation.
    match std::fs::symlink_metadata(dst) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => std::fs::rename(staging, dst),
        Ok(_) => Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists)),
        Err(error) => Err(error),
    }
}

/// Whether `a` and the stored path string `b` refer to the same file, comparing canonical
/// forms when both resolve (falling back to a lexical compare otherwise).
fn same_path(a: &Path, b: &str) -> bool {
    let b = Path::new(b);
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => {
            #[cfg(windows)]
            {
                record_path_key(a) == record_path_key(b)
            }
            #[cfg(not(windows))]
            {
                a == b
            }
        }
    }
}

/// Whether two stored path STRINGS refer to the same file. Records hold already-canonicalized
/// paths (e.g. `\\?\C:\...` on Windows), but a plan built this run may hold the plain form of the
/// same file; a raw string `==`/`contains` would then miss the match. Canonical-form compare (with
/// a lexical fallback when a side can't be resolved) so prev-vs-new membership checks — pre-seed
/// and `retire_leftovers` — recognize logically-identical paths regardless of prefix/case.
fn same_path_s(a: &str, b: &str) -> bool {
    same_path(Path::new(a), b)
}

fn path_exists_no_follow(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

/// Whether `list` already contains a path referring to the same file as `p` (`same_path_s`).
fn contains_same_path(list: &[String], p: &str) -> bool {
    list.iter().any(|x| same_path_s(x, p))
}

#[derive(Clone, Copy)]
enum RecordPathClass {
    LiveFile,
    BackupFile,
    AdditiveFile,
    Ue4ssDirectory,
    CleanupHolder,
}

fn validate_record(record_root_hint: &Path, record: &DeployRecord) -> Result<()> {
    let selected_root = record_root(record_root_hint);
    let root_metadata = std::fs::symlink_metadata(&selected_root)
        .map_err(io("reading selected game-root metadata for deploy record"))?;
    if metadata_is_link(&root_metadata) || !root_metadata.is_dir() {
        return Err(ModError::Other(format!(
            "selected game root is not a real directory: {}",
            selected_root.display()
        )));
    }
    let canonical_root = std::fs::canonicalize(&selected_root)
        .map_err(io("canonicalizing selected game root for deploy record"))?;

    for (live, backup, _) in &record.backups {
        let live_path = validate_record_path(&canonical_root, live, RecordPathClass::LiveFile)?;
        let backup_path =
            validate_record_path(&canonical_root, backup, RecordPathClass::BackupFile)?;
        let expected_backup = bak_path(&live_path);
        if record_path_key(&backup_path) != record_path_key(&expected_backup) {
            return Err(ModError::Other(format!(
                "deploy record backup is not the exact .gore-bak sibling of its live file: {} -> {}",
                live_path.display(),
                backup_path.display()
            )));
        }
    }
    for (backup, hash) in &record.backup_hashes {
        validate_record_path(&canonical_root, backup, RecordPathClass::BackupFile)?;
        if !hash.starts_with("sha256:")
            || hash.len() != "sha256:".len() + 64
            || !hash["sha256:".len()..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ModError::Other(format!(
                "deploy record contains an invalid backup SHA-256 identity for {backup}"
            )));
        }
        if !record
            .backups
            .iter()
            .any(|(_, stored_backup, _)| same_path(Path::new(backup), stored_backup))
        {
            return Err(ModError::Other(format!(
                "deploy record contains an untracked backup identity: {backup}"
            )));
        }
    }

    for path in record
        .ue4ss_mod_dir
        .iter()
        .chain(record.stale_ue4ss_dirs.iter())
        .chain(record.ue4ss_mod_dirs.iter())
    {
        validate_record_path(&canonical_root, path, RecordPathClass::Ue4ssDirectory)?;
    }
    for path in record
        .texture_triplets
        .iter()
        .chain(record.managed_paks.iter())
    {
        validate_record_path(&canonical_root, path, RecordPathClass::AdditiveFile)?;
    }
    for path in record.deployed_hashes.keys() {
        if validate_record_path(&canonical_root, path, RecordPathClass::LiveFile).is_err()
            && validate_record_path(&canonical_root, path, RecordPathClass::AdditiveFile).is_err()
        {
            return Err(ModError::Other(format!(
                "deploy record hash key is not an allowed game target: {path}"
            )));
        }
    }
    for path in record.recovery_file_hashes.keys() {
        if validate_record_path(&canonical_root, path, RecordPathClass::LiveFile).is_err()
            && validate_record_path(&canonical_root, path, RecordPathClass::AdditiveFile).is_err()
        {
            return Err(ModError::Other(format!(
                "deploy record recovery hash key is not an allowed game target: {path}"
            )));
        }
    }
    for path in record
        .ue4ss_tree_fingerprints
        .keys()
        .chain(record.recovery_tree_fingerprints.keys())
    {
        validate_record_path(&canonical_root, path, RecordPathClass::Ue4ssDirectory)?;
    }
    for (original, holder) in &record.ue4ss_cleanup_claims {
        validate_record_path(&canonical_root, original, RecordPathClass::Ue4ssDirectory)?;
        validate_record_path(&canonical_root, holder, RecordPathClass::Ue4ssDirectory)?;
        let original_path = Path::new(original);
        let holder_path = Path::new(holder);
        if same_path(original_path, holder)
            || !holder_path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".gore-ue4ss-delete-"))
        {
            return Err(ModError::Other(format!(
                "UE4SS cleanup holder is not a distinct generated delete holder: {holder}"
            )));
        }
        if !record
            .stale_ue4ss_dirs
            .iter()
            .any(|stored| same_path(holder_path, stored))
        {
            return Err(ModError::Other(format!(
                "UE4SS cleanup holder is not tracked as stale: {holder}"
            )));
        }
        let trusted = record
            .trusted_ue4ss_tombstones
            .iter()
            .any(|stored| same_path(holder_path, stored));
        if !trusted {
            let original_tracked = record
                .ue4ss_mod_dir
                .iter()
                .chain(record.ue4ss_mod_dirs.iter())
                .chain(record.stale_ue4ss_dirs.iter())
                .any(|stored| same_path(original_path, stored));
            let original_identities = tree_identities_for_path(record, original_path);
            let holder_identities = tree_identities_for_path(record, holder_path);
            if !original_tracked
                || original_identities.is_empty()
                || holder_identities != original_identities
            {
                return Err(ModError::Other(format!(
                    "UE4SS cleanup claim is not authorized by the tracked original identity: {original}"
                )));
            }
        }
    }
    for holder in &record.trusted_ue4ss_tombstones {
        validate_record_path(&canonical_root, holder, RecordPathClass::Ue4ssDirectory)?;
        if !record
            .ue4ss_cleanup_claims
            .values()
            .any(|stored| same_path(Path::new(holder), stored))
        {
            return Err(ModError::Other(format!(
                "trusted UE4SS tombstone has no cleanup claim: {holder}"
            )));
        }
    }
    for (source, claim) in &record.file_cleanup_claims {
        let source_path = Path::new(source);
        let is_live =
            validate_record_path(&canonical_root, source, RecordPathClass::LiveFile).is_ok();
        let is_backup =
            validate_record_path(&canonical_root, source, RecordPathClass::BackupFile).is_ok();
        let is_additive =
            validate_record_path(&canonical_root, source, RecordPathClass::AdditiveFile).is_ok();
        if !is_live && !is_backup && !is_additive {
            return Err(ModError::Other(format!(
                "file cleanup claim source is not an allowed target: {source}"
            )));
        }
        validate_record_path(
            &canonical_root,
            &claim.holder,
            RecordPathClass::CleanupHolder,
        )?;
        if claim.expected_hashes.is_empty()
            || claim.expected_hashes.iter().any(|hash| {
                !((hash.starts_with("sha256:")
                    && hash.len() == "sha256:".len() + 64
                    && hash["sha256:".len()..]
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()))
                    || (hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())))
            })
        {
            return Err(ModError::Other(format!(
                "file cleanup claim has an invalid expected identity: {source}"
            )));
        }
        // A trusted tombstone can only delete its already-isolated holder. The source was already
        // atomically claimed and authenticated, and backup bookkeeping may have been pruned in the
        // same durable transition. Requiring the original authorization fields here would make
        // that crash-safe terminal state impossible to deserialize.
        if record
            .trusted_file_tombstones
            .iter()
            .any(|stored| same_path(Path::new(&claim.holder), stored))
        {
            continue;
        }
        match (&claim.restore_from, &claim.restore_hash) {
            (None, None) => {
                if is_live {
                    return Err(ModError::Other(format!(
                        "delete-only cleanup claim cannot target a live game file: {source}"
                    )));
                }
                if is_backup {
                    let tracked = record
                        .backups
                        .iter()
                        .any(|(_, backup, _)| same_path(source_path, backup));
                    let recorded = backup_hash_for_path(source_path, &record.backup_hashes);
                    if !tracked
                        || recorded.is_none()
                        || claim.expected_hashes.len() != 1
                        || recorded != claim.expected_hashes.first()
                    {
                        return Err(ModError::Other(format!(
                            "backup cleanup claim is not authorized by its tracked identity: {source}"
                        )));
                    }
                } else {
                    let tracked = record
                        .texture_triplets
                        .iter()
                        .chain(record.managed_paks.iter())
                        .any(|stored| same_path(source_path, stored));
                    let recorded = file_identities_for_path(record, source_path);
                    if !tracked
                        || claim
                            .expected_hashes
                            .iter()
                            .any(|hash| !recorded.contains(hash))
                    {
                        return Err(ModError::Other(format!(
                            "additive cleanup claim is not authorized by its tracked identity: {source}"
                        )));
                    }
                }
            }
            (Some(backup), Some(hash)) => {
                if !is_live {
                    return Err(ModError::Other(format!(
                        "restore cleanup claim must target a live game file: {source}"
                    )));
                }
                validate_record_path(&canonical_root, backup, RecordPathClass::BackupFile)?;
                if !hash.starts_with("sha256:")
                    || hash.len() != "sha256:".len() + 64
                    || !hash["sha256:".len()..]
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(ModError::Other(format!(
                        "file cleanup restore has an invalid backup identity: {source}"
                    )));
                }
                let expected_backup = bak_path(source_path);
                let tracked_pair = record.backups.iter().any(|(live, stored_backup, _)| {
                    same_path(source_path, live)
                        && same_path(&expected_backup, stored_backup)
                        && same_path(Path::new(backup), stored_backup)
                });
                let recorded_backup_hash =
                    backup_hash_for_path(Path::new(backup), &record.backup_hashes);
                let recorded_live_hashes = file_identities_for_path(record, source_path);
                if record_path_key(Path::new(backup)) != record_path_key(&expected_backup)
                    || !tracked_pair
                    || recorded_backup_hash != Some(hash)
                    || claim
                        .expected_hashes
                        .iter()
                        .any(|identity| !recorded_live_hashes.contains(identity))
                {
                    return Err(ModError::Other(format!(
                        "file cleanup restore is not authorized by the matching live/backup record: {source}"
                    )));
                }
            }
            _ => {
                return Err(ModError::Other(format!(
                    "file cleanup restore source/hash must be present together: {source}"
                )))
            }
        }
    }
    for holder in &record.trusted_file_tombstones {
        validate_record_path(&canonical_root, holder, RecordPathClass::CleanupHolder)?;
        if !record
            .file_cleanup_claims
            .values()
            .any(|claim| same_path(Path::new(holder), &claim.holder))
        {
            return Err(ModError::Other(format!(
                "trusted file tombstone has no cleanup claim: {holder}"
            )));
        }
    }
    Ok(())
}

fn validate_record_path(root: &Path, stored: &str, class: RecordPathClass) -> Result<PathBuf> {
    let path = Path::new(stored);
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err(ModError::Other(format!(
            "deploy record contains a non-absolute or non-normal path: {stored}"
        )));
    }
    let relative = record_relative_path(root, path).ok_or_else(|| {
        ModError::Other(format!(
            "deploy record path escapes the selected game root '{}': {stored}",
            root.display()
        ))
    })?;
    if relative.as_os_str().is_empty() || !record_path_matches_class(&relative, class) {
        return Err(ModError::Other(format!(
            "deploy record path is outside its allowed target class: {stored}"
        )));
    }

    // Walk without following the final component (or any ancestor). A canonical containment check
    // alone would still accept an in-root symlink/reparse point, which could later be retargeted
    // between validation and destructive cleanup.
    let mut current = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    let mut missing_ancestor = false;
    for (index, component) in components.iter().enumerate() {
        let std::path::Component::Normal(name) = component else {
            return Err(ModError::Other(format!(
                "deploy record contains a non-normal path component: {stored}"
            )));
        };
        current.push(name);
        if missing_ancestor {
            continue;
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata_is_link(&metadata) {
                    return Err(ModError::Other(format!(
                        "deploy record path crosses a symlink or reparse point: {}",
                        current.display()
                    )));
                }
                if index + 1 < components.len() && !metadata.is_dir() {
                    return Err(ModError::Other(format!(
                        "deploy record path crosses a non-directory ancestor: {}",
                        current.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing_ancestor = true;
            }
            Err(error) => {
                return Err(io("reading deploy-record target metadata")(error));
            }
        }
    }
    Ok(root.join(relative))
}

fn record_relative_path(root: &Path, path: &Path) -> Option<PathBuf> {
    if let Ok(relative) = path.strip_prefix(root) {
        return Some(relative.to_path_buf());
    }
    #[cfg(windows)]
    {
        fn normalized(path: &Path) -> String {
            let mut value = path.to_string_lossy().replace('\\', "/");
            if let Some(rest) = value.strip_prefix("//?/UNC/") {
                value = format!("//{rest}");
            } else if let Some(rest) = value.strip_prefix("//?/") {
                value = rest.to_string();
            }
            value.trim_end_matches('/').to_lowercase()
        }
        let root = normalized(root);
        let path = normalized(path);
        let prefix = format!("{root}/");
        path.strip_prefix(&prefix)
            .map(|relative| PathBuf::from(relative.replace('/', "\\")))
    }
    #[cfg(not(windows))]
    None
}

fn record_path_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        let value = value
            .strip_prefix("//?/UNC/")
            .map(|rest| format!("//{rest}"))
            .or_else(|| value.strip_prefix("//?/").map(str::to_string))
            .unwrap_or(value);
        value.to_lowercase()
    }
    #[cfg(not(windows))]
    value
}

fn record_path_matches_class(relative: &Path, class: RecordPathClass) -> bool {
    let parts: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => {
                Some(value.to_string_lossy().to_ascii_lowercase())
            }
            _ => None,
        })
        .collect();
    let file_name_safe = || {
        relative
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_safe_filename)
    };
    match class {
        RecordPathClass::LiveFile => {
            fixed_live_file(&parts, file_name_safe()) || loose_target_allowed(relative)
        }
        RecordPathClass::BackupFile => {
            let relative = relative.to_string_lossy();
            relative.ends_with(".gore-bak")
                && record_path_matches_class(
                    Path::new(&relative[..relative.len() - ".gore-bak".len()]),
                    RecordPathClass::LiveFile,
                )
        }
        RecordPathClass::AdditiveFile => {
            parts.len() == 5
                && parts[0] == "g1r"
                && parts[1] == "content"
                && parts[2] == "paks"
                && parts[3] == "~mods"
                && matches!(
                    Path::new(&parts[4])
                        .extension()
                        .and_then(|ext| ext.to_str()),
                    Some("pak" | "utoc" | "ucas")
                )
                && file_name_safe()
        }
        RecordPathClass::Ue4ssDirectory => {
            (parts.len() == 6
                && parts[0] == "g1r"
                && parts[1] == "binaries"
                && parts[2] == "win64"
                && parts[3] == "ue4ss"
                && parts[4] == "mods"
                && file_name_safe())
                || (parts.len() == 5
                    && parts[0] == "g1r"
                    && parts[1] == "binaries"
                    && parts[2] == "win64"
                    && parts[3] == "ue4ss"
                    && (parts[4].starts_with(".gore-ue4ss-stage-")
                        || parts[4].starts_with(".gore-ue4ss-old-")
                        || parts[4].starts_with(".gore-ue4ss-delete-"))
                    && file_name_safe())
        }
        RecordPathClass::CleanupHolder => {
            parts.len() == 1 && parts[0].starts_with(".gore-mod-cleanup-") && file_name_safe()
        }
    }
}

/// The four FIXED in-place targets, each of which has its own deploy mechanism: the localization
/// cache, an FMOD bank, the precompiled script cache, a voice ZIP. `parts` are the lowercased
/// normal components of the game-root-relative path.
fn fixed_live_file(parts: &[String], file_name_safe: bool) -> bool {
    (parts.len() == 4
        && parts[0] == "g1r"
        && parts[1] == "story"
        && parts[2] == "cache"
        && parts[3].ends_with(".lcache")
        && file_name_safe)
        || (parts.len() == 5
            && parts[0] == "g1r"
            && parts[1] == "content"
            && parts[2] == "fmod"
            && parts[3] == "desktop"
            && parts[4].ends_with(".bank")
            && file_name_safe)
        || (parts.len() == 3
            && parts[0] == "g1r"
            && parts[1] == "script"
            && parts[2] == "precompiledscript_shipping.cache")
        || (parts.len() == 4
            && parts[0] == "g1r"
            && parts[1] == "story"
            && parts[2] == "voiceover"
            && parts[3].ends_with(".zip")
            && file_name_safe)
}

/// May a bundle REPLACE the loose game file at `relative` (a game-root-relative path)?
///
/// This is the whole new safety surface of the `files` section, and it is deliberately one
/// function: build-time authoring, manager import and every deploy-record write ask exactly this
/// question, so a destination can never be accepted at one layer and refused at another. Widening
/// `RecordPathClass::LiveFile` to "any relative path" instead would let a hostile or corrupted
/// record name `G1R/Binaries/Win64/G1R-Win64-Shipping.exe` and have undeploy write a "backup"
/// over it.
///
/// Allowed: `G1R/Content/**` and `G1R/Config/**` — the two subtrees Unreal reads loose files from.
/// `G1R/Binaries/**` is therefore excluded by construction, along with `Story`, `Script`, `Saved`
/// and anything at the install root.
///
/// Denied inside those subtrees:
/// * `G1R/Content/Paks/**` — owned by [`RecordPathClass::AdditiveFile`] and by the IoStore
///   containers; a loose write there would fight a pak this same deploy installs.
/// * anything ending in `.gore-bak` — that spelling IS how the restore machinery names a snapshot,
///   so accepting one would let a record aim a restore at its own backup.
/// * anything matching a fixed live-file shape — a second class for one file would bypass the
///   checks that class carries (bank key, lcache decode, voice reseal).
/// * the deploy record itself, under any spelling.
fn loose_target_allowed(relative: &Path) -> bool {
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        // Unlike `record_path_matches_class`, this is also called on authored input that has not
        // been through `validate_record_path` yet, so a non-normal component is refused here
        // rather than silently dropped.
        let std::path::Component::Normal(value) = component else {
            return false;
        };
        let Some(text) = value.to_str() else {
            return false;
        };
        if !is_safe_filename(text) || text.eq_ignore_ascii_case(RECORD_NAME) {
            return false;
        }
        parts.push(text.to_ascii_lowercase());
    }
    // `G1R/<subtree>/<file>` at minimum: a bare file at the install root is never a loose target,
    // and neither is a whole directory.
    if parts.len() < 3 || parts[0] != "g1r" {
        return false;
    }
    if parts[1] != "content" && parts[1] != "config" {
        return false;
    }
    if parts[1] == "content" && parts[2] == "paks" {
        return false;
    }
    let last = parts.last().expect("length was checked above");
    if last.ends_with(".gore-bak") {
        return false;
    }
    // Every component was checked with `is_safe_filename` above, so the file-name predicate the
    // fixed shapes take is already satisfied.
    !fixed_live_file(&parts, true)
}

/// The `*.gore-bak` paths the active deployment owns, or `None` when nothing is deployed.
///
/// Exists so a caller can tell a backup that belongs to the deployment from one that does not,
/// without being handed the record to interpret. `gore doctor` is the caller: a `.gore-bak` beside
/// a game file is normal while a deployment holds it, and a leftover from an unrelated in-place
/// edit is exactly what it is looking for. Knowing only THAT a record exists made every backup on
/// the install look like part of it.
pub fn deployed_backup_paths(game_root: &Path) -> Result<Option<Vec<PathBuf>>> {
    let game_root = abs_root(game_root);
    let Some(stored) = read_record(&game_root)? else {
        return Ok(None);
    };
    Ok(Some(
        stored
            .record
            .backups
            .into_iter()
            .map(|(_, backup, _)| PathBuf::from(backup))
            .collect(),
    ))
}

fn read_record(game_root: &Path) -> Result<Option<StoredDeployRecord>> {
    let path = record_path(game_root);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io("reading deploy record metadata")(error)),
    }
    let raw = read_regular_file_limited(&path, "deploy record", MAX_DEPLOY_RECORD_BYTES).map_err(
        |error| {
            ModError::Other(format!(
                "reading deploy record {}: {error}; recovery record was left untouched",
                path.display()
            ))
        },
    )?;
    let record: DeployRecord = serde_json::from_slice(&raw).map_err(|error| {
        ModError::Other(format!(
            "parsing deploy record {}: {error}; recovery record was left untouched",
            path.display()
        ))
    })?;
    validate_record(game_root, &record).map_err(|error| {
        ModError::Other(format!(
            "rejecting unsafe deploy record {}: {error}; recovery record was left untouched",
            path.display()
        ))
    })?;
    Ok(Some(StoredDeployRecord { record, raw }))
}

fn recovery_required_error() -> ModError {
    ModError::Other(
        "RECOVERY_REQUIRED: a previous deploy was interrupted; run undeploy to restore the game \
         before applying another deployment"
            .into(),
    )
}

/// Read-only persistent recovery gate for compiler/install-state UIs.
///
/// This reuses the same bounded, no-follow, schema/path-validated deploy-record reader as deploy
/// and pristine resolution. It never repairs, removes, rewrites, or publishes anything.
pub fn deploy_recovery_required(game_root: &Path) -> Result<bool> {
    Ok(read_record(game_root)?
        .as_ref()
        .is_some_and(|stored| stored.record.phase == DeployPhase::RecoveryRequired))
}

/// Pristine bytes to rebuild a modded file from, plus whether the live file has DRIFTED from what
/// we previously deployed there (e.g. Steam verified/updated it). Normally the preserved
/// `*.gore-bak` is the pristine source; but if `prev` recorded a hash for this file and the
/// current live no longer matches it while a backup exists, that backup is stale (pre-update) —
/// rebuilding from it would write an old asset over the newer game file. In that case the
/// (updated) live IS the new pristine and the caller must refresh the stale backup. Never writes.
#[derive(Debug, Clone)]
pub(crate) struct PristineSource {
    pub(crate) path: PathBuf,
    pub(crate) drifted: bool,
}

/// Select the pristine source without materializing it. Drift is decided by a streaming hash and
/// every hash/open error is surfaced; an I/O failure is never silently interpreted as "no drift".
/// Legacy/path-only records are deliberately fail-closed when live and backup differ: without a
/// deployed hash there is no proof that restoring/reusing the backup would not overwrite a later
/// game update. Byte-identical live/backup pairs remain safely cleanable.
pub(crate) fn select_pristine_source(
    live: &Path,
    prev: Option<&DeployRecord>,
) -> Result<PristineSource> {
    // Even callers that only need the drift bit must not hash an attacker-sized path forever.
    ensure_pristine_sources_bounded(live, gore_vo::Limits::default().max_archive_bytes)?;
    let live_metadata = std::fs::symlink_metadata(live).map_err(io(&format!(
        "reading live pristine metadata {}",
        live.display()
    )))?;
    if metadata_is_link(&live_metadata) || !live_metadata.is_file() {
        return Err(ModError::Other(format!(
            "live pristine source is not a regular non-link file: {}",
            live.display()
        )));
    }

    let backup = bak_path(live);
    match std::fs::symlink_metadata(&backup) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if prev.is_some_and(|record| {
                record
                    .backups
                    .iter()
                    .any(|(stored_live, stored_backup, _)| {
                        same_path(live, stored_live) && same_path(&backup, stored_backup)
                    })
            }) {
                return Err(ModError::Other(format!(
                    "active deployment's authenticated pristine backup is missing: {}",
                    backup.display()
                )));
            }
            return Ok(PristineSource {
                path: live.to_path_buf(),
                drifted: false,
            });
        }
        Err(error) => return Err(io("reading pristine backup metadata")(error)),
        Ok(metadata) if metadata_is_link(&metadata) || !metadata.is_file() => {
            return Err(ModError::Other(format!(
                "pristine backup is not a regular non-link file: {}",
                backup.display()
            )));
        }
        Ok(_) => {}
    }

    let backup_identity =
        prev.and_then(|record| backup_hash_for_path(&backup, &record.backup_hashes));
    if let Some(expected) = backup_identity {
        if !expected.starts_with("sha256:")
            || !file_matches_recorded_hash_result(&backup, expected)?
        {
            return Err(ModError::Other(format!(
                "pristine backup content no longer matches its recorded identity: {}",
                backup.display()
            )));
        }
    } else {
        // A path-only backup is not a trustworthy restore source.  The sole safe legacy case is
        // when using it is observationally identical to using the current live file.
        if sha256_file(live)? != sha256_file(&backup)? {
            return Err(ModError::Other(format!(
                "unverifiable legacy pristine backup for '{}': live and backup differ but no backup SHA-256 is recorded; refusing path-only restore/rebuild",
                live.display()
            )));
        }
        return Ok(PristineSource {
            path: backup,
            drifted: false,
        });
    }

    let live_key = live.display().to_string();
    if let Some(expected) = prev.and_then(|p| deployed_hash_for_path(&live_key, &p.deployed_hashes))
    {
        if file_matches_recorded_hash_result(live, expected)? {
            return Ok(PristineSource {
                path: backup,
                drifted: false,
            });
        }
        return Ok(PristineSource {
            path: live.to_path_buf(),
            drifted: true,
        });
    }

    // A new-format backup identity proves the restore source, but without a live identity we still
    // cannot decide whether differing live bytes are ours or a later game update.
    if sha256_file(live)? == backup_identity.expect("checked above").as_str() {
        Ok(PristineSource {
            path: backup,
            drifted: false,
        })
    } else {
        Err(ModError::Other(format!(
            "unverifiable legacy live deployment for '{}': live differs from its authenticated backup but no deployed hash is recorded",
            live.display()
        )))
    }
}

pub(crate) fn read_pristine_bounded(
    live: &Path,
    prev: Option<&DeployRecord>,
    max_bytes: u64,
) -> Result<(Vec<u8>, bool)> {
    ensure_pristine_sources_bounded(live, max_bytes)?;
    let source = select_pristine_source(live, prev)?;
    let bytes = read_regular_file_limited(
        &source.path,
        &format!("pristine patch base for {}", live.display()),
        max_bytes,
    )?;
    Ok((bytes, source.drifted))
}

fn ensure_pristine_sources_bounded(live: &Path, max_bytes: u64) -> Result<()> {
    for (path, label) in [
        (live.to_path_buf(), "live pristine source"),
        (bak_path(live), "pristine backup"),
    ] {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata_is_link(&metadata) || !metadata.is_file() {
                    return Err(ModError::Other(format!(
                        "{label} is not a regular non-link file: {}",
                        path.display()
                    )));
                }
                if metadata.len() > max_bytes {
                    return Err(ModError::Other(format!(
                        "{label} exceeds the {max_bytes}-byte limit: {} bytes at {}",
                        metadata.len(),
                        path.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && path != live => {}
            Err(error) => return Err(io(&format!("reading {label} metadata"))(error)),
        }
    }
    Ok(())
}

pub(crate) fn bak_path(live: &Path) -> PathBuf {
    let mut s = live.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

/// The PRISTINE precompiled-script cache bytes deploy would use for `game_root`, honoring drift:
/// if a `*.gore-bak` backup exists but the live cache has DRIFTED from what we last deployed there
/// (game update/verify), the backup is stale and the (updated) live cache is the new pristine.
/// Reuses the same [`read_pristine_bounded`]/[`read_record`] logic so the compile base matches
/// the bytes the splice will later be applied against. Never writes.
pub fn pristine_script_cache(game_root: &Path) -> Result<Vec<u8>> {
    let script_cache = resolve_game_paths(game_root).script_cache;
    let record = read_record(game_root)?;
    let prior = record.as_ref().map(|stored| &stored.record);
    if prior.is_some_and(|record| record.phase == DeployPhase::RecoveryRequired) {
        return Err(recovery_required_error());
    }
    read_pristine_bounded(&script_cache, prior, MAX_PRISTINE_PATCH_BYTES)
        .map(|(bytes, _drifted)| bytes)
}

fn file_identities_for_path(record: &DeployRecord, path: &Path) -> Vec<String> {
    let key = path.display().to_string();
    let mut identities = Vec::new();
    if let Some(hash) = deployed_hash_for_path(&key, &record.deployed_hashes) {
        identities.push(hash.clone());
    }
    if let Some(hashes) = values_for_path(path, &record.recovery_file_hashes) {
        identities.extend(hashes.iter().cloned());
    }
    identities.sort();
    identities.dedup();
    identities
}

fn path_matches_any_file_identity(path: &Path, identities: &[String]) -> Result<bool> {
    for identity in identities {
        if file_matches_recorded_hash_result(path, identity)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn unique_cleanup_holder(game_root: &Path) -> Result<PathBuf> {
    let root = record_root(game_root);
    let holder = tempfile::Builder::new()
        .prefix(".gore-mod-cleanup-")
        .tempdir_in(&root)
        .map_err(io("creating durable cleanup holder"))?
        .keep();
    sync_parent_directory(&root)?;
    Ok(holder)
}

/// Once an authenticated backup has been isolated and its tombstone is trusted, remove the
/// corresponding live/backup ownership in that same durable record transition. Otherwise a crash
/// after deleting the holder but before the outer undeploy loop prunes the tuple can leave a
/// byte-identical live file permanently blocked on a backup that we already deleted.
fn prune_completed_backup_claim(record: &mut DeployRecord, backup: &Path) -> bool {
    let lives: Vec<String> = record
        .backups
        .iter()
        .filter(|(_, stored_backup, _)| same_path(backup, stored_backup))
        .map(|(live, _, _)| live.clone())
        .collect();
    if lives.is_empty() {
        return false;
    }
    record
        .backups
        .retain(|(_, stored_backup, _)| !same_path(backup, stored_backup));
    for live in lives {
        remove_deployed_hash_for_path(&live, &mut record.deployed_hashes);
        remove_vec_map_path(Path::new(&live), &mut record.recovery_file_hashes);
    }
    remove_backup_hash_for_path(backup, &mut record.backup_hashes);
    true
}

/// Atomically claim `source` into a durable, non-scanned holder, authenticate the claimed object,
/// optionally restore an authenticated backup to the original name with no-clobber publication,
/// then tombstone/delete the holder.  Every state transition is persisted before the corresponding
/// destructive step, so `process_file_cleanup_claims` can resume after a crash.
fn durable_file_cleanup(
    game_root: &Path,
    record: &mut DeployRecord,
    source: &Path,
    mut expected_hashes: Vec<String>,
    restore: Option<(&Path, &str)>,
) -> Result<()> {
    #[cfg(test)]
    if take_injected_durable_remove_failure(source) {
        return Err(ModError::Other(format!(
            "injected durable-remove failure for {}",
            source.display()
        )));
    }
    if expected_hashes.is_empty() {
        return Err(ModError::Other(format!(
            "refusing path-only file cleanup without an exact identity: {}",
            source.display()
        )));
    }
    expected_hashes.sort();
    expected_hashes.dedup();
    let source_key = source.display().to_string();
    if !record
        .file_cleanup_claims
        .keys()
        .any(|stored| same_path(source, stored))
    {
        if !path_exists_no_follow(source) {
            return Ok(());
        }
        let holder = unique_cleanup_holder(game_root)?;
        let claim = FileCleanupClaim {
            holder: holder.display().to_string(),
            expected_hashes,
            restore_from: restore.map(|(path, _)| path.display().to_string()),
            restore_hash: restore.map(|(_, hash)| hash.to_string()),
        };
        record.file_cleanup_claims.insert(source_key.clone(), claim);
        if let Err(error) = write_record_file(game_root, record) {
            record.file_cleanup_claims.remove(&source_key);
            let _ = remove_empty_dir_durable(&holder, "removing unused cleanup holder");
            return Err(error);
        }
    }
    advance_file_cleanup_claim(game_root, record, source)
}

fn advance_file_cleanup_claim(
    game_root: &Path,
    record: &mut DeployRecord,
    source: &Path,
) -> Result<()> {
    let source_key = record
        .file_cleanup_claims
        .keys()
        .find(|stored| same_path(source, stored))
        .cloned()
        .ok_or_else(|| {
            ModError::Other(format!(
                "missing durable file cleanup claim: {}",
                source.display()
            ))
        })?;
    let claim = record
        .file_cleanup_claims
        .get(&source_key)
        .cloned()
        .expect("claim key was selected above");
    let holder = PathBuf::from(&claim.holder);
    let claimed = holder.join("claimed");
    let trusted = record
        .trusted_file_tombstones
        .iter()
        .any(|stored| same_path(&holder, stored));

    if !trusted {
        let holder_metadata = std::fs::symlink_metadata(&holder)
            .map_err(io("reading durable file cleanup holder"))?;
        if metadata_is_link(&holder_metadata) || !holder_metadata.is_dir() {
            return Err(ModError::Other(format!(
                "durable file cleanup holder is not a real directory: {}",
                holder.display()
            )));
        }
        if !path_exists_no_follow(&claimed) {
            if !path_exists_no_follow(source) {
                return Err(ModError::Other(format!(
                    "file cleanup source and prepared holder payload are both missing: {}",
                    source.display()
                )));
            }
            promote_path_noclobber(source, &claimed)?;
        }
        let claimed_metadata = std::fs::symlink_metadata(&claimed)
            .map_err(io("reading claimed cleanup file metadata"))?;
        if metadata_is_link(&claimed_metadata) || !claimed_metadata.is_file() {
            return Err(ModError::Other(format!(
                "claimed cleanup object is not a regular file: {}",
                claimed.display()
            )));
        }
        if !path_matches_any_file_identity(&claimed, &claim.expected_hashes)? {
            // We atomically claimed an external replacement. Return that exact object without
            // clobber; if another creator won meanwhile, retain the claim for manual recovery.
            promote_path_noclobber(&claimed, source).map_err(|error| {
                ModError::Other(format!(
                    "claimed file '{}' is not deployment-owned and could not be returned to '{}': \
                     {error}",
                    claimed.display(),
                    source.display()
                ))
            })?;
            record
                .trusted_file_tombstones
                .push(holder.display().to_string());
            write_record_file(game_root, record)?;
            remove_dir_all_durable(&holder, "removing abandoned file cleanup holder")?;
            record
                .trusted_file_tombstones
                .retain(|stored| !same_path(&holder, stored));
            record.file_cleanup_claims.remove(&source_key);
            write_record_file(game_root, record)?;
            return Err(ModError::Other(format!(
                "refusing to clean externally changed file: {}",
                source.display()
            )));
        }

        if let (Some(restore_from), Some(restore_hash)) = (&claim.restore_from, &claim.restore_hash)
        {
            let restore_from = Path::new(restore_from);
            if !restore_hash.starts_with("sha256:")
                || !file_matches_recorded_hash_result(restore_from, restore_hash)?
            {
                return Err(ModError::Other(format!(
                    "authenticated restore source changed before publication: {}",
                    restore_from.display()
                )));
            }
            if !path_exists_no_follow(source) {
                let parent = source.parent().ok_or_else(|| {
                    ModError::Other(format!(
                        "restore destination has no parent: {}",
                        source.display()
                    ))
                })?;
                let candidate =
                    verified_temp_copy_in(restore_from, parent, ".gore-restore-stage-")?;
                if sha256_file(candidate.path())? != restore_hash.as_str() {
                    return Err(ModError::Other(format!(
                        "restore source changed while being staged: {}",
                        restore_from.display()
                    )));
                }
                if let Err(error) = publish_noclobber_temp(candidate, source) {
                    if !path_exists_no_follow(source) {
                        return Err(error);
                    }
                    // A racing creator won the original name. It is external and remains untouched;
                    // our authenticated old deployment stays isolated in the holder below.
                }
            }
        }

        record
            .trusted_file_tombstones
            .push(holder.display().to_string());
        record.trusted_file_tombstones.sort();
        record.trusted_file_tombstones.dedup();
        if claim.restore_from.is_none() {
            prune_completed_backup_claim(record, source);
        }
        write_record_file(game_root, record)?;
    } else if claim.restore_from.is_none() && prune_completed_backup_claim(record, source) {
        // Older/interrupted trusted records may still carry the tuple. Persist its pruning before
        // deleting the only remaining tombstone payload.
        write_record_file(game_root, record)?;
    }

    // Once trusted state is durable, a prior crash may have left any prefix of a recursive holder
    // delete. The holder is outside scanned game trees and can be retried without re-authenticating
    // the now-partial contents.
    remove_dir_all_durable(&holder, "removing trusted file cleanup tombstone")?;
    record
        .trusted_file_tombstones
        .retain(|stored| !same_path(&holder, stored));
    record.file_cleanup_claims.remove(&source_key);
    write_record_file(game_root, record)
}

fn process_file_cleanup_claims(game_root: &Path, record: &mut DeployRecord) -> Result<()> {
    let sources: Vec<PathBuf> = record
        .file_cleanup_claims
        .keys()
        .map(PathBuf::from)
        .collect();
    for source in sources {
        advance_file_cleanup_claim(game_root, record, &source)?;
    }
    Ok(())
}

fn tree_identities_for_path(record: &DeployRecord, path: &Path) -> Vec<String> {
    let mut identities = Vec::new();
    if let Some(fingerprint) = tree_fingerprint_for_path(path, &record.ue4ss_tree_fingerprints) {
        identities.push(fingerprint.clone());
    }
    if let Some(fingerprints) = values_for_path(path, &record.recovery_tree_fingerprints) {
        identities.extend(fingerprints.iter().cloned());
    }
    identities.sort();
    identities.dedup();
    identities
}

fn path_matches_any_tree_identity(path: &Path, identities: &[String]) -> bool {
    identities
        .iter()
        .any(|identity| tree_matches_recorded_fingerprint(path, identity))
}

fn unique_ue4ss_cleanup_holder(game_root: &Path) -> Result<PathBuf> {
    let ue4ss_root = resolve_game_paths(game_root)
        .ue4ss_mods
        .parent()
        .ok_or_else(|| ModError::Other("UE4SS Mods directory has no parent".into()))?
        .to_path_buf();
    std::fs::create_dir_all(&ue4ss_root).map_err(io("creating UE4SS cleanup root"))?;
    let holder = tempfile::Builder::new()
        .prefix(".gore-ue4ss-delete-")
        .tempdir_in(&ue4ss_root)
        .map_err(io("creating durable UE4SS cleanup holder"))?
        .keep();
    remove_empty_dir_durable(&holder, "reserving unique UE4SS cleanup holder name")?;
    Ok(holder)
}

fn remove_ue4ss_tracking(record: &mut DeployRecord, path: &Path) {
    if record
        .ue4ss_mod_dir
        .as_deref()
        .is_some_and(|stored| same_path(path, stored))
    {
        record.ue4ss_mod_dir = None;
    }
    record
        .ue4ss_mod_dirs
        .retain(|stored| !same_path(path, stored));
    record
        .stale_ue4ss_dirs
        .retain(|stored| !same_path(path, stored));
    prune_tree_identity(record, path);
}

fn durable_ue4ss_cleanup(
    game_root: &Path,
    record: &mut DeployRecord,
    original: &Path,
) -> Result<()> {
    if !record
        .ue4ss_cleanup_claims
        .keys()
        .any(|stored| same_path(original, stored))
    {
        if !path_exists_no_follow(original) {
            remove_ue4ss_tracking(record, original);
            return Ok(());
        }
        let expected = tree_identities_for_path(record, original);
        if expected.is_empty() || !path_matches_any_tree_identity(original, &expected) {
            return Err(ModError::Other(format!(
                "refusing to claim externally changed or unverifiable UE4SS tree: {}",
                original.display()
            )));
        }
        let holder = unique_ue4ss_cleanup_holder(game_root)?;
        let original_key = original.display().to_string();
        let holder_key = holder.display().to_string();
        record
            .ue4ss_cleanup_claims
            .insert(original_key.clone(), holder_key.clone());
        if !contains_same_path(&record.stale_ue4ss_dirs, &holder_key) {
            record.stale_ue4ss_dirs.push(holder_key.clone());
        }
        record
            .ue4ss_tree_fingerprints
            .insert(holder_key.clone(), expected[0].clone());
        if expected.len() > 1 {
            record
                .recovery_tree_fingerprints
                .insert(holder_key.clone(), expected[1..].to_vec());
        }
        if let Err(error) = write_record_file(game_root, record) {
            record.ue4ss_cleanup_claims.remove(&original_key);
            remove_ue4ss_tracking(record, &holder);
            return Err(error);
        }
    }
    advance_ue4ss_cleanup_claim(game_root, record, original)
}

fn advance_ue4ss_cleanup_claim(
    game_root: &Path,
    record: &mut DeployRecord,
    original: &Path,
) -> Result<()> {
    let original_key = record
        .ue4ss_cleanup_claims
        .keys()
        .find(|stored| same_path(original, stored))
        .cloned()
        .ok_or_else(|| {
            ModError::Other(format!(
                "missing durable UE4SS cleanup claim: {}",
                original.display()
            ))
        })?;
    let holder_key = record
        .ue4ss_cleanup_claims
        .get(&original_key)
        .cloned()
        .expect("claim key was selected above");
    let holder = PathBuf::from(&holder_key);
    let trusted = record
        .trusted_ue4ss_tombstones
        .iter()
        .any(|stored| same_path(&holder, stored));

    if !trusted {
        if !path_exists_no_follow(&holder) {
            if !path_exists_no_follow(original) {
                return Err(ModError::Other(format!(
                    "UE4SS cleanup source and prepared holder are both missing: {}",
                    original.display()
                )));
            }
            let expected = tree_identities_for_path(record, original);
            if expected.is_empty() || !path_matches_any_tree_identity(original, &expected) {
                return Err(ModError::Other(format!(
                    "UE4SS cleanup source changed before atomic claim: {}",
                    original.display()
                )));
            }
            promote_path_noclobber(original, &holder)?;
        } else {
            let holder_metadata = std::fs::symlink_metadata(&holder)
                .map_err(io("reading UE4SS cleanup holder metadata"))?;
            if metadata_is_link(&holder_metadata) || !holder_metadata.is_dir() {
                return Err(ModError::Other(format!(
                    "UE4SS cleanup holder is not a real directory: {}",
                    holder.display()
                )));
            }
        }
        let expected_holder = tree_identities_for_path(record, &holder);
        if expected_holder.is_empty() {
            return Err(ModError::Other(format!(
                "UE4SS cleanup holder has no moved-tree identity: {}",
                holder.display()
            )));
        }
        if !path_matches_any_tree_identity(&holder, &expected_holder) {
            if !path_exists_no_follow(original) {
                promote_path_noclobber(&holder, original).map_err(|error| {
                    ModError::Other(format!(
                        "claimed UE4SS tree failed identity verification and could not be returned \
                         to '{}': {error}",
                        original.display()
                    ))
                })?;
            }
            return Err(ModError::Other(format!(
                "refusing to delete UE4SS tree that changed during atomic claim: {}",
                original.display()
            )));
        }
        if !contains_same_path(&record.trusted_ue4ss_tombstones, &holder_key) {
            record.trusted_ue4ss_tombstones.push(holder_key.clone());
        }
        remove_ue4ss_tracking(record, original);
        // `remove_ue4ss_tracking` must not discard the holder identities/state we still need.
        record
            .ue4ss_cleanup_claims
            .insert(original_key.clone(), holder_key.clone());
        write_record_file(game_root, record)?;
    }

    remove_dir_all_durable(&holder, "removing trusted UE4SS cleanup tombstone")?;
    record
        .trusted_ue4ss_tombstones
        .retain(|stored| !same_path(&holder, stored));
    record.ue4ss_cleanup_claims.remove(&original_key);
    remove_ue4ss_tracking(record, &holder);
    write_record_file(game_root, record)
}

fn process_ue4ss_cleanup_claims(game_root: &Path, record: &mut DeployRecord) -> Result<()> {
    let originals: Vec<PathBuf> = record
        .ue4ss_cleanup_claims
        .keys()
        .map(PathBuf::from)
        .collect();
    for original in originals {
        advance_ue4ss_cleanup_claim(game_root, record, &original)?;
    }
    Ok(())
}

/// Undeploy: restore every live file from its backup and remove the UE4SS mod. Each entry is
/// finalized INDEPENDENTLY — restore (or skip-if-drifted) AND delete its backup as a unit, then
/// drop it from `record` — so a later locked backup can't leave earlier, already-deleted backups
/// dangling in a retained record. Returns true only if EVERYTHING was handled; otherwise the
/// still-pending entries remain in `record` so the caller can persist a pruned record and retry.
/// (Deploy rollback uses [`Undo`] instead, to restore the exact prior state.)
fn restore_record(game_root: &Path, record: &mut DeployRecord) -> Result<Vec<String>> {
    process_file_cleanup_claims(game_root, record)?;
    process_ue4ss_cleanup_claims(game_root, record)?;
    let mut failures = Vec::new();
    let mut index = 0;
    while index < record.backups.len() {
        let entry = record.backups[index].clone();
        let live_s = entry.0.clone();
        let bak_s = entry.1.clone();
        let (live, bak) = (Path::new(&live_s), Path::new(&bak_s));
        let completion = match safe_to_restore(&live_s, record) {
            Err(error) => Err(error),
            // Missing or externally drifted live bytes win. Never recreate/replace them from an
            // older backup; only the authenticated backup cleanup below remains.
            Ok(false) => Ok(()),
            Ok(true) if !bak.exists() => Err(ModError::Other(format!(
                "recorded backup is missing for '{}': {}",
                live.display(),
                bak.display()
            ))),
            Ok(true) => {
                let identities = file_identities_for_path(record, live);
                if identities.is_empty() {
                    // The only legacy case admitted by `safe_to_restore` is byte-identical, so no
                    // live write is needed and no path-only ownership is adopted.
                    if files_equal(live, bak)? {
                        Ok(())
                    } else {
                        Err(ModError::Other(format!(
                            "refusing legacy path-only restore for {}",
                            live.display()
                        )))
                    }
                } else {
                    let backup_hash = backup_hash_for_path(bak, &record.backup_hashes)
                        .ok_or_else(|| {
                            ModError::Other(format!(
                                "authenticated backup identity is missing for {}",
                                bak.display()
                            ))
                        })?
                        .clone();
                    durable_file_cleanup(
                        game_root,
                        record,
                        live,
                        identities,
                        Some((bak, &backup_hash)),
                    )
                }
            }
        };

        if let Err(error) = completion {
            failures.push(error.to_string());
            index += 1;
            continue;
        }

        if path_exists_no_follow(bak) {
            let backup_identity = match backup_hash_for_path(bak, &record.backup_hashes) {
                Some(hash) => hash.clone(),
                None if path_exists_no_follow(live) && files_equal(live, bak)? => {
                    let hash = sha256_file(bak)?;
                    record
                        .backup_hashes
                        .insert(bak.display().to_string(), hash.clone());
                    hash
                }
                None => {
                    failures.push(format!(
                        "refusing to delete legacy path-only backup: {}",
                        bak.display()
                    ));
                    index += 1;
                    continue;
                }
            };
            if let Err(error) =
                durable_file_cleanup(game_root, record, bak, vec![backup_identity], None)
            {
                failures.push(error.to_string());
                index += 1;
                continue;
            }
        }

        // The backup was either absent or durably claimed/deleted above. Persist pruning so a
        // retry only handles entries whose authenticated cleanup remains pending.
        let removed_entry = record
            .backups
            .iter()
            .position(|(stored_live, stored_backup, _)| {
                same_path(live, stored_live) && same_path(bak, stored_backup)
            })
            .map(|position| (position, record.backups.remove(position)));
        let removed_hash = remove_deployed_hash_for_path(&live_s, &mut record.deployed_hashes);
        let removed_recovery = take_vec_map_path(live, &mut record.recovery_file_hashes);
        let removed_backup_hash = remove_backup_hash_for_path(bak, &mut record.backup_hashes);
        if let Err(error) = write_record_file(game_root, record) {
            if let Some((position, removed_entry)) = removed_entry {
                record
                    .backups
                    .insert(position.min(record.backups.len()), removed_entry);
            }
            if let Some((key, value)) = removed_hash {
                record.deployed_hashes.insert(key, value);
            }
            if let Some((key, value)) = removed_recovery {
                record.recovery_file_hashes.insert(key, value);
            }
            if let Some((key, value)) = removed_backup_hash {
                record.backup_hashes.insert(key, value);
            }
            return Err(ModError::Other(format!(
                "persisting undeploy backup progress for '{}': {error}",
                live.display()
            )));
        }
        // The current index now addresses the next entry because this one stayed pruned.
    }
    if let Some(dir) = record.ue4ss_mod_dir.clone() {
        let path = Path::new(&dir);
        if !tree_cleanup_is_owned(record, path) {
            failures.push(format!(
                "refusing to remove externally changed or unverifiable UE4SS directory: {}",
                path.display()
            ));
        } else {
            match durable_ue4ss_cleanup(game_root, record, path) {
                Ok(()) => {}
                Err(error) => failures.push(error.to_string()),
            }
        }
    }
    let stale = record.stale_ue4ss_dirs.clone();
    for dir in stale {
        let path = Path::new(&dir);
        if !tree_cleanup_is_owned(record, path) {
            failures.push(format!(
                "refusing to remove externally changed or unverifiable UE4SS cleanup directory: {}",
                path.display()
            ));
        } else {
            match durable_ue4ss_cleanup(game_root, record, path) {
                Ok(()) => {}
                Err(error) => {
                    failures.push(error.to_string());
                }
            }
        }
    }
    // Additive texture triplet files in `~mods` (no backup) — delete them. A failed delete
    // (locked) must KEEP the entry and fail the undeploy (all_ok=false), so the record is not
    // deleted and a retry can still remove the lingering override; otherwise the triplet would
    // be orphaned on disk with nothing tracking it.
    for f in record.texture_triplets.clone() {
        let p = Path::new(&f);
        if !file_cleanup_is_owned(record, p) {
            failures.push(format!(
                "refusing to remove externally changed or unverifiable texture triplet: {}",
                p.display()
            ));
        } else {
            let identities = file_identities_for_path(record, p);
            let cleanup = if path_exists_no_follow(p) {
                durable_file_cleanup(game_root, record, p, identities, None)
            } else {
                Ok(())
            };
            match cleanup {
                Ok(()) => {
                    record
                        .texture_triplets
                        .retain(|stored| !same_path(p, stored));
                    prune_file_identity(record, p);
                }
                Err(error) => {
                    failures.push(error.to_string());
                }
            }
        }
    }
    // Manager-installed pak/triplet files in `~mods` (pure additions, no backup) — delete
    // them, with the same keep-on-failure accounting as `texture_triplets` above.
    for f in record.managed_paks.clone() {
        let p = Path::new(&f);
        if !file_cleanup_is_owned(record, p) {
            failures.push(format!(
                "refusing to remove externally changed or unverifiable managed pak: {}",
                p.display()
            ));
        } else {
            let identities = file_identities_for_path(record, p);
            let cleanup = if path_exists_no_follow(p) {
                durable_file_cleanup(game_root, record, p, identities, None)
            } else {
                Ok(())
            };
            match cleanup {
                Ok(()) => {
                    record.managed_paks.retain(|stored| !same_path(p, stored));
                    prune_file_identity(record, p);
                }
                Err(error) => {
                    failures.push(error.to_string());
                }
            }
        }
    }
    // Manager-installed UE4SS mod dirs — remove each with the same semantics as the single
    // `ue4ss_mod_dir` above: cleaned entries are dropped, failed ones kept for a retry.
    for dir in record.ue4ss_mod_dirs.clone() {
        let path = Path::new(&dir);
        if !tree_cleanup_is_owned(record, path) {
            failures.push(format!(
                "refusing to remove externally changed or unverifiable manager UE4SS directory: {}",
                path.display()
            ));
        } else {
            match durable_ue4ss_cleanup(game_root, record, path) {
                Ok(()) => {}
                Err(error) => {
                    failures.push(error.to_string());
                }
            }
        }
    }
    Ok(failures)
}

/// Undo the active gore-mod deployment at `game_root`: restore every backup and remove the
/// UE4SS mod. No-op if nothing is deployed.
pub fn undeploy(game_root: &Path) -> Result<Option<DeployRecord>> {
    // Match deploy's absolutization so the record file is found regardless of the caller's cwd.
    let game_root = abs_root(game_root);
    // Preserve the established no-op behavior without creating a lock file. A present record is
    // always re-read after ownership is acquired, so two concurrent undeploy/deploy processes can
    // never act on this unlocked observation.
    if read_record(&game_root)?.is_none() {
        return Ok(None);
    }
    let mutation = acquire_live_install_mutation(&game_root, "gore-mod:undeploy")?;
    let result = undeploy_guarded(&game_root);
    finish_live_install_mutation(result, mutation)
}

fn undeploy_guarded(game_root: &Path) -> Result<Option<DeployRecord>> {
    let rp = record_path(game_root);
    let Some(stored) = read_record(game_root)? else {
        return Ok(None);
    };
    let bytes = stored.raw;
    let mut record = stored.record;
    // This is the final process check after shared ownership and immediately before the recovery
    // record becomes undeploy's first live-install write. It is not an OS launch barrier.
    gore_as::compile::require_shipping_game_process_closed().map_err(|error| {
        ModError::Other(format!(
            "INSTALL_MUTATION_BLOCKED: final pre-write process check: {error}"
        ))
    })?;
    // Mark recovery durably before the first filesystem mutation. A crash at any later point
    // cannot make status/apply report a completed deployment while undeploy is only half done.
    record.phase = DeployPhase::RecoveryRequired;
    write_record_file(game_root, &record).map_err(|error| {
        ModError::Other(format!(
            "persisting undeploy recovery state before cleanup: {error}"
        ))
    })?;

    let failures = restore_record(game_root, &mut record)?;
    if failures.is_empty() {
        remove_file_durable(&rp, "removing completed deploy record")?;
        // Return the original record (pre-pruning) for reporting.
        Ok(Some(serde_json::from_slice(&bytes)?))
    } else {
        // Persist the PRUNED record so a retry only processes what's still pending — entries whose
        // file was restored and backup deleted are not re-attempted (and won't fail the next run on
        // a now-missing backup). Then report failure so the user can resolve the lock and retry.
        write_record_file(game_root, &record)?;
        Err(ModError::Other(format!(
            "some game files could not be restored (locked or unwritable); the remaining backups \
             and a pruned deploy record were kept — close the game and retry undeploy: {}",
            failures.join(" | ")
        )))
    }
}

/// Back up `live` to `live.gore-bak` if no backup exists yet (preserving the pristine file),
/// register it in `record`, and return the backup path. The backup is the pristine source.
pub(crate) fn backup(live: &Path, record: &mut DeployRecord) -> Result<(PathBuf, bool)> {
    let live_meta = std::fs::symlink_metadata(live).map_err(io("reading live backup metadata"))?;
    if metadata_is_link(&live_meta) || !live_meta.is_file() {
        return Err(ModError::Other(format!(
            "game file is not a safe regular file: {}",
            live.display()
        )));
    }
    let bak = bak_path(live);
    let created = match std::fs::symlink_metadata(&bak) {
        Ok(metadata) => {
            if metadata_is_link(&metadata) || !metadata.is_file() {
                return Err(ModError::Other(format!(
                    "backup path is not a safe regular file: {}",
                    bak.display()
                )));
            }
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Build, sync and byte-verify a sibling temp first. Publishing is no-clobber, so a
            // crash/disk-full condition cannot leave a truncated file masquerading as pristine.
            let temp = verified_temp_copy(live, ".gore-backup-stage-")?;
            publish_noclobber_temp(temp, &bak)?;
            true
        }
        Err(error) => return Err(io("reading backup metadata")(error)),
    };
    let backup_hash = match sha256_file(&bak) {
        Ok(hash) => hash,
        Err(error) if created => {
            return match remove_file_durable(&bak, "removing unverifiable new backup") {
                Ok(()) => Err(error),
                Err(cleanup) => Err(ModError::Other(format!(
                    "{error}; cleanup also failed for newly-created backup '{}': {cleanup}",
                    bak.display()
                ))),
            };
        }
        Err(error) => return Err(error),
    };
    record.backups.push((
        live.display().to_string(),
        bak.display().to_string(),
        created,
    ));
    record
        .backup_hashes
        .insert(bak.display().to_string(), backup_hash);
    Ok((bak, created))
}

/// Copy a regular, non-link file to a durable same-directory temporary and verify every byte.
/// Keeping the file in the same directory permits an atomic rollback rename on all target
/// filesystems and makes disk exhaustion fail before any source/backup is removed.
fn verified_temp_copy(source: &Path, prefix: &str) -> Result<tempfile::NamedTempFile> {
    let parent = source.parent().ok_or_else(|| {
        ModError::Other(format!(
            "snapshot source has no parent: {}",
            source.display()
        ))
    })?;
    verified_temp_copy_in(source, parent, prefix)
}

fn verified_temp_copy_in(
    source: &Path,
    temp_parent: &Path,
    prefix: &str,
) -> Result<tempfile::NamedTempFile> {
    let metadata = std::fs::symlink_metadata(source).map_err(io(&format!(
        "reading snapshot metadata {}",
        source.display()
    )))?;
    if metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(ModError::Other(format!(
            "snapshot source is not a safe regular file: {}",
            source.display()
        )));
    }
    let mut temp = tempfile::Builder::new()
        .prefix(prefix)
        .tempfile_in(temp_parent)
        .map_err(io("creating snapshot temp"))?;
    let mut input = std::fs::File::open(source)
        .map_err(io(&format!("opening snapshot source {}", source.display())))?;
    std::io::copy(&mut input, temp.as_file_mut()).map_err(io("copying snapshot"))?;
    temp.as_file().sync_all().map_err(io("syncing snapshot"))?;
    if !files_equal(source, temp.path())? {
        return Err(ModError::Other(format!(
            "snapshot verification mismatch for {}",
            source.display()
        )));
    }
    Ok(temp)
}

fn files_equal(left: &Path, right: &Path) -> Result<bool> {
    let left_len = std::fs::metadata(left)
        .map_err(io("reading comparison source metadata"))?
        .len();
    let right_len = std::fs::metadata(right)
        .map_err(io("reading comparison target metadata"))?
        .len();
    if left_len != right_len {
        return Ok(false);
    }
    let mut left = std::io::BufReader::new(
        std::fs::File::open(left).map_err(io("opening comparison source"))?,
    );
    let mut right = std::io::BufReader::new(
        std::fs::File::open(right).map_err(io("opening comparison target"))?,
    );
    let mut left_buf = vec![0u8; 1024 * 1024];
    let mut right_buf = vec![0u8; 1024 * 1024];
    loop {
        let left_read = left
            .read(&mut left_buf)
            .map_err(io("reading comparison source"))?;
        let right_read = right
            .read(&mut right_buf)
            .map_err(io("reading comparison target"))?;
        if left_read != right_read || left_buf[..left_read] != right_buf[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

/// Atomically replace `destination` with a streaming copy of `source`. The staged sibling is
/// synced and byte-compared before publication, so a short read, disk-full write, or source change
/// cannot publish a truncated pristine restore. This is the restore counterpart to `atomic_write`
/// for multi-GiB voice archives: memory use remains fixed instead of `fs::read`-sized.
#[cfg(test)]
fn atomic_copy(source: &Path, destination: &Path) -> Result<()> {
    if source.parent() != destination.parent() {
        return Err(ModError::Other(format!(
            "atomic-copy source and destination must be siblings: {} -> {}",
            source.display(),
            destination.display()
        )));
    }
    atomic_publish_copy(source, destination)
}

/// Publish a disk-backed candidate to `destination` with fixed memory use. The source may live on
/// another volume: it is first streamed into a verified unique sibling of the destination, then
/// durably atomically promoted using the same platform-specific path as [`atomic_write`].
fn atomic_publish_copy(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(test)]
    if take_injected_atomic_write_failure(destination) {
        return Err(ModError::Other(format!(
            "injected atomic-write failure for {}",
            destination.display()
        )));
    }
    let parent = destination.parent().ok_or_else(|| {
        ModError::Other(format!(
            "atomic-copy destination has no parent: {}",
            destination.display()
        ))
    })?;
    let temp = verified_temp_copy_in(source, parent, ".gore-copy-stage-")?;
    publish_atomic_temp(temp, destination)?;
    sync_parent_directory(parent)
}

#[cfg(test)]
thread_local! {
    static FAIL_ATOMIC_WRITE: std::cell::RefCell<Option<PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
    static FAIL_DURABLE_REMOVE: std::cell::RefCell<Option<PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
    static FAIL_UE4SS_PROMOTION: std::cell::RefCell<Option<PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
    static FAIL_VOICE_BUNDLE_WRITE: std::cell::RefCell<Option<PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
    static REPLACE_ADDITIVE_BEFORE_APPLY: std::cell::RefCell<Option<(PathBuf, Vec<u8>)>> = const {
        std::cell::RefCell::new(None)
    };
    static REPLACE_UE4SS_BEFORE_APPLY: std::cell::RefCell<Option<(PathBuf, Vec<u8>)>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
pub(crate) fn fail_next_atomic_write(path: &Path) {
    FAIL_ATOMIC_WRITE.with(|slot| *slot.borrow_mut() = Some(path.to_path_buf()));
}

#[cfg(test)]
fn take_injected_atomic_write_failure(path: &Path) -> bool {
    FAIL_ATOMIC_WRITE.with(|slot| {
        let matches = slot
            .borrow()
            .as_ref()
            .is_some_and(|expected| same_path(path, &expected.display().to_string()));
        if matches {
            slot.borrow_mut().take();
        }
        matches
    })
}

#[cfg(test)]
fn fail_next_durable_remove(path: &Path) {
    FAIL_DURABLE_REMOVE.with(|slot| *slot.borrow_mut() = Some(path.to_path_buf()));
}

#[cfg(test)]
fn take_injected_durable_remove_failure(path: &Path) -> bool {
    FAIL_DURABLE_REMOVE.with(|slot| {
        let matches = slot
            .borrow()
            .as_ref()
            .is_some_and(|expected| same_path(path, &expected.display().to_string()));
        if matches {
            slot.borrow_mut().take();
        }
        matches
    })
}

#[cfg(test)]
fn fail_next_ue4ss_promotion(path: &Path) {
    FAIL_UE4SS_PROMOTION.with(|slot| *slot.borrow_mut() = Some(path.to_path_buf()));
}

#[cfg(test)]
fn take_injected_ue4ss_promotion_failure(path: &Path) -> bool {
    FAIL_UE4SS_PROMOTION.with(|slot| {
        let matches = slot
            .borrow()
            .as_ref()
            .is_some_and(|expected| same_path(path, &expected.display().to_string()));
        if matches {
            slot.borrow_mut().take();
        }
        matches
    })
}

#[cfg(test)]
fn fail_next_voice_bundle_write(path: &Path) {
    FAIL_VOICE_BUNDLE_WRITE.with(|slot| *slot.borrow_mut() = Some(path.to_path_buf()));
}

#[cfg(test)]
fn take_injected_voice_bundle_write_failure(path: &Path) -> bool {
    FAIL_VOICE_BUNDLE_WRITE.with(|slot| {
        let matches = slot
            .borrow()
            .as_ref()
            .is_some_and(|expected| same_path(path, &expected.display().to_string()));
        if matches {
            slot.borrow_mut().take();
        }
        matches
    })
}

#[cfg(test)]
fn replace_additive_before_apply(path: &Path, bytes: &[u8]) {
    REPLACE_ADDITIVE_BEFORE_APPLY.with(|slot| {
        *slot.borrow_mut() = Some((path.to_path_buf(), bytes.to_vec()));
    });
}

#[cfg(test)]
fn apply_injected_additive_replacement(path: &Path) -> Result<()> {
    REPLACE_ADDITIVE_BEFORE_APPLY.with(|slot| {
        let replacement = slot
            .borrow()
            .as_ref()
            .filter(|(expected, _)| same_path(path, &expected.display().to_string()))
            .cloned();
        if let Some((_, bytes)) = replacement {
            slot.borrow_mut().take();
            std::fs::write(path, bytes).map_err(io("injecting additive replacement"))?;
        }
        Ok(())
    })
}

#[cfg(test)]
fn replace_ue4ss_before_apply(path: &Path, sentinel: &[u8]) {
    REPLACE_UE4SS_BEFORE_APPLY.with(|slot| {
        *slot.borrow_mut() = Some((path.to_path_buf(), sentinel.to_vec()));
    });
}

#[cfg(test)]
fn apply_injected_ue4ss_replacement(path: &Path) -> Result<()> {
    REPLACE_UE4SS_BEFORE_APPLY.with(|slot| {
        let replacement = slot
            .borrow()
            .as_ref()
            .filter(|(expected, _)| same_path(path, &expected.display().to_string()))
            .cloned();
        if let Some((_, bytes)) = replacement {
            slot.borrow_mut().take();
            if path_exists_no_follow(path) {
                std::fs::remove_dir_all(path).map_err(io("injecting UE4SS replacement removal"))?;
            }
            std::fs::create_dir_all(path).map_err(io("injecting UE4SS replacement directory"))?;
            std::fs::write(path.join("external-sentinel.txt"), bytes)
                .map_err(io("injecting UE4SS replacement sentinel"))?;
        }
        Ok(())
    })
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(test)]
    if take_injected_atomic_write_failure(path) {
        return Err(ModError::Other(format!(
            "injected atomic-write failure for {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        ModError::Other(format!(
            "atomic-write target has no parent: {}",
            path.display()
        ))
    })?;
    // A unique create-new sibling prevents concurrent deploys and stale crashed temp files from
    // sharing/truncating one predictable `<target>.gore-tmp` path.
    let mut temp = tempfile::Builder::new()
        .prefix(".gore-write-")
        .tempfile_in(parent)
        .map_err(io("creating atomic-write temp"))?;
    temp.as_file_mut()
        .write_all(bytes)
        .map_err(io("writing atomic-write temp"))?;
    temp.as_file()
        .sync_all()
        .map_err(io("syncing atomic-write temp"))?;
    publish_atomic_temp(temp, path)?;
    sync_parent_directory(parent)
}

#[cfg(not(windows))]
fn publish_noclobber_temp(temp: tempfile::NamedTempFile, path: &Path) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        ModError::Other(format!(
            "backup destination has no parent: {}",
            path.display()
        ))
    })?;
    temp.persist_noclobber(path)
        .map_err(|error| io("publishing backup")(error.error))?;
    sync_parent_directory(parent)
}

#[cfg(windows)]
fn publish_noclobber_temp(temp: tempfile::NamedTempFile, path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, SetFileAttributesW, FILE_ATTRIBUTE_NORMAL, MOVEFILE_WRITE_THROUGH,
    };

    let temp = temp.into_temp_path();
    let old_wide: Vec<u16> = temp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let new_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // No REPLACE_EXISTING flag: a racing creator wins and this pristine snapshot is never allowed
    // to overwrite it. WRITE_THROUGH makes the new backup name durable before it enters a record.
    unsafe {
        if SetFileAttributesW(old_wide.as_ptr(), FILE_ATTRIBUTE_NORMAL) == 0 {
            return Err(io("normalizing backup temp")(
                std::io::Error::last_os_error(),
            ));
        }
        if MoveFileExW(old_wide.as_ptr(), new_wide.as_ptr(), MOVEFILE_WRITE_THROUGH) == 0 {
            return Err(io("publishing durable no-clobber backup")(
                std::io::Error::last_os_error(),
            ));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn publish_atomic_temp(temp: tempfile::NamedTempFile, path: &Path) -> Result<()> {
    // `persist` performs one replacing rename while keeping the temp on the target filesystem.
    temp.persist(path)
        .map_err(|error| io("publishing atomic write")(error.error))?;
    Ok(())
}

#[cfg(windows)]
fn publish_atomic_temp(temp: tempfile::NamedTempFile, path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, SetFileAttributesW, FILE_ATTRIBUTE_NORMAL, MOVEFILE_REPLACE_EXISTING,
        MOVEFILE_WRITE_THROUGH,
    };

    // `tempfile::persist` uses a replacing MoveFileExW but not WRITE_THROUGH. Consume the open file
    // into a cleanup-owning TempPath, normalize its temporary attribute, then publish with the
    // Windows durability equivalent of syncing the parent directory entry.
    let temp = temp.into_temp_path();
    let old_wide: Vec<u16> = temp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let new_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both buffers are stable, NUL-terminated UTF-16 paths for the duration of each call.
    unsafe {
        if SetFileAttributesW(old_wide.as_ptr(), FILE_ATTRIBUTE_NORMAL) == 0 {
            return Err(io("normalizing atomic-write temp")(
                std::io::Error::last_os_error(),
            ));
        }
        if MoveFileExW(
            old_wide.as_ptr(),
            new_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) == 0
        {
            return Err(io("publishing durable atomic write")(
                std::io::Error::last_os_error(),
            ));
        }
    }
    Ok(())
}

/// Make a published directory-entry change durable. Unix exposes directory fsync directly. On
/// Windows does not expose a portable directory-fsync operation, so [`publish_atomic_temp`] uses a
/// replacing `MoveFileExW` with `MOVEFILE_WRITE_THROUGH` as the publication durability barrier.
#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> Result<()> {
    std::fs::File::open(parent)
        .map_err(io("opening parent directory for sync"))?
        .sync_all()
        .map_err(io("syncing parent directory"))
}

#[cfg(windows)]
fn sync_parent_directory(_parent: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn sync_parent_directory(_parent: &Path) -> Result<()> {
    Ok(())
}

fn remove_file_durable(path: &Path, context: &'static str) -> Result<()> {
    #[cfg(test)]
    if take_injected_durable_remove_failure(path) {
        return Err(ModError::Other(format!(
            "injected durable-remove failure for {}",
            path.display()
        )));
    }
    match std::fs::remove_file(path) {
        Ok(()) => {
            let parent = path.parent().ok_or_else(|| {
                ModError::Other(format!("removed path has no parent: {}", path.display()))
            })?;
            sync_parent_directory(parent)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(context)(error)),
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(src).map_err(io(&format!(
        "reading UE4SS source metadata {}",
        src.display()
    )))?;
    if metadata_is_link(&metadata) || !metadata.is_dir() {
        return Err(ModError::Other(format!(
            "UE4SS source must be a real directory: {}",
            src.display()
        )));
    }
    std::fs::create_dir_all(dst).map_err(io("create UE4SS staging dir"))?;
    let mut entries = 0u64;
    let mut total_bytes = 0u64;
    copy_dir_bounded(src, src, dst, &mut entries, &mut total_bytes)
}

fn copy_dir_bounded(
    root: &Path,
    src: &Path,
    dst: &Path,
    entries: &mut u64,
    total_bytes: &mut u64,
) -> Result<()> {
    for entry in std::fs::read_dir(src).map_err(io(&format!("reading {}", src.display())))? {
        let entry = entry.map_err(io("reading UE4SS directory entry"))?;
        let from = entry.path();
        let metadata = std::fs::symlink_metadata(&from).map_err(io(&format!(
            "reading UE4SS payload metadata {}",
            from.display()
        )))?;
        if metadata_is_link(&metadata) {
            return Err(ModError::Other(format!(
                "UE4SS source contains a symbolic link or reparse point: {}",
                from.display()
            )));
        }
        let relative = from.strip_prefix(root).map_err(|_| {
            ModError::Other(format!("UE4SS source escaped its root: {}", from.display()))
        })?;
        let portable = relative.to_str().ok_or_else(|| {
            ModError::Other(format!(
                "UE4SS source path is not UTF-8: {}",
                from.display()
            ))
        })?;
        if !is_safe_rel_path(&portable.replace('\\', "/")) {
            return Err(ModError::Other(format!(
                "UE4SS source contains a non-portable path: {}",
                from.display()
            )));
        }
        *entries = entries
            .checked_add(1)
            .ok_or_else(|| ModError::Other("UE4SS source entry count overflow".into()))?;
        if *entries > MAX_UE4SS_TREE_ENTRIES {
            return Err(ModError::Other(format!(
                "UE4SS source exceeds the {MAX_UE4SS_TREE_ENTRIES}-entry limit"
            )));
        }

        let to = dst.join(entry.file_name());
        if metadata.is_dir() {
            std::fs::create_dir(&to).map_err(io("creating UE4SS staging subdirectory"))?;
            copy_dir_bounded(root, &from, &to, entries, total_bytes)?;
            sync_parent_directory(&to)?;
        } else if metadata.is_file() {
            if metadata.len() > MAX_UE4SS_FILE_BYTES {
                return Err(ModError::Other(format!(
                    "UE4SS file exceeds the {MAX_UE4SS_FILE_BYTES}-byte limit: {}",
                    from.display()
                )));
            }
            let remaining = MAX_UE4SS_TREE_BYTES
                .checked_sub(*total_bytes)
                .ok_or_else(|| ModError::Other("UE4SS source byte budget underflow".into()))?;
            let allowed = remaining.min(MAX_UE4SS_FILE_BYTES);
            let mut input = std::fs::File::open(&from)
                .map_err(io(&format!("opening UE4SS source file {}", from.display())))?;
            let opened_len = input
                .metadata()
                .map_err(io("reading opened UE4SS source metadata"))?
                .len();
            if opened_len > allowed {
                return Err(ModError::Other(format!(
                    "UE4SS source exceeds its remaining {allowed}-byte budget: {}",
                    from.display()
                )));
            }
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&to)
                .map_err(io("creating UE4SS staging file"))?;
            let copied = std::io::copy(
                &mut (&mut input).take(allowed.saturating_add(1)),
                &mut output,
            )
            .map_err(io("copying UE4SS staging file"))?;
            if copied > allowed
                || copied != opened_len
                || input
                    .metadata()
                    .map_err(io("rechecking UE4SS source metadata"))?
                    .len()
                    != opened_len
            {
                return Err(ModError::Other(format!(
                    "UE4SS source changed or exceeded limits while being copied: {}",
                    from.display()
                )));
            }
            output
                .sync_all()
                .map_err(io("syncing UE4SS staging file"))?;
            std::fs::set_permissions(&to, metadata.permissions())
                .map_err(io("preserving UE4SS file permissions"))?;
            output
                .sync_all()
                .map_err(io("syncing UE4SS staging permissions"))?;
            *total_bytes = total_bytes
                .checked_add(copied)
                .ok_or_else(|| ModError::Other("UE4SS source byte total overflow".into()))?;
        } else {
            return Err(ModError::Other(format!(
                "UE4SS source contains a non-file payload: {}",
                from.display()
            )));
        }
    }
    sync_parent_directory(dst)
}

#[cfg(test)]
mod canonical_tempfile {
    pub(crate) fn tempdir() -> std::io::Result<::tempfile::TempDir> {
        // Production records use `abs_root`; mirror that when Windows exposes TEMP through an
        // 8.3 alias.
        let root = std::fs::canonicalize(std::env::temp_dir())?;
        ::tempfile::tempdir_in(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_tempfile as tempfile;
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
    use aes::Aes256;
    use gore_modgen::gen::OverrideValue;
    use std::cell::{Cell, RefCell};
    use std::io::{Read, Write};
    use std::rc::Rc;

    const TEST_LCACHE_AES_KEY: &[u8; 32] = b"8f93ff6fa254d9c536ad88c1ff1d812b";

    #[cfg(unix)]
    fn make_test_file_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_test_file_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test file symlink failed: {error}"),
        }
    }

    #[cfg(unix)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_test_dir_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test directory symlink failed: {error}"),
        }
    }

    pub(crate) fn test_ogg(sample_rate: u32) -> Vec<u8> {
        let mut data = include_bytes!("../../gore-vo/testdata/tiny-vorbis.ogg").to_vec();
        let ident = data
            .windows(7)
            .position(|window| window == b"\x01vorbis")
            .expect("fixture has Vorbis identification");
        data[ident + 12..ident + 16].copy_from_slice(&sample_rate.to_le_bytes());

        let mut offset = 0usize;
        while offset < data.len() {
            let segment_count = usize::from(data[offset + 26]);
            let header_len = 27 + segment_count;
            let body_len = data[offset + 27..offset + header_len]
                .iter()
                .map(|value| usize::from(*value))
                .sum::<usize>();
            let page_len = header_len + body_len;
            data[offset + 22..offset + 26].fill(0);
            let crc = test_ogg_crc(&data[offset..offset + page_len]);
            data[offset + 22..offset + 26].copy_from_slice(&crc.to_le_bytes());
            offset += page_len;
        }
        data
    }

    fn test_ogg_crc(bytes: &[u8]) -> u32 {
        let mut crc = 0u32;
        for &byte in bytes {
            crc ^= u32::from(byte) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 != 0 {
                    (crc << 1) ^ 0x04c1_1db7
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    pub(crate) fn write_test_voice_zip(path: &Path, entries: &[(&str, &[u8])]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            zip.start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().sync_all().unwrap();
    }

    pub(crate) fn read_test_zip_entry(path: &Path, member: &str) -> Option<Vec<u8>> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut entry = zip.by_name(member).ok()?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        Some(bytes)
    }

    fn observe_test_voice_archive(path: &Path, member: &str) -> VoiceArchiveObservation {
        let archive = gore_vo::ArchiveIndex::open(path, gore_vo::Limits::default()).unwrap();
        let seal = archive.seal();
        let member_proof = archive
            .entries()
            .iter()
            .find(|entry| entry.path == member)
            .map_or(VoiceMemberProof::Absent, |entry| {
                VoiceMemberProof::Present {
                    uncompressed_size: entry.uncompressed_size,
                    crc32: entry.crc32,
                }
            });
        VoiceArchiveObservation {
            archive_size: seal.size,
            archive_sha256: seal
                .sha256
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            member_proof,
        }
    }

    fn test_sealed_voice_replace(
        archive_path: &str,
        ogg: Vec<u8>,
        observation: VoiceArchiveObservation,
    ) -> SealedVoiceArchiveReplace {
        SealedVoiceArchiveReplace {
            archive: "German.zip".into(),
            archive_path: archive_path.into(),
            ogg,
            observation,
        }
    }

    fn test_sealed_voice_meta() -> ModMeta {
        ModMeta {
            name: "ManagedVoice".into(),
            version: "1".into(),
            author: "tester".into(),
        }
    }

    fn test_voice_generation() -> VoiceExecutableGenerationSeal {
        let bytes = b"fixture-game-exe";
        VoiceExecutableGenerationSeal {
            byte_len: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    fn write_test_game_executable(game: &Path, bytes: &[u8]) -> VoiceExecutableGenerationSeal {
        let executable = resolve_game_paths(game).executable;
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, bytes).unwrap();
        VoiceExecutableGenerationSeal {
            byte_len: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    fn install_mutation_fixture(root: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let game = root.join("game");
        let win64 = game.join("G1R/Binaries/Win64");
        std::fs::create_dir_all(&win64).unwrap();
        std::fs::create_dir_all(win64.join("ue4ss/Mods")).unwrap();
        std::fs::write(win64.join("G1R-Win64-Shipping.exe"), b"offline-stub").unwrap();
        let script = game.join("G1R/Script");
        std::fs::create_dir_all(&script).unwrap();
        let shipping = script.join("PrecompiledScript_Shipping.Cache");
        std::fs::write(&shipping, b"OFFLINE-SHIPPING").unwrap();

        let bundle = root.join("bundle");
        let source = bundle.join("ue4ss/LockProbe");
        std::fs::create_dir_all(source.join("Scripts")).unwrap();
        std::fs::write(source.join("enabled.txt"), b"").unwrap();
        std::fs::write(source.join("Scripts/main.lua"), b"return {}").unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta {
                name: "LockProbe".into(),
                version: "1".into(),
                author: "offline-test".into(),
            },
            components: vec![Component::Ue4ssLua {
                name: "LockProbe".into(),
                path: "ue4ss/LockProbe".into(),
                targets: Vec::new(),
                opaque: false,
            }],
        };
        std::fs::write(
            bundle.join("gore-mod.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let deployed = game.join("G1R/Binaries/Win64/ue4ss/Mods/LockProbe");
        (game, bundle, deployed, shipping)
    }

    #[test]
    fn a_running_game_blocks_the_deploy_before_anything_is_written() {
        // The other half of the seam above. Every other test in this file states that nothing has
        // the tree open, so this is the one that proves the refusal still exists and still comes
        // before the first write — otherwise "state it closed" would be indistinguishable from
        // having quietly removed the check.
        let temp = tempfile::tempdir().unwrap();
        let (game, bundle, deployed, _) = install_mutation_fixture(temp.path());

        let _game_process = StatedGameProcess::running();
        let error = deploy(&bundle, &game).unwrap_err().to_string();

        assert!(error.contains("INSTALL_MUTATION_BLOCKED"), "got: {error}");
        assert!(error.contains("close the game"), "got: {error}");
        assert!(!deployed.exists(), "a refused deploy published a file");
        assert!(!record_path(&game).exists(), "a refused deploy left a record");
    }

    #[test]
    fn active_compile_guard_blocks_deploy_and_undeploy_before_live_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let (game, bundle, deployed, _) = install_mutation_fixture(temp.path());

        let compile =
            gore_as::compile::InstallMutationGuard::acquire(&game, "gore-as:test").unwrap();
        let lock_bytes = std::fs::read(compile.path()).unwrap();
        let error = deploy(&bundle, &game).unwrap_err().to_string();
        assert!(error.contains("INSTALL_MUTATION_BLOCKED"), "got: {error}");
        assert!(!deployed.exists());
        assert!(!record_path(&game).exists());
        assert_eq!(std::fs::read(compile.path()).unwrap(), lock_bytes);
        drop(compile);

        deploy(&bundle, &game).unwrap();
        assert!(deployed.exists());
        let record_before = std::fs::read(record_path(&game)).unwrap();
        let compile =
            gore_as::compile::InstallMutationGuard::acquire(&game, "gore-as:test").unwrap();
        let error = undeploy(&game).unwrap_err().to_string();
        assert!(error.contains("INSTALL_MUTATION_BLOCKED"), "got: {error}");
        assert!(deployed.exists());
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), record_before);
        drop(compile);

        assert!(undeploy(&game).unwrap().is_some());
        assert!(!deployed.exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn gore_mod_mutation_guard_blocks_compile_for_deploy_and_undeploy_owners() {
        let temp = tempfile::tempdir().unwrap();
        let (game, _, _, shipping) = install_mutation_fixture(temp.path());

        for owner in ["gore-mod:deploy", "gore-mod:undeploy"] {
            let guard = acquire_live_install_mutation(&game, owner).unwrap();
            let output = temp.path().join(format!("{owner}.Cache").replace(':', "-"));
            let error = gore_as::compile::precompile(&gore_as::compile::PrecompileOpts {
                game_dir: game.clone(),
                src: None,
                out: Some(output.clone()),
                backup: false,
            })
            .unwrap_err();
            assert!(
                error.contains("install mutation is active"),
                "owner={owner}: {error}"
            );
            assert_eq!(std::fs::read(&shipping).unwrap(), b"OFFLINE-SHIPPING");
            assert!(!output.exists());
            assert!(!game.join(".gore-as-compile.lock").exists());
            assert!(!game.join(".gore-as-compile-recovery").exists());
            assert!(guard.path().exists());
            drop(guard);
        }
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn gore_as_recovery_artifacts_block_deploy_and_undeploy_until_recovered() {
        let temp = tempfile::tempdir().unwrap();
        let (game, bundle, deployed, _) = install_mutation_fixture(temp.path());
        let recovery = game.join(".gore-as-compile-recovery");
        std::fs::create_dir(&recovery).unwrap();
        std::fs::write(recovery.join("README.txt"), b"KEEP").unwrap();

        let error = deploy(&bundle, &game).unwrap_err().to_string();
        assert!(error.contains("RecoveryJournal"), "got: {error}");
        assert!(!deployed.exists());
        assert_eq!(std::fs::read(recovery.join("README.txt")).unwrap(), b"KEEP");
        std::fs::remove_dir_all(&recovery).unwrap();

        deploy(&bundle, &game).unwrap();
        let record_before = std::fs::read(record_path(&game)).unwrap();
        std::fs::create_dir(&recovery).unwrap();
        std::fs::write(recovery.join("README.txt"), b"KEEP").unwrap();
        let error = undeploy(&game).unwrap_err().to_string();
        assert!(error.contains("RecoveryJournal"), "got: {error}");
        assert!(deployed.exists());
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), record_before);
        assert_eq!(std::fs::read(recovery.join("README.txt")).unwrap(), b"KEEP");
        std::fs::remove_dir_all(&recovery).unwrap();

        undeploy(&game).unwrap();
        assert!(!deployed.exists());
    }

    #[test]
    fn commit_plan_rejects_a_record_changed_after_prepare_under_the_guard() {
        let temp = tempfile::tempdir().unwrap();
        let (game, _, _, _) = install_mutation_fixture(temp.path());
        let active = DeployRecord {
            mod_name: "ConcurrentWinner".into(),
            ..Default::default()
        };
        write_record_file(&game, &active).unwrap();
        let active_bytes = std::fs::read(record_path(&game)).unwrap();

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            DeployPlan::default(),
            DeployRecord {
                mod_name: "StalePlan".into(),
                ..Default::default()
            },
            None,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("DEPLOY_BASIS_CHANGED"), "got: {error}");
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), active_bytes);
        assert!(!game.join(".gore-install-mutation.lock").exists());
    }

    #[test]
    fn deploy_recovery_probe_is_read_only_and_persists_across_calls() {
        let temp = tempfile::tempdir().unwrap();
        let (game, _, _, _) = install_mutation_fixture(temp.path());
        assert!(!deploy_recovery_required(&game).unwrap());
        assert!(!record_path(&game).exists());

        let mut record = DeployRecord {
            mod_name: "InterruptedDeploy".into(),
            ..Default::default()
        };
        record.phase = DeployPhase::RecoveryRequired;
        write_record_file(&game, &record).unwrap();
        let bytes = std::fs::read(record_path(&game)).unwrap();

        assert!(deploy_recovery_required(&game).unwrap());
        assert!(
            deploy_recovery_required(&game).unwrap(),
            "a fresh read must recover the persistent state without process-local memory"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), bytes);

        record.phase = DeployPhase::Applied;
        write_record_file(&game, &record).unwrap();
        let applied = std::fs::read(record_path(&game)).unwrap();
        assert!(!deploy_recovery_required(&game).unwrap());
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), applied);
    }

    fn test_voice_replace_spec(
        name: &str,
        ogg_path: &Path,
        observation: Option<VoiceArchiveObservation>,
    ) -> BuildSpec {
        BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![VoiceArchiveEdit {
                archive: "German.zip".into(),
                op: VoicePatchOp::Replace,
                archive_path: "NPC/Hero/hello.ogg".into(),
                ogg_path: ogg_path.display().to_string(),
                observation,
            }],
        }
    }

    fn test_lcache_fstring(text: &str) -> Vec<u8> {
        if text.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut out = Vec::new();
        if text.is_ascii() {
            let mut raw = text.as_bytes().to_vec();
            raw.push(0);
            out.extend_from_slice(&(raw.len() as i32).to_le_bytes());
            out.extend_from_slice(&raw);
        } else {
            let mut raw = Vec::new();
            for unit in text.encode_utf16() {
                raw.extend_from_slice(&unit.to_le_bytes());
            }
            raw.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&(-((raw.len() / 2) as i32)).to_le_bytes());
            out.extend_from_slice(&raw);
        }
        out
    }

    /// Minimal real encrypted lcache used to exercise the studio LocPatch prepare path.
    fn test_lcache_with_languages(languages: &[&str]) -> Vec<u8> {
        test_lcache_with_pairs(languages, &[("german", "Käse")])
    }

    /// The same cache with the single id's language slots spelled out, so a test can build the
    /// sparse shape the real cache has — an id carrying two generations of one language, or
    /// carrying fewer languages than the header declares.
    fn test_lcache_with_pairs(languages: &[&str], pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut plain = Vec::new();
        plain.push(0);
        plain.extend_from_slice(&(b"LCACHE".len() as i32).to_le_bytes());
        plain.extend_from_slice(b"LCACHE");
        plain.extend_from_slice(&(languages.len() as i32).to_le_bytes());
        for language in languages {
            plain.extend_from_slice(&test_lcache_fstring(language));
        }
        plain.extend_from_slice(&1i32.to_le_bytes());
        plain.extend_from_slice(&test_lcache_fstring("itfo_cheese"));
        plain.extend_from_slice(&(pairs.len() as i32).to_le_bytes());
        for (language, value) in pairs {
            plain.extend_from_slice(&test_lcache_fstring(language));
            plain.extend_from_slice(&test_lcache_fstring(value));
        }
        plain.extend_from_slice(&test_lcache_fstring(""));
        plain.extend_from_slice(&0i32.to_le_bytes());
        let pad = (16 - plain.len() % 16) % 16;
        plain.resize(plain.len() + pad, 0);

        let cipher = Aes256::new(GenericArray::from_slice(TEST_LCACHE_AES_KEY));
        let mut encrypted = plain;
        for block in encrypted.chunks_mut(16) {
            cipher.encrypt_block(GenericArray::from_mut_slice(block));
        }
        encrypted
    }

    fn test_lcache() -> Vec<u8> {
        test_lcache_with_languages(&["german"])
    }

    fn prepare_test_loc_patch(
        edits: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> Result<DeployPlan> {
        prepare_test_loc_patch_with_cache(edits, test_lcache())
    }

    fn prepare_test_loc_patch_with_cache(
        edits: &BTreeMap<String, BTreeMap<String, String>>,
        lcache: Vec<u8>,
    ) -> Result<DeployPlan> {
        let temp = tempfile::tempdir().unwrap();
        let bundle_dir = temp.path().join("bundle");
        std::fs::create_dir_all(bundle_dir.join("loc")).unwrap();
        std::fs::write(
            bundle_dir.join("loc/edits.json"),
            serde_json::to_vec(edits).unwrap(),
        )
        .unwrap();

        let game = temp.path().join("game");
        let cache_dir = game.join("G1R/Story/Cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("AlkimiaLocalization_0.lcache"), lcache).unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta {
                name: "NewLoc".into(),
                version: String::new(),
                author: String::new(),
            },
            components: vec![Component::LocPatch {
                path: "loc/edits.json".into(),
            }],
        };
        prepare(&bundle_dir, &manifest, &resolve_game_paths(&game), None)
    }

    /// [8] Distinct mod names that sanitize to the SAME stem (chars folded to `_`)
    /// must still produce DIFFERENT texture triplet names, so one mod's mounted pak
    /// can't clobber another's. The triplet stem mirrors the deploy code:
    /// `zzz_{sanitize(name)}_{name_hash(name)}_{idx}_tex_P`.
    #[test]
    fn distinct_mod_names_folding_to_same_stem_get_distinct_triplets() {
        let a = "A+B";
        let b = "A B";
        // Sanitize alone collides...
        assert_eq!(sanitize(a), sanitize(b), "precondition: stems must collide");
        // ...but the hash of the ORIGINAL name disambiguates.
        assert_ne!(name_hash(a), name_hash(b), "name_hash must differ");

        let name_for =
            |n: &str, idx: usize| format!("zzz_{}_{}_{}_tex_P", sanitize(n), name_hash(n), idx);
        assert_ne!(name_for(a, 0), name_for(b, 0), "triplet names must differ");
        // Same name, different component index -> still distinct (bug [2]).
        assert_ne!(
            name_for(a, 0),
            name_for(a, 1),
            "per-component names must differ"
        );
        // Stable across calls (no RNG / SipHash).
        assert_eq!(name_hash(a), name_hash(a));
    }

    #[test]
    fn build_bundle_overrides_loc_audio() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("tone.wav");
        // minimal 16-bit PCM WAV, 1 sample
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36u32 + 2).to_le_bytes());
        w.extend_from_slice(b"WAVE");
        w.extend_from_slice(b"fmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&48000u32.to_le_bytes());
        w.extend_from_slice(&96000u32.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&2u32.to_le_bytes());
        w.extend_from_slice(&0i16.to_le_bytes());
        std::fs::write(&wav, &w).unwrap();

        let mut loc = BTreeMap::new();
        let mut langs = BTreeMap::new();
        langs.insert("german_new".to_string(), "Käse".to_string());
        loc.insert("itfo_cheese".to_string(), langs);

        let spec = BuildSpec {
            meta: ModMeta {
                name: "MyMod".into(),
                version: "1.0".into(),
                author: "me".into(),
            },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ItFo_Apple".into(),
                field: "m_Value".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(500),
            }],
            loc_edits: loc,
            audio: vec![AudioReplacement {
                bank: "SFX.bank".into(),
                sample: "SFX_UI_X".into(),
                wav_path: wav.display().to_string(),
            }],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };

        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("ue4ss/MyMod/Scripts/main.lua"));
        assert!(bundle.files.contains_key("ue4ss/MyMod/enabled.txt"));
        assert!(bundle.files.contains_key("loc/edits.json"));
        assert!(bundle.files.contains_key("audio/manifest.json"));
        assert!(bundle.files.contains_key("audio/0_SFX_bank__SFX_UI_X.wav"));
        assert!(bundle.files.contains_key("gore-mod.json"));
        assert_eq!(bundle.manifest.components.len(), 3);

        // round-trip manifest
        let mj = &bundle.files["gore-mod.json"];
        let m: ModManifest = serde_json::from_slice(mj).unwrap();
        assert_eq!(m.mod_meta.name, "MyMod");
    }

    #[test]
    fn studio_loc_patch_adds_missing_id_and_skips_unsupported_language() {
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("goremod_new_dialog".into())
            .or_default()
            .extend([
                ("German".into(), "Alte Zeile".into()),
                ("german".into(), "Neue Zeile".into()),
                ("english".into(), "New line".into()),
            ]);
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("english".into(), "Cheese".into());

        let plan = prepare_test_loc_patch(&edits).unwrap();
        assert_eq!(plan.writes.len(), 1);
        let decoded = gore_loc::loc::Lcache::decode(&plan.writes[0].1).unwrap();
        let exported = decoded.export(false);
        assert_eq!(exported["goremod_new_dialog"]["german"], "Neue Zeile");
        assert!(!exported["goremod_new_dialog"].contains_key("english"));
        assert_eq!(exported["goremod_new_dialog"].len(), 1);
        assert_eq!(exported["itfo_cheese"]["german"], "Käse");
    }

    /// The cache is sparse: declaring a language does not mean every id carries it. An edit that
    /// names a declared language the id has no slot for cannot land, and used to be dropped
    /// without a word — while the standalone `gore loc import` reported the same miss by name.
    /// That silence is what made a mis-targeted translation look like a broken tool.
    #[test]
    fn studio_loc_patch_reports_an_edit_the_target_id_has_no_slot_for() {
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("english".into(), "Cheese".into());

        // english IS declared here, so this is not the unknown-language skip above; itfo_cheese
        // still carries german alone.
        let plan = prepare_test_loc_patch_with_cache(
            &edits,
            test_lcache_with_languages(&["german", "english"]),
        )
        .unwrap();

        assert_eq!(plan.writes.len(), 1, "the deploy still proceeds");
        assert_eq!(
            plan.loc_skipped.len(),
            1,
            "the unwritable edit must be reported: {:?}",
            plan.loc_skipped
        );
        assert!(
            plan.loc_skipped[0].contains("itfo_cheese")
                && plan.loc_skipped[0].contains("english"),
            "the warning names the id and the language: {:?}",
            plan.loc_skipped
        );

        let decoded = gore_loc::loc::Lcache::decode(&plan.writes[0].1).unwrap();
        let exported = decoded.export(false);
        assert_eq!(exported["itfo_cheese"]["german"], "Käse");
        assert!(!exported["itfo_cheese"].contains_key("english"));
    }

    /// Writing the older generation of a language the id also carries a newer one for lands in
    /// the file and is never displayed. This is the 2,147-id case in the shipped German cache and
    /// the one the guide's own example used to teach.
    #[test]
    fn studio_loc_patch_warns_when_the_edited_generation_is_shadowed() {
        let cache = test_lcache_with_pairs(
            &["german", "german_new"],
            &[("german", "Käse"), ("german_new", "Bergkäse")],
        );
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("german".into(), "Emmentaler".into());

        let plan = prepare_test_loc_patch_with_cache(&edits, cache).unwrap();
        assert_eq!(
            plan.loc_shadowed.len(),
            1,
            "a shadowed edit must be reported: {:?}",
            plan.loc_shadowed
        );
        assert!(
            plan.loc_shadowed[0].contains("german_new")
                && plan.loc_shadowed[0].contains("itfo_cheese"),
            "the warning names the id and the generation that wins: {:?}",
            plan.loc_shadowed
        );
        // The edit still lands: it was a legitimate write, just not a visible one.
        let decoded = gore_loc::loc::Lcache::decode(&plan.writes[0].1).unwrap();
        assert_eq!(decoded.export(false)["itfo_cheese"]["german"], "Emmentaler");
    }

    /// Writing both generations is what the guide recommends, so it must not be nagged about —
    /// otherwise the warning fires on every correct edit and gets tuned out.
    #[test]
    fn studio_loc_patch_stays_silent_when_the_winning_generation_is_written_too() {
        let cache = test_lcache_with_pairs(
            &["german", "german_new"],
            &[("german", "Käse"), ("german_new", "Bergkäse")],
        );
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits.entry("itfo_cheese".into()).or_default().extend([
            ("german".into(), "Emmentaler".into()),
            ("german_new".into(), "Emmentaler".into()),
        ]);

        let plan = prepare_test_loc_patch_with_cache(&edits, cache).unwrap();
        assert!(
            plan.loc_shadowed.is_empty(),
            "writing both generations is correct and must not warn: {:?}",
            plan.loc_shadowed
        );
    }

    /// An edit that never landed must not appear under the note that says edits were written.
    ///
    /// The shadow check used to run before the write, so an id carrying `german_new` and no
    /// `german` raised BOTH findings for one edit: skipped, because there was no slot, and shadowed,
    /// as though it had been written and merely hidden. The two readings call for opposite
    /// responses — change the spec, or leave a working deployment alone — so reporting one edit as
    /// both is worse than reporting neither.
    #[test]
    fn studio_loc_patch_does_not_call_an_unwritable_edit_shadowed() {
        let cache = test_lcache_with_pairs(
            &["german", "german_new"],
            // The id carries only the newer generation, which is the shape 31,590 shipped ids have.
            &[("german_new", "Bergkäse")],
        );
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("german".into(), "Emmentaler".into());

        let plan = prepare_test_loc_patch_with_cache(&edits, cache).unwrap();
        assert_eq!(
            plan.loc_skipped.len(),
            1,
            "the edit had no slot to land in: {:?}",
            plan.loc_skipped
        );
        assert!(
            plan.loc_shadowed.is_empty(),
            "nothing was written, so nothing can be shadowed: {:?}",
            plan.loc_shadowed
        );
    }

    /// The counterpart, so the warning cannot become noise on every correct edit.
    #[test]
    fn studio_loc_patch_stays_silent_when_every_edit_lands() {
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("itfo_cheese".into())
            .or_default()
            .insert("german".into(), "Bergkäse".into());

        let plan = prepare_test_loc_patch(&edits).unwrap();
        assert!(
            plan.loc_skipped.is_empty() && plan.loc_shadowed.is_empty(),
            "an edit that lands must not warn: {:?}",
            plan.loc_skipped
        );
        let decoded = gore_loc::loc::Lcache::decode(&plan.writes[0].1).unwrap();
        assert_eq!(decoded.export(false)["itfo_cheese"]["german"], "Bergkäse");
    }

    #[test]
    fn studio_loc_patch_folds_new_id_aliases_before_add() {
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry("GoreMod_Mixed_Case".into())
            .or_default()
            .insert("german".into(), "Deutsche Zeile".into());
        edits
            .entry("goremod_mixed_case".into())
            .or_default()
            .insert("english".into(), "English line".into());

        let plan = prepare_test_loc_patch_with_cache(
            &edits,
            test_lcache_with_languages(&["german", "english"]),
        )
        .unwrap();
        let decoded = gore_loc::loc::Lcache::decode(&plan.writes[0].1).unwrap();
        let exported = decoded.export(false);
        let matches: Vec<_> = exported
            .iter()
            .filter(|(id, _)| id.eq_ignore_ascii_case("goremod_mixed_case"))
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "case aliases must produce one lcache group"
        );
        assert_eq!(matches[0].1["german"], "Deutsche Zeile");
        assert_eq!(matches[0].1["english"], "English line");
    }

    #[test]
    fn studio_loc_patch_propagates_unexpected_add_key_errors() {
        let mut edits: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        edits
            .entry(String::new())
            .or_default()
            .insert("german".into(), "Text".into());
        let err = prepare_test_loc_patch(&edits).unwrap_err();
        assert!(
            err.to_string()
                .contains("localization key must not be empty"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn empty_name_rejected() {
        let spec = BuildSpec {
            meta: ModMeta {
                name: "".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        assert!(build_bundle(&spec).is_err());
    }

    #[test]
    fn build_emits_texture_patch() {
        let dir = std::env::temp_dir().join("gore-mod-tex-build");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("img.png");
        std::fs::write(&png, b"\x89PNG\r\n\x1a\nfake").unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "TestMod".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![TextureReplacement {
                asset: "/Game/UI/T_X".into(),
                image_path: png.display().to_string(),
            }],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("texture/manifest.json"));
        assert!(bundle
            .files
            .keys()
            .any(|k| k.starts_with("texture/") && k.ends_with(".png")));
        assert!(matches!(bundle.manifest.components.last(),
            Some(Component::TexturePatch { assets, .. }) if assets == &vec!["/Game/UI/T_X".to_string()]));
    }

    #[test]
    fn build_emits_angelscript_patch() {
        let dir = std::env::temp_dir().join("gore-mod-as-build");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mini = dir.join("mod.cache");
        std::fs::write(&mini, b"MINI-CACHE-BYTES").unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "AsMod".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![ScriptModule {
                op: "add".into(),
                module_name: "MyMod".into(),
                mini_cache: mini.display().to_string(),
            }],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("scripts/manifest.json"));
        assert!(bundle.files.contains_key("scripts/0_MyMod.cache"));
        assert_eq!(bundle.files["scripts/0_MyMod.cache"], b"MINI-CACHE-BYTES");
        assert!(matches!(bundle.manifest.components.last(),
            Some(Component::AngelScriptPatch { path }) if path == "scripts"));
        // manifest round-trips to the typed entry
        let m: Vec<ScriptEntry> =
            serde_json::from_slice(&bundle.files["scripts/manifest.json"]).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].op, "add");
        assert_eq!(m[0].module, "MyMod");
        assert_eq!(m[0].mini, "scripts/0_MyMod.cache");
    }

    /// Every asset section of one spec, written the way a user writes it: bare filenames sitting
    /// beside the spec. `set_current_dir` is deliberately not used — cargo runs these tests
    /// multi-threaded in one process and the working directory is global, so the base is driven
    /// through the parameter instead.
    fn test_spec_with_bare_asset_names(name: &str) -> BuildSpec {
        BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![AudioReplacement {
                bank: "SFX.bank".into(),
                sample: "Foo".into(),
                wav_path: "click.wav".into(),
            }],
            texture: vec![TextureReplacement {
                asset: "/Game/UI/T_X".into(),
                image_path: "cursor.png".into(),
            }],
            files: vec![LooseFileReplacement {
                game_path: "G1R/Content/Slate/Cursors/Normal/Normal.PNG".into(),
                source_path: "Normal.PNG".into(),
            }],
            pak_files: vec![],
            scripts: vec![ScriptModule {
                op: "add".into(),
                module_name: "MyModule".into(),
                mini_cache: "mod.cache".into(),
            }],
            dialog_topics: vec![],
            voice: vec![VoiceArchiveEdit {
                archive: "German.zip".into(),
                op: VoicePatchOp::Replace,
                archive_path: "NPC/Hero/hello.ogg".into(),
                ogg_path: "line.ogg".into(),
                observation: None,
            }],
        }
    }

    fn write_test_bare_assets(dir: &Path) {
        std::fs::write(dir.join("click.wav"), b"WAV-BYTES").unwrap();
        std::fs::write(dir.join("cursor.png"), b"PNG-BYTES").unwrap();
        std::fs::write(dir.join("mod.cache"), b"MINI-BYTES").unwrap();
        std::fs::write(dir.join("Normal.PNG"), b"CURSOR-BYTES").unwrap();
        std::fs::write(dir.join("line.ogg"), test_ogg(32_000)).unwrap();
    }

    #[test]
    fn build_bundle_resolves_spec_relative_assets() {
        // The failure this exists for: a spec and its assets in one directory, built from anywhere
        // else. Every path in the spec then named a file that did not exist relative to the
        // process, and the only clue was a bare filename in the error.
        let assets = tempfile::tempdir().unwrap();
        write_test_bare_assets(assets.path());

        let bundle =
            build_bundle_relative_to(&test_spec_with_bare_asset_names("Relative"), assets.path())
                .unwrap();
        assert_eq!(bundle.files["audio/0_SFX_bank__Foo.wav"], b"WAV-BYTES");
        assert_eq!(bundle.files["texture/0__Game_UI_T_X.png"], b"PNG-BYTES");
        assert_eq!(bundle.files["scripts/0_MyModule.cache"], b"MINI-BYTES");
        assert_eq!(bundle.files["voice/payload/0.ogg"], test_ogg(32_000));
        let files: BTreeMap<String, String> =
            serde_json::from_slice(&bundle.files["files/manifest.json"]).unwrap();
        let payload = &files["G1R/Content/Slate/Cursors/Normal/Normal.PNG"];
        assert_eq!(bundle.files[payload], b"CURSOR-BYTES");
    }

    #[test]
    fn absolute_spec_paths_ignore_the_base() {
        // Mod Studio fills every path from a file picker, so they are absolute and belong to no
        // spec directory at all. Rebasing one would break the GUI.
        let base = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        write_test_bare_assets(elsewhere.path());
        let mut spec = test_spec_with_bare_asset_names("Absolute");
        spec.audio[0].wav_path = elsewhere.path().join("click.wav").display().to_string();
        spec.texture[0].image_path = elsewhere.path().join("cursor.png").display().to_string();
        spec.files[0].source_path = elsewhere.path().join("Normal.PNG").display().to_string();
        spec.scripts[0].mini_cache = elsewhere.path().join("mod.cache").display().to_string();
        spec.voice[0].ogg_path = elsewhere.path().join("line.ogg").display().to_string();

        let bundle = build_bundle_relative_to(&spec, base.path()).unwrap();
        assert_eq!(bundle.files["audio/0_SFX_bank__Foo.wav"], b"WAV-BYTES");
    }

    #[test]
    fn a_bank_named_by_a_full_path_is_refused_by_build_rather_than_by_deploy() {
        // The session this exists for: a spec whose bank read
        // "D:\SteamLibrary\...\FMOD\Desktop\Music.bank" built happily and printed "built bundle:
        // … (4 components, 9 files)". Only the deploy refused it, four calls later. The planner
        // keeps its own copy of the guard for bundles this process did not build; this one stops
        // a spec that can never deploy from reporting a successful build.
        let assets = tempfile::tempdir().unwrap();
        write_test_bare_assets(assets.path());
        let mut spec = test_spec_with_bare_asset_names("BankByPath");
        spec.audio[0].bank =
            r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\FMOD\Desktop\Music.bank"
                .into();

        let error = build_bundle_relative_to(&spec, assets.path())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("Music.bank"),
            "the refusal must quote what it was given: {error}"
        );
        assert!(
            error.contains("G1R/Content/FMOD/Desktop"),
            "the refusal must name the constraint, not just call the value unsafe: {error}"
        );
        assert!(
            error.contains("SFX.bank"),
            "the refusal must show a spelling that works: {error}"
        );
    }

    #[test]
    fn a_missing_spec_relative_asset_is_reported_by_its_resolved_path_and_section() {
        // A bare filename in the error is no longer even the path that was opened, and a spec with
        // ten audio entries gave no clue which one failed.
        let assets = tempfile::tempdir().unwrap();
        let error =
            build_bundle_relative_to(&test_spec_with_bare_asset_names("Missing"), assets.path())
                .unwrap_err()
                .to_string();
        let resolved = assets.path().join("click.wav");
        assert!(
            error.contains(&resolved.display().to_string()) && error.contains("audio[0]"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_bundle_wrapper_still_resolves_against_the_process_directory() {
        // gore-ffi and the manager's own re-builds pass a spec that never came from a file, so the
        // legacy entry point must keep its cwd-relative contract exactly.
        let error = build_bundle(&test_spec_with_bare_asset_names("Wrapper"))
            .unwrap_err()
            .to_string();
        let expected = Path::new(".").join("click.wav");
        assert!(
            error.contains(&expected.display().to_string()),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_emits_file_patch() {
        let dir = tempfile::tempdir().unwrap();
        let cursor = dir.path().join("Normal.PNG");
        std::fs::write(&cursor, b"CURSOR-BYTES").unwrap();
        let spec = test_loose_spec(
            "FileMod",
            vec![
                LooseFileReplacement {
                    game_path: "G1R/Content/Slate/Cursors/Normal/Normal.PNG".into(),
                    source_path: cursor.display().to_string(),
                },
                LooseFileReplacement {
                    game_path: "G1R/Config/DefaultEngine.ini".into(),
                    source_path: cursor.display().to_string(),
                },
            ],
        );

        let bundle = build_bundle(&spec).unwrap();
        let map: BTreeMap<String, String> =
            serde_json::from_slice(&bundle.files["files/manifest.json"]).unwrap();
        assert_eq!(
            map.keys().cloned().collect::<Vec<_>>(),
            vec![
                "G1R/Config/DefaultEngine.ini".to_string(),
                "G1R/Content/Slate/Cursors/Normal/Normal.PNG".to_string(),
            ]
        );
        for payload in map.values() {
            assert_eq!(bundle.files[payload], b"CURSOR-BYTES");
        }
        assert!(matches!(bundle.manifest.components.last(),
            Some(Component::FilePatch { path, targets })
                if path == "files" && targets.len() == 2 && targets[0] == "G1R/Config/DefaultEngine.ini"));
    }

    #[test]
    fn build_refuses_a_loose_destination_the_deploy_record_would_reject() {
        // Authoring is the first place the destination allowlist is asked about, and it has to
        // give the same answer the record layer gives — otherwise a bundle builds happily and
        // dies at deploy, after the user has shipped it.
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("payload.bin");
        std::fs::write(&source, b"payload").unwrap();
        for game_path in [
            "G1R/Binaries/Win64/G1R-Win64-Shipping.exe",
            "G1R/Content/Paks/~mods/zzz_Other_P.pak",
            "G1R/Content/FMOD/Desktop/SFX.bank",
            "G1R/Story/Cache/AlkimiaLocalization_0.lcache",
            "G1R/Content/Slate/Cursors/Normal/Normal.PNG.gore-bak",
            "../outside.bin",
            "G1R\\Content\\Slate\\Normal.PNG",
        ] {
            let spec = test_loose_spec(
                "Refused",
                vec![LooseFileReplacement {
                    game_path: game_path.into(),
                    source_path: source.display().to_string(),
                }],
            );
            let error = build_bundle(&spec).unwrap_err().to_string();
            assert!(
                error.contains("loose file game path"),
                "{game_path} was not refused: {error}"
            );
        }

        let spec = test_loose_spec(
            "Doubled",
            vec![
                LooseFileReplacement {
                    game_path: "G1R/Content/Slate/Cursors/Normal/Normal.PNG".into(),
                    source_path: source.display().to_string(),
                },
                LooseFileReplacement {
                    game_path: "g1r/content/slate/cursors/normal/normal.png".into(),
                    source_path: source.display().to_string(),
                },
            ],
        );
        let error = build_bundle(&spec).unwrap_err().to_string();
        assert!(error.contains("duplicate files entry"), "{error}");
    }

    fn test_loose_spec(name: &str, files: Vec<LooseFileReplacement>) -> BuildSpec {
        BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files,
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        }
    }

    fn test_pak_files_spec(name: &str, pak_files: Vec<LooseFileReplacement>) -> BuildSpec {
        BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files,
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        }
    }

    /// A plain V11 pak in the exact shape the shipped containers use — mount `../../../`, entries
    /// spelled `G1R/…` — sitting directly in `G1R/Content/Paks` where the shadow oracle looks.
    fn write_base_pak(game: &Path, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        let paks = game.join("G1R").join("Content").join("Paks");
        std::fs::create_dir_all(&paks).unwrap();
        let path = paks.join(format!("{name}.pak"));
        let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
        {
            let mut writer = repak::PakBuilder::new().writer(
                &mut file,
                repak::Version::V11,
                "../../../".to_string(),
                None,
            );
            for &(entry, bytes) in entries {
                writer.write_file(entry, false, bytes).unwrap();
            }
            writer.write_index().unwrap();
        }
        file.flush().unwrap();
        path
    }

    fn read_test_pak_entry(pak: &Path, entry: &str) -> Vec<u8> {
        let mut file = std::io::BufReader::new(std::fs::File::open(pak).unwrap());
        let reader = repak::PakBuilder::new().reader(&mut file).unwrap();
        reader.get(entry, &mut file).unwrap()
    }

    #[test]
    fn root_manifest_format_selection_is_exact_and_byte_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("payload.bin");
        std::fs::write(&source, b"deterministic-payload").unwrap();

        let base_spec = test_loose_spec(
            "BaseFormat",
            vec![LooseFileReplacement {
                game_path: "G1R/Content/Movies/Intro.bk2".into(),
                source_path: source.display().to_string(),
            }],
        );
        let base_first = build_bundle(&base_spec).unwrap();
        let base_second = build_bundle(&base_spec).unwrap();
        assert_eq!(base_first.manifest.format, MOD_MANIFEST_FORMAT_BASE);
        assert_eq!(base_first.files, base_second.files);
        assert_eq!(
            base_first.files["gore-mod.json"],
            serde_json::to_vec_pretty(&base_first.manifest).unwrap()
        );
        validate_mod_manifest_format(&base_first.manifest).unwrap();

        let pak_spec = test_pak_files_spec(
            "PakFormat",
            vec![LooseFileReplacement {
                game_path: TEST_CURSOR.into(),
                source_path: source.display().to_string(),
            }],
        );
        let pak_first = build_bundle(&pak_spec).unwrap();
        let pak_second = build_bundle(&pak_spec).unwrap();
        assert_eq!(
            pak_first.manifest.format,
            MOD_MANIFEST_FORMAT_PAK_FILE_PATCH
        );
        assert_eq!(pak_first.files, pak_second.files);
        assert_eq!(
            pak_first.files["gore-mod.json"],
            serde_json::to_vec_pretty(&pak_first.manifest).unwrap()
        );
        validate_mod_manifest_format(&pak_first.manifest).unwrap();
    }

    #[test]
    fn manifest_format_rejects_component_mismatches_and_unknown_versions() {
        let manifest = |format, components| ModManifest {
            format,
            mod_meta: ModMeta {
                name: "ContractProbe".into(),
                version: String::new(),
                author: String::new(),
            },
            components,
        };
        let pak = || Component::PakFilePatch {
            path: "pak_files".into(),
            targets: vec![TEST_CURSOR.into()],
        };

        validate_mod_manifest_format(&manifest(1, vec![])).unwrap();
        validate_mod_manifest_format(&manifest(2, vec![pak()])).unwrap();

        let old_format = validate_mod_manifest_format(&manifest(1, vec![pak()]))
            .unwrap_err()
            .to_string();
        assert!(
            old_format.contains("format 1") && old_format.contains("pak_file_patch"),
            "unexpected error: {old_format}"
        );

        let gratuitous_bump = validate_mod_manifest_format(&manifest(2, vec![]))
            .unwrap_err()
            .to_string();
        assert!(
            gratuitous_bump.contains("format 2")
                && gratuitous_bump.contains("requires")
                && gratuitous_bump.contains("pak_file_patch"),
            "unexpected error: {gratuitous_bump}"
        );

        for format in [0, 3, u32::MAX] {
            let unknown = validate_mod_manifest_format(&manifest(format, vec![pak()]))
                .unwrap_err()
                .to_string();
            assert!(
                unknown.contains(&format.to_string()) && unknown.contains("want 1 or 2"),
                "unexpected error for {format}: {unknown}"
            );
        }
    }

    #[test]
    fn write_bundle_rejects_a_forged_format_before_clearing_output() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("payload.bin");
        std::fs::write(&source, b"payload").unwrap();
        let spec = test_pak_files_spec(
            "ForgedFormat",
            vec![LooseFileReplacement {
                game_path: TEST_CURSOR.into(),
                source_path: source.display().to_string(),
            }],
        );
        let mut bundle = build_bundle(&spec).unwrap();
        bundle.manifest.format = 1;
        bundle.files.insert(
            "gore-mod.json".into(),
            serde_json::to_vec_pretty(&bundle.manifest).unwrap(),
        );

        let output = dir.path().join("existing-output");
        std::fs::create_dir(&output).unwrap();
        let sentinel = output.join("sentinel.txt");
        std::fs::write(&sentinel, b"keep").unwrap();

        let error = write_bundle(&output, &bundle).unwrap_err().to_string();
        assert!(
            error.contains("format 1") && error.contains("pak_file_patch"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
        assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
    }

    #[test]
    fn parallel_pak_prepares_keep_unique_exact_sources_until_plan_drop() {
        let dir = tempfile::tempdir().unwrap();
        let make_bundle = |folder: &str, name: &str, target: &str, payload: &[u8]| {
            let root = dir.path().join(folder);
            std::fs::create_dir_all(&root).unwrap();
            let source = root.join("payload.bin");
            std::fs::write(&source, payload).unwrap();
            let spec = test_pak_files_spec(
                name,
                vec![LooseFileReplacement {
                    game_path: target.into(),
                    source_path: source.display().to_string(),
                }],
            );
            let bundle = root.join("bundle");
            write_bundle(&bundle, &build_bundle(&spec).unwrap()).unwrap();
            bundle
        };
        let first_bundle = make_bundle("first", "ParallelAlpha", TEST_CURSOR, b"alpha-payload");
        let second_target = "G1R/Content/Movies/Intro.bk2";
        let second_bundle = make_bundle("second", "ParallelBeta", second_target, b"beta-payload");
        let game = dir.path().join("game");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

        let first_barrier = barrier.clone();
        let first_game = game.clone();
        let first = std::thread::spawn(move || {
            first_barrier.wait();
            prepare_pak_file_component(
                &first_bundle,
                "pak_files",
                "ParallelAlpha",
                0,
                &resolve_game_paths(&first_game),
            )
        });
        let second_barrier = barrier.clone();
        let second_game = game.clone();
        let second = std::thread::spawn(move || {
            second_barrier.wait();
            prepare_pak_file_component(
                &second_bundle,
                "pak_files",
                "ParallelBeta",
                0,
                &resolve_game_paths(&second_game),
            )
        });
        barrier.wait();

        let (first_files, first_root) = first.join().unwrap().unwrap();
        let (second_files, second_root) = second.join().unwrap().unwrap();
        let (first_source, first_destination) = &first_files[0];
        let (second_source, second_destination) = &second_files[0];
        assert_ne!(first_root.path(), second_root.path());
        assert!(first_source.exists());
        assert!(second_source.exists());
        assert_eq!(
            read_test_pak_entry(first_source, TEST_CURSOR),
            b"alpha-payload"
        );
        assert_eq!(
            read_test_pak_entry(second_source, second_target),
            b"beta-payload"
        );

        let first_name = format!(
            "zzz_ParallelAlpha_{}_0_files_P.pak",
            name_hash("ParallelAlpha")
        );
        let second_name = format!(
            "zzz_ParallelBeta_{}_0_files_P.pak",
            name_hash("ParallelBeta")
        );
        assert_eq!(first_source.file_name().unwrap(), first_name.as_str());
        assert_eq!(first_destination.file_name().unwrap(), first_name.as_str());
        assert_eq!(second_source.file_name().unwrap(), second_name.as_str());
        assert_eq!(
            second_destination.file_name().unwrap(),
            second_name.as_str()
        );

        // A repeated prepare gets another owned root while preserving exact published bytes/name.
        let first_bundle = dir.path().join("first/bundle");
        let (repeat_files, repeat_root) = prepare_pak_file_component(
            &first_bundle,
            "pak_files",
            "ParallelAlpha",
            0,
            &resolve_game_paths(&game),
        )
        .unwrap();
        assert_ne!(first_root.path(), repeat_root.path());
        assert_eq!(
            std::fs::read(first_source).unwrap(),
            std::fs::read(&repeat_files[0].0).unwrap()
        );
        assert_eq!(first_destination, &repeat_files[0].1);

        let roots = [
            first_root.path().to_path_buf(),
            second_root.path().to_path_buf(),
            repeat_root.path().to_path_buf(),
        ];
        drop((first_root, second_root, repeat_root));
        for root in roots {
            assert!(!root.exists(), "temporary root leaked: {}", root.display());
        }
    }

    /// The additive route is DECLARED, never inferred from the install. `pak_files` gets its own
    /// component, its own bundle subdir and its own manifest, so one bundle keeps one footprint on
    /// every machine — the two sections may even name the same destination, said twice on purpose.
    #[test]
    fn a_pak_files_section_produces_its_own_component_beside_the_in_place_one() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();
        let mut spec = test_pak_files_spec(
            "BothRoutes",
            vec![LooseFileReplacement {
                game_path: TEST_CURSOR.into(),
                source_path: source.display().to_string(),
            }],
        );
        spec.files = vec![LooseFileReplacement {
            game_path: "G1R/Content/Movies/Intro.bk2".into(),
            source_path: source.display().to_string(),
        }];

        let bundle = build_bundle(&spec).unwrap();
        assert_eq!(bundle.manifest.format, MOD_MANIFEST_FORMAT_PAK_FILE_PATCH);
        assert!(matches!(
            bundle.manifest.components.as_slice(),
            [
                Component::FilePatch { path: files, targets: file_targets },
                Component::PakFilePatch { path: paks, targets: pak_targets },
            ] if files == "files"
                && paks == "pak_files"
                && file_targets == &vec!["G1R/Content/Movies/Intro.bk2".to_string()]
                && pak_targets == &vec![TEST_CURSOR.to_string()]
        ));
        assert!(bundle.files.contains_key("pak_files/manifest.json"));
        assert!(
            bundle
                .files
                .keys()
                .any(|rel| rel.starts_with("pak_files/0_")),
            "the payload must be staged under its own section: {:?}",
            bundle.files.keys().collect::<Vec<_>>()
        );
    }

    /// The experiment this gate exists for: all eight cursor PNGs were replaced on disk and the
    /// cursor did not change, because `G1R-Windows.pak` carries them and Unreal consults a mounted
    /// pak before the filesystem. A deploy that succeeds and changes nothing is the one failure the
    /// toolkit can never observe afterwards, so it is refused before the backup is taken — naming
    /// the container that wins and the section that does reach the file.
    #[test]
    fn deploy_refuses_an_in_place_target_a_mounted_pak_already_carries() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join(loose_relative_os_path(TEST_CURSOR));
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"shipped-cursor").unwrap();
        write_base_pak(&game, "G1R-Windows", &[(TEST_CURSOR, b"packed-cursor")]);
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();

        let bundle_dir = test_cursor_bundle(dir.path(), "ShadowedCursor", &source);
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("already packed in G1R-Windows.pak"),
            "the refusal must name the container that wins: {error}"
        );
        assert!(
            error.contains("pak_files"),
            "the refusal must name the section that reaches it instead: {error}"
        );
        assert_eq!(
            std::fs::read(&live).unwrap(),
            b"shipped-cursor",
            "the refusal must land before any write"
        );
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    /// A destination NO pak carries stays reachable in place. Without this, the gate could pass by
    /// refusing everything the moment an install has containers at all — which is every install.
    #[test]
    fn deploy_still_replaces_a_loose_target_no_pak_carries() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join(loose_relative_os_path(TEST_CURSOR));
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"shipped-cursor").unwrap();
        write_base_pak(
            &game,
            "G1R-Windows",
            &[("G1R/Content/Movies/Intro.bk2", b"packed-movie")],
        );
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();

        let bundle_dir = test_cursor_bundle(dir.path(), "UnshadowedCursor", &source);
        deploy(&bundle_dir, &game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"modded-cursor");
    }

    /// The `pak_files` route is additive end to end: one `zzz_…_files_P.pak` in `~mods`, no backup
    /// to keep, and undeploy deletes exactly what was added. The destination need not exist on disk
    /// at all — on the shipped build the whole `G1R/Config` subtree is like that.
    #[test]
    fn pak_files_publish_one_additive_pak_that_undeploy_deletes() {
        // Keep one public deploy/undeploy roundtrip on the caller's ambient path spelling.
        let dir = ::tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(game.join("G1R").join("Content")).unwrap();
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();
        let spec = test_pak_files_spec(
            "PakCursor",
            vec![LooseFileReplacement {
                game_path: TEST_CURSOR.into(),
                source_path: source.display().to_string(),
            }],
        );
        let bundle_dir = dir.path().join("bundle-pak-cursor");
        write_bundle(&bundle_dir, &build_bundle(&spec).unwrap()).unwrap();

        let record = deploy(&bundle_dir, &game).unwrap();
        assert!(
            record.backups.is_empty(),
            "an additive component must take no backup: {:?}",
            record.backups
        );
        let mut published: Vec<PathBuf> =
            std::fs::read_dir(game.join("G1R").join("Content").join("Paks").join("~mods"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
        published.sort();
        assert_eq!(published.len(), 1, "published: {published:?}");
        let pak = published.remove(0);
        let name = pak.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("zzz_PakCursor_") && name.ends_with("_files_P.pak"),
            "the pak must mirror the triplet naming convention: {name}"
        );
        assert_eq!(
            gore_tex::container::list_pak_files(&pak).unwrap(),
            vec![TEST_CURSOR.to_string()],
            "the entry must be spelled exactly as the base container spells it"
        );
        assert!(
            !game.join(loose_relative_os_path(TEST_CURSOR)).exists(),
            "an additive component must not create the destination on disk"
        );

        undeploy(&game).unwrap();
        assert!(!pak.exists(), "undeploy must delete what it added");
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn build_emits_reproducible_versioned_voice_patch() {
        let dir = tempfile::tempdir().unwrap();
        let replace = dir.path().join("replace.ogg");
        let add = dir.path().join("add.ogg");
        std::fs::write(&replace, test_ogg(32_000)).unwrap();
        std::fs::write(&add, test_ogg(48_000)).unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "VoiceMod".into(),
                version: "1".into(),
                author: "tester".into(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![
                VoiceArchiveEdit {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Replace,
                    archive_path: "NPC/Hero/hello.ogg".into(),
                    ogg_path: replace.display().to_string(),
                    observation: None,
                },
                VoiceArchiveEdit {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Add,
                    archive_path: "GORE/new.ogg".into(),
                    ogg_path: add.display().to_string(),
                    observation: None,
                },
            ],
        };

        let first = build_bundle(&spec).unwrap();
        let second = build_bundle(&spec).unwrap();
        assert_eq!(first.files, second.files);
        assert!(matches!(
            first.manifest.components.as_slice(),
            [Component::VoiceArchivePatch { path }] if path == "voice"
        ));
        let manifest: VoicePatchManifest =
            serde_json::from_slice(&first.files["voice/manifest.json"]).unwrap();
        assert_eq!(manifest.format, 1);
        assert_eq!(manifest.edits.len(), 2);
        assert_eq!(manifest.edits[0].ogg, "voice/payload/0.ogg");
        assert_eq!(manifest.edits[1].ogg, "voice/payload/1.ogg");
        assert_eq!(
            first.files["voice/payload/0.ogg"],
            std::fs::read(&replace).unwrap()
        );

        let bad = dir.path().join("bad.ogg");
        std::fs::write(&bad, b"not ogg").unwrap();
        let mut invalid = spec.clone();
        invalid.voice[1].ogg_path = bad.display().to_string();
        assert!(matches!(build_bundle(&invalid), Err(ModError::Voice(_))));
        invalid.voice[1].ogg_path = add.display().to_string();
        invalid.voice[1].archive_path = "../escape.ogg".into();
        assert!(build_bundle(&invalid).is_err());
        invalid.voice[1].archive_path = "GORE/new.ogg".into();
        invalid.voice[1].archive = "COM¹.zip".into();
        assert!(build_bundle(&invalid).is_err());
        invalid.voice[1].archive = "German.zip".into();
        invalid.voice[1].archive_path = "NPC/LPT².ogg".into();
        assert!(build_bundle(&invalid).is_err());
    }

    #[test]
    fn sealed_voice_build_emits_v2_and_rejects_mixed_or_invalid_contracts() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&archive_path, &[("NPC/Hero/hello.ogg", &original)]);
        let replacement = dir.path().join("replace.ogg");
        let addition = dir.path().join("add.ogg");
        std::fs::write(&replacement, test_ogg(32_000)).unwrap();
        std::fs::write(&addition, test_ogg(48_000)).unwrap();

        let present = observe_test_voice_archive(&archive_path, "NPC/Hero/hello.ogg");
        let absent = observe_test_voice_archive(&archive_path, "GORE/new.ogg");
        let spec = BuildSpec {
            meta: ModMeta {
                name: "SealedVoice".into(),
                version: "1".into(),
                author: "tester".into(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![
                VoiceArchiveEdit {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Replace,
                    archive_path: "NPC/Hero/hello.ogg".into(),
                    ogg_path: replacement.display().to_string(),
                    observation: Some(present.clone()),
                },
                VoiceArchiveEdit {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Add,
                    archive_path: "GORE/new.ogg".into(),
                    ogg_path: addition.display().to_string(),
                    observation: Some(absent.clone()),
                },
            ],
        };

        let bundle = build_bundle(&spec).unwrap();
        let manifest: VoicePatchManifest =
            serde_json::from_slice(&bundle.files["voice/manifest.json"]).unwrap();
        assert_eq!(manifest.format, 2);
        assert_eq!(manifest.executable_generation, None);
        assert_eq!(manifest.edits[0].observation, Some(present.clone()));
        assert_eq!(manifest.edits[1].observation, Some(absent));
        assert!(manifest
            .edits
            .iter()
            .all(|edit| edit.payload_seal.is_none()));

        let mut mixed = spec.clone();
        mixed.voice[1].observation = None;
        let error = build_bundle(&mixed).err().unwrap().to_string();
        assert!(
            error.contains("mixes sealed and legacy"),
            "unexpected error: {error}"
        );

        let mut wrong_proof = spec.clone();
        wrong_proof.voice[0]
            .observation
            .as_mut()
            .unwrap()
            .member_proof = VoiceMemberProof::Absent;
        let error = build_bundle(&wrong_proof).err().unwrap().to_string();
        assert!(error.contains("replace") && error.contains("present"));

        let mut zero_size = spec.clone();
        zero_size.voice[0]
            .observation
            .as_mut()
            .unwrap()
            .member_proof = VoiceMemberProof::Present {
            uncompressed_size: 0,
            crc32: 0,
        };
        let error = build_bundle(&zero_size).err().unwrap().to_string();
        assert!(error.contains("non-zero"), "unexpected error: {error}");

        let mut uppercase_hash = spec.clone();
        uppercase_hash.voice[0]
            .observation
            .as_mut()
            .unwrap()
            .archive_sha256 = "A".repeat(64);
        let error = build_bundle(&uppercase_hash).err().unwrap().to_string();
        assert!(
            error.contains("lowercase hexadecimal"),
            "unexpected error: {error}"
        );

        let mut zero_archive = spec.clone();
        zero_archive.voice[0]
            .observation
            .as_mut()
            .unwrap()
            .archive_size = 0;
        let error = build_bundle(&zero_archive).err().unwrap().to_string();
        assert!(
            error.contains("archive size must be non-zero"),
            "unexpected error: {error}"
        );

        let mut legacy_with_observation = manifest.clone();
        legacy_with_observation.format = 1;
        assert!(validate_voice_manifest(&legacy_with_observation)
            .unwrap_err()
            .to_string()
            .contains("format 1"));
        let mut v2_without_observation = manifest.clone();
        v2_without_observation.edits[0].observation = None;
        assert!(validate_voice_manifest(&v2_without_observation)
            .unwrap_err()
            .to_string()
            .contains("format 2"));
        let mut v2_with_payload_seal = manifest.clone();
        v2_with_payload_seal.edits[0].payload_seal = Some(VoicePayloadSeal {
            byte_len: 1,
            sha256: "0".repeat(64),
        });
        assert!(validate_voice_manifest(&v2_with_payload_seal)
            .unwrap_err()
            .to_string()
            .contains("format 2"));
        // Committed v2 semantics remain case-insensitive later-wins, including duplicates within
        // one component. Format 3 is the first canonical-unique contract.
        let mut duplicate_v2 = manifest.clone();
        let mut duplicate = duplicate_v2.edits[0].clone();
        duplicate.archive = duplicate.archive.to_ascii_uppercase();
        duplicate.archive_path = duplicate.archive_path.to_ascii_uppercase();
        duplicate_v2.edits.push(duplicate);
        validate_voice_manifest(&duplicate_v2).unwrap();
        let mut unsupported = manifest.clone();
        unsupported.format = 4;
        assert!(validate_voice_manifest(&unsupported)
            .unwrap_err()
            .to_string()
            .contains("want 1, 2, or 3"));
        let mut disagreeing = manifest;
        disagreeing.edits[1]
            .observation
            .as_mut()
            .unwrap()
            .archive_size += 1;
        assert!(validate_voice_manifest(&disagreeing)
            .unwrap_err()
            .to_string()
            .contains("disagree"));
    }

    #[test]
    fn committed_v2_manifest_fixture_remains_byte_exact_and_duplicate_later_wins() {
        let fixture = format!(
            concat!(
                "{{\n  \"format\": 2,\n  \"edits\": [\n    {{\n      \"archive\": \"German.zip\",\n      \"op\": \"replace\",\n      \"archive_path\": \"NPC/Hero/hello.ogg\",\n      \"ogg\": \"voice/first.ogg\",\n      \"observation\": {{\n        \"archive_size\": 123,\n        \"archive_sha256\": \"{}\",\n        \"member_proof\": {{\n          \"state\": \"present\",\n          \"uncompressed_size\": 7,\n          \"crc32\": 9\n        }}\n      }}\n    }},\n    {{\n      \"archive\": \"GERMAN.ZIP\",\n      \"op\": \"replace\",\n      \"archive_path\": \"NPC/HERO/HELLO.OGG\",\n      \"ogg\": \"voice/second.ogg\",\n      \"observation\": {{\n        \"archive_size\": 123,\n        \"archive_sha256\": \"{}\",\n        \"member_proof\": {{\n          \"state\": \"present\",\n          \"uncompressed_size\": 7,\n          \"crc32\": 9\n        }}\n      }}\n    }}\n  ]\n}}"
            ),
            "0".repeat(64),
            "0".repeat(64),
        );
        let manifest: VoicePatchManifest = serde_json::from_str(&fixture).unwrap();
        validate_voice_manifest(&manifest).unwrap();
        assert_eq!(manifest.executable_generation, None);
        assert!(manifest
            .edits
            .iter()
            .all(|edit| edit.payload_seal.is_none()));
        assert_eq!(serde_json::to_string_pretty(&manifest).unwrap(), fixture);

        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("v2-bundle");
        std::fs::create_dir_all(bundle.join("voice")).unwrap();
        let first = test_ogg(32_000);
        let second = test_ogg(44_100);
        std::fs::write(bundle.join("voice/manifest.json"), fixture).unwrap();
        std::fs::write(bundle.join("voice/first.ogg"), &first).unwrap();
        std::fs::write(bundle.join("voice/second.ogg"), &second).unwrap();
        let mut pending = PendingVoiceEdits::new();
        let mut order = 0;
        merge_voice_component(&bundle, "voice", &mut pending, &mut order).unwrap();
        assert_eq!(pending.edits.len(), 1);
        let winner = pending.edits.values().next().unwrap();
        assert_eq!(winner.archive, "GERMAN.ZIP");
        assert_eq!(winner.archive_path, "NPC/HERO/HELLO.OGG");
        assert_eq!(winner.ogg, second);
    }

    #[test]
    fn semantic_install_root_accepts_direct_g1r_case_insensitively() {
        let install = PathBuf::from("install-root");
        for spelling in ["G1R", "g1r", "G1r"] {
            let child = install.join(spelling);
            assert_eq!(semantic_install_root(&child), install);
            assert_eq!(
                resolve_game_paths(&child).executable,
                child.join("Binaries/Win64/G1R-Win64-Shipping.exe")
            );
        }
        assert_eq!(semantic_install_root(&install), install);
    }

    #[test]
    fn byte_backed_sealed_voice_bundle_is_deterministic_and_exactly_verifiable() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("German.zip");
        let first_original = test_ogg(16_000);
        let second_original = test_ogg(22_050);
        write_test_voice_zip(
            &archive,
            &[
                ("NPC/Hero/hello.ogg", &first_original),
                ("NPC/Hero/bye.ogg", &second_original),
            ],
        );
        let first_observation = observe_test_voice_archive(&archive, "NPC/Hero/hello.ogg");
        let second_observation = observe_test_voice_archive(&archive, "NPC/Hero/bye.ogg");
        let replacements = vec![
            test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                first_observation.clone(),
            ),
            test_sealed_voice_replace(
                "NPC/Hero/bye.ogg",
                test_ogg(44_100),
                second_observation.clone(),
            ),
        ];

        let first = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            replacements.clone(),
        )
        .unwrap();
        let second = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            replacements,
        )
        .unwrap();
        assert_eq!(first.files, second.files);
        assert!(matches!(
            first.manifest.components.as_slice(),
            [Component::VoiceArchivePatch { path }] if path == "voice"
        ));
        let manifest: VoicePatchManifest =
            serde_json::from_slice(&first.files["voice/manifest.json"]).unwrap();
        assert_eq!(manifest.format, 3);
        assert_eq!(
            manifest.executable_generation,
            Some(test_voice_generation())
        );
        assert!(manifest
            .edits
            .iter()
            .all(|edit| edit.op == VoicePatchOp::Replace));
        assert_eq!(manifest.edits[0].observation, Some(first_observation));
        assert_eq!(manifest.edits[1].observation, Some(second_observation));
        assert_eq!(manifest.edits[0].ogg, "voice/payload/0.ogg");
        assert_eq!(manifest.edits[1].ogg, "voice/payload/1.ogg");
        assert_eq!(
            manifest.edits[0].payload_seal,
            Some(voice_payload_seal(&first.files["voice/payload/0.ogg"]))
        );

        let output = dir.path().join("managed-voice-bundle");
        write_voice_bundle_new(&output, &first).unwrap();
        verify_sealed_voice_bundle(&output).unwrap();
        assert_eq!(
            std::fs::read(output.join("voice/payload/0.ogg")).unwrap(),
            first.files["voice/payload/0.ogg"]
        );
    }

    #[test]
    fn byte_backed_sealed_voice_build_rejects_invalid_contracts_and_budgets() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&archive, &[("NPC/Hero/hello.ogg", &original)]);
        let present = observe_test_voice_archive(&archive, "NPC/Hero/hello.ogg");
        let valid =
            test_sealed_voice_replace("NPC/Hero/hello.ogg", test_ogg(32_000), present.clone());

        assert!(build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![]
        )
        .err()
        .unwrap()
        .to_string()
        .contains("at least one"));

        let mut unsafe_target = valid.clone();
        unsafe_target.archive_path = "../escape.ogg".into();
        assert!(build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![unsafe_target]
        )
        .is_err());

        let mut duplicate = valid.clone();
        duplicate.archive = duplicate.archive.to_ascii_uppercase();
        duplicate.archive_path = duplicate.archive_path.to_ascii_uppercase();
        let error = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![valid.clone(), duplicate],
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("duplicate deployment target"));

        let mut absent = valid.clone();
        absent.observation.member_proof = VoiceMemberProof::Absent;
        let error = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![absent],
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("replace") && error.contains("present"));

        let mut invalid_ogg = valid.clone();
        invalid_ogg.ogg = b"not an Ogg stream".to_vec();
        assert!(matches!(
            build_sealed_voice_bundle(
                test_sealed_voice_meta(),
                test_voice_generation(),
                vec![invalid_ogg]
            ),
            Err(ModError::Voice(_))
        ));

        let mut unsafe_meta = test_sealed_voice_meta();
        unsafe_meta.name = "../ManagedVoice".into();
        assert!(build_sealed_voice_bundle(
            unsafe_meta,
            test_voice_generation(),
            vec![valid.clone()]
        )
        .is_err());

        let mut disagreeing = valid.clone();
        disagreeing.archive_path = "NPC/Hero/other.ogg".into();
        disagreeing.observation.archive_size += 1;
        let error = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![valid, disagreeing],
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("disagree"), "unexpected error: {error}");

        assert_eq!(voice_payload_byte_limit(0, 123).unwrap(), 123);
        assert_eq!(
            voice_payload_byte_limit(MAX_PENDING_VOICE_OGG_BYTES - 7, 123).unwrap(),
            7
        );
        assert!(voice_payload_byte_limit(MAX_PENDING_VOICE_OGG_BYTES + 1, 123).is_err());
    }

    #[test]
    fn sealed_voice_verifier_rejects_extra_missing_and_mutated_content() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&archive, &[("NPC/Hero/hello.ogg", &original)]);
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                observe_test_voice_archive(&archive, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();

        let extra_file = dir.path().join("extra-file");
        write_bundle(&extra_file, &bundle).unwrap();
        std::fs::write(extra_file.join("voice/unexpected.bin"), b"extra").unwrap();
        let error = verify_sealed_voice_bundle(&extra_file)
            .unwrap_err()
            .to_string();
        assert!(error.contains("extra") && error.contains("unexpected.bin"));

        let extra_dir = dir.path().join("extra-dir");
        write_bundle(&extra_dir, &bundle).unwrap();
        std::fs::create_dir(extra_dir.join("empty-extra")).unwrap();
        let error = verify_sealed_voice_bundle(&extra_dir)
            .unwrap_err()
            .to_string();
        assert!(error.contains("directory layout") && error.contains("empty-extra"));

        let missing_payload = dir.path().join("missing-payload");
        write_bundle(&missing_payload, &bundle).unwrap();
        std::fs::remove_file(missing_payload.join("voice/payload/0.ogg")).unwrap();
        assert!(verify_sealed_voice_bundle(&missing_payload).is_err());

        let missing_manifest = dir.path().join("missing-manifest");
        write_bundle(&missing_manifest, &bundle).unwrap();
        std::fs::remove_file(missing_manifest.join("voice/manifest.json")).unwrap();
        assert!(verify_sealed_voice_bundle(&missing_manifest).is_err());

        let extra_component = dir.path().join("extra-component");
        write_bundle(&extra_component, &bundle).unwrap();
        let mut root_manifest: ModManifest =
            serde_json::from_slice(&bundle.files["gore-mod.json"]).unwrap();
        root_manifest.components.push(Component::LocPatch {
            path: "loc/edits.json".into(),
        });
        std::fs::write(
            extra_component.join("gore-mod.json"),
            serde_json::to_vec_pretty(&root_manifest).unwrap(),
        )
        .unwrap();
        let error = verify_sealed_voice_bundle(&extra_component)
            .unwrap_err()
            .to_string();
        assert!(error.contains("exactly one voice component"));

        let missing_component = dir.path().join("missing-component");
        write_bundle(&missing_component, &bundle).unwrap();
        root_manifest.components.clear();
        std::fs::write(
            missing_component.join("gore-mod.json"),
            serde_json::to_vec_pretty(&root_manifest).unwrap(),
        )
        .unwrap();
        assert!(verify_sealed_voice_bundle(&missing_component).is_err());

        let additive = dir.path().join("additive");
        write_bundle(&additive, &bundle).unwrap();
        let mut voice_manifest: VoicePatchManifest =
            serde_json::from_slice(&bundle.files["voice/manifest.json"]).unwrap();
        voice_manifest.edits[0].op = VoicePatchOp::Add;
        voice_manifest.edits[0]
            .observation
            .as_mut()
            .unwrap()
            .member_proof = VoiceMemberProof::Absent;
        std::fs::write(
            additive.join("voice/manifest.json"),
            serde_json::to_vec_pretty(&voice_manifest).unwrap(),
        )
        .unwrap();
        let error = verify_sealed_voice_bundle(&additive)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("format 3") && error.contains("replace"),
            "unexpected error: {error}"
        );

        let noncanonical = dir.path().join("noncanonical");
        write_bundle(&noncanonical, &bundle).unwrap();
        voice_manifest = serde_json::from_slice(&bundle.files["voice/manifest.json"]).unwrap();
        voice_manifest.edits[0].ogg = "voice/payload/00.ogg".into();
        std::fs::write(
            noncanonical.join("voice/manifest.json"),
            serde_json::to_vec_pretty(&voice_manifest).unwrap(),
        )
        .unwrap();
        let error = verify_sealed_voice_bundle(&noncanonical)
            .unwrap_err()
            .to_string();
        assert!(error.contains("non-canonical payload path"));

        let corrupt = dir.path().join("corrupt");
        write_bundle(&corrupt, &bundle).unwrap();
        std::fs::write(corrupt.join("voice/payload/0.ogg"), b"not ogg").unwrap();
        assert!(matches!(
            verify_sealed_voice_bundle(&corrupt),
            Err(ModError::Voice(_))
        ));

        let valid_substitution = dir.path().join("valid-substitution");
        write_bundle(&valid_substitution, &bundle).unwrap();
        std::fs::write(
            valid_substitution.join("voice/payload/0.ogg"),
            test_ogg(48_000),
        )
        .unwrap();
        let error = verify_sealed_voice_bundle(&valid_substitution)
            .unwrap_err()
            .to_string();
        assert!(error.contains("content seal"), "unexpected error: {error}");

        let linked = dir.path().join("linked");
        write_bundle(&linked, &bundle).unwrap();
        let payload = linked.join("voice/payload/0.ogg");
        let external = dir.path().join("external.ogg");
        std::fs::write(&external, test_ogg(44_100)).unwrap();
        std::fs::remove_file(&payload).unwrap();
        if make_test_file_link(&external, &payload) {
            let error = verify_sealed_voice_bundle(&linked).unwrap_err().to_string();
            assert!(error.contains("symbolic link") || error.contains("reparse point"));
        }
    }

    #[test]
    fn new_voice_bundle_writer_never_clobbers_and_cleans_its_failed_tree() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&archive, &[("NPC/Hero/hello.ogg", &original)]);
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                observe_test_voice_archive(&archive, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();

        let existing_dir = dir.path().join("existing-dir");
        std::fs::create_dir(&existing_dir).unwrap();
        std::fs::write(existing_dir.join("sentinel.txt"), b"keep me").unwrap();
        let error = write_voice_bundle_new(&existing_dir, &bundle)
            .unwrap_err()
            .to_string();
        assert!(error.contains("already exists"));
        assert_eq!(
            std::fs::read(existing_dir.join("sentinel.txt")).unwrap(),
            b"keep me"
        );

        let existing_file = dir.path().join("existing-file");
        std::fs::write(&existing_file, b"keep this too").unwrap();
        assert!(write_voice_bundle_new(&existing_file, &bundle).is_err());
        assert_eq!(std::fs::read(&existing_file).unwrap(), b"keep this too");

        let invalid_target = dir.path().join("invalid-bundle");
        let mut invalid_bundle = Bundle {
            files: bundle.files.clone(),
            manifest: ModManifest {
                format: bundle.manifest.format,
                mod_meta: bundle.manifest.mod_meta.clone(),
                components: vec![],
            },
        };
        invalid_bundle.files.insert(
            "gore-mod.json".into(),
            serde_json::to_vec_pretty(&invalid_bundle.manifest).unwrap(),
        );
        assert!(write_voice_bundle_new(&invalid_target, &invalid_bundle).is_err());
        assert!(!invalid_target.exists());

        let failed_target = dir.path().join("failed-write");
        fail_next_voice_bundle_write(&failed_target);
        let error = write_voice_bundle_new(&failed_target, &bundle)
            .unwrap_err()
            .to_string();
        assert!(error.contains("injected"));
        assert!(!failed_target.exists());

        let real_parent = dir.path().join("real-parent");
        let linked_parent = dir.path().join("linked-parent");
        std::fs::create_dir(&real_parent).unwrap();
        if make_test_dir_link(&real_parent, &linked_parent) {
            let linked_target = linked_parent.join("must-not-exist");
            let error = write_voice_bundle_new(&linked_target, &bundle)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("symbolic-link") || error.contains("reparse"),
                "unexpected error: {error}"
            );
            assert!(!real_parent.join("must-not-exist").exists());
        }

        // Exercise the exact create-to-anchor window. Even when the directory just created by
        // the writer is renamed and replaced by an in-parent link before its first open, no bundle
        // file may reach the link target and cleanup must not traverse it.
        let raced_target = dir.path().join("raced-write");
        let parked_target = dir.path().join("raced-write-before-link");
        let race_destination = dir.path().join("race-destination");
        std::fs::create_dir(&race_destination).unwrap();
        std::fs::write(
            race_destination.join("sentinel.txt"),
            b"outside stays intact",
        )
        .unwrap();
        let link_installed = Rc::new(Cell::new(false));
        let link_installed_in_hook = link_installed.clone();
        let parked_target_in_hook = parked_target.clone();
        let race_destination_in_hook = race_destination.clone();
        mgr::model::inject_create_child_directory_race(move |created| {
            // A retained Windows parent handle may make this replacement impossible. That is
            // already the desired result, so only install the hostile link when the rename wins.
            if std::fs::rename(created, &parked_target_in_hook).is_err() {
                return;
            }
            if make_test_dir_link(&race_destination_in_hook, created) {
                link_installed_in_hook.set(true);
            } else {
                std::fs::rename(&parked_target_in_hook, created).unwrap();
            }
        });
        let raced_result = write_voice_bundle_new(&raced_target, &bundle);
        if link_installed.get() {
            let error = raced_result.unwrap_err().to_string();
            assert!(
                error.contains("symbolic link")
                    || error.contains("symbolic-link")
                    || error.contains("reparse point")
                    || error.contains("reparse"),
                "unexpected error: {error}"
            );
            assert!(!race_destination.join("gore-mod.json").exists());
            assert!(!race_destination.join("voice").exists());
            assert_eq!(
                std::fs::read(race_destination.join("sentinel.txt")).unwrap(),
                b"outside stays intact"
            );
            if std::fs::remove_file(&raced_target).is_err() {
                std::fs::remove_dir(&raced_target).unwrap();
            }
            std::fs::remove_dir(&parked_target).unwrap();
        } else {
            raced_result.unwrap();
        }

        let sealed_target = dir.path().join("disk-sealed");
        write_voice_bundle_new(&sealed_target, &bundle).unwrap();
        let expected = canonical_voice_bundle_tree_seal(
            bundle
                .files
                .iter()
                .map(|(path, bytes)| (path.clone(), bytes.clone())),
        )
        .unwrap();
        assert_eq!(
            seal_voice_bundle_disk_tree(&sealed_target).unwrap(),
            expected
        );

        // A byte-valid Ogg substitution must fail the disk-tree seal rather than returning the
        // seal of an unverified replacement tree.
        std::fs::write(sealed_target.join("voice/payload/0.ogg"), test_ogg(48_000)).unwrap();
        let error = seal_voice_bundle_disk_tree(&sealed_target)
            .unwrap_err()
            .to_string();
        assert!(error.contains("content seal"), "unexpected error: {error}");
    }

    #[test]
    fn staged_voice_bundle_retries_collisions_promotes_and_cleans_owned_failures() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&archive, &[("NPC/Hero/hello.ogg", &original)]);
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            test_voice_generation(),
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                observe_test_voice_archive(&archive, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();

        let raced_collision = Rc::new(RefCell::new(None));
        let raced_collision_in_hook = raced_collision.clone();
        mgr::model::inject_create_child_directory_precreate_race(move |candidate| {
            std::fs::create_dir(candidate).unwrap();
            std::fs::write(candidate.join("sentinel.txt"), b"racing creator owns this").unwrap();
            *raced_collision_in_hook.borrow_mut() = Some(candidate.to_path_buf());
        });
        let final_target = dir.path().join("published-voice");
        let staged = write_voice_bundle_staged_new(&final_target, &bundle).unwrap();
        let staging_path = staged.path().to_path_buf();
        let collision_path = raced_collision.borrow().clone().unwrap();
        assert_ne!(staging_path, collision_path);
        assert_eq!(
            std::fs::read(collision_path.join("sentinel.txt")).unwrap(),
            b"racing creator owns this"
        );
        staged.promote_new().unwrap();
        assert!(!staging_path.exists());
        verify_sealed_voice_bundle(&final_target).unwrap();
        assert_eq!(
            std::fs::read(collision_path.join("sentinel.txt")).unwrap(),
            b"racing creator owns this"
        );

        let occupied_final = dir.path().join("occupied-final");
        std::fs::create_dir(&occupied_final).unwrap();
        std::fs::write(occupied_final.join("sentinel.txt"), b"final owner wins").unwrap();
        let collided_stage = write_voice_bundle_staged_new(&occupied_final, &bundle).unwrap();
        let collided_stage_path = collided_stage.path().to_path_buf();
        let error = collided_stage.promote_new().unwrap_err();
        assert_eq!(error.kind(), VoiceBundleStagingErrorKind::OperationFailed);
        assert!(error.cleanup_confirmed());
        assert!(!collided_stage_path.exists());
        assert_eq!(
            std::fs::read(occupied_final.join("sentinel.txt")).unwrap(),
            b"final owner wins"
        );

        let aborted =
            write_voice_bundle_staged_new(&dir.path().join("aborted-final"), &bundle).unwrap();
        let aborted_path = aborted.path().to_path_buf();
        aborted.abort().unwrap();
        assert!(!aborted_path.exists());
    }

    #[test]
    fn format3_voice_deploy_requires_exact_executable_and_payload_generation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let pristine = std::fs::read(&live).unwrap();
        let generation = write_test_game_executable(&game, b"v3-game-generation");
        let replacement = test_ogg(44_100);
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            generation.clone(),
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                replacement.clone(),
                observe_test_voice_archive(&live, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();
        let bundle_dir = dir.path().join("bundle");
        write_bundle(&bundle_dir, &bundle).unwrap();
        deploy(&bundle_dir, &game).unwrap();
        assert_eq!(
            read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            replacement
        );
        assert_eq!(std::fs::read(bak_path(&live)).unwrap(), pristine);

        undeploy(&game).unwrap();
        assert_eq!(
            read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            original
        );

        // A byte-valid substituted payload is rejected by generic deployment too, before any
        // backup or record can be created.
        let substituted = dir.path().join("substituted");
        write_bundle(&substituted, &bundle).unwrap();
        std::fs::write(substituted.join("voice/payload/0.ogg"), test_ogg(48_000)).unwrap();
        let before = std::fs::read(&live).unwrap();
        let error = deploy(&substituted, &game).unwrap_err().to_string();
        assert!(error.contains("content seal"), "unexpected error: {error}");
        assert_eq!(std::fs::read(&live).unwrap(), before);
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());

        // An executable hotfix before prepare invalidates the complete v3 generation.
        std::fs::write(resolve_game_paths(&game).executable, b"post-build-hotfix").unwrap();
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("does not match") && error.contains("executable"),
            "unexpected error: {error}"
        );
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn format3_executable_is_rechecked_at_the_last_pre_mutation_gate() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let generation = write_test_game_executable(&game, b"prepare-generation");
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            generation,
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                observe_test_voice_archive(&live, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();
        let bundle_dir = dir.path().join("bundle");
        write_bundle(&bundle_dir, &bundle).unwrap();
        let gp = resolve_game_paths(&game);
        let plan = prepare(&bundle_dir, &bundle.manifest, &gp, None).unwrap();

        std::fs::write(&gp.executable, b"hotfix-after-prepare").unwrap();
        let before = std::fs::read(&live).unwrap();
        let error = commit_plan(
            &gp,
            &game,
            plan,
            DeployRecord {
                mod_name: "ManagedVoice".into(),
                ..Default::default()
            },
            None,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("does not match") && error.contains("executable"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), before);
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn inspection_resolver_uses_authenticated_active_pristine_and_tracks_hotfix_drift() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let generation = write_test_game_executable(&game, b"inspection-generation");
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            generation,
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                observe_test_voice_archive(&live, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();
        let bundle_dir = dir.path().join("bundle");
        write_bundle(&bundle_dir, &bundle).unwrap();
        deploy(&bundle_dir, &game).unwrap();
        let record_before = std::fs::read(record_path(&game)).unwrap();

        let pristine = resolve_pristine_voice_archive_for_inspection(&game, "German.zip").unwrap();
        assert!(same_path(
            &pristine.path,
            &bak_path(&live).display().to_string()
        ));
        assert!(!pristine.drifted);
        assert_eq!(
            read_test_zip_entry(&pristine.path, "NPC/Hero/hello.ogg").unwrap(),
            original
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), record_before);

        let hotfix = test_ogg(22_050);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &hotfix)]);
        let refreshed = resolve_pristine_voice_archive_for_inspection(&game, "German.zip").unwrap();
        assert!(same_path(&refreshed.path, &live.display().to_string()));
        assert!(refreshed.drifted);
        assert_eq!(
            read_test_zip_entry(&refreshed.path, "NPC/Hero/hello.ogg").unwrap(),
            hotfix
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), record_before);

        std::fs::remove_file(bak_path(&live)).unwrap();
        let error = resolve_pristine_voice_archive_for_inspection(&game, "German.zip")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("authenticated pristine backup is missing"),
            "unexpected error: {error}"
        );

        let mut interrupted: DeployRecord = serde_json::from_slice(&record_before).unwrap();
        interrupted.phase = DeployPhase::RecoveryRequired;
        std::fs::write(
            record_path(&game),
            serde_json::to_vec_pretty(&interrupted).unwrap(),
        )
        .unwrap();
        let error = resolve_pristine_voice_archive_for_inspection(&game, "German.zip")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("RECOVERY_REQUIRED"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn in_install_voice_over_link_is_rejected_for_inspection_and_deploy() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let story = game.join("G1R/Story");
        let linked_voice_over = story.join("VoiceOver");
        let redirected_voice_over = game.join("redirected-voice-over");
        std::fs::create_dir_all(&story).unwrap();
        std::fs::create_dir(&redirected_voice_over).unwrap();
        let redirected_live = redirected_voice_over.join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&redirected_live, &[("NPC/Hero/hello.ogg", &original)]);
        if !make_test_dir_link(&redirected_voice_over, &linked_voice_over) {
            return;
        }

        let inspect_error = resolve_pristine_voice_archive_for_inspection(&game, "German.zip")
            .unwrap_err()
            .to_string();
        assert!(
            inspect_error.contains("symbolic link")
                || inspect_error.contains("reparse point")
                || inspect_error.contains("reparse"),
            "unexpected inspection error: {inspect_error}"
        );

        let generation = write_test_game_executable(&game, b"linked-voice-over-generation");
        let bundle = build_sealed_voice_bundle(
            test_sealed_voice_meta(),
            generation,
            vec![test_sealed_voice_replace(
                "NPC/Hero/hello.ogg",
                test_ogg(32_000),
                observe_test_voice_archive(&redirected_live, "NPC/Hero/hello.ogg"),
            )],
        )
        .unwrap();
        let bundle_dir = dir.path().join("linked-voice-bundle");
        write_bundle(&bundle_dir, &bundle).unwrap();
        let archive_before = std::fs::read(&redirected_live).unwrap();
        let deploy_error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            deploy_error.contains("symbolic link")
                || deploy_error.contains("reparse point")
                || deploy_error.contains("reparse"),
            "unexpected deployment error: {deploy_error}"
        );
        assert_eq!(std::fs::read(&redirected_live).unwrap(), archive_before);
        assert!(!bak_path(&redirected_live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn format3_loadout_rejects_conflicting_executable_generations() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&archive, &[("NPC/Hero/hello.ogg", &original)]);
        let observation = observe_test_voice_archive(&archive, "NPC/Hero/hello.ogg");
        let make_bundle = |name: &str, generation: VoiceExecutableGenerationSeal| {
            build_sealed_voice_bundle(
                ModMeta {
                    name: name.into(),
                    version: String::new(),
                    author: String::new(),
                },
                generation,
                vec![test_sealed_voice_replace(
                    "NPC/Hero/hello.ogg",
                    test_ogg(32_000),
                    observation.clone(),
                )],
            )
            .unwrap()
        };
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        write_bundle(&first, &make_bundle("First", test_voice_generation())).unwrap();
        let other_generation = VoiceExecutableGenerationSeal {
            byte_len: 5,
            sha256: format!("{:x}", Sha256::digest(b"other")),
        };
        write_bundle(&second, &make_bundle("Second", other_generation)).unwrap();

        let mut pending = PendingVoiceEdits::new();
        let mut order = 0;
        merge_voice_component(&first, "voice", &mut pending, &mut order).unwrap();
        let error = merge_voice_component(&second, "voice", &mut pending, &mut order)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("conflicting executable generations"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn voice_bundle_inputs_are_rejected_by_length_before_unbounded_reads() {
        let dir = tempfile::tempdir().unwrap();

        // The build path used to `fs::read` first and only then ask gore-vo to enforce this limit.
        // A logical oversized file now fails from metadata without allocating its full length.
        let oversized_ogg = dir.path().join("oversized.ogg");
        std::fs::File::create(&oversized_ogg)
            .unwrap()
            .set_len(gore_vo::Limits::default().max_ogg_bytes as u64 + 1)
            .unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "BoundedVoice".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![VoiceArchiveEdit {
                archive: "German.zip".into(),
                op: VoicePatchOp::Add,
                archive_path: "GORE/oversized.ogg".into(),
                ogg_path: oversized_ogg.display().to_string(),
                observation: None,
            }],
        };
        let error = build_bundle(&spec).err().expect("oversized Ogg must fail");
        assert!(
            error.to_string().contains("exceeds the") && error.to_string().contains("voice Ogg"),
            "unexpected error: {error}"
        );

        // Installed bundle manifests have their own finite bound and fail before JSON allocation.
        let bundle = dir.path().join("bundle");
        let voice = bundle.join("voice");
        std::fs::create_dir_all(&voice).unwrap();
        std::fs::File::create(voice.join("manifest.json"))
            .unwrap()
            .set_len(MAX_VOICE_MANIFEST_BYTES + 1)
            .unwrap();
        let mut pending = PendingVoiceEdits::new();
        let mut next_order = 0;
        let error = merge_voice_component(&bundle, "voice", &mut pending, &mut next_order)
            .expect_err("oversized manifest must fail");
        assert!(
            error.to_string().contains("voice manifest")
                && error.to_string().contains("exceeds the"),
            "unexpected error: {error}"
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn voice_merge_enforces_deployment_wide_budget_before_payload_allocation() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("bundle");
        std::fs::create_dir_all(bundle.join("voice")).unwrap();
        std::fs::write(bundle.join("voice/payload.ogg"), test_ogg(44_100)).unwrap();
        std::fs::write(
            bundle.join("voice/manifest.json"),
            serde_json::to_vec(&VoicePatchManifest {
                format: 1,
                executable_generation: None,
                edits: vec![VoicePatchEntry {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Add,
                    archive_path: "GORE/new.ogg".into(),
                    ogg: "voice/payload.ogg".into(),
                    observation: None,
                    payload_seal: None,
                }],
            })
            .unwrap(),
        )
        .unwrap();
        let mut pending = PendingVoiceEdits::new();
        pending.retained_ogg_bytes = MAX_PENDING_VOICE_OGG_BYTES - 1;
        let mut next_order = 0;

        let error =
            merge_voice_component(&bundle, "voice", &mut pending, &mut next_order).unwrap_err();
        assert!(
            error.to_string().contains("voice Ogg payload")
                && error.to_string().contains("1-byte limit"),
            "unexpected error: {error}"
        );
        assert!(pending.edits.is_empty());
        assert_eq!(pending.retained_ogg_bytes, MAX_PENDING_VOICE_OGG_BYTES - 1);
        assert_eq!(next_order, 0);
    }

    #[test]
    fn streaming_hash_and_atomic_copy_match_in_memory_hash_without_full_file_buffers() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("German.zip.gore-bak");
        let destination = dir.path().join("German.zip");
        let mut bytes = Vec::with_capacity(2 * 1024 * 1024 + 37);
        for index in 0..(2 * 1024 * 1024 + 37) {
            bytes.push((index % 251) as u8);
        }
        std::fs::write(&source, &bytes).unwrap();
        std::fs::write(&destination, b"previous deployed bytes").unwrap();

        assert_eq!(content_hash_file(&source).unwrap(), content_hash(&bytes));
        atomic_copy(&source, &destination).unwrap();
        assert!(files_equal(&source, &destination).unwrap());
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        assert!(!PathBuf::from(format!("{}.gore-tmp", destination.display())).exists());

        let mut hashes = BTreeMap::new();
        hashes.insert(destination.display().to_string(), content_hash(&bytes));
        let record = DeployRecord {
            backups: vec![(
                destination.display().to_string(),
                source.display().to_string(),
                false,
            )],
            deployed_hashes: hashes,
            backup_hashes: BTreeMap::from([(
                source.display().to_string(),
                sha256_file(&source).unwrap(),
            )]),
            ..Default::default()
        };
        assert!(safe_to_restore(&destination.display().to_string(), &record).unwrap());
        std::fs::OpenOptions::new()
            .write(true)
            .open(&destination)
            .unwrap()
            .write_all(b"changed")
            .unwrap();
        assert!(!safe_to_restore(&destination.display().to_string(), &record).unwrap());
    }

    #[test]
    fn file_hash_helpers_fit_on_a_small_windows_style_stack() {
        use sha2::Digest as _;

        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("hash-source.bin");
        let mut bytes = Vec::with_capacity(2 * 1024 * 1024 + 37);
        for index in 0..(2 * 1024 * 1024 + 37) {
            bytes.push((index % 251) as u8);
        }
        std::fs::write(&source, &bytes).unwrap();
        let expected_content = content_hash(&bytes);
        let expected_sha256 = format!("sha256:{:x}", sha2::Sha256::digest(&bytes));

        let worker = std::thread::Builder::new()
            .name("gore-mod-small-stack-hash".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                assert_eq!(content_hash_file(&source).unwrap(), expected_content);
                assert_eq!(sha256_file(&source).unwrap(), expected_sha256);
            })
            .unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn voice_archive_name_is_portable_and_not_a_windows_device_or_ads() {
        assert!(is_safe_voice_archive("german_new.zip"));
        for unsafe_name in [
            "CON.zip",
            "con.ZIP",
            "NUL.zip",
            "COM1.zip",
            "COM¹.zip",
            "LPT9.zip",
            "LPT².zip",
            "name:stream.zip",
            "trailing.zip.",
            "trailing.zip ",
            "folder/name.zip",
            "folder\\name.zip",
        ] {
            assert!(
                !is_safe_voice_archive(unsafe_name),
                "unsafe archive was accepted: {unsafe_name:?}"
            );
        }
        assert!(!is_safe_voice_member("NPC/LPT³.ogg"));
    }

    #[test]
    fn sealed_voice_replace_deploys_against_the_exact_observed_archive() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let pristine = std::fs::read(&live).unwrap();
        let observation = observe_test_voice_archive(&live, "NPC/Hero/hello.ogg");
        let replacement = dir.path().join("replacement.ogg");
        std::fs::write(&replacement, test_ogg(32_000)).unwrap();
        let bundle_dir = dir.path().join("bundle");
        write_bundle(
            &bundle_dir,
            &build_bundle(&test_voice_replace_spec(
                "SealedReplace",
                &replacement,
                Some(observation),
            ))
            .unwrap(),
        )
        .unwrap();

        deploy(&bundle_dir, &game).unwrap();
        assert_eq!(
            read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            std::fs::read(&replacement).unwrap()
        );
        assert_eq!(std::fs::read(bak_path(&live)).unwrap(), pristine);
        undeploy(&game).unwrap();
    }

    #[test]
    fn sealed_voice_rejects_a_hotfixed_archive_non_destructively() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let observation = observe_test_voice_archive(&live, "NPC/Hero/hello.ogg");
        let replacement = dir.path().join("replacement.ogg");
        std::fs::write(&replacement, test_ogg(32_000)).unwrap();
        let bundle_dir = dir.path().join("bundle");
        write_bundle(
            &bundle_dir,
            &build_bundle(&test_voice_replace_spec(
                "HotfixGuard",
                &replacement,
                Some(observation),
            ))
            .unwrap(),
        )
        .unwrap();

        let hotfixed = test_ogg(22_050);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &hotfixed)]);
        let before = std::fs::read(&live).unwrap();
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("archive changed"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), before);
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn sealed_voice_rejects_member_metadata_mismatch_non_destructively() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let mut observation = observe_test_voice_archive(&live, "NPC/Hero/hello.ogg");
        match &mut observation.member_proof {
            VoiceMemberProof::Present { crc32, .. } => *crc32 ^= 1,
            VoiceMemberProof::Absent => panic!("test member must be present"),
        }
        let replacement = dir.path().join("replacement.ogg");
        std::fs::write(&replacement, test_ogg(32_000)).unwrap();
        let bundle_dir = dir.path().join("bundle");
        write_bundle(
            &bundle_dir,
            &build_bundle(&test_voice_replace_spec(
                "MemberGuard",
                &replacement,
                Some(observation),
            ))
            .unwrap(),
        )
        .unwrap();

        let before = std::fs::read(&live).unwrap();
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("member proof mismatch") && error.contains("metadata changed"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), before);
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn sealed_voice_rejects_a_false_absent_proof_non_destructively() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let existing = test_ogg(16_000);
        write_test_voice_zip(&live, &[("GORE/new.ogg", &existing)]);
        let mut observation = observe_test_voice_archive(&live, "GORE/new.ogg");
        observation.member_proof = VoiceMemberProof::Absent;
        let addition = dir.path().join("addition.ogg");
        std::fs::write(&addition, test_ogg(32_000)).unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "AbsentGuard".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![VoiceArchiveEdit {
                archive: "German.zip".into(),
                op: VoicePatchOp::Add,
                archive_path: "GORE/new.ogg".into(),
                ogg_path: addition.display().to_string(),
                observation: Some(observation),
            }],
        };
        let bundle_dir = dir.path().join("bundle");
        write_bundle(&bundle_dir, &build_bundle(&spec).unwrap()).unwrap();

        let before = std::fs::read(&live).unwrap();
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("member proof mismatch") && error.contains("expected to be absent"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), before);
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn voice_deploy_rewrites_from_pristine_preserves_backup_and_fails_non_destructively() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let live = game.join("G1R/Story/VoiceOver/German.zip");
        let original = test_ogg(16_000);
        write_test_voice_zip(
            &live,
            &[
                ("NPC/Hero/hello.ogg", &original),
                ("metadata.txt", b"untouched"),
            ],
        );
        let pristine_zip = std::fs::read(&live).unwrap();

        let replacement_one = dir.path().join("one.ogg");
        let replacement_two = dir.path().join("two.ogg");
        let added = dir.path().join("added.ogg");
        std::fs::write(&replacement_one, test_ogg(32_000)).unwrap();
        std::fs::write(&replacement_two, test_ogg(44_100)).unwrap();
        std::fs::write(&added, test_ogg(48_000)).unwrap();

        let voice_spec = |name: &str, voice: Vec<VoiceArchiveEdit>| BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice,
        };
        let first_spec = voice_spec(
            "VoiceOne",
            vec![
                // Same logical target with losing spelling/op. The later entry below must retain
                // its exact spelling AND Replace operation; keeping this Add would fail because
                // the member already exists case-insensitively in the pristine archive.
                VoiceArchiveEdit {
                    archive: "german.ZIP".into(),
                    op: VoicePatchOp::Add,
                    archive_path: "npc/hero/HELLO.OGG".into(),
                    ogg_path: added.display().to_string(),
                    observation: None,
                },
                VoiceArchiveEdit {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Replace,
                    archive_path: "NPC/Hero/hello.ogg".into(),
                    ogg_path: replacement_one.display().to_string(),
                    observation: None,
                },
                VoiceArchiveEdit {
                    archive: "German.zip".into(),
                    op: VoicePatchOp::Add,
                    archive_path: "GORE/added.ogg".into(),
                    ogg_path: added.display().to_string(),
                    observation: None,
                },
            ],
        );
        let first_dir = dir.path().join("bundle-one");
        write_bundle(&first_dir, &build_bundle(&first_spec).unwrap()).unwrap();
        let first_record = deploy(&first_dir, &game).unwrap();
        assert_eq!(
            read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            std::fs::read(&replacement_one).unwrap()
        );
        assert!(read_test_zip_entry(&live, "GORE/added.ogg").is_some());
        let backup = bak_path(&live);
        assert_eq!(std::fs::read(&backup).unwrap(), pristine_zip);
        assert_eq!(first_record.deployed_hashes.len(), 1);
        assert!(first_record
            .deployed_hashes
            .values()
            .any(|hash| hash == &content_hash(&std::fs::read(&live).unwrap())));

        // A missing archive is a hard prepare error. It must leave the active deployment and its
        // record byte-for-byte intact rather than silently producing a partial voice patch.
        let before_failed_live = std::fs::read(&live).unwrap();
        let before_failed_record = std::fs::read(record_path(&game)).unwrap();
        let missing_spec = voice_spec(
            "Missing",
            vec![VoiceArchiveEdit {
                archive: "Missing.zip".into(),
                op: VoicePatchOp::Add,
                archive_path: "GORE/new.ogg".into(),
                ogg_path: added.display().to_string(),
                observation: None,
            }],
        );
        let missing_dir = dir.path().join("bundle-missing");
        write_bundle(&missing_dir, &build_bundle(&missing_spec).unwrap()).unwrap();
        assert!(deploy(&missing_dir, &game).is_err());
        assert_eq!(std::fs::read(&live).unwrap(), before_failed_live);
        assert_eq!(
            std::fs::read(record_path(&game)).unwrap(),
            before_failed_record
        );

        // Redeploy contains no add. It must rebuild from the prior pristine backup, so the first
        // mod's added member disappears and the original backup is never overwritten.
        let second_spec = voice_spec(
            "VoiceTwo",
            vec![VoiceArchiveEdit {
                archive: "German.zip".into(),
                op: VoicePatchOp::Replace,
                archive_path: "NPC/Hero/hello.ogg".into(),
                ogg_path: replacement_two.display().to_string(),
                observation: None,
            }],
        );
        let second_dir = dir.path().join("bundle-two");
        write_bundle(&second_dir, &build_bundle(&second_spec).unwrap()).unwrap();
        deploy(&second_dir, &game).unwrap();
        assert_eq!(
            read_test_zip_entry(&live, "NPC/Hero/hello.ogg").unwrap(),
            std::fs::read(&replacement_two).unwrap()
        );
        assert!(read_test_zip_entry(&live, "GORE/added.ogg").is_none());
        assert_eq!(std::fs::read(&backup).unwrap(), pristine_zip);

        undeploy(&game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), pristine_zip);
        assert!(!backup.exists());
    }

    #[test]
    fn voice_deploy_rejects_a_differing_untracked_backup() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/german_new.zip");
        let original = test_ogg(16_000);
        let stale = test_ogg(22_050);
        write_test_voice_zip(&live, &[("NPC/Hero/hello.ogg", &original)]);
        let backup = bak_path(&live);
        write_test_voice_zip(&backup, &[("NPC/Hero/hello.ogg", &stale)]);
        let before_live = std::fs::read(&live).unwrap();
        let before_backup = std::fs::read(&backup).unwrap();

        let replacement = dir.path().join("replacement.ogg");
        std::fs::write(&replacement, test_ogg(32_000)).unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "AmbiguousBackup".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![VoiceArchiveEdit {
                archive: "german_new.zip".into(),
                op: VoicePatchOp::Replace,
                archive_path: "NPC/Hero/hello.ogg".into(),
                ogg_path: replacement.display().to_string(),
                observation: None,
            }],
        };
        let bundle = dir.path().join("bundle");
        write_bundle(&bundle, &build_bundle(&spec).unwrap()).unwrap();
        let error = deploy(&bundle, &game).unwrap_err().to_string();
        assert!(
            error.contains("untracked backup"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), before_live);
        assert_eq!(std::fs::read(&backup).unwrap(), before_backup);
        assert!(!record_path(&game).exists());
    }

    /// prepare() must reject a manifest whose op is neither add nor edit, naming the module.
    #[test]
    fn prepare_rejects_bad_script_op() {
        let dir = std::env::temp_dir().join("gore-mod-as-prep-badop");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/0_M.cache"), b"x").unwrap();
        let entries = vec![ScriptEntry {
            op: "nope".into(),
            module: "M".into(),
            mini: "scripts/0_M.cache".into(),
        }];
        std::fs::write(
            dir.join("scripts/manifest.json"),
            serde_json::to_vec(&entries).unwrap(),
        )
        .unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta {
                name: "M".into(),
                version: String::new(),
                author: String::new(),
            },
            components: vec![Component::AngelScriptPatch {
                path: "scripts".into(),
            }],
        };
        // A game dir whose script cache file exists (content irrelevant — op is rejected first).
        let game = dir.join("game");
        let script_dir = game.join("G1R/Script");
        std::fs::create_dir_all(&script_dir).unwrap();
        std::fs::write(script_dir.join("PrecompiledScript_Shipping.Cache"), b"base").unwrap();
        let gp = resolve_game_paths(&game);
        let err = prepare(&dir, &manifest, &gp, None).unwrap_err();
        assert!(err.to_string().contains("invalid script op"), "got: {err}");
    }

    /// prepare() must error clearly when the game has no script cache.
    #[test]
    fn prepare_errors_when_no_script_cache() {
        let dir = std::env::temp_dir().join("gore-mod-as-prep-nocache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/0_M.cache"), b"x").unwrap();
        let entries = vec![ScriptEntry {
            op: "add".into(),
            module: "M".into(),
            mini: "scripts/0_M.cache".into(),
        }];
        std::fs::write(
            dir.join("scripts/manifest.json"),
            serde_json::to_vec(&entries).unwrap(),
        )
        .unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta {
                name: "M".into(),
                version: String::new(),
                author: String::new(),
            },
            components: vec![Component::AngelScriptPatch {
                path: "scripts".into(),
            }],
        };
        let gp = resolve_game_paths(&dir.join("empty-game"));
        let err = prepare(&dir, &manifest, &gp, None).unwrap_err();
        assert!(
            err.to_string().contains("script cache not found"),
            "got: {err}"
        );
    }

    /// Full splice against a real game install. Run with:
    ///   GORE_TEST_GAME="C:/.../Gothic 1 Remake"  cargo test -p gore-mod -- --ignored real_script_deploy
    #[test]
    #[ignore]
    fn real_script_deploy_add_roundtrips() {
        use gore_as::cache::walk_modules::module_count;
        let Ok(game) = std::env::var("GORE_TEST_GAME") else {
            return;
        };
        let game = std::path::PathBuf::from(game);
        let gp = resolve_game_paths(&game);
        assert!(
            gp.script_cache.exists(),
            "no script cache at {}",
            gp.script_cache.display()
        );
        // A mini-cache produced by `gore as extract`/`script_compile` (Task 11). Provide its path:
        let Ok(mini) = std::env::var("GORE_TEST_MINI") else {
            eprintln!("set GORE_TEST_MINI to a 1-module mini-cache to run the splice");
            return;
        };
        let dir = std::env::temp_dir().join("gore-mod-as-real");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "RealAsMod".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![ScriptModule {
                op: "add".into(),
                module_name: "ignored_for_add".into(),
                mini_cache: mini,
            }],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        write_bundle(&dir, &bundle).unwrap();
        let manifest = bundle.manifest;
        let before = module_count(&std::fs::read(&gp.script_cache).unwrap());
        let plan = prepare(&dir, &manifest, &gp, None).unwrap();
        let (_, spliced) = plan.writes.last().unwrap();
        assert_eq!(
            module_count(spliced),
            before + 1,
            "splice should add exactly one module"
        );
    }

    #[test]
    fn retire_deletes_prev_texture_triplets_not_in_new_plan() {
        // A prior deploy left a triplet in ~mods; the new deploy has no (or a differently-named)
        // texture component. retire_leftovers must delete the stale triplet + prune it from the
        // record so it neither lingers mounted nor escapes a later undeploy.
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        let dir = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&dir).unwrap();
        let old: Vec<String> = [
            "zzz_Old_tex_P.utoc",
            "zzz_Old_tex_P.ucas",
            "zzz_Old_tex_P.pak",
        ]
        .iter()
        .map(|n| {
            let p = dir.join(n);
            std::fs::write(&p, b"x").unwrap();
            p.display().to_string()
        })
        .collect();
        let hashes: BTreeMap<String, String> = old
            .iter()
            .map(|path| (path.clone(), sha256_file(Path::new(path)).unwrap()))
            .collect();
        let prev = DeployRecord {
            mod_name: "Old".into(),
            texture_triplets: old.clone(),
            deployed_hashes: hashes.clone(),
            ..Default::default()
        };
        // The new record was pre-seeded with the prev triplets (as deploy() step (b) does).
        let mut record = DeployRecord {
            mod_name: "New".into(),
            texture_triplets: old.clone(),
            deployed_hashes: hashes,
            ..Default::default()
        };
        // New plan has NO texture triplets (e.g. a non-texture mod) -> all prev ones are stale.
        let plan = DeployPlan::default();
        let changed = retire_leftovers(&game, &[], Some(&prev), &plan, &mut record).unwrap();
        assert!(changed);
        for f in &old {
            assert!(
                !std::path::Path::new(f).exists(),
                "stale triplet not deleted: {f}"
            );
        }
        assert!(
            record.texture_triplets.is_empty(),
            "stale triplets not pruned from record"
        );
    }

    #[test]
    fn undeploy_removes_recorded_texture_triplets() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let files: Vec<String> = ["zzz_M_tex_P.utoc", "zzz_M_tex_P.ucas", "zzz_M_tex_P.pak"]
            .iter()
            .map(|n| {
                let p = mods.join(n);
                std::fs::write(&p, b"x").unwrap();
                p.display().to_string()
            })
            .collect();
        let rec = DeployRecord {
            mod_name: "M".into(),
            texture_triplets: files.clone(),
            deployed_hashes: files
                .iter()
                .map(|path| (path.clone(), sha256_file(Path::new(path)).unwrap()))
                .collect(),
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();
        undeploy(&game).unwrap();
        for f in &files {
            assert!(
                !std::path::Path::new(f).exists(),
                "triplet not removed: {f}"
            );
        }
    }

    #[test]
    fn undeploy_persists_backup_progress_before_a_later_missing_backup() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let restored_live = game.join("G1R/Story/VoiceOver/restored.zip");
        let restored_bak = bak_path(&restored_live);
        let missing_live = game.join("G1R/Story/VoiceOver/missing-backup.zip");
        let missing_bak = bak_path(&missing_live);
        std::fs::create_dir_all(restored_live.parent().unwrap()).unwrap();
        std::fs::write(&restored_live, b"modded-one").unwrap();
        std::fs::write(&restored_bak, b"pristine-one").unwrap();
        std::fs::write(&missing_live, b"modded-two").unwrap();

        let restored_key = restored_live.display().to_string();
        let missing_key = missing_live.display().to_string();
        let record = DeployRecord {
            backups: vec![
                (
                    restored_key.clone(),
                    restored_bak.display().to_string(),
                    true,
                ),
                (missing_key.clone(), missing_bak.display().to_string(), true),
            ],
            deployed_hashes: BTreeMap::from([
                (restored_key, content_hash(b"modded-one")),
                (missing_key.clone(), content_hash(b"modded-two")),
            ]),
            backup_hashes: BTreeMap::from([(
                restored_bak.display().to_string(),
                sha256_file(&restored_bak).unwrap(),
            )]),
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&record).unwrap()).unwrap();

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("could not be restored"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&restored_live).unwrap(), b"pristine-one");
        assert!(!restored_bak.exists());

        let pending: DeployRecord =
            serde_json::from_slice(&std::fs::read(record_path(&game)).unwrap()).unwrap();
        assert_eq!(pending.phase, DeployPhase::RecoveryRequired);
        assert_eq!(pending.backups.len(), 1);
        assert!(same_path_s(&pending.backups[0].0, &missing_key));
        assert!(pending
            .deployed_hashes
            .keys()
            .all(|key| same_path_s(key, &missing_key)));
    }

    #[test]
    fn undeploy_record_marker_failure_precedes_every_live_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let live = game.join("G1R/Story/VoiceOver/target.zip");
        let bak = bak_path(&live);
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"modded").unwrap();
        std::fs::write(&bak, b"pristine").unwrap();
        let record = DeployRecord {
            backups: vec![(live.display().to_string(), bak.display().to_string(), true)],
            ..Default::default()
        };
        let record_bytes = serde_json::to_vec(&record).unwrap();
        let path = record_path(&game);
        std::fs::write(&path, &record_bytes).unwrap();
        fail_next_atomic_write(&path);

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("before cleanup")
                && error.to_string().contains("injected atomic-write"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), b"modded");
        assert_eq!(std::fs::read(&bak).unwrap(), b"pristine");
        assert_eq!(std::fs::read(&path).unwrap(), record_bytes);
    }

    #[test]
    fn undeploy_backup_delete_failure_is_returned_and_durably_retracked() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let live = game.join("G1R/Story/VoiceOver/target.zip");
        let bak = bak_path(&live);
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"modded").unwrap();
        std::fs::write(&bak, b"pristine").unwrap();
        let live_key = live.display().to_string();
        let record = DeployRecord {
            backups: vec![(live_key.clone(), bak.display().to_string(), true)],
            deployed_hashes: BTreeMap::from([(live_key, content_hash(b"modded"))]),
            backup_hashes: BTreeMap::from([(
                bak.display().to_string(),
                sha256_file(&bak).unwrap(),
            )]),
            ..Default::default()
        };
        let path = record_path(&game);
        std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        fail_next_durable_remove(&bak);

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("injected durable-remove"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), b"pristine");
        assert_eq!(std::fs::read(&bak).unwrap(), b"pristine");
        let retry: DeployRecord = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(retry.phase, DeployPhase::RecoveryRequired);
        assert_eq!(retry.backups.len(), 1);
        assert_eq!(retry.deployed_hashes.len(), 1);

        undeploy(&game).unwrap();
        assert!(!path.exists());
        assert!(!bak.exists());
        assert_eq!(std::fs::read(&live).unwrap(), b"pristine");
    }

    #[test]
    fn undeploy_final_record_delete_failure_is_propagated_and_retryable() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let path = record_path(&game);
        std::fs::write(&path, serde_json::to_vec(&DeployRecord::default()).unwrap()).unwrap();
        fail_next_durable_remove(&path);

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("injected durable-remove"),
            "unexpected error: {error}"
        );
        let retry: DeployRecord = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(retry.phase, DeployPhase::RecoveryRequired);

        undeploy(&game).unwrap();
        assert!(!path.exists());
    }

    /// A record written by a pre-manager build (only the original fields) must still parse,
    /// with the v2 fields defaulted: no owner (legacy/studio), empty loadout/dirs/paks.
    #[test]
    fn legacy_record_json_parses_owner_empty() {
        let json = r#"{
            "mod_name": "OldMod",
            "ue4ss_mod_dir": "C:/game/G1R/Binaries/Win64/ue4ss/Mods/OldMod",
            "backups": [["C:/game/G1R/Story/Cache/A.lcache", "C:/game/G1R/Story/Cache/A.lcache.gore-bak", true]]
        }"#;
        let rec: DeployRecord = serde_json::from_str(json).unwrap();
        assert_eq!(rec.mod_name, "OldMod");
        assert_eq!(
            rec.ue4ss_mod_dir.as_deref(),
            Some("C:/game/G1R/Binaries/Win64/ue4ss/Mods/OldMod")
        );
        assert_eq!(rec.backups.len(), 1);
        assert_eq!(rec.owner, "");
        assert!(rec.loadout.is_empty());
        assert!(rec.ue4ss_mod_dirs.is_empty());
        assert!(rec.managed_paks.is_empty());
        assert!(rec.ue4ss_tree_fingerprints.is_empty());
        assert!(rec.recovery_file_hashes.is_empty());
        assert!(rec.recovery_tree_fingerprints.is_empty());
        assert_eq!(rec.phase, DeployPhase::Applied);
    }

    /// Undeploying a v2 record must delete its manager-installed pak files and remove each
    /// of its UE4SS mod dirs, exactly like the single-mod fields.
    #[test]
    fn restore_removes_managed_paks_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let pak = mods.join("zzz_mgr_A_P.pak");
        std::fs::write(&pak, b"pak").unwrap();
        let ue4ss_dir = game.join("G1R/Binaries/Win64/ue4ss/Mods/MgrModA");
        std::fs::create_dir_all(&ue4ss_dir).unwrap();
        std::fs::write(ue4ss_dir.join("enabled.txt"), b"").unwrap();
        let rec = DeployRecord {
            mod_name: "manager".into(),
            owner: "manager".into(),
            ue4ss_mod_dirs: vec![ue4ss_dir.display().to_string()],
            managed_paks: vec![pak.display().to_string()],
            deployed_hashes: BTreeMap::from([(
                pak.display().to_string(),
                sha256_file(&pak).unwrap(),
            )]),
            ue4ss_tree_fingerprints: BTreeMap::from([(
                ue4ss_dir.display().to_string(),
                tree_fingerprint(&ue4ss_dir).unwrap(),
            )]),
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();
        let restored = undeploy(&game).unwrap();
        assert!(
            restored.is_some(),
            "undeploy should report the restored record"
        );
        assert!(!pak.exists(), "managed pak not removed");
        assert!(!ue4ss_dir.exists(), "managed ue4ss dir not removed");
    }

    #[test]
    fn rel_path_with_dotdot_is_unsafe() {
        assert!(!is_safe_rel_path("G1R/Content/../../../Foo"));
        assert!(is_safe_rel_path("G1R/Content/UI/Textures/T_X"));
    }

    #[test]
    fn bundle_paths_reject_portable_windows_aliases_and_ads() {
        for unsafe_component in [
            "name:stream",
            "trailing.",
            "trailing ",
            "CON",
            "NUL.txt",
            "COM1.lua",
            "COM¹.lua",
            "LPT².bank",
            "LPT³",
        ] {
            assert!(
                !is_safe_mod_name(unsafe_component),
                "unsafe component accepted: {unsafe_component:?}"
            );
            assert!(
                !is_safe_filename(unsafe_component),
                "unsafe filename accepted: {unsafe_component:?}"
            );
            assert!(
                !is_safe_rel_path(&format!("payload/{unsafe_component}")),
                "unsafe relative component accepted: {unsafe_component:?}"
            );
        }
        assert!(!is_safe_rel_path("payload\\file.bin"));
        assert!(is_safe_mod_name("Normal-Mod_1"));
        assert!(is_safe_rel_path("payload/sub/file.bin"));
    }

    #[test]
    fn deploy_bounds_root_manifest_and_rejects_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("bundle");
        let game = dir.path().join("game");
        std::fs::create_dir_all(&bundle).unwrap();
        let manifest_path = bundle.join("gore-mod.json");
        std::fs::File::create(&manifest_path)
            .unwrap()
            .set_len(MAX_BUNDLE_MANIFEST_BYTES + 1)
            .unwrap();
        let oversized = deploy(&bundle, &game).unwrap_err();
        assert!(
            oversized.to_string().contains("gore-mod.json")
                && oversized.to_string().contains("exceeds the"),
            "unexpected error: {oversized}"
        );

        std::fs::write(
            &manifest_path,
            serde_json::to_vec(&serde_json::json!({
                "format": 3,
                "mod": {"name": "Future", "version": "", "author": ""},
                "components": []
            }))
            .unwrap(),
        )
        .unwrap();
        let future = deploy(&bundle, &game).unwrap_err();
        assert!(
            future
                .to_string()
                .contains("unsupported gore-mod manifest format 3 (want 1 or 2)"),
            "unexpected error: {future}"
        );
        assert!(
            !game.exists(),
            "an unknown format must fail before resolving or writing the game tree"
        );
    }

    #[test]
    fn deploy_rejects_format_component_mismatches_before_any_effect() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("bundle");
        let game = dir.path().join("game-must-not-exist");
        std::fs::create_dir_all(&bundle).unwrap();

        let write_manifest = |manifest: &ModManifest| {
            std::fs::write(
                bundle.join("gore-mod.json"),
                serde_json::to_vec_pretty(manifest).unwrap(),
            )
            .unwrap();
        };
        let meta = ModMeta {
            name: "Mismatch".into(),
            version: String::new(),
            author: String::new(),
        };

        write_manifest(&ModManifest {
            format: 1,
            mod_meta: meta.clone(),
            components: vec![Component::PakFilePatch {
                path: "missing-pak-files".into(),
                targets: vec![TEST_CURSOR.into()],
            }],
        });
        let old_format = deploy(&bundle, &game).unwrap_err().to_string();
        assert!(
            old_format.contains("format 1") && old_format.contains("pak_file_patch"),
            "unexpected error: {old_format}"
        );
        assert!(!game.exists());

        write_manifest(&ModManifest {
            format: 2,
            mod_meta: meta,
            components: vec![Component::FilePatch {
                path: "missing-files".into(),
                targets: vec![TEST_CURSOR.into()],
            }],
        });
        let gratuitous_bump = deploy(&bundle, &game).unwrap_err().to_string();
        assert!(
            gratuitous_bump.contains("format 2")
                && gratuitous_bump.contains("requires")
                && gratuitous_bump.contains("pak_file_patch"),
            "unexpected error: {gratuitous_bump}"
        );
        assert!(!game.exists());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn bundle_file_resolver_rejects_root_ancestor_and_final_links() {
        let dir = tempfile::tempdir().unwrap();
        let real_bundle = dir.path().join("real-bundle");
        std::fs::create_dir_all(real_bundle.join("component")).unwrap();
        std::fs::write(real_bundle.join("component/real.json"), b"{}").unwrap();

        let root_link = dir.path().join("bundle-link");
        if !make_test_dir_link(&real_bundle, &root_link) {
            return;
        }
        let root_error = read_safe_bundle_file(
            &root_link,
            Path::new("component/real.json"),
            "component manifest",
            MAX_BUNDLE_MANIFEST_BYTES,
        )
        .unwrap_err();
        assert!(root_error
            .to_string()
            .contains("bundle root must be a real"));

        let outside = dir.path().join("outside.json");
        std::fs::write(&outside, b"{}").unwrap();
        let final_link = real_bundle.join("component/final.json");
        if !make_test_file_link(&outside, &final_link) {
            return;
        }
        let final_error = read_safe_bundle_file(
            &real_bundle,
            Path::new("component/final.json"),
            "component manifest",
            MAX_BUNDLE_MANIFEST_BYTES,
        )
        .unwrap_err();
        assert!(final_error.to_string().contains("symbolic link or reparse"));

        let linked_parent = real_bundle.join("linked-parent");
        if !make_test_dir_link(&real_bundle.join("component"), &linked_parent) {
            return;
        }
        let ancestor_error = read_safe_bundle_file(
            &real_bundle,
            Path::new("linked-parent/real.json"),
            "component manifest",
            MAX_BUNDLE_MANIFEST_BYTES,
        )
        .unwrap_err();
        assert!(ancestor_error
            .to_string()
            .contains("symbolic link or reparse"));
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn ue4ss_tree_resolver_rejects_root_and_nested_links() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("bundle");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("enabled.txt"), b"").unwrap();

        let linked_root = bundle.join("LinkedMod");
        if !make_test_dir_link(&outside, &linked_root) {
            return;
        }
        let root_error =
            resolve_safe_bundle_tree(&bundle, Path::new("LinkedMod"), "UE4SS component")
                .unwrap_err();
        assert!(root_error.to_string().contains("only real directories"));

        let real_mod = bundle.join("RealMod");
        std::fs::create_dir_all(&real_mod).unwrap();
        std::fs::write(real_mod.join("enabled.txt"), b"").unwrap();
        let nested = real_mod.join("nested");
        if !make_test_dir_link(&outside, &nested) {
            return;
        }
        let nested_error =
            resolve_safe_bundle_tree(&bundle, Path::new("RealMod"), "UE4SS component").unwrap_err();
        assert!(nested_error
            .to_string()
            .contains("symbolic link or reparse"));
    }

    /// The Ue4ssLua component must carry the `Class.Field` CDO targets of the spec's
    /// overrides (sorted, deduped) — the mod-manager's conflict-detection contract.
    #[test]
    fn build_bundle_fills_ue4ss_targets() {
        let spec = BuildSpec {
            meta: ModMeta {
                name: "TgtMod".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            // Deliberately unsorted so the sort is observable.
            overrides: vec![
                SingleOverride {
                    class: "ClassB".into(),
                    field: "FieldY".into(),
                    module: "Angelscript".into(),
                    value: OverrideValue::Int(1),
                },
                SingleOverride {
                    class: "ClassA".into(),
                    field: "FieldX".into(),
                    module: "Angelscript".into(),
                    value: OverrideValue::Int(2),
                },
            ],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        let expected = vec!["ClassA.FieldX".to_string(), "ClassB.FieldY".to_string()];
        let Some(Component::Ue4ssLua {
            targets, opaque, ..
        }) = bundle.manifest.components.first()
        else {
            panic!("expected a Ue4ssLua component");
        };
        assert_eq!(targets, &expected);
        assert!(!*opaque);
        assert!(std::str::from_utf8(&bundle.files["gore-mod.json"])
            .unwrap()
            .contains("\"opaque\": false"));
        // And the serialized manifest round-trips them.
        let m: ModManifest = serde_json::from_slice(&bundle.files["gore-mod.json"]).unwrap();
        assert!(matches!(
            m.components.first(),
            Some(Component::Ue4ssLua { targets, .. }) if targets == &expected
        ));
    }

    /// A format-1 manifest written before `targets` existed must still parse, with the
    /// field defaulting to empty.
    #[test]
    fn manifest_v1_json_without_targets_parses() {
        let json = r#"{
            "format": 1,
            "mod": { "name": "OldMod", "version": "", "author": "" },
            "components": [
                { "type": "ue4ss_lua", "name": "OldMod", "path": "ue4ss/OldMod" }
            ]
        }"#;
        let m: ModManifest = serde_json::from_str(json).unwrap();
        assert!(matches!(
            m.components.first(),
            Some(Component::Ue4ssLua {
                name,
                path,
                targets,
                opaque: false,
            }) if name == "OldMod" && path == "ue4ss/OldMod" && targets.is_empty()
        ));
    }

    #[test]
    fn explicit_precise_targetless_lua_manifest_roundtrips() {
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta {
                name: "PreciseEmpty".into(),
                version: String::new(),
                author: String::new(),
            },
            components: vec![Component::Ue4ssLua {
                name: "PreciseEmpty".into(),
                path: "ue4ss/PreciseEmpty".into(),
                targets: vec![],
                opaque: false,
            }],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("\"opaque\":false"), "{json}");
        let parsed: ModManifest = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            parsed.components.as_slice(),
            [Component::Ue4ssLua {
                targets,
                opaque: false,
                ..
            }] if targets.is_empty()
        ));
    }

    /// Old serialized build specs remain compatible through defaulted additive domain fields.
    #[test]
    fn legacy_build_spec_without_voice_defaults_to_empty() {
        let spec: BuildSpec = serde_json::from_str(
            r#"{
                "meta": { "name": "OldSpec", "version": "", "author": "" },
                "overrides": [], "loc_edits": {}, "audio": [], "texture": [], "scripts": []
            }"#,
        )
        .unwrap();
        assert!(spec.dialog_topics.is_empty());
        assert!(spec.voice.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn read_pristine_matches_drift_hash_by_windows_path_identity() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("MixedCaseVoice.zip");
        std::fs::write(&live, b"new-hotfix-live").unwrap();
        let backup = bak_path(&live);
        std::fs::write(&backup, b"old-pre-hotfix-backup").unwrap();
        let mut prev = DeployRecord::default();
        prev.deployed_hashes.insert(
            live.display().to_string().to_ascii_uppercase(),
            content_hash(b"previous-modded-live"),
        );
        prev.backup_hashes.insert(
            backup.display().to_string().to_ascii_uppercase(),
            sha256_file(&backup).unwrap(),
        );

        let (pristine, drifted) =
            read_pristine_bounded(&live, Some(&prev), MAX_PRISTINE_PATCH_BYTES).unwrap();
        assert!(
            drifted,
            "case-only record spelling must not hide live drift"
        );
        assert_eq!(pristine, b"new-hotfix-live");
    }

    #[test]
    fn pristine_selection_is_bounded_and_legacy_differences_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("base.bank");
        std::fs::write(&live, b"12345").unwrap();
        let oversized = read_pristine_bounded(&live, None, 4).unwrap_err();
        assert!(
            oversized.to_string().contains("4-byte limit"),
            "unexpected error: {oversized}"
        );

        std::fs::write(bak_path(&live), b"different-backup").unwrap();
        let legacy = select_pristine_source(&live, None).unwrap_err();
        assert!(
            legacy
                .to_string()
                .contains("unverifiable legacy pristine backup"),
            "unexpected error: {legacy}"
        );

        let mut malformed = DeployRecord::default();
        malformed
            .deployed_hashes
            .insert(live.display().to_string(), "not-a-hash".into());
        malformed.backup_hashes.insert(
            bak_path(&live).display().to_string(),
            sha256_file(&bak_path(&live)).unwrap(),
        );
        let hash_error = select_pristine_source(&live, Some(&malformed)).unwrap_err();
        assert!(
            hash_error
                .to_string()
                .contains("invalid recorded legacy content hash"),
            "unexpected error: {hash_error}"
        );
    }

    #[test]
    fn undeploy_does_not_recreate_a_live_file_removed_by_game_update() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/removed.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = bak_path(&live);
        std::fs::write(&backup, b"old-pristine").unwrap();
        let record = DeployRecord {
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(
                live.display().to_string(),
                content_hash(b"previous-deployed"),
            )]),
            backup_hashes: BTreeMap::from([(
                backup.display().to_string(),
                sha256_file(&backup).unwrap(),
            )]),
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();

        undeploy(&game).unwrap();
        assert!(!live.exists(), "removed game file must not be recreated");
        assert!(!backup.exists(), "stale backup should be retired");
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn recovery_identity_restores_atomic_write_before_final_hash_record() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/interrupted.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = bak_path(&live);
        std::fs::write(&live, b"newly-published").unwrap();
        std::fs::write(&backup, b"pristine").unwrap();
        let live_key = live.display().to_string();
        let record = DeployRecord {
            phase: DeployPhase::RecoveryRequired,
            backups: vec![(live_key.clone(), backup.display().to_string(), true)],
            recovery_file_hashes: BTreeMap::from([(
                live_key,
                vec![content_hash(b"newly-published")],
            )]),
            backup_hashes: BTreeMap::from([(
                backup.display().to_string(),
                sha256_file(&backup).unwrap(),
            )]),
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();

        undeploy(&game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"pristine");
        assert!(!backup.exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn legacy_in_place_record_never_restores_differing_backup_by_path_only() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/legacy.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = bak_path(&live);
        std::fs::write(&live, b"possibly-external-live").unwrap();
        std::fs::write(&backup, b"old-backup").unwrap();
        let record = DeployRecord {
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();

        let error = undeploy(&game).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unverifiable legacy restore backup"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), b"possibly-external-live");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old-backup");
        assert!(record_path(&game).exists());
    }

    /// The single-mod deploy must refuse to clobber a manager-owned deployment; it would
    /// silently retire the whole multi-mod loadout.
    #[test]
    fn deploy_refuses_manager_record() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let rec = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();

        // A minimal valid bundle (one override → one Ue4ssLua component).
        let spec = BuildSpec {
            meta: ModMeta {
                name: "Solo".into(),
                version: String::new(),
                author: String::new(),
            },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ClassA".into(),
                field: "FieldX".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(1),
            }],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle_dir = dir.path().join("bundle");
        write_bundle(&bundle_dir, &build_bundle(&spec).unwrap()).unwrap();

        let err = deploy(&bundle_dir, &game).unwrap_err();
        assert!(err.to_string().contains("manager"), "got: {err}");
        // The guard must trip BEFORE any commit work: the manager record is untouched.
        let after: DeployRecord =
            serde_json::from_slice(&std::fs::read(record_path(&game)).unwrap()).unwrap();
        assert_eq!(after.owner, "manager");
        assert_eq!(after.mod_name, "loadout");
    }

    // ── multi-mod commit_plan hardening ────────────────────────────────────────────────

    /// Create a small real UE4SS mod source dir (with an `enabled.txt`) under `base/<name>`.
    fn make_mod_src(base: &Path, name: &str) -> PathBuf {
        let src = base.join(name);
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("enabled.txt"), b"").unwrap();
        src
    }

    fn prepare_test_plan_identities(plan: &mut DeployPlan) {
        let mut prior = DeployRecord::default();
        for (_, dst) in &plan.ue4ss_dirs {
            if dst.exists() {
                prior
                    .ue4ss_tree_fingerprints
                    .insert(dst.display().to_string(), tree_fingerprint(dst).unwrap());
            }
        }
        for (_, dst) in plan.texture_triplets.iter().chain(plan.managed_paks.iter()) {
            if dst.exists() {
                prior
                    .deployed_hashes
                    .insert(dst.display().to_string(), sha256_file(dst).unwrap());
            }
        }
        prepare_target_identities(plan, Some(&prior)).unwrap();
    }

    fn ue4ss_transaction_dirs(root: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<_> = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with(".gore-ue4ss-")
            })
            .collect();
        paths.sort();
        paths
    }

    #[test]
    fn ue4ss_swap_uses_unique_holders_without_touching_predictable_stale_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("game/G1R/Binaries/Win64/ue4ss");
        let mods = root.join("Mods");
        let dst = mods.join("SameName");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("payload.txt"), b"old").unwrap();
        let src = make_mod_src(dir.path(), "source-new");
        std::fs::write(src.join("payload.txt"), b"new").unwrap();

        // These are the old deterministic names. They may be crash artifacts or belong to another
        // process; this deployment must neither reuse nor delete them.
        let stale_new = root.join("SameName.gore-new");
        let stale_old = root.join("SameName.gore-old");
        std::fs::create_dir_all(&stale_new).unwrap();
        std::fs::create_dir_all(&stale_old).unwrap();
        std::fs::write(stale_new.join("sentinel"), b"new-sentinel").unwrap();
        std::fs::write(stale_old.join("sentinel"), b"old-sentinel").unwrap();

        let mut plan = DeployPlan {
            ue4ss_dirs: vec![(src, dst.clone())],
            ..Default::default()
        };
        prepare_test_plan_identities(&mut plan);
        let mut undo = Undo::default();
        let mut record = DeployRecord::default();
        stage(&plan, &mut record, &mut undo).unwrap();
        apply_writes(&plan, &mut undo).unwrap();
        assert_eq!(std::fs::read(dst.join("payload.txt")).unwrap(), b"new");
        assert_eq!(undo.ue4ss_swaps.len(), 1);
        assert_eq!(
            undo.ue4ss_swaps[0].state,
            Ue4ssSwapState::PromotedReplacement
        );
        assert!(undo.ue4ss_swaps[0].holder.as_ref().unwrap().exists());

        let remaining = undo.discard();
        assert_eq!(remaining.len(), 1);
        let game = dir.path().join("game");
        record.phase = DeployPhase::RecoveryRequired;
        write_record_file(&game, &record).unwrap();
        for path in remaining {
            durable_ue4ss_cleanup(&game, &mut record, &path).unwrap();
        }
        assert!(ue4ss_transaction_dirs(&root).is_empty());
        assert_eq!(
            std::fs::read(stale_new.join("sentinel")).unwrap(),
            b"new-sentinel"
        );
        assert_eq!(
            std::fs::read(stale_old.join("sentinel")).unwrap(),
            b"old-sentinel"
        );
    }

    #[test]
    fn ue4ss_promotion_failure_tracks_old_before_promotion_and_rollback_restores_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("game/G1R/Binaries/Win64/ue4ss");
        let mods = root.join("Mods");
        let dst = mods.join("SameName");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("payload.txt"), b"old").unwrap();
        let src = make_mod_src(dir.path(), "source-new");
        std::fs::write(src.join("payload.txt"), b"new").unwrap();
        let mut plan = DeployPlan {
            ue4ss_dirs: vec![(src, dst.clone())],
            ..Default::default()
        };
        prepare_test_plan_identities(&mut plan);
        let mut undo = Undo::default();
        let mut record = DeployRecord::default();
        stage(&plan, &mut record, &mut undo).unwrap();
        fail_next_ue4ss_promotion(&dst);

        let error = apply_writes(&plan, &mut undo).unwrap_err();
        assert!(
            error.to_string().contains("injected UE4SS promotion"),
            "unexpected error: {error}"
        );
        assert_eq!(
            undo.ue4ss_swaps.len(),
            1,
            "old directory must be rollback-owned before promotion"
        );
        assert_eq!(undo.ue4ss_swaps[0].state, Ue4ssSwapState::OldMoved);
        assert!(!dst.exists());
        let transactions = ue4ss_transaction_dirs(&root);
        assert_eq!(transactions.len(), 1, "only the old holder may remain");
        assert_eq!(
            transactions[0],
            *undo.ue4ss_swaps[0].holder.as_ref().unwrap()
        );

        let failures = undo.rollback();
        assert!(failures.is_empty(), "rollback failures: {failures:?}");
        assert_eq!(std::fs::read(dst.join("payload.txt")).unwrap(), b"old");
        assert!(ue4ss_transaction_dirs(&root).is_empty());
    }

    #[test]
    fn commit_refuses_unowned_existing_ue4ss_directory_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        let dst = mods.join("ManualMod");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("manual.txt"), b"keep me").unwrap();
        let src = make_mod_src(dir.path(), "replacement");
        let plan = DeployPlan {
            ue4ss_dirs: vec![(src, dst.clone())],
            ..Default::default()
        };

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            plan,
            DeployRecord::default(),
            None,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("unowned existing UE4SS"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(dst.join("manual.txt")).unwrap(), b"keep me");
        assert!(!record_path(&game).exists());
        assert!(ue4ss_transaction_dirs(mods.parent().unwrap()).is_empty());
    }

    #[test]
    fn commit_refuses_unowned_existing_managed_pak_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let src = dir.path().join("replacement.pak");
        let dst = game.join("G1R/Content/Paks/~mods/manual.pak");
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&src, b"replacement").unwrap();
        std::fs::write(&dst, b"manual").unwrap();
        let plan = DeployPlan {
            managed_paks: vec![(src, dst.clone())],
            ..Default::default()
        };

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            plan,
            DeployRecord {
                owner: "manager".into(),
                ..Default::default()
            },
            None,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("unowned existing additive"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&dst).unwrap(), b"manual");
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn commit_refuses_unowned_existing_texture_triplet_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let src = dir.path().join("replacement.utoc");
        let dst = game.join("G1R/Content/Paks/~mods/manual.utoc");
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&src, b"replacement").unwrap();
        std::fs::write(&dst, b"manual").unwrap();
        let plan = DeployPlan {
            texture_triplets: vec![(src, dst.clone())],
            ..Default::default()
        };

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            plan,
            DeployRecord::default(),
            None,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("unowned existing additive"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&dst).unwrap(), b"manual");
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn commit_near_revalidation_preserves_racing_additive_replacements() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let dst = mods.join("owned.pak");
        let src = dir.path().join("replacement.pak");
        std::fs::write(&dst, b"previous-owned").unwrap();
        std::fs::write(&src, b"new-deployment").unwrap();
        let prior_record = DeployRecord {
            owner: "manager".into(),
            managed_paks: vec![dst.display().to_string()],
            deployed_hashes: BTreeMap::from([(
                dst.display().to_string(),
                sha256_file(&dst).unwrap(),
            )]),
            ..Default::default()
        };
        write_record_file(&game, &prior_record).unwrap();
        let prior_raw = std::fs::read(record_path(&game)).unwrap();
        let prior = read_record(&game).unwrap().unwrap();
        replace_additive_before_apply(&dst, b"external-race-winner");

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            DeployPlan {
                managed_paks: vec![(src, dst.clone())],
                ..Default::default()
            },
            DeployRecord {
                owner: "manager".into(),
                ..Default::default()
            },
            Some(prior),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("changed before apply"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&dst).unwrap(), b"external-race-winner");
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), prior_raw);
    }

    #[test]
    fn fresh_additive_publication_never_clobbers_a_racing_creator() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let dst = game.join("G1R/Content/Paks/~mods/fresh.pak");
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        let src = dir.path().join("source.pak");
        std::fs::write(&src, b"new-deployment").unwrap();
        replace_additive_before_apply(&dst, b"external-race-winner");

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            DeployPlan {
                managed_paks: vec![(src, dst.clone())],
                ..Default::default()
            },
            DeployRecord {
                owner: "manager".into(),
                ..Default::default()
            },
            None,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("appeared after preflight"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&dst).unwrap(), b"external-race-winner");
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn fresh_ue4ss_publication_never_clobbers_a_racing_creator() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let dst = game.join("G1R/Binaries/Win64/ue4ss/Mods/Fresh");
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        let src = make_mod_src(dir.path(), "fresh-source");
        replace_ue4ss_before_apply(&dst, b"external-race-winner");

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            DeployPlan {
                ue4ss_dirs: vec![(src, dst.clone())],
                ..Default::default()
            },
            DeployRecord {
                owner: "manager".into(),
                ..Default::default()
            },
            None,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("appeared after preflight"),
            "unexpected error: {error}"
        );
        assert_eq!(
            std::fs::read(dst.join("external-sentinel.txt")).unwrap(),
            b"external-race-winner"
        );
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn commit_near_revalidation_preserves_racing_ue4ss_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let dst = game.join("G1R/Binaries/Win64/ue4ss/Mods/Owned");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("old.txt"), b"previous-owned").unwrap();
        let src = make_mod_src(dir.path(), "new-source");
        let prior_record = DeployRecord {
            owner: "manager".into(),
            ue4ss_mod_dirs: vec![dst.display().to_string()],
            ue4ss_tree_fingerprints: BTreeMap::from([(
                dst.display().to_string(),
                tree_fingerprint(&dst).unwrap(),
            )]),
            ..Default::default()
        };
        write_record_file(&game, &prior_record).unwrap();
        let prior_raw = std::fs::read(record_path(&game)).unwrap();
        let prior = read_record(&game).unwrap().unwrap();
        replace_ue4ss_before_apply(&dst, b"external-tree");

        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            DeployPlan {
                ue4ss_dirs: vec![(src, dst.clone())],
                ..Default::default()
            },
            DeployRecord {
                owner: "manager".into(),
                ..Default::default()
            },
            Some(prior),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("changed before apply"),
            "unexpected error: {error}"
        );
        assert_eq!(
            std::fs::read(dst.join("external-sentinel.txt")).unwrap(),
            b"external-tree"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), prior_raw);
    }

    #[test]
    fn undeploy_never_deletes_same_path_external_file_or_tree() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let pak = game.join("G1R/Content/Paks/~mods/owned.pak");
        let ue4ss = game.join("G1R/Binaries/Win64/ue4ss/Mods/Owned");
        std::fs::create_dir_all(pak.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&ue4ss).unwrap();
        std::fs::write(&pak, b"deployed-pak").unwrap();
        std::fs::write(ue4ss.join("deployed.txt"), b"deployed-tree").unwrap();
        let record = DeployRecord {
            owner: "manager".into(),
            managed_paks: vec![pak.display().to_string()],
            ue4ss_mod_dirs: vec![ue4ss.display().to_string()],
            deployed_hashes: BTreeMap::from([(
                pak.display().to_string(),
                sha256_file(&pak).unwrap(),
            )]),
            ue4ss_tree_fingerprints: BTreeMap::from([(
                ue4ss.display().to_string(),
                tree_fingerprint(&ue4ss).unwrap(),
            )]),
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();
        std::fs::write(&pak, b"external-pak").unwrap();
        std::fs::write(ue4ss.join("external.txt"), b"external-tree").unwrap();

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("externally changed"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&pak).unwrap(), b"external-pak");
        assert_eq!(
            std::fs::read(ue4ss.join("external.txt")).unwrap(),
            b"external-tree"
        );
        assert!(record_path(&game).exists());
    }

    #[test]
    fn legacy_path_only_ownership_is_parsed_but_never_auto_adopted() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let pak = game.join("G1R/Content/Paks/~mods/legacy.pak");
        std::fs::create_dir_all(pak.parent().unwrap()).unwrap();
        std::fs::write(&pak, b"unknown-owner").unwrap();
        let legacy = DeployRecord {
            owner: "manager".into(),
            managed_paks: vec![pak.display().to_string()],
            ..Default::default()
        };
        write_record_file(&game, &legacy).unwrap();

        let parsed = read_record(&game).unwrap().unwrap();
        assert!(parsed.record.deployed_hashes.is_empty());
        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("unverifiable"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&pak).unwrap(), b"unknown-owner");
        assert!(record_path(&game).exists());
    }

    #[test]
    fn interrupted_ue4ss_staging_dirs_are_recorded_and_undeploy_cleans_them() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let root = game.join("G1R/Binaries/Win64/ue4ss");
        let dst = root.join("Mods/FreshMod");
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        let src = make_mod_src(dir.path(), "source");
        let mut plan = DeployPlan {
            ue4ss_dirs: vec![(src, dst)],
            ..Default::default()
        };
        prepare_test_plan_identities(&mut plan);
        let mut record = DeployRecord::default();
        let mut undo = Undo::default();
        stage(&plan, &mut record, &mut undo).unwrap();
        let transaction_dirs = undo.ue4ss_transaction_dirs();
        assert_eq!(transaction_dirs.len(), 1);
        assert!(transaction_dirs.iter().all(|path| path.exists()));
        assert!(transaction_dirs
            .iter()
            .all(|path| contains_same_path(&record.stale_ue4ss_dirs, &path.display().to_string())));
        record.phase = DeployPhase::RecoveryRequired;
        write_record_file(&game, &record).unwrap();

        // Simulate process loss: the in-memory cleanup guard disappears, leaving only the durable
        // recovery record to identify the random transaction directory.
        std::mem::forget(undo);
        undeploy(&game).unwrap();
        assert!(transaction_dirs.iter().all(|path| !path.exists()));
        assert!(ue4ss_transaction_dirs(&root).is_empty());
    }

    #[test]
    fn interrupted_ue4ss_move_aside_holder_is_recovered_from_durable_record() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let root = game.join("G1R/Binaries/Win64/ue4ss");
        let dst = root.join("Mods/OwnedMod");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("old.txt"), b"previous deployment").unwrap();
        let src = make_mod_src(dir.path(), "source");
        let mut plan = DeployPlan {
            ue4ss_dirs: vec![(src, dst.clone())],
            ..Default::default()
        };
        prepare_test_plan_identities(&mut plan);
        let mut record = DeployRecord::default();
        let mut undo = Undo::default();
        stage(&plan, &mut record, &mut undo).unwrap();
        let swap = &mut undo.ue4ss_swaps[0];
        std::fs::rename(&dst, swap.old.as_ref().unwrap()).unwrap();
        swap.state = Ue4ssSwapState::OldMoved;
        let transaction_dirs = undo.ue4ss_transaction_dirs();
        record.phase = DeployPhase::RecoveryRequired;
        write_record_file(&game, &record).unwrap();

        // Process loss after the old mod moved but before promotion: only the record survives.
        std::mem::forget(undo);
        undeploy(&game).unwrap();
        assert!(!dst.exists());
        assert!(transaction_dirs.iter().all(|path| !path.exists()));
        assert!(ue4ss_transaction_dirs(&root).is_empty());
    }

    /// FIX 1: a manager-owned deployment records EVERY UE4SS dir in `ue4ss_mod_dirs` and leaves the
    /// legacy single `ue4ss_mod_dir` unset (studio behavior — first-into-legacy — is unchanged).
    #[test]
    fn manager_record_puts_all_ue4ss_dirs_in_vec() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        std::fs::create_dir_all(&mods).unwrap();
        let src_a = make_mod_src(dir.path(), "SrcA");
        let src_b = make_mod_src(dir.path(), "SrcB");
        let gp = resolve_game_paths(&game);

        let plan = DeployPlan {
            ue4ss_dirs: vec![(src_a, mods.join("ModA")), (src_b, mods.join("ModB"))],
            ..Default::default()
        };
        let record = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ..Default::default()
        };
        let rec = commit_plan(&gp, &game, plan, record, None).unwrap();

        assert!(
            rec.ue4ss_mod_dir.is_none(),
            "manager must not use the legacy single dir field"
        );
        assert_eq!(rec.ue4ss_mod_dirs.len(), 2, "all dirs go into the vec");
        assert!(rec.ue4ss_mod_dirs.iter().any(|d| d.ends_with("ModA")));
        assert!(rec.ue4ss_mod_dirs.iter().any(|d| d.ends_with("ModB")));
        assert!(
            mods.join("ModA").exists() && mods.join("ModB").exists(),
            "both dirs installed"
        );
    }

    /// FIX 2: a manager deployment mirrors its footprint (managed paks → `texture_triplets`, UE4SS
    /// dirs → `stale_ue4ss_dirs`) into the legacy record fields a pre-v2 binary understands, and a
    /// v2 undeploy still removes everything exactly once with no double-error.
    #[test]
    fn manager_record_mirrors_footprint_into_legacy_fields() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        std::fs::create_dir_all(&mods).unwrap();
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&paks).unwrap();
        let src = make_mod_src(dir.path(), "SrcA");
        let pak_src = dir.path().join("lib_A.pak");
        std::fs::write(&pak_src, b"PAK").unwrap();
        let pak_dst = paks.join("zzz_mgr_A_P.pak");
        let ue4ss_dst = mods.join("MgrModA");
        let gp = resolve_game_paths(&game);

        let plan = DeployPlan {
            ue4ss_dirs: vec![(src, ue4ss_dst.clone())],
            managed_paks: vec![(pak_src, pak_dst.clone())],
            ..Default::default()
        };
        let record = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ..Default::default()
        };
        let rec = commit_plan(&gp, &game, plan, record, None).unwrap();

        // Real (v2) fields.
        assert!(rec.ue4ss_mod_dir.is_none());
        assert_eq!(rec.ue4ss_mod_dirs.len(), 1);
        assert_eq!(rec.managed_paks.len(), 1);
        // Legacy mirror an old binary reads.
        assert!(
            same_path_contains(&rec.texture_triplets, &pak_dst),
            "pak not mirrored into texture_triplets"
        );
        assert!(
            same_path_contains(&rec.stale_ue4ss_dirs, &ue4ss_dst),
            "dir not mirrored into stale_ue4ss_dirs"
        );
        assert!(
            pak_dst.exists() && ue4ss_dst.exists(),
            "footprint installed"
        );

        // v2 undeploy removes everything once, no error (the mirror + real field share paths, so
        // the second removal pass must be a harmless no-op).
        let restored = undeploy(&game).unwrap();
        assert!(
            restored.is_some(),
            "undeploy should report a restored record"
        );
        assert!(!pak_dst.exists(), "managed pak not removed");
        assert!(!ue4ss_dst.exists(), "managed ue4ss dir not removed");
        assert!(
            !record_path(&game).exists(),
            "record file should be gone after a clean undeploy"
        );
    }

    /// FIX 3(a): `commit_plan` rejects a self-colliding plan (two entries writing the same dst)
    /// BEFORE touching the game.
    #[test]
    fn commit_plan_rejects_duplicate_dsts() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&paks).unwrap();
        let a = dir.path().join("a.pak");
        let b = dir.path().join("b.pak");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let clash = paks.join("zzz_dup_P.pak");
        let gp = resolve_game_paths(&game);

        // Two different srcs, SAME dst — must be rejected.
        let plan = DeployPlan {
            managed_paks: vec![(a, clash.clone()), (b, clash.clone())],
            ..Default::default()
        };
        let record = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ..Default::default()
        };
        let err = commit_plan(&gp, &game, plan, record, None).unwrap_err();
        assert!(
            err.to_string().contains("duplicate deploy target"),
            "got: {err}"
        );
        // Nothing was written and no record file created (guard tripped before stage).
        assert!(!clash.exists());
        assert!(!record_path(&game).exists());
    }

    /// FIX 4: prev-vs-new membership uses `same_path`, so differently-spelled paths that resolve
    /// to the SAME file are recognized as identical and the new deploy's file is NOT retired.
    #[test]
    fn retire_tolerates_noncanonical_prev_paths() {
        // This test needs distinct ambient and canonical spellings of the same path.
        let dir = ::tempfile::tempdir().unwrap();
        let paks = dir.path().join("mods");
        std::fs::create_dir_all(&paks).unwrap();
        let dst = paks.join("zzz_keep_P.pak");
        std::fs::write(&dst, b"keep").unwrap();
        // Use the canonical `\\?\` spelling on Windows and a harmless `/./` spelling on Unix.
        #[cfg(windows)]
        let recorded_alias = std::fs::canonicalize(&dst).unwrap().display().to_string();
        #[cfg(not(windows))]
        let recorded_alias = format!(
            "{}/./{}",
            paks.display(),
            dst.file_name().unwrap().to_string_lossy()
        );
        let plain = dst.display().to_string();
        assert_ne!(
            recorded_alias, plain,
            "precondition: recorded alias must differ from the plain path"
        );

        let prev = DeployRecord {
            mod_name: "Old".into(),
            texture_triplets: vec![recorded_alias],
            ..Default::default()
        };
        // The new plan re-creates the SAME file (plain path form).
        let plan = DeployPlan {
            texture_triplets: vec![(dst.clone(), dst.clone())],
            ..Default::default()
        };
        let mut record = DeployRecord {
            mod_name: "New".into(),
            texture_triplets: vec![plain.clone()],
            ..Default::default()
        };

        let _ = retire_leftovers(dir.path(), &[], Some(&prev), &plan, &mut record).unwrap();
        assert!(
            dst.exists(),
            "file the new deploy re-creates must NOT be retired despite path-form mismatch"
        );
    }

    /// MINOR (b): redeploying over a prev MANAGER record whose managed pak + UE4SS dir are NOT in
    /// the new plan retires both (files gone, entries pruned).
    #[test]
    fn redeploy_retires_prev_managed_paks_and_folds_ue4ss_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Binaries/Win64/ue4ss/Mods");
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(&paks).unwrap();

        // Prev manager deployment: one managed pak + one UE4SS dir, both present on disk.
        let prev_pak = paks.join("zzz_prev_P.pak");
        std::fs::write(&prev_pak, b"old").unwrap();
        let prev_dir = mods.join("PrevMod");
        std::fs::create_dir_all(&prev_dir).unwrap();
        std::fs::write(prev_dir.join("enabled.txt"), b"").unwrap();
        let prev = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ue4ss_mod_dirs: vec![prev_dir.display().to_string()],
            managed_paks: vec![prev_pak.display().to_string()],
            // as a real prev manager record would also carry the legacy mirror
            stale_ue4ss_dirs: vec![prev_dir.display().to_string()],
            texture_triplets: vec![prev_pak.display().to_string()],
            deployed_hashes: BTreeMap::from([(
                prev_pak.display().to_string(),
                sha256_file(&prev_pak).unwrap(),
            )]),
            ue4ss_tree_fingerprints: BTreeMap::from([(
                prev_dir.display().to_string(),
                tree_fingerprint(&prev_dir).unwrap(),
            )]),
            ..Default::default()
        };
        std::fs::write(record_path(&game), serde_json::to_vec(&prev).unwrap()).unwrap();
        let prev = read_record(&game).unwrap().unwrap();

        // New manager deployment: a DIFFERENT mod, nothing overlapping the prev footprint.
        let src = make_mod_src(dir.path(), "SrcNew");
        let new_dir = mods.join("NewMod");
        let gp = resolve_game_paths(&game);
        let plan = DeployPlan {
            ue4ss_dirs: vec![(src, new_dir.clone())],
            ..Default::default()
        };
        let record = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ..Default::default()
        };
        let rec = commit_plan(&gp, &game, plan, record, Some(prev)).unwrap();

        assert!(!prev_pak.exists(), "prev managed pak not retired");
        assert!(!prev_dir.exists(), "prev ue4ss dir not retired");
        assert!(new_dir.exists(), "new ue4ss dir should be installed");
        assert!(
            !same_path_contains(&rec.managed_paks, &prev_pak),
            "prev pak still tracked"
        );
        assert!(
            !same_path_contains(&rec.texture_triplets, &prev_pak),
            "prev pak still tracked (mirror)"
        );
        assert!(
            !same_path_contains(&rec.stale_ue4ss_dirs, &prev_dir),
            "prev dir still tracked (mirror)"
        );
        assert!(
            !same_path_contains(&rec.ue4ss_mod_dirs, &prev_dir),
            "prev dir still tracked"
        );
    }

    /// MINOR (b): if a managed-pak copy fails mid-apply, the rollback removes the fresh pak dst(s)
    /// already copied and restores the pre-deploy record (here: none).
    #[test]
    fn rollback_removes_fresh_managed_pak_copy() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let paks = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&paks).unwrap();
        // pak1 has a valid src (gets copied); pak2's src is missing (copy fails → rollback).
        let pak1_src = dir.path().join("ok.pak");
        std::fs::write(&pak1_src, b"ok").unwrap();
        let pak1_dst = paks.join("zzz_ok_P.pak");
        let pak2_src = dir.path().join("missing.pak"); // never created
        let pak2_dst = paks.join("zzz_missing_P.pak");
        let gp = resolve_game_paths(&game);

        let plan = DeployPlan {
            managed_paks: vec![(pak1_src, pak1_dst.clone()), (pak2_src, pak2_dst.clone())],
            ..Default::default()
        };
        let record = DeployRecord {
            mod_name: "loadout".into(),
            owner: "manager".into(),
            ..Default::default()
        };
        let err = commit_plan(&gp, &game, plan, record, None).unwrap_err();
        assert!(
            err.to_string().contains("reading SHA-256 source metadata"),
            "expected a source preflight failure, got: {err}"
        );

        // Rollback must have deleted the already-copied fresh pak1, left no stray pak2, and (since
        // there was no prior record) removed the record file it wrote before applying.
        assert!(
            !pak1_dst.exists(),
            "fresh managed pak copy not removed on rollback"
        );
        assert!(!pak2_dst.exists());
        assert!(
            !record_path(&game).exists(),
            "pre-write record should be gone (there was none)"
        );
    }

    #[test]
    fn additive_overwrite_uses_disk_snapshot_and_rollback_restores_it() {
        let dir = tempfile::tempdir().unwrap();
        let paks = dir.path().join("mods");
        std::fs::create_dir_all(&paks).unwrap();
        let src = dir.path().join("new.pak");
        let dst = paks.join("same-name.pak");
        std::fs::write(&src, b"new-pak-content").unwrap();
        std::fs::write(&dst, b"old-active-pak-content").unwrap();

        let mut plan = DeployPlan {
            managed_paks: vec![(src, dst.clone())],
            ..Default::default()
        };
        prepare_test_plan_identities(&mut plan);
        let mut undo = Undo::default();
        apply_writes(&plan, &mut undo).unwrap();

        assert_eq!(std::fs::read(&dst).unwrap(), b"new-pak-content");
        assert_eq!(undo.texture_files.len(), 1);
        let snapshot = undo.texture_files[0]
            .prior
            .as_ref()
            .expect("an overwritten pak needs a disk snapshot");
        let snapshot_path = snapshot.to_path_buf();
        assert!(snapshot_path.is_file(), "snapshot must exist on disk");
        assert_eq!(
            std::fs::read(&snapshot_path).unwrap(),
            b"old-active-pak-content"
        );

        let failures = undo.rollback();
        assert!(failures.is_empty(), "rollback failures: {failures:?}");
        assert_eq!(std::fs::read(&dst).unwrap(), b"old-active-pak-content");
        assert!(
            !snapshot_path.exists(),
            "successful atomic restore must consume the snapshot"
        );
    }

    #[test]
    fn rollback_failure_is_reported_and_retains_recovery_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.bin");
        std::fs::write(&old, b"recover-me").unwrap();
        let snapshot = verified_temp_copy(&old, ".rollback-test-")
            .unwrap()
            .into_temp_path();
        let snapshot_path = snapshot.to_path_buf();

        // A directory at the live-file path makes the file-to-path rename fail deterministically
        // on every supported platform. The rollback must report that failure and keep the only
        // recovery copy instead of deleting it when TempPath drops.
        let blocked_live = dir.path().join("blocked-live.bin");
        std::fs::create_dir(&blocked_live).unwrap();
        let mut undo = Undo::default();
        undo.files.push(LiveFileUndo {
            live: blocked_live.clone(),
            snapshot,
            published_hash: Some(content_hash(b"published")),
            backup: None,
        });
        let failures = undo.rollback();
        assert_eq!(failures.len(), 1, "failures: {failures:?}");
        assert!(failures[0].contains("live file"));
        assert!(failures[0].contains("snapshot retained"));
        assert!(snapshot_path.is_file(), "recovery snapshot was discarded");
        assert_eq!(std::fs::read(&snapshot_path).unwrap(), b"recover-me");

        let reported =
            with_rollback_failures(ModError::Other("primary write failure".into()), failures);
        let text = reported.to_string();
        assert!(text.contains("primary write failure"), "{text}");
        assert!(text.contains("ROLLBACK_INCOMPLETE"), "{text}");
        assert!(
            text.contains(&snapshot_path.display().to_string()),
            "{text}"
        );
    }

    #[test]
    fn incomplete_live_rollback_does_not_replace_recovery_record() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let attempted_record = serde_json::to_vec(&DeployRecord {
            phase: DeployPhase::RecoveryRequired,
            ..Default::default()
        })
        .unwrap();
        std::fs::write(record_path(&game), &attempted_record).unwrap();

        let old = game.join("old.bin");
        std::fs::write(&old, b"old-live-bytes").unwrap();
        let snapshot = verified_temp_copy(&old, ".rollback-record-test-")
            .unwrap()
            .into_temp_path();
        let blocked_live = game.join("blocked-live.bin");
        std::fs::create_dir(&blocked_live).unwrap();
        let mut undo = Undo::default();
        undo.files.push(LiveFileUndo {
            live: blocked_live,
            snapshot,
            published_hash: Some(content_hash(b"published")),
            backup: None,
        });

        let error = rollback_commit_error(
            ModError::Other("late write failed".into()),
            undo,
            &game,
            Some(b"previous-record"),
        );
        let text = error.to_string();
        assert!(text.contains("ROLLBACK_INCOMPLETE"), "{text}");
        assert!(
            text.contains("on-disk deploy record intentionally left"),
            "{text}"
        );
        assert_eq!(
            std::fs::read(record_path(&game)).unwrap(),
            attempted_record,
            "the attempted-deploy record is the only recovery map for a partial new footprint"
        );
    }

    #[test]
    fn duplicate_detection_includes_in_place_writes_and_cross_kind_targets() {
        // This test needs distinct ambient and canonical spellings of the same path.
        let dir = ::tempfile::tempdir().unwrap();
        let target = dir.path().join("target.bin");
        std::fs::write(&target, b"old").unwrap();
        let alias = std::fs::canonicalize(&target).unwrap();

        let duplicate_writes = DeployPlan {
            writes: vec![
                (target.clone(), b"one".to_vec()),
                (alias.clone(), b"two".to_vec()),
            ],
            ..Default::default()
        };
        assert!(first_duplicate_dst(&duplicate_writes).is_some());

        let cross_kind = DeployPlan {
            writes: vec![(target, b"new".to_vec())],
            managed_paks: vec![(dir.path().join("source.pak"), alias)],
            ..Default::default()
        };
        assert!(first_duplicate_dst(&cross_kind).is_some());
    }

    #[test]
    fn failed_live_restore_keeps_new_backup_for_recovery_record() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot_source = dir.path().join("old-live.bin");
        std::fs::write(&snapshot_source, b"old active bytes").unwrap();
        let snapshot = verified_temp_copy(&snapshot_source, ".rollback-live-")
            .unwrap()
            .into_temp_path();
        let blocked_live = dir.path().join("blocked-live.bin");
        std::fs::create_dir(&blocked_live).unwrap();
        let backup = bak_path(&blocked_live);
        std::fs::write(&backup, b"current pristine bytes").unwrap();

        let mut undo = Undo::default();
        undo.files.push(LiveFileUndo {
            live: blocked_live,
            snapshot,
            published_hash: Some(content_hash(b"published")),
            backup: Some(BackupUndo {
                path: backup.clone(),
                created: true,
                stale_snapshot: None,
                published_hash: sha256_file(&backup).unwrap(),
            }),
        });
        let failures = undo.rollback();

        assert_eq!(failures.len(), 1, "failures: {failures:?}");
        assert_eq!(
            std::fs::read(&backup).unwrap(),
            b"current pristine bytes",
            "the retained recovery record must not point at a deleted backup"
        );
    }

    #[test]
    fn failed_drift_live_restore_keeps_refresh_and_never_restores_stale_backup() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot_source = dir.path().join("old-live.bin");
        std::fs::write(&snapshot_source, b"old active bytes").unwrap();
        let live_snapshot = verified_temp_copy(&snapshot_source, ".rollback-live-")
            .unwrap()
            .into_temp_path();
        let stale_source = dir.path().join("stale-backup.bin");
        std::fs::write(&stale_source, b"stale pre-hotfix pristine").unwrap();
        let stale_snapshot = verified_temp_copy(&stale_source, ".rollback-stale-")
            .unwrap()
            .into_temp_path();
        let blocked_live = dir.path().join("blocked-live.bin");
        std::fs::create_dir(&blocked_live).unwrap();
        let backup = bak_path(&blocked_live);
        std::fs::write(&backup, b"refreshed hotfix pristine").unwrap();

        let mut undo = Undo::default();
        undo.files.push(LiveFileUndo {
            live: blocked_live,
            snapshot: live_snapshot,
            published_hash: Some(content_hash(b"published")),
            backup: Some(BackupUndo {
                path: backup.clone(),
                created: true,
                stale_snapshot: Some(stale_snapshot),
                published_hash: sha256_file(&backup).unwrap(),
            }),
        });
        let failures = undo.rollback();

        assert_eq!(failures.len(), 1, "failures: {failures:?}");
        assert_eq!(
            std::fs::read(&backup).unwrap(),
            b"refreshed hotfix pristine",
            "failed live restore must not replace recovery's fresh backup with stale drift bytes"
        );
    }

    #[test]
    fn commit_rollback_restores_exact_original_record_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let raw = br#"{
  "future_field": { "must": "survive byte for byte" },
  "backups": [], "ue4ss_mod_dir": null,
  "mod_name": "Old"
}
"#;
        std::fs::write(record_path(&game), raw).unwrap();
        let prior = read_record(&game).unwrap().unwrap();

        let source = dir.path().join("first.pak");
        std::fs::write(&source, b"new pak").unwrap();
        let plan = DeployPlan {
            managed_paks: vec![
                (source, mods.join("first.pak")),
                (dir.path().join("missing.pak"), mods.join("second.pak")),
            ],
            ..Default::default()
        };
        let error = commit_plan(
            &resolve_game_paths(&game),
            &game,
            plan,
            DeployRecord {
                owner: "manager".into(),
                ..Default::default()
            },
            Some(prior),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("reading SHA-256 source metadata"),
            "{error}"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), raw);
    }

    #[test]
    fn atomic_write_uses_unique_temp_and_ignores_stale_predictable_temp() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("record.json");
        let stale = PathBuf::from(format!("{}.gore-tmp", target.display()));
        std::fs::write(&stale, b"stale temp from crashed process").unwrap();

        atomic_write(&target, b"durable new record").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"durable new record");
        assert_eq!(
            std::fs::read(&stale).unwrap(),
            b"stale temp from crashed process"
        );
    }

    #[test]
    fn deploy_rejects_corrupt_unreadable_and_recovery_required_records() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let bundle = dir.path().join("bundle");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        let manifest = ModManifest {
            format: 1,
            mod_meta: ModMeta {
                name: "GuardProbe".into(),
                version: String::new(),
                author: String::new(),
            },
            components: vec![Component::Ue4ssLua {
                name: "GuardProbe".into(),
                path: "ue4ss/GuardProbe".into(),
                targets: Vec::new(),
                opaque: false,
            }],
        };
        std::fs::write(
            bundle.join("gore-mod.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let corrupt = b"{ corrupt deploy record";
        std::fs::write(record_path(&game), corrupt).unwrap();
        let error = deploy(&bundle, &game).unwrap_err();
        assert!(
            error.to_string().contains("parsing deploy record"),
            "{error}"
        );
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), corrupt);

        std::fs::remove_file(record_path(&game)).unwrap();
        std::fs::create_dir(record_path(&game)).unwrap();
        let error = deploy(&bundle, &game).unwrap_err();
        assert!(
            error.to_string().contains("reading deploy record"),
            "{error}"
        );
        assert!(record_path(&game).is_dir());

        std::fs::remove_dir(record_path(&game)).unwrap();
        let recovery = DeployRecord {
            phase: DeployPhase::RecoveryRequired,
            ..Default::default()
        };
        let recovery_bytes = serde_json::to_vec(&recovery).unwrap();
        std::fs::write(record_path(&game), &recovery_bytes).unwrap();
        let error = deploy(&bundle, &game).unwrap_err();
        assert!(error.to_string().contains("RECOVERY_REQUIRED"), "{error}");
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), recovery_bytes);
    }

    #[test]
    fn deploy_record_rejects_escape_foreign_root_and_wrong_backup_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let foreign = dir.path().join("foreign");
        let voice = game.join("G1R/Story/VoiceOver");
        std::fs::create_dir_all(&voice).unwrap();
        std::fs::create_dir_all(&foreign).unwrap();
        let sentinel = foreign.join("sentinel.pak");
        std::fs::write(&sentinel, b"external-sentinel").unwrap();

        let traversal = game.join("G1R/Content/Paks/~mods/../../../../../foreign/sentinel.pak");
        let cases = [
            DeployRecord {
                managed_paks: vec![traversal.display().to_string()],
                ..Default::default()
            },
            DeployRecord {
                managed_paks: vec![sentinel.display().to_string()],
                ..Default::default()
            },
            DeployRecord {
                backups: vec![(
                    voice.join("live.zip").display().to_string(),
                    voice.join("different.zip.gore-bak").display().to_string(),
                    true,
                )],
                ..Default::default()
            },
            DeployRecord {
                backups: vec![(
                    voice.join("live.zip.gore-bak").display().to_string(),
                    voice
                        .join("live.zip.gore-bak.gore-bak")
                        .display()
                        .to_string(),
                    true,
                )],
                ..Default::default()
            },
        ];
        for record in cases {
            let raw = serde_json::to_vec(&record).unwrap();
            std::fs::write(record_path(&game), &raw).unwrap();
            let error = undeploy(&game).unwrap_err();
            assert!(
                error.to_string().contains("unsafe deploy record"),
                "unexpected error: {error}"
            );
            assert_eq!(std::fs::read(&sentinel).unwrap(), b"external-sentinel");
            assert_eq!(std::fs::read(record_path(&game)).unwrap(), raw);
        }
    }

    #[test]
    fn deploy_record_rejects_symlink_or_reparse_target_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let sentinel = dir.path().join("external.pak");
        std::fs::write(&sentinel, b"external-sentinel").unwrap();
        let linked = mods.join("linked.pak");
        if !make_test_file_link(&sentinel, &linked) {
            return;
        }
        let record = DeployRecord {
            managed_paks: vec![linked.display().to_string()],
            ..Default::default()
        };
        let raw = serde_json::to_vec(&record).unwrap();
        std::fs::write(record_path(&game), &raw).unwrap();

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("symlink or reparse"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"external-sentinel");
        assert_eq!(std::fs::read(record_path(&game)).unwrap(), raw);
    }

    #[test]
    fn a_loose_target_is_a_content_or_config_file_that_belongs_to_no_other_class() {
        // The one genuinely new safety decision the `files` section introduces. Widening the
        // live-file class to "any relative path" would let a corrupted or hostile record name the
        // game executable and have undeploy write a "backup" over it, so the accepted set is
        // pinned here directly instead of only being exercised through a deploy.
        for path in [
            "G1R/Content/Slate/Cursors/Normal/Normal.PNG",
            "G1R/Content/Movies/Intro.mp4",
            "G1R/Config/DefaultEngine.ini",
        ] {
            let relative = loose_relative_os_path(path);
            assert!(loose_target_allowed(&relative), "{path}");
            assert!(
                record_path_matches_class(&relative, RecordPathClass::LiveFile),
                "{path}"
            );
            let backup = loose_relative_os_path(&format!("{path}.gore-bak"));
            assert!(
                record_path_matches_class(&backup, RecordPathClass::BackupFile),
                "{path}"
            );
        }
        for path in [
            "G1R/Binaries/Win64/G1R-Win64-Shipping.exe",
            "G1R/Binaries/Win64/ue4ss/Mods/Other/Scripts/main.lua",
            "G1R/Content/Paks/~mods/zzz_Other_P.pak",
            "G1R/Content/Paks/G1R-Windows.utoc",
            "G1R/Content/Slate/Cursors/Normal/Normal.PNG.gore-bak",
            "G1R/Content/FMOD/Desktop/SFX.bank",
            "G1R/Story/Cache/AlkimiaLocalization_0.lcache",
            "G1R/Story/VoiceOver/german_new.zip",
            "G1R/Script/PrecompiledScript_Shipping.Cache",
            "G1R/Saved/Config/WindowsNoEditor/Engine.ini",
            "G1R/Content",
            "gore-mod.deployed.json",
            "G1R/Content/gore-mod.deployed.json",
            "G1R/Content/../../outside.bin",
        ] {
            assert!(
                !loose_target_allowed(&loose_relative_os_path(path)),
                "{path} must not be reachable as a loose file"
            );
        }
        // The four dedicated in-place targets keep their own class; the loose gate only refuses to
        // ALSO claim them, so their existing deploy mechanisms are untouched.
        for path in [
            "G1R/Content/FMOD/Desktop/SFX.bank",
            "G1R/Story/Cache/AlkimiaLocalization_0.lcache",
            "G1R/Story/VoiceOver/german_new.zip",
            "G1R/Script/PrecompiledScript_Shipping.Cache",
        ] {
            assert!(
                record_path_matches_class(&loose_relative_os_path(path), RecordPathClass::LiveFile),
                "{path}"
            );
        }
    }

    const TEST_CURSOR: &str = "G1R/Content/Slate/Cursors/Normal/Normal.PNG";

    fn test_cursor_bundle(dir: &Path, name: &str, source: &Path) -> PathBuf {
        let bundle_dir = dir.join(format!("bundle-{name}"));
        let spec = test_loose_spec(
            name,
            vec![LooseFileReplacement {
                game_path: TEST_CURSOR.into(),
                source_path: source.display().to_string(),
            }],
        );
        write_bundle(&bundle_dir, &build_bundle(&spec).unwrap()).unwrap();
        bundle_dir
    }

    #[test]
    fn loose_file_deploy_backs_up_replaces_and_undeploy_restores() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join(loose_relative_os_path(TEST_CURSOR));
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"shipped-cursor").unwrap();
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();

        let bundle_dir = test_cursor_bundle(dir.path(), "CursorMod", &source);
        let record = deploy(&bundle_dir, &game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"modded-cursor");
        assert_eq!(std::fs::read(bak_path(&live)).unwrap(), b"shipped-cursor");
        assert_eq!(record.backups.len(), 1, "backups: {:?}", record.backups);

        undeploy(&game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"shipped-cursor");
        assert!(!bak_path(&live).exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn loose_file_deploy_refuses_a_target_this_install_does_not_ship() {
        // Replace-only, and the refusal has to land in prepare: an add-new loose file would need
        // delete-on-undeploy semantics, which `validate_record` refuses against a live game file.
        // Failing later would mean a backup had already been taken for some other component.
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(game.join("G1R/Content")).unwrap();
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();

        let bundle_dir = test_cursor_bundle(dir.path(), "GhostCursor", &source);
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("loose file target does not exist"),
            "unexpected error: {error}"
        );
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn loose_file_deploy_refuses_a_symlinked_target_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join(loose_relative_os_path(TEST_CURSOR));
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let sentinel = dir.path().join("external.png");
        std::fs::write(&sentinel, b"external-sentinel").unwrap();
        if !make_test_file_link(&sentinel, &live) {
            return;
        }
        let source = dir.path().join("Normal.PNG");
        std::fs::write(&source, b"modded-cursor").unwrap();

        let bundle_dir = test_cursor_bundle(dir.path(), "LinkedCursor", &source);
        let error = deploy(&bundle_dir, &game).unwrap_err().to_string();
        assert!(
            error.contains("not a regular non-link file"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"external-sentinel");
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn loose_file_drift_refreshes_the_backup_and_undeploy_keeps_the_updated_file() {
        // Steam replacing the file underneath a deployed mod must not resurrect the pre-update
        // bytes: the stale backup is dropped, the newer file becomes the pristine, and a later
        // undeploy restores THAT — the same contract the .lcache and the banks already have.
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join(loose_relative_os_path(TEST_CURSOR));
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        std::fs::write(&live, b"shipped-cursor").unwrap();
        let first_source = dir.path().join("one.png");
        let second_source = dir.path().join("two.png");
        std::fs::write(&first_source, b"cursor-one").unwrap();
        std::fs::write(&second_source, b"cursor-two").unwrap();

        deploy(
            &test_cursor_bundle(dir.path(), "CursorOne", &first_source),
            &game,
        )
        .unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"cursor-one");

        std::fs::write(&live, b"patched-by-steam").unwrap();
        deploy(
            &test_cursor_bundle(dir.path(), "CursorTwo", &second_source),
            &game,
        )
        .unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"cursor-two");
        assert_eq!(
            std::fs::read(bak_path(&live)).unwrap(),
            b"patched-by-steam",
            "the stale pre-update backup must be replaced, not kept"
        );

        undeploy(&game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"patched-by-steam");
        assert!(!bak_path(&live).exists());
    }

    #[test]
    fn deploy_record_size_limit_fails_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let sentinel = game.join("sentinel.txt");
        std::fs::write(&sentinel, b"untouched").unwrap();
        std::fs::File::create(record_path(&game))
            .unwrap()
            .set_len(MAX_DEPLOY_RECORD_BYTES + 1)
            .unwrap();

        let error = undeploy(&game).unwrap_err();
        assert!(
            error.to_string().contains("deploy record") && error.to_string().contains("byte limit"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"untouched");
        assert_eq!(
            std::fs::metadata(record_path(&game)).unwrap().len(),
            MAX_DEPLOY_RECORD_BYTES + 1
        );
    }

    #[test]
    fn undeploy_never_restores_from_a_replaced_backup() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/owned.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = bak_path(&live);
        std::fs::write(&live, b"deployed").unwrap();
        std::fs::write(&backup, b"pristine").unwrap();
        let record = DeployRecord {
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(
                live.display().to_string(),
                content_hash(b"deployed"),
            )]),
            backup_hashes: BTreeMap::from([(
                backup.display().to_string(),
                sha256_file(&backup).unwrap(),
            )]),
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();

        std::fs::write(&backup, b"external-backup-replacement").unwrap();
        let error = undeploy(&game).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("backup content no longer matches"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live).unwrap(), b"deployed");
        assert_eq!(
            std::fs::read(&backup).unwrap(),
            b"external-backup-replacement"
        );
        assert!(record_path(&game).exists());
    }

    #[test]
    fn rollback_preserves_external_file_that_replaced_our_publication() {
        let dir = tempfile::tempdir().unwrap();
        let prior = dir.path().join("prior.bin");
        let live = dir.path().join("live.bin");
        std::fs::write(&prior, b"prior-state").unwrap();
        std::fs::write(&live, b"external-race-winner").unwrap();
        let snapshot = verified_temp_copy(&prior, ".rollback-external-")
            .unwrap()
            .into_temp_path();
        let snapshot_path = snapshot.to_path_buf();
        let mut undo = Undo::default();
        undo.files.push(LiveFileUndo {
            live: live.clone(),
            snapshot,
            published_hash: Some(content_hash(b"our-publication")),
            backup: None,
        });

        let failures = undo.rollback();
        assert_eq!(failures.len(), 1, "failures: {failures:?}");
        assert_eq!(std::fs::read(&live).unwrap(), b"external-race-winner");
        assert_eq!(std::fs::read(&snapshot_path).unwrap(), b"prior-state");
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".gore-rollback-current-")
        }));
    }

    #[test]
    fn interrupted_file_claim_preserves_a_new_same_path_winner() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let pak = game.join("G1R/Content/Paks/~mods/owned.pak");
        std::fs::create_dir_all(pak.parent().unwrap()).unwrap();
        std::fs::write(&pak, b"managed-pak").unwrap();
        let expected = sha256_file(&pak).unwrap();
        let holder = game.join(".gore-mod-cleanup-crash-test");
        std::fs::create_dir_all(&holder).unwrap();
        let record = DeployRecord {
            phase: DeployPhase::RecoveryRequired,
            managed_paks: vec![pak.display().to_string()],
            deployed_hashes: BTreeMap::from([(pak.display().to_string(), expected.clone())]),
            file_cleanup_claims: BTreeMap::from([(
                pak.display().to_string(),
                FileCleanupClaim {
                    holder: holder.display().to_string(),
                    expected_hashes: vec![expected],
                    restore_from: None,
                    restore_hash: None,
                },
            )]),
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();
        std::fs::rename(&pak, holder.join("claimed")).unwrap();
        std::fs::write(&pak, b"external-race-winner").unwrap();

        let error = undeploy(&game).unwrap_err();
        assert!(error.to_string().contains("externally changed"), "{error}");
        assert_eq!(std::fs::read(&pak).unwrap(), b"external-race-winner");
        assert!(!holder.exists(), "owned cleanup holder must be retired");
        assert!(record_path(&game).exists());
    }

    #[test]
    fn trusted_partial_ue4ss_tombstone_is_retryable_after_crash() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let original = game.join("G1R/Binaries/Win64/ue4ss/Mods/Owned");
        std::fs::create_dir_all(&original).unwrap();
        std::fs::write(original.join("one.txt"), b"one").unwrap();
        std::fs::write(original.join("two.txt"), b"two").unwrap();
        let expected = tree_fingerprint(&original).unwrap();
        let holder = game.join("G1R/Binaries/Win64/ue4ss/.gore-ue4ss-delete-crash-test");
        std::fs::rename(&original, &holder).unwrap();
        std::fs::remove_file(holder.join("one.txt")).unwrap();
        let record = DeployRecord {
            phase: DeployPhase::RecoveryRequired,
            stale_ue4ss_dirs: vec![holder.display().to_string()],
            ue4ss_tree_fingerprints: BTreeMap::from([(holder.display().to_string(), expected)]),
            ue4ss_cleanup_claims: BTreeMap::from([(
                original.display().to_string(),
                holder.display().to_string(),
            )]),
            trusted_ue4ss_tombstones: vec![holder.display().to_string()],
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();

        undeploy(&game).unwrap();
        assert!(!holder.exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn trusted_deleted_backup_tombstone_prunes_identical_live_without_sticking() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let live = game.join("G1R/Story/VoiceOver/identical.zip");
        std::fs::create_dir_all(live.parent().unwrap()).unwrap();
        let backup = bak_path(&live);
        std::fs::write(&live, b"same-bytes").unwrap();
        std::fs::write(&backup, b"same-bytes").unwrap();
        let backup_hash = sha256_file(&backup).unwrap();
        std::fs::remove_file(&backup).unwrap();
        let holder = game.join(".gore-mod-cleanup-deleted-backup-crash");
        let record = DeployRecord {
            phase: DeployPhase::RecoveryRequired,
            backups: vec![(
                live.display().to_string(),
                backup.display().to_string(),
                true,
            )],
            deployed_hashes: BTreeMap::from([(
                live.display().to_string(),
                content_hash(b"same-bytes"),
            )]),
            backup_hashes: BTreeMap::from([(backup.display().to_string(), backup_hash.clone())]),
            file_cleanup_claims: BTreeMap::from([(
                backup.display().to_string(),
                FileCleanupClaim {
                    holder: holder.display().to_string(),
                    expected_hashes: vec![backup_hash],
                    restore_from: None,
                    restore_hash: None,
                },
            )]),
            trusted_file_tombstones: vec![holder.display().to_string()],
            ..Default::default()
        };
        write_record_file(&game, &record).unwrap();

        undeploy(&game).unwrap();
        assert_eq!(std::fs::read(&live).unwrap(), b"same-bytes");
        assert!(!backup.exists());
        assert!(!holder.exists());
        assert!(!record_path(&game).exists());
    }

    #[test]
    fn record_rejects_cross_target_restore_claim() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let voice = game.join("G1R/Story/VoiceOver");
        std::fs::create_dir_all(&voice).unwrap();
        let live_a = voice.join("a.zip");
        let live_b = voice.join("b.zip");
        let backup_a = bak_path(&live_a);
        let backup_b = bak_path(&live_b);
        for (path, bytes) in [
            (&live_a, b"live-a".as_slice()),
            (&live_b, b"live-b".as_slice()),
            (&backup_a, b"backup-a".as_slice()),
            (&backup_b, b"backup-b".as_slice()),
        ] {
            std::fs::write(path, bytes).unwrap();
        }
        let holder = game.join(".gore-mod-cleanup-restore-test");
        std::fs::create_dir_all(&holder).unwrap();
        let live_a_hash = content_hash(b"live-a");
        let backup_a_hash = sha256_file(&backup_a).unwrap();
        let backup_b_hash = sha256_file(&backup_b).unwrap();
        let record = DeployRecord {
            backups: vec![
                (
                    live_a.display().to_string(),
                    backup_a.display().to_string(),
                    true,
                ),
                (
                    live_b.display().to_string(),
                    backup_b.display().to_string(),
                    true,
                ),
            ],
            deployed_hashes: BTreeMap::from([(live_a.display().to_string(), live_a_hash.clone())]),
            backup_hashes: BTreeMap::from([
                (backup_a.display().to_string(), backup_a_hash),
                (backup_b.display().to_string(), backup_b_hash.clone()),
            ]),
            file_cleanup_claims: BTreeMap::from([(
                live_a.display().to_string(),
                FileCleanupClaim {
                    holder: holder.display().to_string(),
                    expected_hashes: vec![live_a_hash],
                    restore_from: Some(backup_b.display().to_string()),
                    restore_hash: Some(backup_b_hash),
                },
            )]),
            ..Default::default()
        };
        let error = write_record_file(&game, &record).unwrap_err();
        assert!(
            error.to_string().contains("matching live/backup record"),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read(&live_a).unwrap(), b"live-a");
    }

    #[test]
    fn record_rejects_untracked_ue4ss_cleanup_claim() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let original = game.join("G1R/Binaries/Win64/ue4ss/Mods/Untracked");
        std::fs::create_dir_all(&original).unwrap();
        std::fs::write(original.join("payload.txt"), b"payload").unwrap();
        let identity = tree_fingerprint(&original).unwrap();
        let holder = game.join("G1R/Binaries/Win64/ue4ss/.gore-ue4ss-delete-untracked");
        let record = DeployRecord {
            stale_ue4ss_dirs: vec![holder.display().to_string()],
            ue4ss_tree_fingerprints: BTreeMap::from([
                (original.display().to_string(), identity.clone()),
                (holder.display().to_string(), identity),
            ]),
            ue4ss_cleanup_claims: BTreeMap::from([(
                original.display().to_string(),
                holder.display().to_string(),
            )]),
            ..Default::default()
        };
        let error = write_record_file(&game, &record).unwrap_err();
        assert!(
            error.to_string().contains("tracked original identity"),
            "unexpected error: {error}"
        );
        assert!(original.exists());
    }

    #[cfg(windows)]
    #[test]
    fn duplicate_detection_folds_missing_windows_path_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let lower = dir.path().join("missing.pak");
        let upper = PathBuf::from(lower.display().to_string().to_ascii_uppercase());
        let plan = DeployPlan {
            managed_paks: vec![
                (dir.path().join("one.pak"), lower),
                (dir.path().join("two.pak"), upper),
            ],
            ..Default::default()
        };
        assert!(first_duplicate_dst(&plan).is_some());
    }

    /// Test helper: does `list` hold a path referring to the same file as `p`?
    fn same_path_contains(list: &[String], p: &Path) -> bool {
        list.iter().any(|x| same_path(p, x))
    }
}
