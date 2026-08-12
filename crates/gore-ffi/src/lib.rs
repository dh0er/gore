//! C ABI for gore-mod's `dart:ffi` bridge. The length-aware, globally bounded transport-v2 entry
//! point carries the JSON command/response protocol.
//!
//! Request:  `{"command": "<name>", "payload": { ... }}`
//! Response: `{"ok": true, ...}` or `{"ok": false, "error": {"code","message"}}`
//!
//! Commands:
//! - `core_info` — returns the stable FFI ABI, crate version, and sorted command capabilities.
//! - `generate_mod` — payload is an [`OverridesConfig`] (keys `meta` +
//!   `override`); returns `{ok, files:{"enabled.txt":"","Scripts/main.lua":...}}`.
//! - `validate` — payload `{config: OverridesConfig, model: ReflectionModel}`;
//!   returns `{ok, valid, errors:[..]}`.
//! - `authoring_npc_archetype_catalog_v1_build_for_game_root` accepts only one game root and
//!   returns the canonical, generation-sealed, read-only NPC archetype catalog. Native code fixes
//!   executable/Binds paths and selects only the deployment-aware pristine Shipping snapshot; it
//!   never writes, launches, builds, deploys, publishes, or claims runtime qualification.
//! - `authoring_story_catalog_v1_build` reads three bounded generation paths and builds the pinned
//!   catalog entirely in memory; `authoring_story_catalog_v1_read` accepts one bounded raw
//!   canonical catalog string and returns a request-bound read-only chooser projection. Neither
//!   command writes game files, publishes a catalog, or launches the game.
//!   `authoring_story_catalog_v1_build_for_game_root` keeps the same boundary but selects the
//!   deployment-aware pristine Shipping cache natively through `gore-mod`; clients supply only the
//!   game root and never parse deployment records or choose backups.
//! - `authoring_store_open_revision3`, `authoring_store_open_revision3_head_bytes`, and
//!   `authoring_store_prepare_revision3_checkpoint` exclusively carry schema revision 3. Opens
//!   return exact canonical head/project JSON; preparation returns only the canonical candidate
//!   head after a full exact reopen and never publishes the fixed head. Their bounded raw envelopes
//!   reject duplicate, unknown, missing, and wrongly typed outer or payload fields.
//!   Head/project JSON crosses the outer protocol as bounded raw strings, preserving canonical-byte
//!   CAS and duplicate-key rejection. Preparing writes immutable objects but never publishes
//!   `gore-project.json`.
//! - `authoring_store_list_revision3_history_v1` returns only the bounded history vector sealed by
//!   one exact current managed head. Orphan Store objects remain invisible.
//!   `authoring_store_prepare_revision3_history_restore_v1` prepares a fully reopened
//!   current-plus-one checkpoint from one retained historical head while retaining the exact
//!   current head as its direct parent. Neither route publishes the fixed head, touches a game or
//!   save, builds, deploys, or grants runtime authority.
//! - `authoring_store_inspect_revision3_npc_source_v1` fully opens one exact-current managed NPC
//!   Draft and verifies its persisted NPC/ScriptModule closure, parent provenance, source seal,
//!   input fingerprint, and exact source regeneration. It accepts no project or game bytes,
//!   writes no Store/game/save state, and grants no compile, build, spawn, runtime, deployment,
//!   or publication authority.
//! - `authoring_store_prepare_revision3_quest_draft_v3` rebuilds fresh native game/catalog
//!   collision authority, consumes the repeated-Quest transaction, imports its structural
//!   artifact, and fully reopens a prepared revision-3 checkpoint. It returns only structural,
//!   build-blocked/runtime-unqualified facts and never publishes the fixed project head.
//! - `authoring_store_inspect_revision3_quest_source_v1` fully opens one exact-current managed
//!   Quest, reconstructs its immutable historical collision basis, and consumes a fresh native
//!   inspection-only capability to return deterministic regenerated source in a sealed plan. It
//!   never accepts project bytes, artifacts, catalogs, or capabilities from the client; writes no
//!   Store/game/save state; and grants no compile, build, runtime, or publication authority.
//! - `authoring_store_prepare_revision3_quest_context_edit_v1` changes only one existing managed
//!   Quest's description, family, and giver. Parent/giver IDs are resolved inside a freshly
//!   rebuilt Story capability bound to the exact Store head and caller-observed catalog seal.
//!   The route prepares and fully reopens an immutable candidate without importing an artifact,
//!   touching the game/save, or publishing the fixed head.
//! - `authoring_store_prepare_revision3_quest_outline_edit_v2` edits the display outline of one
//!   exact-current Quest while retaining stable objective slots and transition behavior.
//!   It binds the owned module and retained plan explicitly, fully reopens only an unpublished
//!   candidate, and accepts no game, compiler, build, deployment, save, or publication authority.
//! - `authoring_store_prepare_revision3_quest_transitions_edit_v1` edits only one exact-current
//!   managed Quest's bounded semantic transition plan and deterministically regenerates its owned
//!   ScriptModule. It fully reopens an unpublished candidate and never accepts a game root, builds, deploys,
//!   touches a game/save, or publishes the fixed head.
//! - `authoring_store_prepare_revision3_npc_draft_v1` rebuilds the fresh Story catalog, broad NPC
//!   archetype linkage, and complete base-game-plus-exact-current script collision inventory, then
//!   prepares one exact-current NPC/ScriptModule checkpoint. It never publishes the fixed head or
//!   grants build, spawn, runtime, deployment, save, or reusable catalog authority.
//! - `authoring_store_prepare_revision3_npc_profile_edit_v1` edits one exact-current managed NPC's
//!   friendly name and/or complete catalog-resolved archetype triple. It rebuilds fresh Story/NPC
//!   catalogs without creating collision authority, fully reopens only an unpublished immutable
//!   candidate, and never writes a game/save or grants build, spawn, runtime, or publication
//!   authority.
//! - `authoring_store_prepare_revision3_dialog_line_v1` creates one new authored dialog line and
//!   either creates or exactly reuses its managed localization, then fully reopens an immutable
//!   unpublished revision-3 checkpoint. It accepts no game root and grants no topic, build,
//!   runtime, deployment, save, or fixed-head publication authority.
//! - `authoring_store_prepare_remove_revision3_story_draft_v1` removes one exact-current managed
//!   NPC/Quest Draft and its uniquely owned generated ScriptModule from an immutable candidate.
//!   It preserves all other entities and AssetStore metadata, fully reopens the candidate, and
//!   accepts no game, save, build, deployment, runtime, blob-deletion, or head-publication
//!   authority.
//! - `authoring_store_read_revision3_dialog_localization_edit_seed_v1` fully reopens one exact
//!   current managed LocalizationEntry twice and returns its complete bounded texts plus only the
//!   DialogLine/VoiceSlot impact facts needed by the editor.
//!   `authoring_store_prepare_revision3_dialog_localization_edit_v1` applies one exact pure text-
//!   map edit and fully reopens an immutable unpublished candidate. Neither route accepts a game
//!   or save path, mutates the fixed head, or grants topic, build, runtime, or publication
//!   authority. The closed preview-V1 and content-index-V1 wires remain unchanged.
//! - `authoring_store_prepare_revision3_voice_take_v1` binds one existing dialog line and exact
//!   locale to one unresolved VoiceSlot, imports one validated Ogg VoiceTake into immutable CAS,
//!   and fully reopens an unpublished candidate. It never selects an unapproved take, resolves a
//!   runtime target, or publishes the fixed project head.
//! - `authoring_store_prepare_revision3_voice_take_selection_v1` selects one existing approved
//!   VoiceTake candidate or clears one exact VoiceSlot selection. It fully verifies current Ogg
//!   assets, prepares and reopens only an immutable candidate, and never reads a game/save or
//!   publishes the fixed project head.
//! - `authoring_store_prepare_revision3_voice_take_removal_v1` removes one exact candidate from
//!   one line/locale slot and atomically clears it when selected. Shared takes remain; final-use
//!   removal preserves every AssetStore entry and never deletes physical Ogg CAS. The route
//!   prepares and fully reopens only an immutable candidate and accepts no game, save, source,
//!   build, deployment, runtime, blob-deletion, or fixed-head publication authority.
//! - `authoring_store_prepare_revision3_dialog_voice_slot_removal_v1` removes one exact empty,
//!   uniquely owned VoiceSlot relationship from a DialogLine and deletes only that empty slot
//!   entity. It prepares and fully reopens an immutable unpublished candidate without accepting
//!   game, save, media, build, deployment, runtime, or fixed-head publication authority.
//! - `authoring_store_prepare_revision3_voice_take_status_v1` changes only one uniquely retained
//!   VoiceTake review status plus its take/project revisions. It fully verifies current Store
//!   assets, independently closes the exact delta, prepares and reopens an immutable candidate,
//!   and accepts no game/save/media/build/deployment or fixed-head publication authority.
//! - `authoring_store_prepare_revision3_voice_target_v1` resolves one existing VoiceSlot against
//!   the first installed archive for its canonical locale. Native code alone derives bounded,
//!   sealed exact-member evidence; the route prepares a fully reopened candidate without editing
//!   the archive or publishing the fixed project head.
//! - `authoring_store_build_revision3_voice_v1` prepares the bounded revision-3 Voice build
//!   artifact route without deploying, editing the installation, or publishing the project head.
//! - `authoring_store_plan_revision3_voice_v1` fully reopens one exact current revision-3 project
//!   around the pure Voice planner and returns only bounded readiness evidence. It accepts no game
//!   installation or output path, creates no artifact, and grants no build or deployment authority.
//! - `authoring_store_plan_revision3_project_build_v1` binds the native whole-project planner to
//!   one fully verified exact-current Store checkpoint and DataAsset-stage registry before and
//!   after planning. It retains the Store root identity, returns only bounded read-only evidence,
//!   and grants no build, artifact, deployment, runtime, publication, game, or save authority.
//! - `authoring_store_register_revision3_voice_take_preview_v1`,
//!   `authoring_store_materialize_revision3_voice_take_preview_v1`, and
//!   `authoring_store_release_revision3_voice_take_preview_v1` create, use, and release one
//!   native-owned opaque system-temp capability. Materialization binds one exact-current
//!   line/localization/locale/slot/candidate-take/asset graph and copies only its fully verified
//!   Ogg bytes into the fixed `preview.ogg` leaf. The lifecycle never exposes the private CAS path,
//!   overwrites output, mutates the Store/project/game/save, builds, deploys, publishes, or grants
//!   runtime qualification.
//! - `authoring_store_build_revision3_reviewed_dataasset_v1` builds exactly one exact-current
//!   reviewed DataAsset stage into a verified no-clobber triplet plus canonical receipt. It
//!   accepts no package/USMAP bytes, selector, replacement, receipt path, overwrite, deployment,
//!   or runtime authority and never writes the Store, game, save, or fixed project head.
//! - `authoring_read_dataasset_extract_receipt_v2` exposes only the verified target and package /
//!   USMAP content facts needed for an explicit pre-publication target confirmation. It is
//!   read-only and returns no local path, raw package bytes, selector, or offset.
//! - `authoring_store_prepare_revision3_dataasset_stage_v1`,
//!   `authoring_store_prepare_revision3_dataasset_edit_v1`,
//!   `authoring_store_list_revision3_dataasset_stages_v1`, and
//!   `authoring_store_prepare_remove_revision3_dataasset_stage_v1` expose the closed revision-3
//!   fixed-leaf DataAsset stage registry. Preparation verifies the full native PatchReceipt-v2
//!   and live-generation chain; mutation routes return only fully reopened unpublished candidate
//!   heads. They grant no build, pack, runtime, artifact, deployment, or head-publication
//!   authority, and stored stage manifests contain no local paths, receipt bytes, or raw offsets.
//! - `authoring_store_read_revision3_content_index_v1` fully reopens one exact current revision-3
//!   project and returns a bounded semantic entity/reference/asset projection without generated
//!   source or blob bytes. It is read-only and grants no build, runtime, or publication authority.
//! - `authoring_store_read_revision3_item_catalog_v1` binds the native embedded item schema and
//!   per-class provenance seals to one exact-current managed project. Its paired
//!   `authoring_store_prepare_revision3_item_patch_v1` route independently resolves class,
//!   provenance, field names, and scalar types before fully reopening an immutable unpublished
//!   candidate. Neither route accepts game/save/build/deployment or fixed-head publication
//!   authority.
//! - `authoring_store_read_revision3_dialog_localization_v1` fully reopens one exact current
//!   revision-3 LocalizationEntry twice and returns only bounded, sorted per-locale text previews.
//!   It accepts no game, save, or caller-supplied project authority and never mutates the Store.
//! - `authoring_store_import_ogg` and `authoring_store_verify_asset` import or verify bounded
//!   content-addressed Ogg assets. `expected_head_json` is a strict CAS token: null means the fixed
//!   head must be absent; a canonical string means it must match exactly.
//! - `voice_archive_list` — payload `{archive}`; returns exact entries plus the captured
//!   `archive_size`/`archive_sha256` seal. Fails with `VOICE_ARCHIVE_LIMIT` for bounded ZIP
//!   metadata violations or `VOICE_RESPONSE_LIMIT` before an oversized JSON result is built.
//! - `voice_archive_match_line` — payload `{archive, loc_id}`; forms exact `${loc_id}.ogg` and
//!   returns every eligible ASCII case-insensitive basename match plus
//!   `unresolved|unique|ambiguous` and the captured archive seal. It rejects unsafe/unextractable
//!   exact collisions, never selects an ambiguous member, searches substrings, or invents a path.
//! - `voice_archive_extract` — payload `{archive, expected_archive_size,
//!   expected_archive_sha256, entry_path, output_root}`; extracts only if that seal still matches.
//!   The FFI caps archive entries at 50,000, central metadata at 64 MiB, entry paths at 1 KiB,
//!   one extracted entry at 256 MiB, aggregate uncompressed metadata at 16 GiB, and list JSON at
//!   8 MiB. Line-match localization IDs are capped at 512 bytes and match JSON at 1 MiB.
//!   Filesystem-path request strings are capped at 32 KiB.
//! - `voice_ogg_inspect_v1` accepts exactly `{ogg_path}` and safely validates one bounded,
//!   single-link regular Ogg file without following links. It returns only the proven codec/page/
//!   stream facts and the exact validated content seal; native paths are never returned.
//! - `dataasset_fixed_inspect_v1` accepts exactly `{uasset_path, usmap_path, export_index?}` and
//!   performs a bounded, offline-only G1R UE5.4 fixed-leaf inspection. It returns exact content
//!   seals and offset-free selectors without paths, patching, deployment, or runtime claims.
//! - `script_compile_report_v1` fails closed unless the deployment-aware pristine cache can be
//!   resolved, runs the optional compiler hook with automatic normal-generator fallback, and
//!   reports diagnostics plus exact live-install restoration separately from compile success.
//! - `authoring_store_check_revision3_project_compiler_v1` fully reopens one exact managed head,
//!   closes and regenerates every Quest/NPC ScriptModule, and checks the sealed set in one shared
//!   compiler run. It returns only bounded exact-current evidence and no source, path, artifact,
//!   build, deployment, publication, or runtime authority.
//! - `script_compile_install_state_v1` is a bounded, strictly read-only native preflight for the
//!   shipping-game process and every known compile/recovery artifact. It returns display-only
//!   paths and never creates, removes, renames, repairs, launches, or writes anything.
//! - `mgr_preflight_v1` accepts one explicit game root plus an optional paired native Manager
//!   Store override and returns seven fixed-order, bounded first-run findings. It never falls back to
//!   config/Steam discovery, reconciles imports, probes by writing, repairs recovery state, or
//!   claims that Apply is ready; all returned paths are display-only evidence.
//! - `authoring_store_inspect_revision3_installed_dataasset_v1` accepts only one exact managed
//!   revision-3 head, installed package-snapshot seals, game/Store roots, and a candidate ordinal.
//!   It rebuilds every native authority and returns bounded whole-package fixed-leaf inspection
//!   evidence without accepting package, output, or USMAP paths.
//! - `authoring_store_prepare_revision3_installed_dataasset_edit_v1` reopens that exact installed
//!   proof, re-inspects the server-selected candidate, applies one typed fixed-leaf edit wholly
//!   in memory, and returns an unpublished revision-3 stage candidate. It never accepts raw
//!   package bytes, receipt paths, output paths, or publication/deployment authority.
//! - `authoring_store_export_revision3_exact_snapshot_v2` derives the exact current Store closure,
//!   strictly reopens one deterministic no-compression ZIP, and publishes the restorable snapshot
//!   no-clobber outside the Store. It never mutates project, game, save, build, deployment, or
//!   runtime state.

mod authoring_content_revision3;
mod authoring_dataasset_build_revision3;
mod authoring_dataasset_package_index_revision3;
mod authoring_dataasset_revision3;
mod authoring_dialog_localization_edit_revision3;
mod authoring_dialog_localization_revision3;
mod authoring_dialog_revision3;
mod authoring_dialog_voice_slot_create_revision3;
mod authoring_dialog_voice_slot_remove_revision3;
mod authoring_history_revision3;
mod authoring_installed_dataasset_inspection_revision3;
mod authoring_item_patch_revision3;
mod authoring_npc_catalog;
mod authoring_project_build_plan_revision3;
mod authoring_project_compiler_revision3;
mod authoring_project_export_revision3;
mod authoring_project_import_revision3;
mod authoring_source_io;
mod authoring_store;
mod authoring_store_root_guard;
mod authoring_story_catalog;
mod authoring_story_compiler_revision3;
mod authoring_story_draft_remove_revision3;
mod authoring_story_npc_greeting_revision3;
mod authoring_story_npc_inspection_revision3;
mod authoring_story_npc_profile_revision3;
mod authoring_story_npc_revision3;
mod authoring_story_quest_context_revision3;
mod authoring_story_quest_inspection_revision3;
mod authoring_story_quest_outline_v2_revision3;
mod authoring_story_quest_revision3;
mod authoring_story_quest_transcript_revision3;
mod authoring_story_quest_transitions_revision3;
mod authoring_voice_batch_revision3;
mod authoring_voice_build_revision3;
mod authoring_voice_media_revision3;
mod authoring_voice_plan_revision3;
mod authoring_voice_preview_revision3;
mod authoring_voice_revision3;
mod authoring_voice_selection_revision3;
mod authoring_voice_take_remove_revision3;
mod authoring_voice_take_status_revision3;
mod authoring_voice_target_revision3;
mod dataasset;
mod mgr_preflight;
mod script_compile_report;
mod texture_preview;
mod transport;
mod voice;

use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::Write;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use image::ImageEncoder;

use gore_loc::{loc_store, paths};
use gore_modgen::gen::{gen_lua, OverridesConfig};
use gore_modgen::validate::validate_config;
use gore_reflect::model::ReflectionModel;

pub use transport::{
    gore_core_execute_v2, gore_core_response_free_v2, gore_core_transport_abi_v2,
    GoreCoreResponseV2,
};

/// Increment only when the current JSON command/response protocol changes incompatibly.
const CORE_PROTOCOL_ABI: u32 = 2;

/// Every command understood by [`dispatch`], kept in bytewise ascending order so capability
/// negotiation is deterministic across builds and platforms.
const CORE_COMMANDS: &[&str] = &[
    "audio_extract",
    "audio_list",
    "authoring_npc_archetype_catalog_v1_build_for_game_root",
    "authoring_read_dataasset_extract_receipt_v2",
    "authoring_store_build_revision3_reviewed_dataasset_v1",
    "authoring_store_build_revision3_voice_v1",
    "authoring_store_check_revision3_npc_compiler_v1",
    authoring_project_compiler_revision3::COMMAND,
    "authoring_store_check_revision3_quest_compiler_v1",
    authoring_project_export_revision3::COMMAND,
    "authoring_store_import_ogg",
    authoring_project_import_revision3::IMPORT_COMMAND,
    authoring_project_import_revision3::COMMAND,
    "authoring_store_inspect_revision3_installed_dataasset_v1",
    "authoring_store_inspect_revision3_npc_source_v1",
    "authoring_store_inspect_revision3_quest_source_v1",
    authoring_voice_media_revision3::COMMAND,
    "authoring_store_list_revision3_dataasset_stages_v1",
    authoring_history_revision3::LIST_COMMAND,
    authoring_voice_preview_revision3::COMMAND,
    "authoring_store_open_revision3",
    "authoring_store_open_revision3_head_bytes",
    authoring_project_build_plan_revision3::COMMAND,
    authoring_voice_batch_revision3::PLAN_COMMAND,
    authoring_voice_plan_revision3::COMMAND,
    "authoring_store_prepare_remove_revision3_dataasset_stage_v1",
    authoring_story_draft_remove_revision3::COMMAND,
    "authoring_store_prepare_revision3_checkpoint",
    "authoring_store_prepare_revision3_dataasset_edit_v1",
    "authoring_store_prepare_revision3_dataasset_stage_v1",
    "authoring_store_prepare_revision3_dialog_line_v1",
    "authoring_store_prepare_revision3_dialog_localization_edit_v1",
    authoring_dialog_voice_slot_create_revision3::COMMAND,
    authoring_dialog_voice_slot_remove_revision3::COMMAND,
    authoring_history_revision3::RESTORE_COMMAND,
    "authoring_store_prepare_revision3_installed_dataasset_edit_v1",
    authoring_item_patch_revision3::PREPARE_COMMAND,
    "authoring_store_prepare_revision3_npc_draft_v1",
    authoring_story_npc_greeting_revision3::COMMAND,
    authoring_story_npc_profile_revision3::COMMAND,
    "authoring_store_prepare_revision3_quest_context_edit_v1",
    "authoring_store_prepare_revision3_quest_draft_v3",
    "authoring_store_prepare_revision3_quest_outline_edit_v2",
    authoring_story_quest_transcript_revision3::COMMAND,
    "authoring_store_prepare_revision3_quest_transitions_edit_v1",
    "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1",
    authoring_voice_batch_revision3::PREPARE_COMMAND,
    authoring_voice_take_remove_revision3::COMMAND,
    "authoring_store_prepare_revision3_voice_take_selection_v1",
    "authoring_store_prepare_revision3_voice_take_status_v1",
    "authoring_store_prepare_revision3_voice_take_v1",
    "authoring_store_prepare_revision3_voice_target_v1",
    "authoring_store_read_revision3_content_index_v1",
    "authoring_store_read_revision3_dataasset_package_index_v1",
    "authoring_store_read_revision3_dialog_localization_edit_seed_v1",
    "authoring_store_read_revision3_dialog_localization_v1",
    authoring_item_patch_revision3::CATALOG_COMMAND,
    authoring_voice_preview_revision3::REGISTER_COMMAND,
    authoring_voice_preview_revision3::RELEASE_COMMAND,
    "authoring_store_verify_asset",
    "authoring_story_catalog_v1_build",
    "authoring_story_catalog_v1_build_for_game_root",
    "authoring_story_catalog_v1_read",
    "core_info",
    "dataasset_fixed_inspect_v1",
    "find_game",
    "generate_mod",
    "loc_extract",
    "loc_find",
    "loc_status",
    "mgr_analyze",
    "mgr_apply",
    "mgr_import",
    "mgr_library_list",
    mgr_preflight::COMMAND,
    "mgr_remove",
    "mgr_set_loadout",
    "mgr_status",
    "mgr_undeploy_all",
    "mod_build",
    "mod_deploy",
    "mod_undeploy",
    "script_compile_install_state_v1",
    "script_compile_report_v1",
    "script_emit_module",
    "script_list_modules",
    "texture_extract",
    "texture_index",
    texture_preview::READ_COMMAND,
    texture_preview::RELEASE_COMMAND,
    "validate",
    "voice_archive_extract",
    "voice_archive_list",
    "voice_archive_match_line",
    "voice_ogg_inspect_v1",
];

// The transport-v2 C ABI entry points live in `transport` and are re-exported above.

/// Pure entry point (no FFI) — also the test seam.
pub fn execute_json(input: &str) -> String {
    // The pure seam uses the same global response budget as the native transport.
    String::from_utf8(transport::execute_json_bounded(input))
        .expect("JSON transport output is always UTF-8")
}

fn err(code: &str, msg: impl Into<String>) -> Value {
    json!({"ok": false, "error": {"code": code, "message": msg.into()}})
}

/// Hard bound on the serialized `error.details` object. Messages are separately bounded at 4096
/// bytes by each command's own `Failure`; details is a larger, separate budget because it carries
/// whole generation seals rather than prose.
const MAX_ERROR_DETAILS_BYTES: usize = 8 * 1024;

/// At most this many supported generations are named. The list is a closed registry of three or so
/// rows; a cap keeps one growing table from ever deciding the size of an error response.
const MAX_ERROR_DETAILS_SUPPORTED_ENTRIES: usize = 16;

/// `err`, plus one bounded machine-readable object.
///
/// Deliberately a second function rather than a parameter on `err`: the other ~38 local `Failure`
/// types have nothing structured to say, and a refusal that carries no facts must keep emitting the
/// exact two-key object every consumer already validates. Details that do not fit the bound are
/// dropped rather than truncated — half a seal is worse than a sentence.
fn err_with_details(code: &str, msg: impl Into<String>, details: Value) -> Value {
    let fits = details.is_object()
        && serde_json::to_string(&details).is_ok_and(|wire| wire.len() <= MAX_ERROR_DETAILS_BYTES);
    if !fits {
        return err(code, msg);
    }
    json!({"ok": false, "error": {"code": code, "message": msg.into(), "details": details}})
}

/// The structured facts behind `CatalogError::UnsupportedGeneration`.
///
/// The error has carried both the observed triple and the supported ones all along; every consumer
/// received one sentence naming neither, so a user whose game updated could not see which of the
/// three inputs had moved. The seals serialize as themselves — `{byte_len, sha256}` per input —
/// which is the same shape the catalog document uses.
fn unsupported_generation_details(
    supported: &[gore_story_catalog::GameGenerationSeal],
    actual: &gore_story_catalog::GameGenerationSeal,
) -> Value {
    let supported: Vec<Value> = supported
        .iter()
        .take(MAX_ERROR_DETAILS_SUPPORTED_ENTRIES)
        .map(|seal| serde_json::to_value(seal).unwrap_or(Value::Null))
        .collect();
    json!({
        "kind": "unsupported_generation",
        "actual": serde_json::to_value(actual).unwrap_or(Value::Null),
        "supported": supported,
    })
}

const MAX_DISPATCH_COMMAND_BYTES: usize = 256;
const MAX_JSON_ENCODED_COMMAND_BYTES: usize = MAX_DISPATCH_COMMAND_BYTES * 6;
const MAX_DISPATCH_SCAN_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DispatchProbeError {
    InvalidJson,
    CommandTooLong,
}

/// Finds the top-level command without allocating attacker-sized JSON keys or payload values.
///
/// Strict raw routes must see their original wire under their command-local cap before a generic
/// `Value` tree exists. A command spelling is the only allocation here, and even its maximally
/// escaped wire form is capped before decoding. Unknown values are skipped with a fixed-depth
/// byte scanner. Once a raw command is found, its closed duplicate-safe parser owns all remaining
/// syntax and schema validation.
fn probe_dispatch_command(input: &str) -> Result<Option<String>, DispatchProbeError> {
    let bytes = input.as_bytes();
    let mut cursor = skip_json_whitespace(bytes, 0);
    if bytes.get(cursor) != Some(&b'{') {
        return Err(DispatchProbeError::InvalidJson);
    }
    cursor += 1;
    let mut last_command = None;

    loop {
        cursor = skip_json_whitespace(bytes, cursor);
        if bytes.get(cursor) == Some(&b'}') {
            cursor = skip_json_whitespace(bytes, cursor + 1);
            return if cursor == bytes.len() {
                Ok(last_command)
            } else {
                Err(DispatchProbeError::InvalidJson)
            };
        }

        let key_start = cursor;
        let key_end = scan_json_string(bytes, key_start)?;
        let is_command = json_string_is_command(input, key_start, key_end)?;
        cursor = skip_json_whitespace(bytes, key_end);
        if bytes.get(cursor) != Some(&b':') {
            return Err(DispatchProbeError::InvalidJson);
        }
        cursor = skip_json_whitespace(bytes, cursor + 1);

        if is_command {
            let command_end = scan_json_string(bytes, cursor)?;
            let encoded_len = command_end.saturating_sub(cursor + 2);
            if encoded_len > MAX_JSON_ENCODED_COMMAND_BYTES {
                return Err(DispatchProbeError::CommandTooLong);
            }
            let command: String = serde_json::from_str(&input[cursor..command_end])
                .map_err(|_| DispatchProbeError::InvalidJson)?;
            if command.len() > MAX_DISPATCH_COMMAND_BYTES {
                return Err(DispatchProbeError::CommandTooLong);
            }
            if command == mgr_preflight::COMMAND
                || command == "mgr_set_loadout"
                || revision3_store_raw_route(&command).is_some()
            {
                return Ok(Some(command));
            }
            last_command = Some(command);
            cursor = command_end;
        } else {
            cursor = skip_json_value(bytes, cursor)?;
        }

        cursor = skip_json_whitespace(bytes, cursor);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b'}') => {
                cursor = skip_json_whitespace(bytes, cursor + 1);
                return if cursor == bytes.len() {
                    Ok(last_command)
                } else {
                    Err(DispatchProbeError::InvalidJson)
                };
            }
            _ => return Err(DispatchProbeError::InvalidJson),
        }
    }
}

fn skip_json_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        cursor += 1;
    }
    cursor
}

fn scan_json_string(bytes: &[u8], start: usize) -> Result<usize, DispatchProbeError> {
    if bytes.get(start) != Some(&b'\"') {
        return Err(DispatchProbeError::InvalidJson);
    }
    let mut cursor = start + 1;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'\"' => return Ok(cursor + 1),
            b'\\' => {
                cursor += 1;
                match bytes.get(cursor) {
                    Some(b'\"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                        cursor += 1;
                    }
                    Some(b'u') => {
                        let digits = bytes
                            .get(cursor + 1..cursor + 5)
                            .ok_or(DispatchProbeError::InvalidJson)?;
                        if !digits.iter().all(u8::is_ascii_hexdigit) {
                            return Err(DispatchProbeError::InvalidJson);
                        }
                        cursor += 5;
                    }
                    _ => return Err(DispatchProbeError::InvalidJson),
                }
            }
            0x00..=0x1f => return Err(DispatchProbeError::InvalidJson),
            _ => cursor += 1,
        }
    }
    Err(DispatchProbeError::InvalidJson)
}

fn json_string_is_command(
    input: &str,
    start: usize,
    end: usize,
) -> Result<bool, DispatchProbeError> {
    const MAX_ENCODED_KEY_BYTES: usize = "command".len() * 6;
    if end.saturating_sub(start + 2) > MAX_ENCODED_KEY_BYTES {
        return Ok(false);
    }
    if &input[start..end] == "\"command\"" {
        return Ok(true);
    }
    let key: String =
        serde_json::from_str(&input[start..end]).map_err(|_| DispatchProbeError::InvalidJson)?;
    Ok(key == "command")
}

fn skip_json_value(bytes: &[u8], start: usize) -> Result<usize, DispatchProbeError> {
    match bytes.get(start) {
        Some(b'\"') => scan_json_string(bytes, start),
        Some(b'{' | b'[') => skip_json_container(bytes, start),
        Some(_) => {
            let mut cursor = start;
            while let Some(byte) = bytes.get(cursor) {
                if matches!(byte, b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t') {
                    break;
                }
                cursor += 1;
            }
            if cursor == start {
                Err(DispatchProbeError::InvalidJson)
            } else {
                Ok(cursor)
            }
        }
        None => Err(DispatchProbeError::InvalidJson),
    }
}

fn skip_json_container(bytes: &[u8], start: usize) -> Result<usize, DispatchProbeError> {
    let mut stack = [0_u8; MAX_DISPATCH_SCAN_DEPTH];
    let mut depth = 1_usize;
    stack[0] = bytes[start];
    let mut cursor = start + 1;

    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'\"' => cursor = scan_json_string(bytes, cursor)?,
            b'{' | b'[' => {
                if depth == MAX_DISPATCH_SCAN_DEPTH {
                    return Err(DispatchProbeError::InvalidJson);
                }
                stack[depth] = byte;
                depth += 1;
                cursor += 1;
            }
            b'}' | b']' => {
                let expected_open = if byte == b'}' { b'{' } else { b'[' };
                if stack[depth - 1] != expected_open {
                    return Err(DispatchProbeError::InvalidJson);
                }
                depth -= 1;
                cursor += 1;
                if depth == 0 {
                    return Ok(cursor);
                }
            }
            _ => cursor += 1,
        }
    }
    Err(DispatchProbeError::InvalidJson)
}

fn revision3_store_raw_route(command: &str) -> Option<fn(&str) -> Value> {
    match command {
        "authoring_store_build_revision3_reviewed_dataasset_v1" => Some(
            authoring_dataasset_build_revision3::build_revision3_reviewed_dataasset_v1_raw,
        ),
        "authoring_store_list_revision3_dataasset_stages_v1" => {
            Some(authoring_dataasset_revision3::list_raw)
        }
        "authoring_read_dataasset_extract_receipt_v2" => {
            Some(authoring_dataasset_revision3::read_extract_raw)
        }
        "authoring_store_build_revision3_voice_v1" => {
            Some(authoring_voice_build_revision3::build_revision3_voice_v1_raw)
        }
        authoring_voice_plan_revision3::COMMAND => {
            Some(authoring_voice_plan_revision3::plan_revision3_voice_v1_raw)
        }
        authoring_project_build_plan_revision3::COMMAND => Some(
            authoring_project_build_plan_revision3::plan_revision3_project_build_v1_raw,
        ),
        authoring_voice_preview_revision3::REGISTER_COMMAND => Some(
            authoring_voice_preview_revision3::register_revision3_voice_take_preview_v1_raw,
        ),
        authoring_voice_preview_revision3::COMMAND => Some(
            authoring_voice_preview_revision3::materialize_revision3_voice_take_preview_v1_raw,
        ),
        authoring_voice_preview_revision3::RELEASE_COMMAND => Some(
            authoring_voice_preview_revision3::release_revision3_voice_take_preview_v1_raw,
        ),
        authoring_voice_batch_revision3::PLAN_COMMAND => {
            Some(authoring_voice_batch_revision3::plan_revision3_voice_batch_v1_raw)
        }
        "authoring_store_check_revision3_npc_compiler_v1" => Some(
            authoring_story_compiler_revision3::check_revision3_npc_compiler_v1_raw,
        ),
        authoring_project_compiler_revision3::COMMAND => Some(
            authoring_project_compiler_revision3::check_revision3_project_compiler_v1_raw,
        ),
        "authoring_store_check_revision3_quest_compiler_v1" => Some(
            authoring_story_compiler_revision3::check_revision3_quest_compiler_v1_raw,
        ),
        authoring_project_export_revision3::COMMAND => Some(
            authoring_project_export_revision3::export_revision3_exact_snapshot_v2_raw,
        ),
        authoring_project_import_revision3::IMPORT_COMMAND => Some(
            authoring_project_import_revision3::import_revision3_exact_snapshot_v2_raw,
        ),
        authoring_project_import_revision3::COMMAND => Some(
            authoring_project_import_revision3::inspect_revision3_exact_snapshot_v2_raw,
        ),
        "authoring_store_inspect_revision3_installed_dataasset_v1" => Some(
            authoring_installed_dataasset_inspection_revision3::inspect_revision3_installed_dataasset_v1_raw,
        ),
        "authoring_store_inspect_revision3_npc_source_v1" => Some(
            authoring_story_npc_inspection_revision3::inspect_revision3_npc_source_v1_raw,
        ),
        "authoring_store_inspect_revision3_quest_source_v1" => Some(
            authoring_story_quest_inspection_revision3::inspect_revision3_quest_source_v1_raw,
        ),
        authoring_voice_media_revision3::COMMAND => Some(
            authoring_voice_media_revision3::inspect_revision3_voice_take_media_v1_raw,
        ),
        authoring_history_revision3::LIST_COMMAND => {
            Some(authoring_history_revision3::list_revision3_history_v1_raw)
        }
        "authoring_store_open_revision3" => Some(authoring_store::open_revision3_raw),
        "authoring_store_open_revision3_head_bytes" => {
            Some(authoring_store::open_revision3_head_bytes_raw)
        }
        "authoring_store_prepare_revision3_checkpoint" => {
            Some(authoring_store::prepare_revision3_checkpoint_raw)
        }
        "authoring_store_prepare_remove_revision3_dataasset_stage_v1" => {
            Some(authoring_dataasset_revision3::prepare_remove_raw)
        }
        authoring_story_draft_remove_revision3::COMMAND => Some(
            authoring_story_draft_remove_revision3::prepare_revision3_story_draft_removal_v1_raw,
        ),
        "authoring_store_prepare_revision3_dataasset_edit_v1" => {
            Some(authoring_dataasset_revision3::prepare_edit_raw)
        }
        "authoring_store_prepare_revision3_dataasset_stage_v1" => {
            Some(authoring_dataasset_revision3::prepare_raw)
        }
        "authoring_store_prepare_revision3_dialog_line_v1" => {
            Some(authoring_dialog_revision3::prepare_revision3_dialog_line_v1_raw)
        }
        "authoring_store_prepare_revision3_dialog_localization_edit_v1" => Some(
            authoring_dialog_localization_edit_revision3::prepare_revision3_dialog_localization_edit_v1_raw,
        ),
        authoring_history_revision3::RESTORE_COMMAND => Some(
            authoring_history_revision3::prepare_revision3_history_restore_v1_raw,
        ),
        "authoring_store_prepare_revision3_installed_dataasset_edit_v1" => Some(
            authoring_installed_dataasset_inspection_revision3::prepare_revision3_installed_dataasset_edit_v1_raw,
        ),
        authoring_item_patch_revision3::PREPARE_COMMAND => Some(
            authoring_item_patch_revision3::prepare_revision3_item_patch_v1_raw,
        ),
        "authoring_store_prepare_revision3_npc_draft_v1" => {
            Some(authoring_story_npc_revision3::prepare_revision3_npc_draft_v1_raw)
        }
        authoring_story_npc_greeting_revision3::COMMAND => Some(
            authoring_story_npc_greeting_revision3::prepare_revision3_npc_greeting_v1_raw,
        ),
        authoring_story_npc_profile_revision3::COMMAND => Some(
            authoring_story_npc_profile_revision3::prepare_revision3_npc_profile_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_quest_context_edit_v1" => Some(
            authoring_story_quest_context_revision3::prepare_revision3_quest_context_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_quest_draft_v3" => {
            Some(authoring_story_quest_revision3::prepare_revision3_quest_draft_v3_raw)
        }
        "authoring_store_prepare_revision3_quest_outline_edit_v2" => Some(
            authoring_story_quest_outline_v2_revision3::prepare_revision3_quest_outline_edit_v2_raw,
        ),
        authoring_story_quest_transcript_revision3::COMMAND => Some(
            authoring_story_quest_transcript_revision3::prepare_revision3_quest_transcript_v1_raw,
        ),
        "authoring_store_prepare_revision3_quest_transitions_edit_v1" => Some(
            authoring_story_quest_transitions_revision3::prepare_revision3_quest_transitions_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1" => Some(
            authoring_installed_dataasset_inspection_revision3::prepare_revision3_reviewed_installed_dataasset_edit_v1_raw,
        ),
        authoring_dialog_voice_slot_create_revision3::COMMAND => Some(
            authoring_dialog_voice_slot_create_revision3::prepare_revision3_dialog_voice_slot_creation_v1_raw,
        ),
        authoring_dialog_voice_slot_remove_revision3::COMMAND => Some(
            authoring_dialog_voice_slot_remove_revision3::prepare_revision3_dialog_voice_slot_removal_v1_raw,
        ),
        authoring_voice_take_remove_revision3::COMMAND => Some(
            authoring_voice_take_remove_revision3::prepare_revision3_voice_take_removal_v1_raw,
        ),
        authoring_voice_batch_revision3::PREPARE_COMMAND => Some(
            authoring_voice_batch_revision3::prepare_revision3_voice_batch_v1_raw,
        ),
        "authoring_store_prepare_revision3_voice_take_selection_v1" => {
            Some(authoring_voice_selection_revision3::prepare_revision3_voice_take_selection_v1_raw)
        }
        "authoring_store_prepare_revision3_voice_take_status_v1" => {
            Some(authoring_voice_take_status_revision3::prepare_revision3_voice_take_status_v1_raw)
        }
        "authoring_store_prepare_revision3_voice_take_v1" => {
            Some(authoring_voice_revision3::prepare_revision3_voice_take_v1_raw)
        }
        "authoring_store_prepare_revision3_voice_target_v1" => {
            Some(authoring_voice_target_revision3::prepare_revision3_voice_target_v1_raw)
        }
        "authoring_store_read_revision3_content_index_v1" => {
            Some(authoring_content_revision3::read_revision3_content_index_v1_raw)
        }
        "authoring_store_read_revision3_dataasset_package_index_v1" => Some(
            authoring_dataasset_package_index_revision3::read_revision3_dataasset_package_index_v1_raw,
        ),
        "authoring_store_read_revision3_dialog_localization_edit_seed_v1" => Some(
            authoring_dialog_localization_edit_revision3::read_revision3_dialog_localization_edit_seed_v1_raw,
        ),
        "authoring_store_read_revision3_dialog_localization_v1" => Some(
            authoring_dialog_localization_revision3::read_revision3_dialog_localization_v1_raw,
        ),
        authoring_item_patch_revision3::CATALOG_COMMAND => Some(
            authoring_item_patch_revision3::read_revision3_item_catalog_v1_raw,
        ),
        "script_compile_install_state_v1" => {
            Some(script_compile_report::install_state_v1_raw)
        }
        "script_compile_report_v1" => Some(script_compile_report::compile_report_v1_raw),
        _ => None,
    }
}

fn dispatch(input: &str) -> Value {
    let command = match probe_dispatch_command(input) {
        Ok(Some(command)) => command,
        Ok(None) => String::new(),
        Err(DispatchProbeError::CommandTooLong) => {
            return err("BAD_REQUEST", "request command exceeds its bounded length");
        }
        Err(DispatchProbeError::InvalidJson) => {
            return err("BAD_REQUEST", "invalid request json");
        }
    };
    if command == mgr_preflight::COMMAND {
        return mgr_preflight::mgr_preflight_v1_raw(input);
    }
    if command == "mgr_set_loadout" {
        return mgr_set_loadout_raw(input);
    }
    // These security-sensitive Store routes see the original wire before any generic
    // payload `Value` exists. Route-local parsers enforce their smaller envelope caps before
    // decoding nested strings.
    if let Some(route) = revision3_store_raw_route(&command) {
        return route(input);
    }
    let req: Value = match serde_json::from_str(input) {
        Ok(value) => value,
        Err(e) => return err("BAD_REQUEST", format!("invalid request json: {e}")),
    };
    let command = req.get("command").and_then(Value::as_str).unwrap_or("");
    let payload = req.get("payload").cloned().unwrap_or(Value::Null);
    match command {
        "core_info" => core_info(),
        "dataasset_fixed_inspect_v1" => dataasset::fixed_inspect_v1_raw(input),
        "generate_mod" => generate_mod(payload),
        "validate" => validate(payload),
        "loc_status" => loc_status(),
        "loc_find" => loc_find(payload),
        "loc_extract" => loc_extract(payload),
        "find_game" => find_game(),
        "audio_list" => audio_list(payload),
        "audio_extract" => audio_extract(payload),
        "mod_build" => mod_build(payload),
        "mod_deploy" => mod_deploy(payload),
        "mod_undeploy" => mod_undeploy(payload),
        "mgr_library_list" => mgr_library_list(payload),
        "mgr_import" => mgr_import(payload),
        "mgr_remove" => mgr_remove(payload),
        "mgr_analyze" => mgr_analyze(payload),
        "mgr_apply" => mgr_apply(payload),
        "mgr_status" => mgr_status(payload),
        "mgr_undeploy_all" => mgr_undeploy_all(payload),
        "texture_index" => texture_index(payload),
        "texture_extract" => texture_extract(payload),
        texture_preview::READ_COMMAND => texture_preview::read(payload),
        texture_preview::RELEASE_COMMAND => texture_preview::release(payload),
        "script_list_modules" => script_list_modules(payload),
        "script_emit_module" => script_emit_module(payload),
        "authoring_npc_archetype_catalog_v1_build_for_game_root" => {
            authoring_npc_catalog::build_for_game_root_v1(payload)
        }
        "authoring_story_catalog_v1_build" => {
            authoring_story_catalog::build_story_catalog_v1(payload)
        }
        "authoring_story_catalog_v1_build_for_game_root" => {
            authoring_story_catalog::build_story_catalog_for_game_root_v1(payload)
        }
        "authoring_story_catalog_v1_read" => {
            authoring_story_catalog::read_story_catalog_v1(payload)
        }
        "authoring_store_import_ogg" => authoring_store::import_ogg(payload),
        "authoring_store_verify_asset" => authoring_store::verify_asset(payload),
        "voice_archive_list" => voice::archive_list(payload),
        "voice_archive_match_line" => voice::archive_match_line(payload),
        "voice_archive_extract" => voice::archive_extract(payload),
        "voice_ogg_inspect_v1" => voice::ogg_inspect_v1_raw(input),
        other => err("UNKNOWN_COMMAND", format!("unknown command: {other}")),
    }
}

/// Cheap, read-only protocol handshake. This deliberately does not inspect the game,
/// filesystem, caches, or any other mutable state.
fn core_info() -> Value {
    json!({
        "ok": true,
        "abi": CORE_PROTOCOL_ABI,
        "version": env!("CARGO_PKG_VERSION"),
        "commands": CORE_COMMANDS,
    })
}

/// `{ok, present, meta?, catalog_path, dir}` — is the shared catalog extracted?
fn loc_status() -> Value {
    let present = loc_store::catalog_present();
    // Only report metadata while the catalog file is present, so stale sidecar
    // meta can't describe a catalog that no longer exists.
    let meta = if present { loc_store::status() } else { None };
    json!({
        "ok": true,
        "present": present,
        "meta": meta,
        "catalog_path": paths::loc_catalog_path().display().to_string(),
        "dir": paths::shared_data_dir().display().to_string(),
    })
}

/// `{ok, found, path?}` — auto-detect (or resolve `payload.lcache`) the .lcache.
fn loc_find(payload: Value) -> Value {
    let hint = payload
        .get("lcache")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let found = loc_store::resolve_lcache(hint.as_deref());
    json!({
        "ok": true,
        "found": found.is_some(),
        "path": found.map(|p| p.display().to_string()),
    })
}

/// Extract to the shared catalog. `payload.lcache` is an optional path hint.
fn loc_extract(payload: Value) -> Value {
    let hint = payload
        .get("lcache")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    match loc_store::extract(hint.as_deref()) {
        Ok(meta) => json!({ "ok": true, "meta": meta }),
        Err(gore_loc::loc_store::LocStoreError::NotFound) => err(
            "LCACHE_NOT_FOUND",
            "could not find AlkimiaLocalization .lcache (auto-detect failed); pick it manually",
        ),
        Err(e) => err("EXTRACT_FAILED", e.to_string()),
    }
}

/// `{ok, found, game_root?, exe?}` — auto-detect the game install via Steam.
fn find_game() -> Value {
    let root = gore_loc::discover::find_game_root();
    let exe = gore_loc::discover::find_game_exe();
    json!({
        "ok": true,
        "found": root.is_some(),
        "game_root": root.map(|p| p.display().to_string()),
        "exe": exe.map(|p| p.display().to_string()),
    })
}

/// Read a bank's PRISTINE bytes for listing/preview. The live bank is the source of truth when it
/// isn't injected yet (a single FSB5): that covers an un-deployed bank AND the case where a
/// `restore` or a Steam update refreshed the live bank, so the Audio tab never lists/previews
/// obsolete samples from a stale `*.gore-bak`. Only when the live bank is already injected
/// (>1 FSB5) do we fall back to the backup, which holds the true original. Mirrors the CLI's
/// `read_pristine_bank`.
/// The install's recovered FMOD key (from the `gore_fmod_key.json` gore-dump writes to
/// `Binaries/Win64`) when present and valid, else the compiled-in constant — so the Audio tab can
/// browse/preview banks on installs whose key changed after a game patch.
fn resolve_fmod_key_for_bank(bank: &str) -> Vec<u8> {
    // bank == <...>/G1R/Content/FMOD/Desktop/<file>.bank; G1R is 4 levels up, then Binaries/Win64.
    if let Some(g1r) = std::path::Path::new(bank).ancestors().nth(4) {
        let key_file = g1r
            .join("Binaries")
            .join("Win64")
            .join("gore_fmod_key.json");
        if let Ok(bytes) = std::fs::read(&key_file) {
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                if v.get("found").and_then(Value::as_bool).unwrap_or(false) {
                    if let Some(k) = v.get("encryption_key").and_then(Value::as_str) {
                        if !k.is_empty() {
                            return k.as_bytes().to_vec();
                        }
                    }
                }
            }
        }
    }
    gore_fmod::GOTHIC_STUDIO_KEY.to_vec()
}

fn read_bank_pristine(bank: &str) -> std::io::Result<Vec<u8>> {
    let live = std::fs::read(bank)?;
    if !gore_fmod::is_pristine_bank(&live) {
        // The live bank is injected (or unparseable) — its true pristine is the backup, if any.
        let bak = format!("{bank}.gore-bak");
        if std::path::Path::new(&bak).exists() {
            return std::fs::read(&bak);
        }
    }
    Ok(live)
}

fn generate_mod(payload: Value) -> Value {
    let cfg: OverridesConfig = match serde_json::from_value(payload) {
        Ok(c) => c,
        Err(e) => return err("BAD_CONFIG", format!("invalid overrides config: {e}")),
    };
    let lua = gen_lua(&cfg);
    json!({
        "ok": true,
        "files": {
            "enabled.txt": "",
            "Scripts/main.lua": lua,
        }
    })
}

/// `{bank}` → `{ok, codec, samples:[{index,name,freq,channels,seconds}]}`
fn audio_list(payload: Value) -> Value {
    let Some(bank) = payload.get("bank").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'bank'");
    };
    let bytes = match read_bank_pristine(bank) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading bank: {e}")),
    };
    let key = match payload.get("key").and_then(Value::as_str) {
        Some("") => return err("BAD_KEY", "encryption key must not be empty"),
        Some(k) => k.as_bytes().to_vec(),
        None => resolve_fmod_key_for_bank(bank),
    };
    let fsb = match gore_fmod::bank_fsb0(&bytes, &key) {
        Ok(f) => f,
        Err(e) => return err("DECODE", e),
    };
    let samples: Vec<Value> = fsb
        .samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let secs = if s.freq > 0 { s.num_samples as f64 / s.freq as f64 } else { 0.0 };
            json!({"index": i, "name": s.name, "freq": s.freq, "channels": s.channels, "seconds": secs})
        })
        .collect();
    json!({"ok": true, "codec": format!("{:?}", fsb.codec), "samples": samples})
}

/// `{bank, sample}` → `{ok, ogg_path}` — extract one Vorbis sample to a temp .ogg for preview.
fn audio_extract(payload: Value) -> Value {
    let (Some(bank), Some(sample)) = (
        payload.get("bank").and_then(Value::as_str),
        payload.get("sample").and_then(Value::as_str),
    ) else {
        return err("BAD_REQUEST", "missing 'bank' or 'sample'");
    };
    let bytes = match read_bank_pristine(bank) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading bank: {e}")),
    };
    let key = match payload.get("key").and_then(Value::as_str) {
        Some("") => return err("BAD_KEY", "encryption key must not be empty"),
        Some(k) => k.as_bytes().to_vec(),
        None => resolve_fmod_key_for_bank(bank),
    };
    let (block, fsb) = match gore_fmod::decrypt_fsb0(&bytes, &key) {
        Ok(v) => v,
        Err(e) => return err("DECODE", e),
    };
    let Some(index) = fsb.samples.iter().position(|s| s.name == sample) else {
        return err("NOT_FOUND", format!("sample not found: {sample}"));
    };
    let wav = match gore_fmod::extract_wav(&block, &fsb, index) {
        Ok(o) => o,
        Err(e) => return err("EXTRACT", e),
    };
    let dir = std::env::temp_dir().join("gore-fmod-preview");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return err("IO", format!("temp dir: {e}"));
    }
    let safe: String = sample
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe}.wav"));
    if let Err(e) = std::fs::write(&path, &wav) {
        return err("IO", format!("writing wav: {e}"));
    }
    json!({"ok": true, "ogg_path": path.display().to_string(), "wav_path": path.display().to_string()})
}

/// The one texture catalog that can authorize previews in this process.
#[derive(Default)]
struct LiveTextureIndexStore {
    indexes: Mutex<VecDeque<Arc<gore_tex::index::TextureIndex>>>,
}

impl LiveTextureIndexStore {
    const MAX_RETAINED_GENERATIONS: usize = 2;

    fn retain(&self, index: Arc<gore_tex::index::TextureIndex>) {
        let mut current = self
            .indexes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        current.retain(|candidate| candidate.build_id != index.build_id);
        current.push_back(index);
        while current.len() > Self::MAX_RETAINED_GENERATIONS {
            current.pop_front();
        }
    }

    fn get_exact(&self, expected_build_id: &str) -> Option<Arc<gore_tex::index::TextureIndex>> {
        let current = self
            .indexes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        current
            .iter()
            .find(|index| index.build_id.as_str() == expected_build_id)
            .cloned()
    }
}

fn live_texture_index_store() -> &'static LiveTextureIndexStore {
    static STORE: OnceLock<LiveTextureIndexStore> = OnceLock::new();
    STORE.get_or_init(LiveTextureIndexStore::default)
}

/// `{ok, build_id, count, entries:{path:package_id_str}}` — load the cached index, building it
/// if absent or if `payload.rebuild` is true. `payload.game` = install dir.
fn texture_index(payload: Value) -> Value {
    let Some(request) = payload.as_object().filter(|request| {
        request.len() == 2 && request.contains_key("game") && request.contains_key("rebuild")
    }) else {
        return err("BAD_REQUEST", "texture index request is invalid");
    };
    let game = match request.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g),
        None => return err("BAD_REQUEST", "missing game"),
    };
    let rebuild = match request.get("rebuild").and_then(Value::as_bool) {
        Some(value) => value,
        None => return err("BAD_REQUEST", "texture index rebuild flag is invalid"),
    };
    let usmap = match gore_tex::paths::usmap(&game) {
        Ok(p) => p,
        Err(e) => return err("USMAP", e.to_string()),
    };
    let utoc = match gore_tex::paths::main_container(&game) {
        Ok(p) => p,
        Err(e) => return err("CONTAINER", e.to_string()),
    };
    let build_id = match gore_tex::index::build_id_for(&utoc, &usmap) {
        Ok(value) => value,
        Err(e) => return err("SOURCE_FINGERPRINT", e.to_string()),
    };
    let cache = gore_tex::paths::texture_index_path_for_build(&build_id);
    // Each cryptographically sealed installed source owns one immutable, disposable cache entry.
    // A successful call establishes separate process-local preview authority below.
    let cached = if rebuild {
        None
    } else {
        gore_tex::index::TextureIndex::load_current(&cache, &build_id)
    };
    let (idx, built_new) = match cached {
        Some(i) => (i, false),
        None => {
            let i = match gore_tex::index::build_index(&utoc, &build_id) {
                Ok(i) => i,
                Err(e) => return err("INDEX_BUILD", e.to_string()),
            };
            (i, true)
        }
    };
    let observed_after = match gore_tex::index::build_id_for(&utoc, &usmap) {
        Ok(value) => value,
        Err(e) => return err("GENERATION_CHANGED", e.to_string()),
    };
    if observed_after != idx.build_id {
        return err(
            "GENERATION_CHANGED",
            "game texture generation changed while the index was being loaded",
        );
    }
    if built_new {
        // The disk cache is a disposable performance artifact. Persist only
        // after proving the source stayed stable, but never let cache I/O
        // suppress the already validated in-memory catalog.
        let _ = idx.save_atomic_immutable(&cache);
    }
    let _ = gore_tex::index::pin_and_prune_managed_texture_cache(&cache);
    let entries: serde_json::Map<String, Value> = idx
        .entries
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.to_string())))
        .collect();
    let response_build_id = idx.build_id.clone();
    let count = idx.entries.len();
    let live_index = Arc::new(idx);
    let response =
        json!({ "ok": true, "build_id": response_build_id, "count": count, "entries": entries });
    live_texture_index_store().retain(live_index);
    response
}

/// Extract one indexed texture into a native-owned, token-bound PNG preview.
/// No temporary path crosses the FFI; callers read and release the opaque
/// capability through `texture_preview_read` / `texture_preview_release`.
fn texture_extract(payload: Value) -> Value {
    let Some(request) = payload.as_object().filter(|request| {
        request.len() == 4
            && request.contains_key("game")
            && request.contains_key("expected_build_id")
            && request.contains_key("asset")
            && request.contains_key("package_id")
    }) else {
        return err("BAD_REQUEST", "texture extract request is invalid");
    };
    let game = match request.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g),
        None => return err("BAD_REQUEST", "missing game"),
    };
    let utoc = match gore_tex::paths::main_container(&game) {
        Ok(p) => p,
        Err(e) => return err("CONTAINER", e.to_string()),
    };
    let usmap = match gore_tex::paths::usmap(&game) {
        Ok(p) => p,
        Err(e) => return err("USMAP", e.to_string()),
    };
    let expected_build_id = match request.get("expected_build_id").and_then(Value::as_str) {
        Some(value)
            if !value.is_empty()
                && value.len() <= 512
                && value.trim() == value
                && !value.chars().any(char::is_control) =>
        {
            value
        }
        _ => return err("BAD_REQUEST", "missing or invalid expected_build_id"),
    };
    let index = match live_texture_index_store().get_exact(expected_build_id) {
        Some(index) => index,
        None => {
            return err(
                "INDEX_REQUIRED",
                "matching live texture index is unavailable",
            )
        }
    };
    let observed_before = match gore_tex::index::build_id_for(&utoc, &usmap) {
        Ok(value) => value,
        Err(e) => return err("SOURCE_FINGERPRINT", e.to_string()),
    };
    if observed_before != expected_build_id {
        return err(
            "STALE_TEXTURE_INDEX",
            "installed texture generation no longer matches the selected index",
        );
    }
    let asset = match request.get("asset").and_then(Value::as_str) {
        Some(value)
            if !value.is_empty()
                && value.len() <= 1024
                && value.trim() == value
                && !value.chars().any(char::is_control) =>
        {
            value
        }
        _ => return err("BAD_REQUEST", "missing asset"),
    };
    let package_id_text = match request.get("package_id").and_then(Value::as_str) {
        Some(value) => value,
        None => return err("BAD_REQUEST", "missing package_id"),
    };
    let package_id = match package_id_text.parse::<u64>() {
        Ok(value) if value.to_string() == package_id_text => value,
        _ => return err("BAD_REQUEST", "package_id must be canonical decimal u64"),
    };
    if index.entries.get(asset).copied() != Some(package_id) {
        return err(
            "TEXTURE_IDENTITY_MISMATCH",
            "asset and package_id do not match the selected texture index",
        );
    }
    // Reserve one of the two process-wide native preview slots before any
    // expensive container conversion or decode begins. The pending capability
    // is automatically cancelled and its exact output handle deleted on every
    // failure path.
    let mut pending_preview = match texture_preview::PendingPreview::create() {
        Ok(preview) => preview,
        Err(e) => return err(e.code, e.message),
    };
    let leaf = asset.rsplit('/').next().unwrap_or("texture").to_string();
    let (info, px) = match gore_tex::index::extract_by_package_id(&utoc, &usmap, package_id, &leaf)
    {
        Ok(value) => value,
        Err(e) => return err("EXTRACT", e.to_string()),
    };
    const MAX_PREVIEW_RGBA_BYTES: usize = 128 * 1024 * 1024;
    let rgba_len = match px.len().checked_mul(4) {
        Some(value) if value <= MAX_PREVIEW_RGBA_BYTES => value,
        _ => return err("TOO_LARGE", "decoded texture preview exceeds 128 MiB"),
    };
    let expected_pixels = match (info.width as usize).checked_mul(info.height as usize) {
        Some(value) => value,
        None => return err("TOO_LARGE", "texture dimensions overflow"),
    };
    if px.len() != expected_pixels {
        return err(
            "DECODE",
            "decoded pixel count does not match texture dimensions",
        );
    }
    let mut buf = Vec::with_capacity(rgba_len);
    for p in px {
        buf.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
    }
    // The PNG is written directly into a native-owned, delete-on-close handle.
    // No ambient temp path crosses the FFI boundary.
    let encoder = image::codecs::png::PngEncoder::new(pending_preview.file_mut());
    if let Err(e) = encoder.write_image(
        &buf,
        info.width,
        info.height,
        image::ExtendedColorType::Rgba8,
    ) {
        return err("PNG", format!("encoding preview: {e}"));
    }
    if let Err(e) = pending_preview.file_mut().flush() {
        return err("PNG", format!("flushing preview: {e}"));
    }
    let observed_after = match gore_tex::index::build_id_for(&utoc, &usmap) {
        Ok(value) => value,
        Err(e) => return err("GENERATION_CHANGED", e.to_string()),
    };
    if observed_after != expected_build_id {
        return err(
            "GENERATION_CHANGED",
            "installed texture generation changed while extracting the preview",
        );
    }
    // `replaceable` is the AUTHORITATIVE capability flag the UI gates the Replace
    // button on (always a plain bool). It requires BOTH a re-encodable
    // texture shape (`replace_supported`) AND a deployable mount root: the deploy
    // path can only place /Game and /Engine assets (`content_mount_rel`), so an
    // asset under any other root (e.g. /DatasmithContent) must report not
    // replaceable rather than appear supported and fail later at build/deploy.
    // `is_virtual`/`vt_layers` are exposed for diagnostics.
    // The request always carries the exact indexed asset identity, so mount
    // eligibility is evaluated without a package-only fallback.
    let deployable_root = gore_tex::paths::content_mount_rel(asset).is_some();
    let replaceable = gore_tex::decode::replace_supported(&info) && deployable_root;
    // Publication is deliberately the final fallible action. Once the token is
    // visible, only response construction remains and the caller owns release.
    let published_preview = match pending_preview.publish() {
        Ok(preview) => preview,
        Err(e) => return err(e.code, e.message),
    };
    json!({ "ok": true, "build_id": expected_build_id, "preview_token": published_preview.token, "png_byte_len": published_preview.byte_len, "png_sha256": published_preview.sha256, "width": info.width, "height": info.height,
        "format": info.format, "replaceable": replaceable,
        "is_virtual": info.is_virtual, "vt_layers": info.vt_layers, "mipmapped": info.mipmapped })
}

/// Load native arities from the `GORE_AS_BINDS` env path if set, else a `Binds.Cache` sitting next
/// to `cache_file`, if present. Mirrors the CLI's `load_native_api` (quietly — no logging) so the
/// CLI and mod-studio resolve the same arities when `GORE_AS_BINDS` is set.
fn as_native_api(cache_file: &std::path::Path) -> Option<gore_as::cache::binds::NativeApi> {
    let path = match std::env::var_os("GORE_AS_BINDS") {
        Some(p) => std::path::PathBuf::from(p),
        None => cache_file.parent()?.join("Binds.Cache"),
    };
    if !path.exists() {
        return None;
    }
    gore_as::cache::binds::NativeApi::load(&path)
}

/// `{cache}` → `{ok, modules:[{name, file}]}` — list modules in a precompiled cache.
fn script_list_modules(payload: Value) -> Value {
    let Some(cache) = payload.get("cache").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'cache'");
    };
    let bytes = match std::fs::read(cache) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading cache: {e}")),
    };
    let mods = match gore_as::cache::model::parse_modules(&bytes) {
        Ok(m) => m,
        Err(e) => return err("PARSE", format!("parsing cache: {e}")),
    };
    let modules: Vec<Value> = mods
        .iter()
        .map(|m| json!({"name": m.name, "file": m.file}))
        .collect();
    json!({"ok": true, "modules": modules})
}

/// `{cache, module}` → `{ok, source}` — emit recompilable .as for one module.
fn script_emit_module(payload: Value) -> Value {
    let (Some(cache), Some(module)) = (
        payload.get("cache").and_then(Value::as_str),
        payload.get("module").and_then(Value::as_str),
    ) else {
        return err("BAD_REQUEST", "missing 'cache' or 'module'");
    };
    let bytes = match std::fs::read(cache) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading cache: {e}")),
    };
    let mut refs = match gore_as::cache::refs::RefResolver::build(&bytes) {
        Ok(r) => r,
        Err(e) => return err("RESOLVER", format!("{e}")),
    };
    let mods = match gore_as::cache::model::parse_modules(&bytes) {
        Ok(m) => m,
        Err(e) => return err("PARSE", format!("{e}")),
    };
    let Some(module_index) = mods.iter().position(|candidate| candidate.name == module) else {
        return err("NOT_FOUND", format!("module not found: {module}"));
    };
    let prepared = match gore_as::cache::emit_all::PreparedEmit::new(
        &mods,
        &mut refs,
        as_native_api(std::path::Path::new(cache)),
    ) {
        Ok(prepared) => prepared,
        Err(error) => return err("EMIT", format!("preparing cache: {error}")),
    };
    let source = match prepared.emit_module(module_index) {
        Ok(source) => source,
        Err(error) => return err("EMIT", format!("emitting module: {error}")),
    };
    json!({"ok": true, "source": source})
}

/// `{out_dir, spec:BuildSpec}` → build the unified bundle into `out_dir`.
fn mod_build(payload: Value) -> Value {
    let Some(out_dir) = payload.get("out_dir").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'out_dir'");
    };
    let spec_val = payload.get("spec").cloned().unwrap_or(Value::Null);
    let spec: gore_mod::BuildSpec = match serde_json::from_value(spec_val) {
        Ok(s) => s,
        Err(e) => return err("BAD_SPEC", format!("invalid build spec: {e}")),
    };
    let bundle = match gore_mod::build_bundle(&spec) {
        Ok(b) => b,
        Err(e) => return err("BUILD_FAILED", e.to_string()),
    };
    let dir = std::path::Path::new(out_dir).join(&spec.meta.name);
    if let Err(e) = gore_mod::write_bundle(&dir, &bundle) {
        return err("IO", e.to_string());
    }
    json!({
        "ok": true,
        "bundle_dir": dir.display().to_string(),
        "components": bundle.manifest.components.len(),
        "files": bundle.files.len(),
    })
}

/// `{bundle_dir, game_root}` → deploy.
fn mod_deploy(payload: Value) -> Value {
    let (Some(bundle_dir), Some(game_root)) = (
        payload.get("bundle_dir").and_then(Value::as_str),
        payload.get("game_root").and_then(Value::as_str),
    ) else {
        return err("BAD_REQUEST", "missing 'bundle_dir' or 'game_root'");
    };
    match gore_mod::deploy(
        std::path::Path::new(bundle_dir),
        std::path::Path::new(game_root),
    ) {
        Ok(rec) => deploy_response(rec),
        Err(e) => err("DEPLOY_FAILED", e.to_string()),
    }
}

/// A successful deploy as this route reports it.
///
/// The two localization lists travel BESIDE the record, not inside it. They describe one run and
/// the record is read back by undeploy and status, which is why `DeployRecord` skips them when it
/// serializes — and why serializing the record was dropping them here as well: a caller through
/// this route was told the deployment had succeeded, with nothing to say that part of its
/// localization patch had never been written.
///
/// Two fields, because they call for opposite responses. A skipped edit was never written and the
/// spec has to change; a shadowed edit WAS written and is simply not the one the game reads, so
/// calling it "did not apply" invites undoing a deployment that worked. The CLI prints them as two
/// things for the same reason.
///
/// Always present, empty or not. A client that cannot tell "no warnings" from "this build does not
/// report them" is back where it started.
fn deploy_response(mut rec: gore_mod::DeployRecord) -> Value {
    let loc_skipped = std::mem::take(&mut rec.loc_skipped);
    let loc_shadowed = std::mem::take(&mut rec.loc_shadowed);
    json!({
        "ok": true,
        "record": serde_json::to_value(rec).unwrap_or(Value::Null),
        "loc_skipped": loc_skipped,
        "loc_shadowed": loc_shadowed,
    })
}

/// `{game_root}` → undeploy the active mod.
fn mod_undeploy(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    match gore_mod::undeploy(std::path::Path::new(game_root)) {
        Ok(rec) => json!({"ok": true, "record": serde_json::to_value(rec).unwrap_or(Value::Null)}),
        Err(e) => err("UNDEPLOY_FAILED", e.to_string()),
    }
}

// ── mod-manager (`mgr_*`) commands ─────────────────────────────────────────────
// Store-backed mgr commands use either the shared Store or one explicit `library_dir` /
// `loadout_path` pair. A lone override cannot identify which two roots belong together and could
// otherwise reconcile a custom file against unrelated shared state.
fn mgr_store_paths_from_options(
    library: Option<&str>,
    loadout: Option<&str>,
) -> Result<(PathBuf, PathBuf), String> {
    match (library, loadout) {
        (None, None) => Ok((
            gore_mod::mgr::paths::library_dir(),
            gore_mod::mgr::paths::loadout_path(),
        )),
        (Some(library), Some(loadout)) => {
            Ok((PathBuf::from(library), PathBuf::from(loadout)))
        }
        _ => Err(
            "library_dir and loadout_path overrides must be supplied together so they identify one manager store"
                .to_string(),
        ),
    }
}

fn mgr_store_paths(payload: &Value) -> Result<(PathBuf, PathBuf), String> {
    let path = |key: &str| match payload.get(key) {
        None => Ok(None),
        Some(Value::String(path)) => Ok(Some(path.as_str())),
        Some(_) => Err(format!("'{key}' must be a string")),
    };
    mgr_store_paths_from_options(path("library_dir")?, path("loadout_path")?)
}

fn mgr_store_paths_or_error(payload: &Value) -> Result<(PathBuf, PathBuf), Value> {
    match mgr_store_paths(payload) {
        Ok(paths) => Ok(paths),
        Err(message) => Err(err("BAD_REQUEST", message)),
    }
}

/// Project one stored manager component onto the GUI wire contract.
///
/// `coverage` is deliberately injected here instead of added to `ComponentInfo`: library sidecars
/// and their fingerprints remain byte/shape compatible while every Manager response receives the
/// current derived truth.
fn mgr_component_wire_value(component: &gore_mod::mgr::ComponentInfo) -> Value {
    let mut value = serde_json::to_value(component).unwrap_or(Value::Null);
    if let Value::Object(object) = &mut value {
        object.insert(
            "coverage".to_string(),
            serde_json::to_value(component.footprint_coverage())
                .unwrap_or_else(|_| Value::String("opaque".to_string())),
        );
    }
    value
}

/// The single shared Manager entry projection for both library listing and import responses.
fn mgr_entry_wire_value(entry: &gore_mod::mgr::ModEntryMeta) -> Value {
    let mut value = serde_json::to_value(entry).unwrap_or(Value::Null);
    if let Value::Object(object) = &mut value {
        object.insert(
            "components".to_string(),
            Value::Array(
                entry
                    .components
                    .iter()
                    .map(mgr_component_wire_value)
                    .collect(),
            ),
        );
    }
    value
}

/// `{library_dir?, loadout_path?}` → `{ok, mods:[ModEntryMeta], loadout:Loadout}`. Overrides are a
/// pair. Raw library +
/// loadout from one strict, natively reconciled Store snapshot.
fn mgr_library_list(payload: Value) -> Value {
    let (lib, lo_path) = match mgr_store_paths_or_error(&payload) {
        Ok(paths) => paths,
        Err(error) => return error,
    };
    let store = match gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        Ok(store) => store,
        Err(e) => return err("IO", e.to_string()),
    };
    json!({
        "ok": true,
        "mods": Value::Array(store.mods().iter().map(mgr_entry_wire_value).collect()),
        "loadout": serde_json::to_value(store.loadout()).unwrap_or(Value::Null),
    })
}

/// `{path, library_dir?, loadout_path?}` → (Store overrides are a pair)
/// `{ok, entry:ModEntryMeta, disposition, matched_by}` — import a source into the
/// library AND register it in the loadout (disabled) if not already present. Mirrors
/// `gore mgr import`: without this, a GUI-imported mod is invisible to apply/status/analyze (which
/// read the on-disk loadout, not the GUI's in-memory reconcile) until some other mutation.
fn mgr_import(payload: Value) -> Value {
    let Some(path) = payload.get("path").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'path'");
    };
    let (lib, lo_path) = match mgr_store_paths_or_error(&payload) {
        Ok(paths) => paths,
        Err(error) => return error,
    };
    let outcome = match gore_mod::mgr::import::import_detailed(&lib, std::path::Path::new(path)) {
        Ok(outcome) => outcome,
        Err(error @ gore_mod::mgr::import::ImportError::DuplicateAmbiguous { .. }) => {
            return err_with_details(
                "IMPORT_DUPLICATE_AMBIGUOUS",
                error.to_string(),
                json!({"candidate_ids": error.candidate_ids()}),
            );
        }
        Err(error @ gore_mod::mgr::import::ImportError::IdentityConflict { .. }) => {
            let candidates = error
                .conflict_candidates()
                .expect("identity-conflict variant carries bounded candidates");
            return err_with_details(
                "IMPORT_IDENTITY_CONFLICT",
                error.to_string(),
                json!({"candidates": candidates}),
            );
        }
        Err(error) => return err("IMPORT_FAILED", error.to_string()),
    };
    let entry = &outcome.entry;
    // Library publication and loadout registration remain two explicit commits. Import has
    // released Library before Store acquires its canonical composite roots and reconciles state.
    // Surface a loadout read/write failure instead of returning ok: apply/status/analyze read the
    // on-disk loadout, so a swallowed error here would leave the imported mod invisible to them. A
    // missing loadout file loads as an empty default (not an error), so first-time imports still
    // register normally.
    if let Err(e) = gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        return err(
            "IO",
            format!(
                "imported '{}' into the library but failed to reconcile the loadout: {e}",
                entry.id
            ),
        );
    }
    json!({
        "ok": true,
        "entry": mgr_entry_wire_value(entry),
        "disposition": outcome.disposition.as_str(),
        "matched_by": outcome.matched_by.as_str(),
    })
}

/// `{id, library_dir?, loadout_path?}` → `{ok, removed:bool}` — delete a library entry and its
/// paired loadout slot (absent id → removed:false).
fn mgr_remove(payload: Value) -> Value {
    let Some(id) = payload.get("id").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'id'");
    };
    let (lib, lo_path) = match mgr_store_paths_or_error(&payload) {
        Ok(paths) => paths,
        Err(error) => return error,
    };
    let removed = match gore_mod::mgr::import::remove(&lib, id) {
        Ok(removed) => removed,
        Err(e) => return err("BAD_REQUEST", e.to_string()),
    };
    if let Err(e) = gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        return err(
            "IO",
            format!("removed '{id}' from the library but failed to reconcile the loadout: {e}"),
        );
    }
    json!({"ok": true, "removed": removed})
}

/// `{loadout:Loadout, library_dir?, loadout_path?}` → `{ok}` — persist the loadout; Store path
/// overrides are a pair.
const MAX_MGR_SET_LOADOUT_REQUEST_BYTES: usize = 1024 * 1024 + 64 * 1024;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MgrSetLoadoutRawRequest {
    command: String,
    payload: MgrSetLoadoutRawPayload,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MgrSetLoadoutRawPayload {
    loadout: gore_mod::mgr::Loadout,
    #[serde(default)]
    library_dir: Option<String>,
    #[serde(default)]
    loadout_path: Option<String>,
}

fn mgr_set_loadout_raw(input: &str) -> Value {
    if input.len() > MAX_MGR_SET_LOADOUT_REQUEST_BYTES {
        return err(
            "BAD_REQUEST",
            "mgr_set_loadout request exceeds its bounded size",
        );
    }
    let request: MgrSetLoadoutRawRequest = match serde_json::from_str(input) {
        Ok(request) => request,
        Err(error) => {
            return err(
                "BAD_REQUEST",
                format!("invalid mgr_set_loadout request: {error}"),
            )
        }
    };
    if request.command != "mgr_set_loadout" {
        return err("BAD_REQUEST", "invalid mgr_set_loadout command");
    }
    if let Err(error) = request.payload.loadout.validate() {
        return err("BAD_REQUEST", format!("invalid loadout: {error}"));
    }
    let encoded_loadout = match serde_json::to_vec_pretty(&request.payload.loadout) {
        Ok(encoded) => encoded,
        Err(error) => return err("BAD_REQUEST", format!("invalid loadout: {error}")),
    };
    if encoded_loadout.len() > 1024 * 1024 {
        return err("BAD_REQUEST", "loadout exceeds its 1048576-byte limit");
    }
    let (lib, lo_path) = match mgr_store_paths_from_options(
        request.payload.library_dir.as_deref(),
        request.payload.loadout_path.as_deref(),
    ) {
        Ok(paths) => paths,
        Err(message) => return err("BAD_REQUEST", message),
    };
    match gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        Ok(mut store) => match store.replace_loadout(request.payload.loadout) {
            Ok(()) => json!({"ok": true}),
            Err(e) => err("IO", e.to_string()),
        },
        Err(e) => err("IO", e.to_string()),
    }
}

/// `{library_dir?, loadout_path?}` → `{ok, conflicts:[Conflict]}` — paired Store overrides; pure
/// conflict analysis of the
/// enabled loadout against the library.
fn mgr_analyze(payload: Value) -> Value {
    let (lib, lo_path) = match mgr_store_paths_or_error(&payload) {
        Ok(paths) => paths,
        Err(error) => return error,
    };
    let store = match gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        Ok(store) => store,
        Err(e) => return err("ANALYZE_FAILED", e.to_string()),
    };
    let conflicts = store.analyze();
    json!({"ok": true, "conflicts": serde_json::to_value(&conflicts).unwrap_or(Value::Null)})
}

/// `{game_root, library_dir?, loadout_path?}` → `{ok, report:ApplyReport}` — paired Store overrides;
/// realize the enabled
/// loadout into one manager deployment. A studio deploy in the way maps to STUDIO_DEPLOY_ACTIVE.
fn mgr_apply(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    let (lib, lo_path) = match mgr_store_paths_or_error(&payload) {
        Ok(paths) => paths,
        Err(error) => return error,
    };
    let store = match gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        Ok(store) => store,
        Err(e) => return err("APPLY_FAILED", e.to_string()),
    };
    match store.apply(std::path::Path::new(game_root)) {
        Ok(report) => {
            json!({"ok": true, "report": serde_json::to_value(&report).unwrap_or(Value::Null)})
        }
        Err(e) => {
            let msg = e.to_string();
            // The apply engine signals a blocking studio deployment as `STUDIO_DEPLOY_ACTIVE:<name>`;
            // surface it as its own code carrying just the mod name so the UI can prompt accordingly.
            match msg.strip_prefix("STUDIO_DEPLOY_ACTIVE:") {
                Some(name) => err("STUDIO_DEPLOY_ACTIVE", name.to_string()),
                None => err("APPLY_FAILED", msg),
            }
        }
    }
}

/// `{game_root, library_dir?, loadout_path?}` → `{ok, status:ManagerStatus}` — paired Store
/// overrides; diff deployed vs
/// target loadout. `library_dir` lets status fingerprint each enabled mod's current content so a
/// same-id re-import (update) is reported as changes-pending rather than in-sync.
fn mgr_status(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    let (lib, lo_path) = match mgr_store_paths_or_error(&payload) {
        Ok(paths) => paths,
        Err(error) => return error,
    };
    let store = match gore_mod::mgr::store::StoreSnapshot::open(&lib, &lo_path) {
        Ok(store) => store,
        Err(e) => return err("STATUS_FAILED", e.to_string()),
    };
    match store.status(std::path::Path::new(game_root)) {
        Ok(status) => {
            json!({"ok": true, "status": serde_json::to_value(&status).unwrap_or(Value::Null)})
        }
        Err(e) => err("STATUS_FAILED", e.to_string()),
    }
}

/// `{game_root}` → `{ok, removed:bool}` — undeploy whatever is active (manager or studio).
fn mgr_undeploy_all(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    match gore_mod::mgr::apply::undeploy_all(std::path::Path::new(game_root)) {
        Ok(removed) => json!({"ok": true, "removed": removed}),
        Err(e) => err("UNDEPLOY_FAILED", e.to_string()),
    }
}

fn validate(payload: Value) -> Value {
    let cfg: OverridesConfig = match payload
        .get("config")
        .cloned()
        .and_then(|c| serde_json::from_value(c).ok())
    {
        Some(c) => c,
        None => return err("BAD_CONFIG", "missing/invalid 'config'"),
    };
    let model: ReflectionModel = match payload
        .get("model")
        .cloned()
        .and_then(|m| serde_json::from_value(m).ok())
    {
        Some(m) => m,
        None => return err("BAD_MODEL", "missing/invalid 'model'"),
    };
    let errors: Vec<String> = validate_config(&cfg, &model)
        .iter()
        .map(ToString::to_string)
        .collect();
    json!({ "ok": true, "valid": errors.is_empty(), "errors": errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_generation_details_are_closed_exact_and_bounded() {
        let supported = gore_story_catalog::known_supported_generations();
        let mut actual = supported[0].clone();
        actual.executable.byte_len += 1;

        let details = unsupported_generation_details(&supported, &actual);
        assert_eq!(
            details,
            json!({
                "kind": "unsupported_generation",
                "actual": actual,
                "supported": supported,
            })
        );
        assert_eq!(details.as_object().unwrap().len(), 3);
        assert!(serde_json::to_string(&details).unwrap().len() <= MAX_ERROR_DETAILS_BYTES);

        let many_supported = vec![
            gore_story_catalog::known_supported_generations()[0].clone();
            MAX_ERROR_DETAILS_SUPPORTED_ENTRIES + 3
        ];
        let bounded = unsupported_generation_details(&many_supported, &actual);
        assert_eq!(
            bounded["supported"].as_array().unwrap().len(),
            MAX_ERROR_DETAILS_SUPPORTED_ENTRIES
        );
    }

    #[test]
    fn structured_error_details_are_optional_and_dropped_whole_when_invalid_or_oversized() {
        let accepted =
            err_with_details("FIXTURE", "fixture", json!({"kind": "fixture", "value": 1}));
        assert_eq!(
            accepted["error"]["details"],
            json!({"kind": "fixture", "value": 1})
        );

        for rejected in [
            Value::Null,
            json!(["not", "an", "object"]),
            json!({"padding": "x".repeat(MAX_ERROR_DETAILS_BYTES)}),
        ] {
            let response = err_with_details("FIXTURE", "fixture", rejected);
            assert_eq!(
                response,
                json!({"ok": false, "error": {"code": "FIXTURE", "message": "fixture"}})
            );
        }
    }

    #[test]
    fn live_texture_index_store_retains_two_overlapping_exact_generations() {
        let store = LiveTextureIndexStore::default();
        assert!(store.get_exact("generation-a").is_none());

        let first = Arc::new(gore_tex::index::TextureIndex {
            build_id: "generation-a".to_string(),
            entries: std::collections::BTreeMap::from([("/Game/A".to_string(), 1)]),
        });
        store.retain(Arc::clone(&first));

        let selected = store.get_exact("generation-a").unwrap();
        assert!(Arc::ptr_eq(&selected, &first));
        assert_eq!(selected.entries.get("/Game/A"), Some(&1));
        assert!(store.get_exact("generation-b").is_none());

        let second = Arc::new(gore_tex::index::TextureIndex {
            build_id: "generation-b".to_string(),
            entries: std::collections::BTreeMap::from([("/Game/B".to_string(), 2)]),
        });
        store.retain(Arc::clone(&second));

        assert!(Arc::ptr_eq(
            &store.get_exact("generation-a").unwrap(),
            &first,
        ));
        assert!(Arc::ptr_eq(
            &store.get_exact("generation-b").unwrap(),
            &second,
        ));

        let third = Arc::new(gore_tex::index::TextureIndex {
            build_id: "generation-c".to_string(),
            entries: std::collections::BTreeMap::from([("/Game/C".to_string(), 3)]),
        });
        store.retain(Arc::clone(&third));

        assert!(store.get_exact("generation-a").is_none());
        assert!(Arc::ptr_eq(
            &store.get_exact("generation-b").unwrap(),
            &second,
        ));
        assert!(Arc::ptr_eq(
            &store.get_exact("generation-c").unwrap(),
            &third,
        ));
    }

    #[test]
    fn raw_store_route_caps_input_before_building_a_generic_payload_tree() {
        let request = format!(
            "{{\"command\":\"authoring_store_read_revision3_content_index_v1\",\"payload\":{{\"padding\":\"{}\"}}}}",
            "x".repeat(2 * 1024 * 1024)
        );
        let response: Value = serde_json::from_str(&execute_json(&request)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_INPUT_LIMIT"
        );
    }

    #[test]
    fn raw_store_route_is_found_after_an_attacker_sized_top_level_key() {
        let request = format!(
            "{{\"{}\":null,\"command\":\"authoring_store_read_revision3_content_index_v1\",\"payload\":{{}}}}",
            "attacker-key".repeat(192 * 1024)
        );
        let response: Value = serde_json::from_str(&execute_json(&request)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_INPUT_LIMIT"
        );
    }

    #[test]
    fn raw_store_route_is_found_after_an_attacker_sized_payload() {
        let request = format!(
            "{{\"payload\":{{\"padding\":\"{}\"}},\"command\":\"authoring_store_read_revision3_content_index_v1\"}}",
            "x".repeat(2 * 1024 * 1024)
        );
        let response: Value = serde_json::from_str(&execute_json(&request)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_INPUT_LIMIT"
        );
    }

    #[test]
    fn dispatch_rejects_an_attacker_sized_command_before_decoding_it() {
        let request = format!(
            "{{\"command\":\"{}\",\"payload\":{{}}}}",
            "x".repeat(2 * 1024 * 1024)
        );
        let response: Value = serde_json::from_str(&execute_json(&request)).unwrap();
        assert_eq!(response["error"]["code"], "BAD_REQUEST");
        assert_eq!(
            response["error"]["message"],
            "request command exceeds its bounded length"
        );
    }

    #[test]
    fn dispatch_recognizes_a_bounded_escaped_raw_command_key_and_value() {
        let request = r#"{"\u0063ommand":"authoring_store_read_revision3_content_index_\u00761","payload":{"padding":"x"}}"#;
        let response: Value = serde_json::from_str(&execute_json(request)).unwrap();
        assert_eq!(
            response["error"]["code"],
            "AUTHORING_REVISION3_CONTENT_REQUEST_INVALID"
        );
    }

    #[test]
    fn generate_mod_returns_files_with_cdo_pattern() {
        let req = r#"{"command":"generate_mod","payload":{
            "meta":{"name":"M","delay_ms":0},
            "override":[{"class":"ItFo_Apple","field":"m_Value","value_int":500}]
        }}"#;
        let v: Value = serde_json::from_str(&execute_json(req)).unwrap();
        assert_eq!(v["ok"], true);
        let lua = v["files"]["Scripts/main.lua"].as_str().unwrap();
        assert!(lua.contains("ItFo_Apple"));
        assert!(lua.contains("Default__"));
        assert_eq!(v["files"]["enabled.txt"], "");
    }

    #[test]
    fn loc_status_reports_shared_catalog_path() {
        let v: Value = serde_json::from_str(&execute_json(r#"{"command":"loc_status"}"#)).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["catalog_path"].as_str().unwrap().contains("gore"));
        assert!(v.get("present").is_some());
    }

    #[test]
    fn core_info_has_exact_schema_and_sorted_commands() {
        let v: Value = serde_json::from_str(&execute_json(
            r#"{"command":"core_info","payload":{"ignored":true}}"#,
        ))
        .unwrap();

        assert_eq!(
            v,
            json!({
                "ok": true,
                "abi": 2,
                "version": env!("CARGO_PKG_VERSION"),
                "commands": [
                    "audio_extract",
                    "audio_list",
                    "authoring_npc_archetype_catalog_v1_build_for_game_root",
                    "authoring_read_dataasset_extract_receipt_v2",
                    "authoring_store_build_revision3_reviewed_dataasset_v1",
                    "authoring_store_build_revision3_voice_v1",
                    "authoring_store_check_revision3_npc_compiler_v1",
                    "authoring_store_check_revision3_project_compiler_v1",
                    "authoring_store_check_revision3_quest_compiler_v1",
                    "authoring_store_export_revision3_exact_snapshot_v2",
                    "authoring_store_import_ogg",
                    "authoring_store_import_revision3_exact_snapshot_v2",
                    "authoring_store_inspect_revision3_exact_snapshot_v2",
                    "authoring_store_inspect_revision3_installed_dataasset_v1",
                    "authoring_store_inspect_revision3_npc_source_v1",
                    "authoring_store_inspect_revision3_quest_source_v1",
                    "authoring_store_inspect_revision3_voice_take_media_v1",
                    "authoring_store_list_revision3_dataasset_stages_v1",
                    "authoring_store_list_revision3_history_v1",
                    "authoring_store_materialize_revision3_voice_take_preview_v1",
                    "authoring_store_open_revision3",
                    "authoring_store_open_revision3_head_bytes",
                    "authoring_store_plan_revision3_project_build_v1",
                    "authoring_store_plan_revision3_voice_batch_v1",
                    "authoring_store_plan_revision3_voice_v1",
                    "authoring_store_prepare_remove_revision3_dataasset_stage_v1",
                    "authoring_store_prepare_remove_revision3_story_draft_v1",
                    "authoring_store_prepare_revision3_checkpoint",
                    "authoring_store_prepare_revision3_dataasset_edit_v1",
                    "authoring_store_prepare_revision3_dataasset_stage_v1",
                    "authoring_store_prepare_revision3_dialog_line_v1",
                    "authoring_store_prepare_revision3_dialog_localization_edit_v1",
                    "authoring_store_prepare_revision3_dialog_voice_slot_creation_v1",
                    "authoring_store_prepare_revision3_dialog_voice_slot_removal_v1",
                    "authoring_store_prepare_revision3_history_restore_v1",
                    "authoring_store_prepare_revision3_installed_dataasset_edit_v1",
                    "authoring_store_prepare_revision3_item_patch_v1",
                    "authoring_store_prepare_revision3_npc_draft_v1",
                    "authoring_store_prepare_revision3_npc_greeting_v1",
                    "authoring_store_prepare_revision3_npc_profile_edit_v1",
                    "authoring_store_prepare_revision3_quest_context_edit_v1",
                    "authoring_store_prepare_revision3_quest_draft_v3",
                    "authoring_store_prepare_revision3_quest_outline_edit_v2",
                    "authoring_store_prepare_revision3_quest_transcript_v1",
                    "authoring_store_prepare_revision3_quest_transitions_edit_v1",
                    "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1",
                    "authoring_store_prepare_revision3_voice_batch_v1",
                    "authoring_store_prepare_revision3_voice_take_removal_v1",
                    "authoring_store_prepare_revision3_voice_take_selection_v1",
                    "authoring_store_prepare_revision3_voice_take_status_v1",
                    "authoring_store_prepare_revision3_voice_take_v1",
                    "authoring_store_prepare_revision3_voice_target_v1",
                    "authoring_store_read_revision3_content_index_v1",
                    "authoring_store_read_revision3_dataasset_package_index_v1",
                    "authoring_store_read_revision3_dialog_localization_edit_seed_v1",
                    "authoring_store_read_revision3_dialog_localization_v1",
                    "authoring_store_read_revision3_item_catalog_v1",
                    "authoring_store_register_revision3_voice_take_preview_v1",
                    "authoring_store_release_revision3_voice_take_preview_v1",
                    "authoring_store_verify_asset",
                    "authoring_story_catalog_v1_build",
                    "authoring_story_catalog_v1_build_for_game_root",
                    "authoring_story_catalog_v1_read",
                    "core_info",
                    "dataasset_fixed_inspect_v1",
                    "find_game",
                    "generate_mod",
                    "loc_extract",
                    "loc_find",
                    "loc_status",
                    "mgr_analyze",
                    "mgr_apply",
                    "mgr_import",
                    "mgr_library_list",
                    "mgr_preflight_v1",
                    "mgr_remove",
                    "mgr_set_loadout",
                    "mgr_status",
                    "mgr_undeploy_all",
                    "mod_build",
                    "mod_deploy",
                    "mod_undeploy",
                    "script_compile_install_state_v1",
                    "script_compile_report_v1",
                    "script_emit_module",
                    "script_list_modules",
                    "texture_extract",
                    "texture_index",
                    "texture_preview_read",
                    "texture_preview_release",
                    "validate",
                    "voice_archive_extract",
                    "voice_archive_list",
                    "voice_archive_match_line",
                    "voice_ogg_inspect_v1",
                ],
            })
        );

        let commands = v["commands"].as_array().unwrap();
        assert!(commands
            .windows(2)
            .all(|pair| pair[0].as_str() < pair[1].as_str()));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_story_catalog_v1_build"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_story_catalog_v1_build_for_game_root"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_story_catalog_v1_read"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_npc_archetype_catalog_v1_build_for_game_root"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_list_revision3_dataasset_stages_v1"));
        assert!(commands.iter().any(
            |command| command == "authoring_store_materialize_revision3_voice_take_preview_v1"
        ));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_register_revision3_voice_take_preview_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_release_revision3_voice_take_preview_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_inspect_revision3_npc_source_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_inspect_revision3_quest_source_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_inspect_revision3_voice_take_media_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_read_dataasset_extract_receipt_v2"));
        assert!(commands.iter().any(
            |command| command == "authoring_store_prepare_remove_revision3_dataasset_stage_v1"
        ));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_remove_revision3_story_draft_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_dataasset_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_dataasset_stage_v1"));
        assert!(commands
            .iter()
            .any(|command| command
                == "authoring_store_prepare_revision3_dialog_localization_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_npc_draft_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_npc_greeting_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_npc_profile_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_context_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_draft_v3"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_outline_edit_v2"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_transcript_v1"));
        assert!(commands.iter().any(
            |command| command == "authoring_store_prepare_revision3_quest_transitions_edit_v1"
        ));
        assert!(commands.iter().any(|command| command
            == "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_item_patch_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_take_selection_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_take_status_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_take_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_target_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_build_revision3_reviewed_dataasset_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_build_revision3_voice_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_plan_revision3_voice_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_plan_revision3_project_build_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_read_revision3_content_index_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_read_revision3_dataasset_package_index_v1"));
        assert!(commands
            .iter()
            .any(|command| command
                == "authoring_store_read_revision3_dialog_localization_edit_seed_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_read_revision3_dialog_localization_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_read_revision3_item_catalog_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "voice_archive_match_line"));
    }

    #[test]
    fn unknown_command_errors() {
        let v: Value = serde_json::from_str(&execute_json(r#"{"command":"nope"}"#)).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "UNKNOWN_COMMAND");
    }

    #[test]
    fn bad_config_errors() {
        let v: Value = serde_json::from_str(&execute_json(
            r#"{"command":"generate_mod","payload":{"meta":{"name":"M"}}}"#,
        ))
        .unwrap();
        assert_eq!(v["ok"], false);
    }

    #[test]
    fn script_list_modules_requires_cache() {
        let v: Value = serde_json::from_str(&execute_json(
            r#"{"command":"script_list_modules","payload":{}}"#,
        ))
        .unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "BAD_REQUEST");
    }

    #[test]
    fn script_emit_module_requires_args() {
        let v: Value = serde_json::from_str(&execute_json(
            r#"{"command":"script_emit_module","payload":{"cache":"x"}}"#,
        ))
        .unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "BAD_REQUEST");
    }

    // ── mgr_* commands ─────────────────────────────────────────────────────────
    // Round-tripped through `execute_json` (the real FFI seam), each against temp `library_dir`
    // / `loadout_path` overrides so nothing touches the shared per-user store.

    fn script_fixture_sia(value: &str) -> Vec<u8> {
        if value.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut output = (value.len() as i32).to_le_bytes().to_vec();
        output.extend_from_slice(value.as_bytes());
        output.push(0);
        output
    }

    fn script_fixture_fstring(value: &str) -> Vec<u8> {
        let mut output = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        output.extend_from_slice(value.as_bytes());
        output.push(0);
        output
    }

    fn empty_script_fixture_cache() -> Vec<u8> {
        let module = "FfiFixture";
        let mut output = vec![0u8; 16];
        output.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
        output.extend_from_slice(&1u32.to_le_bytes());
        output.extend_from_slice(&script_fixture_fstring(module));
        output.extend_from_slice(&script_fixture_sia(module));
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i64.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&script_fixture_sia(""));
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&0i32.to_le_bytes());
        output.extend_from_slice(&script_fixture_sia("FfiFixture.as"));
        output.extend_from_slice(&0i32.to_le_bytes());
        for _ in 0..gore_as::cache::tables::N_TABLES {
            output.extend_from_slice(&0i32.to_le_bytes());
        }
        output
    }

    #[test]
    fn script_emit_module_matches_the_prepared_emit_all_file() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("fixture.cache");
        let bytes = empty_script_fixture_cache();
        std::fs::write(&cache, &bytes).unwrap();

        let request = json!({
            "command": "script_emit_module",
            "payload": {"cache": cache, "module": "FfiFixture"}
        });
        let response: Value = serde_json::from_str(&execute_json(&request.to_string())).unwrap();
        assert_eq!(response["ok"], true, "{response}");

        let modules = gore_as::cache::model::parse_modules(&bytes).unwrap();
        let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).unwrap();
        let output = temp.path().join("all");
        gore_as::cache::emit_all::emit_all_tree(&modules, &mut refs, None, &output).unwrap();
        let expected = std::fs::read_to_string(output.join("FfiFixture.as")).unwrap();
        assert_eq!(response["source"].as_str(), Some(expected.as_str()));
    }

    /// Run a command with a JSON `payload` value, returning the parsed response.
    fn mgr_call(command: &str, payload: Value) -> Value {
        let req = json!({"command": command, "payload": payload});
        serde_json::from_str(&execute_json(&req.to_string())).unwrap()
    }

    fn write_mgr_library_entry(library: &std::path::Path, id: &str) {
        let entry = library.join(id);
        std::fs::create_dir_all(&entry).unwrap();
        let meta = gore_mod::mgr::ModEntryMeta {
            id: id.into(),
            kind: gore_mod::mgr::ModKind::ForeignPak,
            name: id.into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-01-01T00:00:00Z".into(),
            source: String::new(),
            components: Vec::new(),
        };
        std::fs::write(
            entry.join(gore_mod::mgr::META_FILE),
            serde_json::to_vec(&meta).unwrap(),
        )
        .unwrap();
    }

    /// Build a real goremod bundle (one item override → a UE4SS Lua component) under `root` and
    /// return its dir, so `mgr_import` has a genuine bundle to ingest.
    fn write_goremod_bundle(root: &std::path::Path, name: &str) -> PathBuf {
        use gore_mod::{build_bundle, BuildSpec, ModMeta};
        use gore_modgen::gen::{OverrideValue, SingleOverride};
        let spec = BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: "1.0".into(),
                author: "t".into(),
            },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ItFo_Apple".into(),
                field: "m_Value".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(500),
            }],
            loc_edits: Default::default(),
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        let dir = root.join(name);
        gore_mod::write_bundle(&dir, &bundle).unwrap();
        dir
    }

    fn copy_mgr_test_tree(source: &std::path::Path, destination: &std::path::Path) {
        std::fs::create_dir_all(destination).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_mgr_test_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn write_mgr_test_zip(source: &std::path::Path, destination: &std::path::Path) {
        fn add(
            writer: &mut zip::ZipWriter<std::fs::File>,
            root: &std::path::Path,
            directory: &std::path::Path,
        ) {
            let mut entries: Vec<_> = std::fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                if entry.file_type().unwrap().is_dir() {
                    add(writer, root, &entry.path());
                    continue;
                }
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                writer
                    .start_file(relative, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer
                    .write_all(&std::fs::read(entry.path()).unwrap())
                    .unwrap();
            }
        }

        let mut writer = zip::ZipWriter::new(std::fs::File::create(destination).unwrap());
        add(&mut writer, source, source);
        writer.finish().unwrap();
    }

    fn mgr_visible_library_snapshot(root: &std::path::Path) -> Vec<(String, Option<Vec<u8>>)> {
        fn walk(
            root: &std::path::Path,
            current: &std::path::Path,
            out: &mut Vec<(String, Option<Vec<u8>>)>,
        ) {
            let mut entries: Vec<_> = std::fs::read_dir(current)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                if current == root && entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if entry.file_type().unwrap().is_dir() {
                    out.push((relative, None));
                    walk(root, &entry.path(), out);
                } else {
                    out.push((relative, Some(std::fs::read(entry.path()).unwrap())));
                }
            }
        }

        let mut snapshot = Vec::new();
        walk(root, root, &mut snapshot);
        snapshot
    }

    #[test]
    fn a_deploy_reports_the_localization_edits_it_could_not_write() {
        // `DeployRecord` skips both lists when it serializes, on purpose: they describe one run and
        // the record is read back by undeploy and status. Handing the serialized record straight
        // to the caller therefore dropped them, and a client on this route saw a successful
        // deployment with no sign that part of its localization patch had never been written.
        // The manager route reports its warnings; this one silently did not.
        let mut rec = gore_mod::DeployRecord {
            mod_name: "Test".into(),
            ..Default::default()
        };
        rec.loc_skipped
            .push("'itfo_cheese': this install's cache declares no 'english' language".into());
        rec.loc_shadowed.push(
            "'itfo_cheese': 'german' was written and 'german_new' is what the game reads".into(),
        );

        let v = super::deploy_response(rec);
        assert_eq!(v["ok"], true, "resp: {v}");
        assert_eq!(
            v["record"]["mod_name"], "Test",
            "the record still travels: {v}"
        );
        assert!(
            v["record"].get("loc_skipped").is_none(),
            "and still without the run-specific lists in it: {v}"
        );
        // Two fields, not one total: a skipped edit was never written and the spec has to change,
        // a shadowed edit landed and is merely not the one displayed. One edit can raise both.
        assert_eq!(v["loc_skipped"].as_array().unwrap().len(), 1, "{v}");
        assert!(
            v["loc_skipped"][0]
                .as_str()
                .unwrap()
                .contains("itfo_cheese"),
            "{v}"
        );
        assert_eq!(v["loc_shadowed"].as_array().unwrap().len(), 1, "{v}");
        assert!(
            v["loc_shadowed"][0]
                .as_str()
                .unwrap()
                .contains("german_new"),
            "{v}"
        );

        // Present and empty on a clean deploy, so "no warnings" is distinguishable from a build
        // that does not report them.
        let clean = super::deploy_response(gore_mod::DeployRecord::default());
        assert_eq!(clean["loc_skipped"].as_array().unwrap().len(), 0, "{clean}");
        assert_eq!(
            clean["loc_shadowed"].as_array().unwrap().len(),
            0,
            "{clean}"
        );
    }

    #[test]
    fn mgr_library_list_empty_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let v = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        assert_eq!(v["ok"], true, "resp: {v}");
        assert_eq!(v["mods"].as_array().unwrap().len(), 0);
        // A missing loadout file is the fresh default (format 1, no entries).
        assert_eq!(v["loadout"]["format"], 1);
        assert_eq!(v["loadout"]["entries"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn mgr_store_path_overrides_must_be_paired() {
        let default = mgr_store_paths_from_options(None, None).unwrap();
        assert_eq!(default.0, gore_mod::mgr::paths::library_dir());
        assert_eq!(default.1, gore_mod::mgr::paths::loadout_path());
        assert!(mgr_store_paths_from_options(Some("library"), None).is_err());
        assert!(mgr_store_paths_from_options(None, Some("loadout.json")).is_err());
        assert_eq!(
            mgr_store_paths_from_options(Some("library"), Some("loadout.json")).unwrap(),
            (PathBuf::from("library"), PathBuf::from("loadout.json")),
        );

        let malformed = mgr_library_list(json!({
            "library_dir": 7,
            "loadout_path": "loadout.json",
        }));
        assert_eq!(malformed["error"]["code"], "BAD_REQUEST");

        let temp = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("tests::mgr_unpaired_store_override_worker")
            .arg("--ignored")
            .arg("--nocapture")
            .env("GORE_FFI_UNPAIRED_STORE_ROOT", temp.path())
            .env("LOCALAPPDATA", temp.path().join("data"))
            .env("APPDATA", temp.path().join("data"))
            .env("XDG_DATA_HOME", temp.path().join("data"))
            .env("HOME", temp.path().join("home"))
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    #[ignore = "worker process with isolated shared Manager paths"]
    fn mgr_unpaired_store_override_worker() {
        let root = PathBuf::from(std::env::var_os("GORE_FFI_UNPAIRED_STORE_ROOT").unwrap());
        let default_library = gore_mod::mgr::paths::library_dir();
        let default_loadout = gore_mod::mgr::paths::loadout_path();
        let custom_library = root.join("custom-library");
        let custom_loadout = root.join("custom-loadout.json");

        write_mgr_library_entry(&default_library, "default-mod");
        write_mgr_library_entry(&custom_library, "custom-mod");
        let default_loadout_bytes =
            br#"{"format":1,"entries":[{"id":"default-mod","enabled":true}]}"#;
        let custom_loadout_bytes =
            br#"{"format":1,"entries":[{"id":"custom-mod","enabled":true}]}"#;
        std::fs::create_dir_all(default_loadout.parent().unwrap()).unwrap();
        std::fs::write(&default_loadout, default_loadout_bytes).unwrap();
        std::fs::write(&custom_loadout, custom_loadout_bytes).unwrap();

        let default_library_before = mgr_visible_library_snapshot(&default_library);
        let custom_library_before = mgr_visible_library_snapshot(&custom_library);
        let replacement = json!({"format":1,"entries":[]});
        for payload in [
            json!({
                "library_dir": custom_library,
                "loadout": replacement,
            }),
            json!({
                "loadout_path": custom_loadout,
                "loadout": replacement,
            }),
        ] {
            let request = json!({"command":"mgr_set_loadout", "payload":payload});
            let response: Value =
                serde_json::from_str(&execute_json(&request.to_string())).unwrap();
            assert_eq!(response["error"]["code"], "BAD_REQUEST", "{response}");
            assert!(response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("must be supplied together"));
            assert_eq!(
                std::fs::read(&default_loadout).unwrap(),
                default_loadout_bytes
            );
            assert_eq!(
                std::fs::read(&custom_loadout).unwrap(),
                custom_loadout_bytes
            );
            assert_eq!(
                mgr_visible_library_snapshot(&default_library),
                default_library_before
            );
            assert_eq!(
                mgr_visible_library_snapshot(&custom_library),
                custom_library_before
            );
        }

        let missing_source = root.join("missing.pak");
        let import = mgr_import(json!({
            "path": missing_source,
            "library_dir": custom_library,
        }));
        assert_eq!(import["error"]["code"], "BAD_REQUEST", "{import}");

        let remove = mgr_remove(json!({
            "id": "custom-mod",
            "library_dir": custom_library,
        }));
        assert_eq!(remove["error"]["code"], "BAD_REQUEST", "{remove}");
        assert_eq!(
            mgr_visible_library_snapshot(&custom_library),
            custom_library_before
        );
        assert_eq!(
            std::fs::read(&default_loadout).unwrap(),
            default_loadout_bytes
        );
        assert_eq!(
            std::fs::read(&custom_loadout).unwrap(),
            custom_loadout_bytes
        );

        let preflight = crate::mgr_preflight::mgr_preflight_v1_raw(
            &json!({
                "command":"mgr_preflight_v1",
                "payload":{
                    "game_root":root.join("game"),
                    "loadout_path":custom_loadout,
                }
            })
            .to_string(),
        );
        assert_eq!(
            preflight["error"]["code"], "MGR_PREFLIGHT_BAD_REQUEST",
            "{preflight}"
        );
        assert!(preflight["error"]["message"]
            .as_str()
            .unwrap()
            .contains("must be supplied together"));

        for root in [
            default_library,
            custom_library,
            default_loadout.parent().unwrap().to_path_buf(),
            custom_loadout.parent().unwrap().to_path_buf(),
        ] {
            assert!(!root.join(".gore-manager-library.lock").exists());
        }
    }

    #[test]
    fn mgr_import_and_list_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let bdir = write_goremod_bundle(tmp.path(), "Probe");

        let imp = mgr_call(
            "mgr_import",
            json!({
                "path": bdir.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(imp["ok"], true, "resp: {imp}");
        let mut top_level_keys: Vec<_> = imp
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        top_level_keys.sort_unstable();
        assert_eq!(
            top_level_keys,
            ["disposition", "entry", "matched_by", "ok"],
            "mgr_import success wire is the old entry plus two additive fields: {imp}"
        );
        assert_eq!(imp["disposition"], "created");
        assert_eq!(imp["matched_by"], "none");
        assert_eq!(imp["entry"]["name"], "Probe");
        assert_eq!(imp["entry"]["kind"], "goremod");
        assert!(
            imp["entry"].get("_manager").is_none()
                && imp["entry"].get("source_sha256").is_none()
                && imp["entry"].get("tree_sha256").is_none(),
            "private identity must not cross FFI: {imp}"
        );
        let id = imp["entry"]["id"].as_str().unwrap().to_string();
        let imported_components = imp["entry"]["components"].as_array().unwrap();
        assert!(!imported_components.is_empty(), "fixture component: {imp}");
        assert!(
            imported_components
                .iter()
                .all(|component| component.get("coverage").is_some()),
            "mgr_import must project mandatory coverage: {imp}"
        );
        assert_eq!(imported_components[0]["coverage"], "exact");

        let sidecar: Value = serde_json::from_slice(
            &std::fs::read(lib.join(&id).join(gore_mod::mgr::META_FILE)).unwrap(),
        )
        .unwrap();
        assert!(
            sidecar["components"]
                .as_array()
                .unwrap()
                .iter()
                .all(|component| component.get("coverage").is_none()),
            "coverage is a derived wire field, not persisted metadata: {sidecar}"
        );
        assert_eq!(sidecar["_manager"]["import_identity"]["format"], 1);
        assert_eq!(
            sidecar["_manager"]["import_identity"]["source_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(
            sidecar["_manager"]["import_identity"]["tree_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );

        let list = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        assert_eq!(list["ok"], true);
        let mods = list["mods"].as_array().unwrap();
        assert_eq!(mods.len(), 1, "one imported mod: {list}");
        assert_eq!(mods[0]["id"], id);
        assert_eq!(mods[0]["name"], "Probe");
        assert_eq!(
            mods[0]["components"], imp["entry"]["components"],
            "list and import must share one entry projection"
        );
        assert!(
            mods[0]["components"]
                .as_array()
                .unwrap()
                .iter()
                .all(|component| component.get("coverage").is_some()),
            "mgr_library_list must project mandatory coverage: {list}"
        );

        // BUG 3: the import must ALSO register a disabled loadout slot (mirror `gore mgr import`),
        // so a GUI-imported mod is visible to apply/status/analyze (which read the on-disk loadout)
        // without waiting for another mutation.
        let entries = list["loadout"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "import must add a loadout entry: {list}");
        assert_eq!(entries[0]["id"], id);
        assert_eq!(
            entries[0]["enabled"], false,
            "new mod is registered DISABLED"
        );
    }

    /// BUG 3, focused: `mgr_import` appends exactly one disabled loadout slot for the new id, and a
    /// RE-import (same source → same id) does NOT duplicate it (keeps the existing slot, incl. an
    /// enabled state a later toggle may have set).
    #[test]
    fn mgr_import_registers_disabled_loadout_slot() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let bdir = write_goremod_bundle(tmp.path(), "Probe");

        let imp = mgr_call(
            "mgr_import",
            json!({
                "path": bdir.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(imp["ok"], true, "resp: {imp}");
        let id = imp["entry"]["id"].as_str().unwrap().to_string();

        // Loadout now carries one disabled slot for the imported id.
        let after_first = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        let entries = after_first["loadout"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["id"], id);
        assert_eq!(entries[0]["enabled"], false);

        // Enable it, then re-import the SAME source: the slot must be preserved (still enabled, no
        // duplicate) — re-import is an update, not a fresh registration.
        mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [{"id": id, "enabled": true}]}
            }),
        );
        let reimp = mgr_call(
            "mgr_import",
            json!({
                "path": bdir.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(reimp["ok"], true, "resp: {reimp}");
        assert_eq!(reimp["disposition"], "unchanged");
        assert_eq!(reimp["matched_by"], "source");
        assert_eq!(reimp["entry"]["id"], id, "same source → same id");

        let after_reimport = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        let entries = after_reimport["loadout"]["entries"].as_array().unwrap();
        assert_eq!(
            entries.len(),
            1,
            "re-import must not duplicate the loadout slot: {after_reimport}"
        );
        assert_eq!(entries[0]["id"], id);
        assert_eq!(
            entries[0]["enabled"], true,
            "re-import preserves the existing enabled state"
        );
    }

    #[test]
    fn mgr_import_moved_source_preserves_id_order_enabled_and_loadout_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let source = write_goremod_bundle(&tmp.path().join("source-root"), "MovedProbe");
        let other = write_goremod_bundle(&tmp.path().join("other-root"), "OtherProbe");
        let first = mgr_call(
            "mgr_import",
            json!({
                "path": source.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        let second = mgr_call(
            "mgr_import",
            json!({
                "path": other.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(first["ok"], true, "{first}");
        assert_eq!(second["ok"], true, "{second}");
        let first_id = first["entry"]["id"].as_str().unwrap();
        let second_id = second["entry"]["id"].as_str().unwrap();
        let set = mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [
                    {"id": second_id, "enabled": false},
                    {"id": first_id, "enabled": true}
                ]}
            }),
        );
        assert_eq!(set["ok"], true, "{set}");
        let loadout_before = std::fs::read(&lo).unwrap();
        let moved = tmp.path().join("rebound").join("MovedProbe");
        std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
        std::fs::rename(&source, &moved).unwrap();

        let rebound = mgr_call(
            "mgr_import",
            json!({
                "path": moved.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(rebound["ok"], true, "{rebound}");
        assert_eq!(rebound["entry"]["id"], first_id);
        assert_eq!(rebound["disposition"], "updated");
        assert_eq!(rebound["matched_by"], "content");
        assert_eq!(
            rebound["entry"]["imported_at"], first["entry"]["imported_at"],
            "pure source rebind preserves deployment fingerprint input"
        );
        assert_eq!(std::fs::read(&lo).unwrap(), loadout_before);
        let listed = mgr_call(
            "mgr_library_list",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(listed["loadout"]["entries"][0]["id"], second_id);
        assert_eq!(listed["loadout"]["entries"][0]["enabled"], false);
        assert_eq!(listed["loadout"]["entries"][1]["id"], first_id);
        assert_eq!(listed["loadout"]["entries"][1]["enabled"], true);
    }

    #[test]
    fn mgr_import_moved_zip_preserves_id_order_enabled_and_loadout_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let bundle = write_goremod_bundle(&tmp.path().join("bundle-root"), "MovedZipProbe");
        let archive = tmp.path().join("download").join("MovedZipProbe.zip");
        std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
        write_mgr_test_zip(&bundle, &archive);
        let other = write_goremod_bundle(&tmp.path().join("other-root"), "OtherZipProbe");

        let imported = mgr_call(
            "mgr_import",
            json!({
                "path": archive.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        let other_imported = mgr_call(
            "mgr_import",
            json!({
                "path": other.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(imported["ok"], true, "{imported}");
        assert_eq!(other_imported["ok"], true, "{other_imported}");
        let imported_id = imported["entry"]["id"].as_str().unwrap();
        let other_id = other_imported["entry"]["id"].as_str().unwrap();
        let set = mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [
                    {"id": other_id, "enabled": false},
                    {"id": imported_id, "enabled": true}
                ]}
            }),
        );
        assert_eq!(set["ok"], true, "{set}");
        let loadout_before = std::fs::read(&lo).unwrap();
        let moved = tmp.path().join("moved").join("renamed.zip");
        std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
        std::fs::rename(&archive, &moved).unwrap();

        let rebound = mgr_call(
            "mgr_import",
            json!({
                "path": moved.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(rebound["ok"], true, "{rebound}");
        assert_eq!(rebound["entry"]["id"], imported_id);
        assert_eq!(rebound["disposition"], "updated");
        assert_eq!(rebound["matched_by"], "content");
        assert_eq!(
            rebound["entry"]["imported_at"],
            imported["entry"]["imported_at"]
        );
        assert_eq!(std::fs::read(&lo).unwrap(), loadout_before);
    }

    #[test]
    fn mgr_import_identity_refusals_are_structured_and_leave_state_byte_identical() {
        // Duplicate-content ambiguity: clone one valid managed entry, give the clone its own valid
        // id and source hint, then approach both through a third source path with equal bytes.
        {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("library");
            let lo = tmp.path().join("loadout.json");
            let source = write_goremod_bundle(&tmp.path().join("source"), "DuplicateProbe");
            let first = mgr_call(
                "mgr_import",
                json!({
                    "path": source.display().to_string(),
                    "library_dir": lib.display().to_string(),
                    "loadout_path": lo.display().to_string()
                }),
            );
            assert_eq!(first["ok"], true, "{first}");
            let first_id = first["entry"]["id"].as_str().unwrap();
            let duplicate_id = "verified-duplicate";
            copy_mgr_test_tree(&lib.join(first_id), &lib.join(duplicate_id));
            let duplicate_sidecar_path = lib.join(duplicate_id).join(gore_mod::mgr::META_FILE);
            let mut duplicate_sidecar: Value =
                serde_json::from_slice(&std::fs::read(&duplicate_sidecar_path).unwrap()).unwrap();
            duplicate_sidecar["id"] = Value::String(duplicate_id.into());
            duplicate_sidecar["_manager"]["import_identity"]["source_sha256"] =
                Value::String("0".repeat(64));
            std::fs::write(
                &duplicate_sidecar_path,
                serde_json::to_vec_pretty(&duplicate_sidecar).unwrap(),
            )
            .unwrap();
            let moved = tmp.path().join("moved").join("DuplicateProbe");
            copy_mgr_test_tree(&source, &moved);
            let library_before = mgr_visible_library_snapshot(&lib);
            let loadout_before = std::fs::read(&lo).unwrap();

            let response = mgr_call(
                "mgr_import",
                json!({
                    "path": moved.display().to_string(),
                    "library_dir": lib.display().to_string(),
                    "loadout_path": lo.display().to_string()
                }),
            );
            assert_eq!(
                response["error"]["code"], "IMPORT_DUPLICATE_AMBIGUOUS",
                "{response}"
            );
            let mut expected_ids = vec![first_id.to_owned(), duplicate_id.to_owned()];
            expected_ids.sort();
            assert_eq!(
                response["error"]["details"],
                json!({"candidate_ids": expected_ids})
            );
            assert!(!response.to_string().contains("sha256"));
            assert!(!response.to_string().contains("_manager"));
            assert_eq!(mgr_visible_library_snapshot(&lib), library_before);
            assert_eq!(std::fs::read(&lo).unwrap(), loadout_before);
            assert!(std::fs::read_dir(&lib).unwrap().all(|entry| {
                let name = entry.unwrap().file_name();
                name == ".gore-manager-library.lock" || !name.to_string_lossy().starts_with('.')
            }));
        }

        // Source-vs-content split: the bound source now stages B's bytes, so guessing either id
        // would be destructive. The refusal must happen before loadout registration.
        {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("library");
            let lo = tmp.path().join("loadout.json");
            let a_dir = tmp.path().join("a");
            let b_dir = tmp.path().join("b");
            std::fs::create_dir(&a_dir).unwrap();
            std::fs::create_dir(&b_dir).unwrap();
            let a = a_dir.join("same_P.pak");
            let b = b_dir.join("same_P.pak");
            std::fs::write(&a, b"old").unwrap();
            std::fs::write(&b, b"new").unwrap();
            let first = mgr_call(
                "mgr_import",
                json!({
                    "path": a.display().to_string(),
                    "library_dir": lib.display().to_string(),
                    "loadout_path": lo.display().to_string()
                }),
            );
            let second = mgr_call(
                "mgr_import",
                json!({
                    "path": b.display().to_string(),
                    "library_dir": lib.display().to_string(),
                    "loadout_path": lo.display().to_string()
                }),
            );
            assert_eq!(first["ok"], true, "{first}");
            assert_eq!(second["ok"], true, "{second}");
            std::fs::write(&a, b"new").unwrap();
            let library_before = mgr_visible_library_snapshot(&lib);
            let loadout_before = std::fs::read(&lo).unwrap();

            let response = mgr_call(
                "mgr_import",
                json!({
                    "path": a.display().to_string(),
                    "library_dir": lib.display().to_string(),
                    "loadout_path": lo.display().to_string()
                }),
            );
            assert_eq!(
                response["error"]["code"], "IMPORT_IDENTITY_CONFLICT",
                "{response}"
            );
            let mut expected = vec![
                json!({
                    "id": first["entry"]["id"],
                    "matched_by": ["entry_id", "source"]
                }),
                json!({
                    "id": second["entry"]["id"],
                    "matched_by": ["content"]
                }),
            ];
            expected.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
            assert_eq!(
                response["error"]["details"],
                json!({"candidates": expected})
            );
            assert!(!response.to_string().contains("sha256"));
            assert!(!response.to_string().contains("_manager"));
            assert_eq!(mgr_visible_library_snapshot(&lib), library_before);
            assert_eq!(std::fs::read(&lo).unwrap(), loadout_before);
            assert!(std::fs::read_dir(&lib).unwrap().all(|entry| {
                let name = entry.unwrap().file_name();
                name == ".gore-manager-library.lock" || !name.to_string_lossy().starts_with('.')
            }));
        }
    }

    #[test]
    fn mgr_import_non_identity_failures_keep_the_import_failed_code() {
        let tmp = tempfile::tempdir().unwrap();
        let unsupported = tmp.path().join("unsupported.7z");
        std::fs::write(&unsupported, b"not an archive").unwrap();
        let response = mgr_call(
            "mgr_import",
            json!({
                "path": unsupported.display().to_string(),
                "library_dir": tmp.path().join("library").display().to_string(),
                "loadout_path": tmp.path().join("loadout.json").display().to_string()
            }),
        );
        assert_eq!(response["ok"], false, "{response}");
        assert_eq!(response["error"]["code"], "IMPORT_FAILED", "{response}");
        assert!(response["error"].get("details").is_none(), "{response}");
    }

    #[test]
    fn mgr_set_loadout_persists() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        write_mgr_library_entry(&lib, "mod-a");
        write_mgr_library_entry(&lib, "mod-b");
        let set = mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [
                    {"id": "mod-a", "enabled": true},
                    {"id": "mod-b", "enabled": false}
                ]}
            }),
        );
        assert_eq!(set["ok"], true, "resp: {set}");

        // library_list must read back exactly what was saved.
        let list = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        let entries = list["loadout"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["id"], "mod-a");
        assert_eq!(entries[0]["enabled"], true);
        assert_eq!(entries[1]["id"], "mod-b");
        assert_eq!(entries[1]["enabled"], false);
    }

    #[test]
    fn mgr_set_loadout_raw_rejects_duplicate_fields_and_oversize_before_store_access() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = serde_json::to_string(&tmp.path().join("library").display().to_string()).unwrap();
        let lo =
            serde_json::to_string(&tmp.path().join("loadout.json").display().to_string()).unwrap();
        let duplicate = format!(
            r#"{{"command":"mgr_set_loadout","payload":{{"library_dir":{lib},"loadout_path":{lo},"loadout":{{"format":1,"format":1,"entries":[]}}}}}}"#
        );
        let response: Value = serde_json::from_str(&execute_json(&duplicate)).unwrap();
        assert_eq!(response["error"]["code"], "BAD_REQUEST", "{response}");
        assert!(!tmp.path().join("library").exists());
        assert!(!tmp.path().join("loadout.json").exists());

        let oversized = format!(
            r#"{{"command":"mgr_set_loadout","payload":{{"library_dir":"{}","loadout":{{"format":1,"entries":[]}}}}}}"#,
            "x".repeat(MAX_MGR_SET_LOADOUT_REQUEST_BYTES)
        );
        let response: Value = serde_json::from_str(&execute_json(&oversized)).unwrap();
        assert_eq!(response["error"]["code"], "BAD_REQUEST", "{response}");
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("bounded size"));
    }

    #[test]
    fn mgr_set_loadout_command_first_routes_before_scanning_a_transport_sized_tail() {
        let mut request = String::from(r#"{"command":"mgr_set_loadout","payload":"#);
        request.push_str(&"x".repeat(crate::transport::MAX_TRANSPORT_REQUEST_BYTES));
        // Deliberately leave the oversized tail syntactically incomplete. Once the bounded
        // command is first, dispatch must hand the untouched wire to the command-local cap rather
        // than scanning/parsing the remaining 64 MiB generic JSON value.
        let response: Value = serde_json::from_str(&execute_json(&request)).unwrap();
        assert_eq!(response["error"]["code"], "BAD_REQUEST", "{response}");
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("bounded size"));
    }

    #[test]
    fn mgr_set_loadout_rejects_non_current_format_without_touching_current_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        write_mgr_library_entry(&lib, "mod-a");
        gore_mod::mgr::loadout::save(
            &lo,
            &gore_mod::mgr::Loadout {
                format: 1,
                entries: vec![gore_mod::mgr::LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: false,
                }],
            },
        )
        .unwrap();
        let before = std::fs::read(&lo).unwrap();
        for format in [0, 2] {
            let response = mgr_call(
                "mgr_set_loadout",
                json!({
                    "library_dir": lib.display().to_string(),
                    "loadout_path": lo.display().to_string(),
                    "loadout": {"format": format, "entries": []}
                }),
            );
            assert_eq!(response["error"]["code"], "BAD_REQUEST", "{response}");
            assert_eq!(std::fs::read(&lo).unwrap(), before);
            assert!(!tmp.path().join(".gore-manager-library.lock").exists());
        }
    }

    #[test]
    fn mgr_set_loadout_duplicate_ids_keep_only_the_first_occurrence() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        write_mgr_library_entry(&lib, "mod-a");
        let response = mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [
                    {"id": "mod-a", "enabled": true},
                    {"id": "mod-a", "enabled": false}
                ]}
            }),
        );
        assert_eq!(response["ok"], true, "{response}");
        let stored = gore_mod::mgr::loadout::load(&lo).unwrap();
        assert_eq!(stored.entries.len(), 1);
        assert!(stored.entries[0].enabled);
    }

    // Removing a mod must also drop it from the persisted loadout (mirror `gore mgr remove`), so a
    // later mgr_apply reading the on-disk loadout does not fail on the deleted mod's metadata.
    #[test]
    fn mgr_remove_drops_loadout_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        write_mgr_library_entry(&lib, "mod-a");
        write_mgr_library_entry(&lib, "mod-b");
        mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [
                    {"id": "mod-a", "enabled": true},
                    {"id": "mod-b", "enabled": true}
                ]}
            }),
        );

        let rem = mgr_call(
            "mgr_remove",
            json!({
                "id": "mod-a",
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(rem["ok"], true, "resp: {rem}");

        let list = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        let entries = list["loadout"]["entries"].as_array().unwrap();
        assert_eq!(
            entries.len(),
            1,
            "removed id must be gone from loadout: {list}"
        );
        assert_eq!(entries[0]["id"], "mod-b");
    }

    #[test]
    fn mgr_analyze_no_conflicts_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let v = mgr_call(
            "mgr_analyze",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        assert_eq!(v["ok"], true, "resp: {v}");
        assert_eq!(v["conflicts"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn mgr_analyze_serializes_unknown_ue4ss_advisory() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let opaque_bundle = write_goremod_bundle(tmp.path(), "Opaque");
        let precise_bundle = write_goremod_bundle(tmp.path(), "Precise");

        let manifest_path = opaque_bundle.join("gore-mod.json");
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        let lua = manifest["components"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|component| component["type"] == "ue4ss_lua")
            .unwrap();
        lua["opaque"] = Value::Bool(true);
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let opaque = mgr_call(
            "mgr_import",
            json!({
                "path": opaque_bundle.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        let precise = mgr_call(
            "mgr_import",
            json!({
                "path": precise_bundle.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        let opaque_id = opaque["entry"]["id"].as_str().unwrap();
        let precise_id = precise["entry"]["id"].as_str().unwrap();
        let set = mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string(),
                "loadout": {"format": 1, "entries": [
                    {"id": opaque_id, "enabled": true},
                    {"id": precise_id, "enabled": true}
                ]}
            }),
        );
        assert_eq!(set["ok"], true, "resp: {set}");

        let response = mgr_call(
            "mgr_analyze",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        let unknown = response["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|conflict| conflict["kind"] == "ue4ss_unknown")
            .expect("unknown UE4SS advisory");
        assert_eq!(unknown["target"], "<unknown>");
        assert_eq!(unknown["severity"], "info");
        assert_eq!(unknown["mods"], json!([opaque_id, precise_id]));
        assert!(unknown.get("winner").is_none());
    }

    #[test]
    fn mgr_status_nothing_deployed() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let v = mgr_call(
            "mgr_status",
            json!({
                "game_root": game.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(v["ok"], true, "resp: {v}");
        assert_eq!(v["status"]["state"], "nothing_deployed");
    }

    /// After importing + enabling a mod and applying it, `mgr_status` against the SAME library
    /// reports `in_sync` — the deploy record's per-mod fingerprints match the current library, so
    /// the fingerprint gate the same-id-update fix added does not falsely fire. Uses a UE4SS mod
    /// (apply only copies its dir — no .lcache/bank fixture needed).
    #[test]
    fn mgr_status_in_sync_after_apply() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        // Minimal game tree: apply only needs the ue4ss Mods dir for a UE4SS-only mod.
        let game = tmp.path().join("game");
        std::fs::create_dir_all(game.join("G1R/Binaries/Win64/ue4ss/Mods")).unwrap();

        // Import a UE4SS bundle → it registers a disabled loadout slot.
        let bdir = write_goremod_bundle(tmp.path(), "Probe");
        let imp = mgr_call(
            "mgr_import",
            json!({
                "path": bdir.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(imp["ok"], true, "resp: {imp}");
        let id = imp["entry"]["id"].as_str().unwrap().to_string();

        // Enable it: set the loadout to [{id, enabled:true}].
        let set = mgr_call(
            "mgr_set_loadout",
            json!({
                "library_dir": lib.display().to_string(),
                "loadout": {"format": 1, "entries": [{"id": id, "enabled": true}]},
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(set["ok"], true, "resp: {set}");

        // Apply → creates a manager deploy record (recording the mod's fingerprint).
        let ap = mgr_call(
            "mgr_apply",
            json!({
                "game_root": game.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(ap["ok"], true, "apply resp: {ap}");

        // Status with the SAME library → in_sync (loadout matches AND fingerprint matches).
        let st = mgr_call(
            "mgr_status",
            json!({
                "game_root": game.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(st["ok"], true, "status resp: {st}");
        assert_eq!(st["status"]["state"], "in_sync", "resp: {st}");
    }

    #[test]
    fn mgr_apply_studio_active_maps_code() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");

        // Pre-seed a STUDIO deploy record (owner == "") straight to the record path so apply's
        // studio guard trips. DeployRecord/record_path aren't public to this crate, so write the
        // JSON file directly — the record lives at <game_root>/gore-mod.deployed.json. Include the
        // fields DeployRecord does NOT default (`mod_name`, `ue4ss_mod_dir`, `backups`) so it
        // deserializes; `owner: ""` marks it a studio (non-manager) deployment.
        std::fs::write(
            game.join("gore-mod.deployed.json"),
            br#"{"mod_name":"SoloMod","ue4ss_mod_dir":null,"backups":[],"owner":""}"#,
        )
        .unwrap();

        let v = mgr_call(
            "mgr_apply",
            json!({
                "game_root": game.display().to_string(),
                "library_dir": lib.display().to_string(),
                "loadout_path": lo.display().to_string()
            }),
        );
        assert_eq!(v["ok"], false, "resp: {v}");
        assert_eq!(v["error"]["code"], "STUDIO_DEPLOY_ACTIVE");
        // The message carries just the blocking studio mod's name.
        assert_eq!(v["error"]["message"], "SoloMod");
    }
}
