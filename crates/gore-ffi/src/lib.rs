//! C ABI for gore-mod's `dart:ffi` bridge. New Studio builds use a length-aware, globally bounded
//! transport-v2 entry point; the bounded C-string entry point remains exported only for older
//! Studio binaries. Both carry the same JSON command/response protocol.
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
//! - `authoring_project_check` — payload `{project_json, profile}` where `project_json` is the
//!   untouched format-2 JSON string and `profile` is `production|experimental`; returns canonical
//!   project JSON, deterministic structured diagnostics, and `blocks_build`. Project input is
//!   capped at 16 MiB by `gore-authoring`; serialized success responses are capped at 64 MiB.
//! - `authoring_logical_npc_clone_draft_v1_generate` and
//!   `authoring_draft_quest_skeleton_v1_generate` accept one bounded raw `input_json` string and
//!   return deterministic offline-only previews. They never write, compile, deploy, or qualify
//!   runtime behavior.
//! - `authoring_project_story_draft_insert_v1` atomically evaluates one raw, duplicate-safe Story
//!   Draft mutation against one exact canonical schema-revision-2 project. Rejections never carry
//!   candidate project JSON.
//! - `authoring_project_story_quest_draft_insert_v1` accepts friendly Quest intent plus one game
//!   root, then rebuilds all catalog/collision provenance natively and runs the existing atomic
//!   transaction. It never accepts provenance from clients or compiles, writes, or publishes.
//! - `authoring_npc_archetype_catalog_v1_build_for_game_root` accepts only one game root and
//!   returns the canonical, generation-sealed, read-only NPC archetype catalog. Native code fixes
//!   executable/Binds paths and selects only the deployment-aware pristine Shipping snapshot; it
//!   never writes, launches, builds, deploys, publishes, or claims runtime qualification.
//! - `authoring_story_build_plan_v1_generate` derives one deterministic, sealed source-inspection
//!   plan from exact canonical revision-2 project bytes. It always remains runtime-unqualified,
//!   build-blocked, and non-publishable and never compiles, writes, deploys, or launches the game.
//! - `authoring_story_catalog_v1_build` reads three bounded generation paths and builds the pinned
//!   catalog entirely in memory; `authoring_story_catalog_v1_read` accepts one bounded raw
//!   canonical catalog string and returns a request-bound read-only chooser projection. Neither
//!   command writes game files, publishes a catalog, or launches the game.
//!   `authoring_story_catalog_v1_build_for_game_root` keeps the same boundary but selects the
//!   deployment-aware pristine Shipping cache natively through `gore-mod`; clients supply only the
//!   game root and never parse deployment records or choose backups.
//! - `authoring_story_inventory_v1_build` reads the same exact pinned generation paths and returns
//!   one canonical, base-game-only collision inventory. It never resolves a mod loadout, writes,
//!   launches the game, qualifies runtime behavior, builds, deploys, or publishes.
//! - `authoring_store_open`, `authoring_store_open_head_bytes`, and
//!   `authoring_store_prepare_checkpoint` retain the frozen schema-revision-1 working-store wire.
//!   Their additive `*_document` counterparts dispatch between closed schema revisions 1 and 2.
//!   The dedicated `authoring_store_open_revision3`, `authoring_store_open_revision3_head_bytes`,
//!   and `authoring_store_prepare_revision3_checkpoint` commands exclusively carry schema
//!   revision 3. Opens return exact canonical head/project JSON; preparation returns only the
//!   canonical candidate head after a full exact reopen. It never publishes the fixed head, and
//!   generic document preparation remains revision-1/2-only. Their bounded raw request envelopes
//!   reject duplicate, unknown, missing, and wrongly typed outer or payload fields.
//!   Head/project JSON crosses the outer protocol as bounded raw strings, preserving canonical-byte
//!   CAS and duplicate-key rejection. Preparing writes immutable objects but never publishes
//!   `gore-project.json`.
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
//! - `authoring_store_prepare_revision3_quest_outline_edit_v1` edits only one exact-current
//!   managed Quest's display outline and deterministically regenerates its owned ScriptModule.
//!   It fully reopens an unpublished candidate, never accepts collision authority from the
//!   client, and never builds, deploys, touches a game/save, or publishes the fixed head.
//! - `authoring_store_prepare_revision3_quest_outline_edit_v2` edits the display outline of one
//!   exact-current semantic Quest while retaining stable objective slots and transition behavior.
//!   It binds the owned module and retained plan explicitly, fully reopens only an unpublished
//!   candidate, and accepts no game, compiler, build, deployment, save, or publication authority.
//! - `authoring_store_prepare_revision3_quest_transitions_edit_v1` edits only one exact-current
//!   managed Quest's bounded semantic transition plan and deterministically regenerates its owned
//!   ScriptModule. It can explicitly upgrade a frozen generator-v2/v3 Quest to generator v4,
//!   fully reopens an unpublished candidate, and never accepts a game root, builds, deploys,
//!   touches a game/save, or publishes the fixed head.
//! - `authoring_store_prepare_revision3_npc_draft_v1` rebuilds the fresh Story catalog, broad NPC
//!   archetype linkage, and complete base-game-plus-exact-current script collision inventory, then
//!   prepares one exact-current NPC/ScriptModule checkpoint. It never publishes the fixed head or
//!   grants build, spawn, runtime, deployment, save, or reusable catalog authority.
//! - `authoring_store_prepare_revision3_dialog_line_v1` creates one new authored dialog line and
//!   either creates or exactly reuses its managed localization, then fully reopens an immutable
//!   unpublished revision-3 checkpoint. It accepts no game root and grants no topic, build,
//!   runtime, deployment, save, or fixed-head publication authority.
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
//! - `authoring_store_prepare_revision3_voice_target_v1` resolves one existing VoiceSlot against
//!   the first installed archive for its canonical locale. Native code alone derives bounded,
//!   sealed exact-member evidence; the route prepares a fully reopened candidate without editing
//!   the archive or publishing the fixed project head.
//! - `authoring_store_build_revision3_voice_v1` prepares the bounded revision-3 Voice build
//!   artifact route without deploying, editing the installation, or publishing the project head.
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
//! - `script_compile_report_v1` is the bounded structured companion to the legacy
//!   `script_compile` command. It fails closed unless the deployment-aware pristine cache can be
//!   resolved, runs the optional compiler hook with automatic normal-generator fallback, and
//!   reports diagnostics plus exact live-install restoration separately from compile success.
//! - `script_compile_install_state_v1` is a bounded, strictly read-only native preflight for the
//!   shipping-game process and every known compile/recovery artifact. It returns display-only
//!   paths and never creates, removes, renames, repairs, launches, or writes anything.
//! - `authoring_store_inspect_revision3_installed_dataasset_v1` accepts only one exact managed
//!   revision-3 head, installed package-snapshot seals, game/Store roots, and a candidate ordinal.
//!   It rebuilds every native authority and returns bounded whole-package fixed-leaf inspection
//!   evidence without accepting package, output, or USMAP paths.
//! - `authoring_store_prepare_revision3_installed_dataasset_edit_v1` reopens that exact installed
//!   proof, re-inspects the server-selected candidate, applies one typed fixed-leaf edit wholly
//!   in memory, and returns an unpublished revision-3 stage candidate. It never accepts raw
//!   package bytes, receipt paths, output paths, or publication/deployment authority.

mod authoring;
mod authoring_content_revision3;
mod authoring_dataasset_package_index_revision3;
mod authoring_dataasset_revision3;
mod authoring_dialog_localization_edit_revision3;
mod authoring_dialog_localization_revision3;
mod authoring_dialog_revision3;
mod authoring_drafts;
mod authoring_installed_dataasset_inspection_revision3;
mod authoring_npc_catalog;
mod authoring_store;
mod authoring_story;
mod authoring_story_build;
mod authoring_story_catalog;
mod authoring_story_compiler_revision3;
mod authoring_story_inventory;
mod authoring_story_npc_inspection_revision3;
mod authoring_story_npc_revision3;
mod authoring_story_quest;
mod authoring_story_quest_context_revision3;
mod authoring_story_quest_inspection_revision3;
mod authoring_story_quest_outline_revision3;
mod authoring_story_quest_outline_v2_revision3;
mod authoring_story_quest_revision3;
mod authoring_story_quest_transitions_revision3;
mod authoring_voice_build_revision3;
mod authoring_voice_revision3;
mod authoring_voice_selection_revision3;
mod authoring_voice_target_revision3;
mod dataasset;
mod script_compile_report;
mod transport;
mod voice;

use serde_json::{json, Value};

use std::path::PathBuf;

use gore_loc::{loc_store, paths};
use gore_modgen::gen::{gen_lua, OverridesConfig};
use gore_modgen::validate::validate_config;
use gore_reflect::model::ReflectionModel;

pub use transport::{
    gore_core_execute, gore_core_execute_v2, gore_core_free, gore_core_response_free_v2,
    gore_core_transport_abi_v2, GoreCoreResponseV2,
};

/// Increment only when the JSON command/response protocol is incompatible. Transport ownership
/// is negotiated independently so old Studio binaries can keep using the legacy C-string exports.
const CORE_PROTOCOL_ABI: u32 = 1;

/// Every command understood by [`dispatch`], kept in bytewise ascending order so capability
/// negotiation is deterministic across builds and platforms.
const CORE_COMMANDS: &[&str] = &[
    "audio_extract",
    "audio_list",
    "authoring_draft_quest_skeleton_v1_generate",
    "authoring_logical_npc_clone_draft_v1_generate",
    "authoring_npc_archetype_catalog_v1_build_for_game_root",
    "authoring_project_check",
    "authoring_project_story_draft_insert_v1",
    "authoring_project_story_quest_draft_insert_v1",
    "authoring_read_dataasset_extract_receipt_v2",
    "authoring_store_build_revision3_voice_v1",
    "authoring_store_check_revision3_npc_compiler_v1",
    "authoring_store_check_revision3_quest_compiler_v1",
    "authoring_store_import_ogg",
    "authoring_store_inspect_revision3_installed_dataasset_v1",
    "authoring_store_inspect_revision3_npc_source_v1",
    "authoring_store_inspect_revision3_quest_source_v1",
    "authoring_store_list_revision3_dataasset_stages_v1",
    "authoring_store_open",
    "authoring_store_open_document",
    "authoring_store_open_head_bytes",
    "authoring_store_open_head_bytes_document",
    "authoring_store_open_revision3",
    "authoring_store_open_revision3_head_bytes",
    "authoring_store_prepare_checkpoint",
    "authoring_store_prepare_document_checkpoint",
    "authoring_store_prepare_remove_revision3_dataasset_stage_v1",
    "authoring_store_prepare_revision3_checkpoint",
    "authoring_store_prepare_revision3_dataasset_edit_v1",
    "authoring_store_prepare_revision3_dataasset_stage_v1",
    "authoring_store_prepare_revision3_dialog_line_v1",
    "authoring_store_prepare_revision3_dialog_localization_edit_v1",
    "authoring_store_prepare_revision3_installed_dataasset_edit_v1",
    "authoring_store_prepare_revision3_npc_draft_v1",
    "authoring_store_prepare_revision3_quest_context_edit_v1",
    "authoring_store_prepare_revision3_quest_draft_v3",
    "authoring_store_prepare_revision3_quest_outline_edit_v1",
    "authoring_store_prepare_revision3_quest_outline_edit_v2",
    "authoring_store_prepare_revision3_quest_transitions_edit_v1",
    "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1",
    "authoring_store_prepare_revision3_voice_take_selection_v1",
    "authoring_store_prepare_revision3_voice_take_v1",
    "authoring_store_prepare_revision3_voice_target_v1",
    "authoring_store_read_revision3_content_index_v1",
    "authoring_store_read_revision3_dataasset_package_index_v1",
    "authoring_store_read_revision3_dialog_localization_edit_seed_v1",
    "authoring_store_read_revision3_dialog_localization_v1",
    "authoring_store_verify_asset",
    "authoring_story_build_plan_v1_generate",
    "authoring_story_catalog_v1_build",
    "authoring_story_catalog_v1_build_for_game_root",
    "authoring_story_catalog_v1_read",
    "authoring_story_inventory_v1_build",
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
    "mgr_remove",
    "mgr_set_loadout",
    "mgr_status",
    "mgr_undeploy_all",
    "mod_build",
    "mod_deploy",
    "mod_undeploy",
    "script_compile",
    "script_compile_install_state_v1",
    "script_compile_report_v1",
    "script_emit_module",
    "script_list_modules",
    "texture_extract",
    "texture_index",
    "validate",
    "voice_archive_extract",
    "voice_archive_list",
    "voice_archive_match_line",
    "voice_ogg_inspect_v1",
];

// The C ABI entry points live in `transport`; they are re-exported above so the Rust API and
// native symbol names remain backward compatible.

/// Pure entry point (no FFI) — also the test seam.
pub fn execute_json(input: &str) -> String {
    // The pure seam uses the same global response budget as both native transports.
    String::from_utf8(transport::execute_json_bounded(input))
        .expect("JSON transport output is always UTF-8")
}

fn err(code: &str, msg: impl Into<String>) -> Value {
    json!({"ok": false, "error": {"code": code, "message": msg.into()}})
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
/// Raw Store routes must see their original wire under their command-local cap before a generic
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
            if revision3_store_raw_route(&command).is_some() {
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
        "authoring_store_list_revision3_dataasset_stages_v1" => {
            Some(authoring_dataasset_revision3::list_raw)
        }
        "authoring_read_dataasset_extract_receipt_v2" => {
            Some(authoring_dataasset_revision3::read_extract_raw)
        }
        "authoring_store_build_revision3_voice_v1" => {
            Some(authoring_voice_build_revision3::build_revision3_voice_v1_raw)
        }
        "authoring_store_check_revision3_npc_compiler_v1" => Some(
            authoring_story_compiler_revision3::check_revision3_npc_compiler_v1_raw,
        ),
        "authoring_store_check_revision3_quest_compiler_v1" => Some(
            authoring_story_compiler_revision3::check_revision3_quest_compiler_v1_raw,
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
        "authoring_store_prepare_revision3_installed_dataasset_edit_v1" => Some(
            authoring_installed_dataasset_inspection_revision3::prepare_revision3_installed_dataasset_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_npc_draft_v1" => {
            Some(authoring_story_npc_revision3::prepare_revision3_npc_draft_v1_raw)
        }
        "authoring_store_prepare_revision3_quest_context_edit_v1" => Some(
            authoring_story_quest_context_revision3::prepare_revision3_quest_context_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_quest_draft_v3" => {
            Some(authoring_story_quest_revision3::prepare_revision3_quest_draft_v3_raw)
        }
        "authoring_store_prepare_revision3_quest_outline_edit_v1" => Some(
            authoring_story_quest_outline_revision3::prepare_revision3_quest_outline_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_quest_outline_edit_v2" => Some(
            authoring_story_quest_outline_v2_revision3::prepare_revision3_quest_outline_edit_v2_raw,
        ),
        "authoring_store_prepare_revision3_quest_transitions_edit_v1" => Some(
            authoring_story_quest_transitions_revision3::prepare_revision3_quest_transitions_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1" => Some(
            authoring_installed_dataasset_inspection_revision3::prepare_revision3_reviewed_installed_dataasset_edit_v1_raw,
        ),
        "authoring_store_prepare_revision3_voice_take_selection_v1" => {
            Some(authoring_voice_selection_revision3::prepare_revision3_voice_take_selection_v1_raw)
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
    // These security-sensitive additive Store routes see the original wire before any generic
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
        "mgr_set_loadout" => mgr_set_loadout(payload),
        "mgr_analyze" => mgr_analyze(payload),
        "mgr_apply" => mgr_apply(payload),
        "mgr_status" => mgr_status(payload),
        "mgr_undeploy_all" => mgr_undeploy_all(payload),
        "texture_index" => texture_index(payload),
        "texture_extract" => texture_extract(payload),
        "script_list_modules" => script_list_modules(payload),
        "script_emit_module" => script_emit_module(payload),
        "script_compile" => script_compile(payload),
        "authoring_draft_quest_skeleton_v1_generate" => {
            authoring_drafts::draft_quest_skeleton(payload)
        }
        "authoring_logical_npc_clone_draft_v1_generate" => {
            authoring_drafts::logical_npc_clone(payload)
        }
        "authoring_npc_archetype_catalog_v1_build_for_game_root" => {
            authoring_npc_catalog::build_for_game_root_v1(payload)
        }
        "authoring_project_check" => authoring::project_check(payload),
        "authoring_project_story_draft_insert_v1" => {
            authoring_story::insert_story_draft_v1(payload)
        }
        "authoring_project_story_quest_draft_insert_v1" => {
            authoring_story_quest::insert_quest_draft_v1(payload)
        }
        "authoring_story_build_plan_v1_generate" => {
            authoring_story_build::generate_story_build_plan_v1(payload)
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
        "authoring_story_inventory_v1_build" => {
            authoring_story_inventory::build_story_inventory_v1(payload)
        }
        "authoring_store_import_ogg" => authoring_store::import_ogg(payload),
        "authoring_store_open" => authoring_store::open(payload),
        "authoring_store_open_document" => authoring_store::open_document(payload),
        "authoring_store_open_head_bytes" => authoring_store::open_head_bytes(payload),
        "authoring_store_open_head_bytes_document" => {
            authoring_store::open_head_bytes_document(payload)
        }
        "authoring_store_prepare_checkpoint" => authoring_store::prepare_checkpoint(payload),
        "authoring_store_prepare_document_checkpoint" => {
            authoring_store::prepare_document_checkpoint(payload)
        }
        "authoring_store_verify_asset" => authoring_store::verify_asset(payload),
        "voice_archive_list" => voice::archive_list(payload),
        "voice_archive_match_line" => voice::archive_match_line(payload),
        "voice_archive_extract" => voice::archive_extract(payload),
        "voice_ogg_inspect_v1" => voice::ogg_inspect_v1_raw(input),
        other => err("UNKNOWN_COMMAND", format!("unknown command: {other}")),
    }
}

/// Cheap, read-only compatibility handshake. This deliberately does not inspect the game,
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

/// `{ok, build_id, count, entries:{path:package_id_str}}` — load the cached index, building it
/// if absent or if `payload.rebuild` is true. `payload.game` = install dir.
fn texture_index(payload: Value) -> Value {
    let game = match payload.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g),
        None => return err("BAD_REQUEST", "missing game"),
    };
    let rebuild = payload
        .get("rebuild")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cache = gore_tex::paths::texture_index_path();
    let usmap = match gore_tex::paths::usmap(&game) {
        Ok(p) => p,
        Err(e) => return err("USMAP", e.to_string()),
    };
    let utoc = match gore_tex::paths::main_container(&game) {
        Ok(p) => p,
        Err(e) => return err("CONTAINER", e.to_string()),
    };
    let build_id = gore_tex::index::build_id_for(&utoc, &usmap);
    // Only reuse the cache when it's current for THIS game build; build_id keys on the .usmap
    // name AND the container's identity, so a game patch (even one keeping the .usmap name) that
    // rewrites the container invalidates a cache mapping paths to outdated package ids.
    let cached = if rebuild {
        None
    } else {
        gore_tex::index::TextureIndex::load_current(&cache, &build_id)
    };
    let mut cache_saved = true; // a loaded cache is, by definition, already persisted
    let idx = match cached {
        Some(i) => i,
        None => {
            let i = match gore_tex::index::build_index(&utoc, &build_id) {
                Ok(i) => i,
                Err(e) => return err("INDEX_BUILD", e.to_string()),
            };
            // Don't silently ignore a failed persist: the index is usable in-memory this call,
            // but a failed write means every later load rebuilds. Surface it (warning + flag)
            // instead of reporting unqualified success.
            if let Err(e) = i.save(&cache) {
                eprintln!("warning: failed to persist texture index cache: {e}");
                cache_saved = false;
            }
            i
        }
    };
    let entries: serde_json::Map<String, Value> = idx
        .entries
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.to_string())))
        .collect();
    json!({ "ok": true, "build_id": idx.build_id, "count": idx.entries.len(), "cache_saved": cache_saved, "entries": entries })
}

/// `{ok, png_path, width, height, format}` — extract a texture to a temp PNG. `payload.game`,
/// and either `payload.package_id` (string) or `payload.asset` (path).
fn texture_extract(payload: Value) -> Value {
    let game = match payload.get("game").and_then(Value::as_str) {
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
    let asset = payload.get("asset").and_then(Value::as_str).unwrap_or("");
    let leaf = asset.rsplit('/').next().unwrap_or("texture").to_string();
    let (info, px) = if let Some(pid) = payload
        .get("package_id")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok())
    {
        match gore_tex::index::extract_by_package_id(&utoc, &usmap, pid, &leaf) {
            Ok(x) => x,
            Err(e) => return err("EXTRACT", e.to_string()),
        }
    } else if !asset.is_empty() {
        let tmp = match gore_tex::paths::unique_temp_dir("gore-tex-ffi-extract") {
            Ok(t) => t,
            Err(_) => return err("IO", "tmp"),
        };
        let ua = match gore_tex::container::unpack_asset(&utoc, &usmap, asset, &tmp) {
            Ok(p) => p,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return err("UNPACK", e.to_string());
            }
        };
        // Surface read failures instead of defaulting to empty bytes (which would yield a
        // misleading PARSE/DECODE error). `.ubulk` is legitimately optional (inline-mip textures).
        macro_rules! read_or_err {
            ($p:expr) => {
                match std::fs::read($p) {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = std::fs::remove_dir_all(&tmp);
                        return err("READ", e.to_string());
                    }
                }
            };
        }
        let ua_bytes = read_or_err!(&ua);
        let uexp_bytes = read_or_err!(ua.with_extension("uexp"));
        let usmap_bytes = read_or_err!(&usmap);
        let ubulk_bytes = match gore_tex::paths::read_optional(&ua.with_extension("ubulk")) {
            Ok(b) => b,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return err("READ", e.to_string());
            }
        };
        let info = match gore_tex::decode::parse(&ua_bytes, &uexp_bytes, &ubulk_bytes, &usmap_bytes)
        {
            Ok(i) => i,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return err("PARSE", e.to_string());
            }
        };
        let px = match gore_tex::decode::to_rgba8(&info) {
            Ok(p) => p,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return err("DECODE", e.to_string());
            }
        };
        let _ = std::fs::remove_dir_all(&tmp);
        (info, px)
    } else {
        return err("BAD_REQUEST", "need package_id or asset");
    };
    let mut buf = Vec::with_capacity(px.len() * 4);
    for p in px {
        buf.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
    }
    // Unique per-request output path: a deterministic name would let two
    // extractions of the same texture (e.g. a stale request finishing after a
    // game/index change) race on one file. Each call owns its own PNG and the UI
    // deletes exactly the file it was handed.
    let out = gore_tex::paths::unique_temp_file(&format!("gore-tex-preview-{leaf}"), "png");
    if image::save_buffer(&out, &buf, info.width, info.height, image::ColorType::Rgba8).is_err() {
        return err("PNG", "save failed");
    }
    // `replaceable` is the AUTHORITATIVE capability flag the UI gates the Replace
    // button on (always a plain bool). It requires BOTH a re-encodable
    // texture shape (`replace_supported`) AND a deployable mount root: the deploy
    // path can only place /Game and /Engine assets (`content_mount_rel`), so an
    // asset under any other root (e.g. /DatasmithContent) must report not
    // replaceable rather than appear supported and fail later at build/deploy.
    // `is_virtual`/`vt_layers` are exposed for diagnostics.
    // Only enforce mount-root deployability when we actually know the asset path.
    // A package_id-only extract passes an empty asset, where mount resolution
    // can't run — don't let that wrongly mark an encodable /Game texture as not
    // replaceable.
    let deployable_root = asset.is_empty() || gore_tex::paths::content_mount_rel(asset).is_some();
    let replaceable = gore_tex::decode::replace_supported(&info) && deployable_root;
    json!({ "ok": true, "png_path": out.display().to_string(), "width": info.width, "height": info.height,
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

/// `{game_dir, op, module_name, rel_path, as_path, work_dir}` → `{ok, mini_path, module}`.
/// `allow_new_symbols` is an explicit opt-in and defaults to false when omitted.
fn script_compile(payload: Value) -> Value {
    let g = |k: &str| payload.get(k).and_then(Value::as_str).map(str::to_string);
    let (
        Some(game_dir),
        Some(op),
        Some(module_name),
        Some(rel_path),
        Some(as_path),
        Some(work_dir),
    ) = (
        g("game_dir"),
        g("op"),
        g("module_name"),
        g("rel_path"),
        g("as_path"),
        g("work_dir"),
    )
    else {
        return err(
            "BAD_REQUEST",
            "missing one of game_dir/op/module_name/rel_path/as_path/work_dir",
        );
    };
    let allow_new_symbols = payload
        .get("allow_new_symbols")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // Keep the legacy response shape, but execute through the same bounded native route as the
    // structured command. In particular, the same install guard spans pristine selection through
    // compile, and gore-as receives only a uniquely-owned child below `work_dir`; no caller-owned
    // `work_dir/tree` is ever recursively reset by this raw compatibility command.
    let request = json!({
        "command": script_compile_report::COMMAND,
        "payload": {
            "allow_new_symbols": allow_new_symbols,
            "as_path": as_path,
            "game_dir": game_dir,
            "module_name": module_name,
            "op": op,
            "rel_path": rel_path,
            "work_dir": work_dir,
        }
    });
    let structured = script_compile_report::compile_report_v1_raw(&request.to_string());
    match structured.get("outcome").and_then(Value::as_str) {
        Some("compiled") => json!({
            "ok": true,
            "mini_path": structured.get("mini_path").cloned().unwrap_or(Value::Null),
            "module": structured.get("module").cloned().unwrap_or(Value::Null),
        }),
        Some("failed") => {
            let recovery_required = structured
                .get("recovery_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let compile_error = structured.get("compile_error");
            let structured_code = compile_error
                .and_then(|value| value.get("code"))
                .and_then(Value::as_str)
                .unwrap_or("COMPILE_FAILED");
            let message = compile_error
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("AngelScript compilation failed");
            let code = if recovery_required {
                "COMPILE_RECOVERY_REQUIRED"
            } else {
                structured_code
            };
            err(code, message)
        }
        // Bad-request envelopes already use the legacy `{ok:false,error:{...}}` shape.
        _ => structured,
    }
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
        Ok(rec) => json!({"ok": true, "record": serde_json::to_value(rec).unwrap_or(Value::Null)}),
        Err(e) => err("DEPLOY_FAILED", e.to_string()),
    }
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
// Every mgr command accepts OPTIONAL `library_dir` / `loadout_path` string overrides so tests
// (and callers wanting an isolated store) can point at temp dirs; when absent the shared
// per-user layout from `gore_mod::mgr::paths` is used, so every gore tool sees one library.

/// The library dir to use: `payload.library_dir` if given, else the shared default.
fn mgr_library_dir(payload: &Value) -> PathBuf {
    match payload.get("library_dir").and_then(Value::as_str) {
        Some(p) => PathBuf::from(p),
        None => gore_mod::mgr::paths::library_dir(),
    }
}

/// The loadout file to use: `payload.loadout_path` if given, else the shared default.
fn mgr_loadout_path(payload: &Value) -> PathBuf {
    match payload.get("loadout_path").and_then(Value::as_str) {
        Some(p) => PathBuf::from(p),
        None => gore_mod::mgr::paths::loadout_path(),
    }
}

/// `{library_dir?, loadout_path?}` → `{ok, mods:[ModEntryMeta], loadout:Loadout}`. Raw library +
/// loadout, unreconciled (the UI reconciles ids against the library itself).
fn mgr_library_list(payload: Value) -> Value {
    let lib = mgr_library_dir(&payload);
    let lo_path = mgr_loadout_path(&payload);
    let mods = match gore_mod::mgr::import::list(&lib) {
        Ok(m) => m,
        Err(e) => return err("IO", e.to_string()),
    };
    let loadout = match gore_mod::mgr::loadout::load(&lo_path) {
        Ok(l) => l,
        Err(e) => return err("BAD_REQUEST", e.to_string()),
    };
    json!({
        "ok": true,
        "mods": serde_json::to_value(&mods).unwrap_or(Value::Null),
        "loadout": serde_json::to_value(&loadout).unwrap_or(Value::Null),
    })
}

/// `{path, library_dir?, loadout_path?}` → `{ok, entry:ModEntryMeta}` — import a source into the
/// library AND register it in the loadout (disabled) if not already present. Mirrors
/// `gore mgr import`: without this, a GUI-imported mod is invisible to apply/status/analyze (which
/// read the on-disk loadout, not the GUI's in-memory reconcile) until some other mutation.
fn mgr_import(payload: Value) -> Value {
    let Some(path) = payload.get("path").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'path'");
    };
    let lib = mgr_library_dir(&payload);
    let entry = match gore_mod::mgr::import::import(&lib, std::path::Path::new(path)) {
        Ok(entry) => entry,
        Err(e) => return err("IMPORT_FAILED", e.to_string()),
    };
    // Register the new mod in the loadout (disabled) so enable/apply can find it. Skip if an entry
    // with this id already exists (a re-import / update): keep its current enabled state + order.
    let lo_path = mgr_loadout_path(&payload);
    // Surface a loadout read/write failure instead of returning ok: apply/status/analyze read the
    // on-disk loadout, so a swallowed error here would leave the imported mod invisible to them. A
    // missing loadout file loads as an empty default (not an error), so first-time imports still
    // register normally.
    match gore_mod::mgr::loadout::load(&lo_path) {
        Ok(mut lo) => {
            if !lo.entries.iter().any(|e| e.id == entry.id) {
                lo.entries.push(gore_mod::mgr::LoadoutEntry {
                    id: entry.id.clone(),
                    enabled: false,
                });
                if let Err(e) = gore_mod::mgr::loadout::save(&lo_path, &lo) {
                    return err(
                        "IO",
                        format!("imported '{}' into the library but failed to register it in the loadout: {e}", entry.id),
                    );
                }
            }
        }
        Err(e) => {
            return err(
                "IO",
                format!(
                    "imported '{}' into the library but failed to read the loadout: {e}",
                    entry.id
                ),
            )
        }
    }
    json!({"ok": true, "entry": serde_json::to_value(&entry).unwrap_or(Value::Null)})
}

/// `{id, library_dir?}` → `{ok, removed:bool}` — delete a library entry (absent id → removed:false).
fn mgr_remove(payload: Value) -> Value {
    let Some(id) = payload.get("id").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'id'");
    };
    let lib = mgr_library_dir(&payload);
    let removed = match gore_mod::mgr::import::remove(&lib, id) {
        Ok(removed) => removed,
        Err(e) => return err("BAD_REQUEST", e.to_string()),
    };
    // Keep the persisted loadout in sync (mirror `gore mgr remove`): a removed mod must not
    // linger as an enabled loadout entry, or a later `mgr_apply` — which reads the on-disk
    // loadout, not the GUI's in-memory reconcile — fails loading the deleted mod's metadata.
    let lo_path = mgr_loadout_path(&payload);
    // Surface a loadout read/write failure instead of returning ok: a dropped save error would let
    // the persisted loadout keep an enabled reference to the now-removed mod, so a later
    // apply/status/analyze (which read the on-disk loadout) would fail or act on a stale target. A
    // missing loadout file loads as an empty default (not an error).
    match gore_mod::mgr::loadout::load(&lo_path) {
        Ok(mut lo) => {
            let before = lo.entries.len();
            lo.entries.retain(|e| e.id != id);
            if lo.entries.len() != before {
                if let Err(e) = gore_mod::mgr::loadout::save(&lo_path, &lo) {
                    return err(
                        "IO",
                        format!(
                            "removed '{id}' from the library but failed to update the loadout: {e}"
                        ),
                    );
                }
            }
        }
        Err(e) => {
            return err(
                "IO",
                format!("removed '{id}' from the library but failed to read the loadout: {e}"),
            )
        }
    }
    json!({"ok": true, "removed": removed})
}

/// `{loadout:Loadout, loadout_path?}` → `{ok}` — persist the loadout.
fn mgr_set_loadout(payload: Value) -> Value {
    let loadout: gore_mod::mgr::Loadout = match payload.get("loadout").cloned() {
        Some(v) => match serde_json::from_value(v) {
            Ok(l) => l,
            Err(e) => return err("BAD_REQUEST", format!("invalid loadout: {e}")),
        },
        None => return err("BAD_REQUEST", "missing 'loadout'"),
    };
    let lo_path = mgr_loadout_path(&payload);
    match gore_mod::mgr::loadout::save(&lo_path, &loadout) {
        Ok(()) => json!({"ok": true}),
        Err(e) => err("IO", e.to_string()),
    }
}

/// `{library_dir?, loadout_path?}` → `{ok, conflicts:[Conflict]}` — pure conflict analysis of the
/// enabled loadout against the library.
fn mgr_analyze(payload: Value) -> Value {
    let lib = mgr_library_dir(&payload);
    let lo_path = mgr_loadout_path(&payload);
    let mods = match gore_mod::mgr::import::list(&lib) {
        Ok(m) => m,
        Err(e) => return err("ANALYZE_FAILED", e.to_string()),
    };
    let loadout = match gore_mod::mgr::loadout::load(&lo_path) {
        Ok(l) => l,
        Err(e) => return err("ANALYZE_FAILED", e.to_string()),
    };
    let refs: Vec<&gore_mod::mgr::ModEntryMeta> = mods.iter().collect();
    let conflicts = gore_mod::mgr::analyze::analyze(&refs, &loadout);
    json!({"ok": true, "conflicts": serde_json::to_value(&conflicts).unwrap_or(Value::Null)})
}

/// `{game_root, library_dir?, loadout_path?}` → `{ok, report:ApplyReport}` — realize the enabled
/// loadout into one manager deployment. A studio deploy in the way maps to STUDIO_DEPLOY_ACTIVE.
fn mgr_apply(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    let lib = mgr_library_dir(&payload);
    let lo_path = mgr_loadout_path(&payload);
    let loadout = match gore_mod::mgr::loadout::load(&lo_path) {
        Ok(l) => l,
        Err(e) => return err("APPLY_FAILED", e.to_string()),
    };
    match gore_mod::mgr::apply::apply_loadout(std::path::Path::new(game_root), &lib, &loadout) {
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

/// `{game_root, library_dir?, loadout_path?}` → `{ok, status:ManagerStatus}` — diff deployed vs
/// target loadout. `library_dir` lets status fingerprint each enabled mod's current content so a
/// same-id re-import (update) is reported as changes-pending rather than in-sync.
fn mgr_status(payload: Value) -> Value {
    let Some(game_root) = payload.get("game_root").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'game_root'");
    };
    let lib = mgr_library_dir(&payload);
    let lo_path = mgr_loadout_path(&payload);
    let loadout = match gore_mod::mgr::loadout::load(&lo_path) {
        Ok(l) => l,
        Err(e) => return err("STATUS_FAILED", e.to_string()),
    };
    match gore_mod::mgr::status::status(std::path::Path::new(game_root), &lib, &loadout) {
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
                "abi": 1,
                "version": env!("CARGO_PKG_VERSION"),
                "commands": [
                    "audio_extract",
                    "audio_list",
                    "authoring_draft_quest_skeleton_v1_generate",
                    "authoring_logical_npc_clone_draft_v1_generate",
                    "authoring_npc_archetype_catalog_v1_build_for_game_root",
                    "authoring_project_check",
                    "authoring_project_story_draft_insert_v1",
                    "authoring_project_story_quest_draft_insert_v1",
                    "authoring_read_dataasset_extract_receipt_v2",
                    "authoring_store_build_revision3_voice_v1",
                    "authoring_store_check_revision3_npc_compiler_v1",
                    "authoring_store_check_revision3_quest_compiler_v1",
                    "authoring_store_import_ogg",
                    "authoring_store_inspect_revision3_installed_dataasset_v1",
                    "authoring_store_inspect_revision3_npc_source_v1",
                    "authoring_store_inspect_revision3_quest_source_v1",
                    "authoring_store_list_revision3_dataasset_stages_v1",
                    "authoring_store_open",
                    "authoring_store_open_document",
                    "authoring_store_open_head_bytes",
                    "authoring_store_open_head_bytes_document",
                    "authoring_store_open_revision3",
                    "authoring_store_open_revision3_head_bytes",
                    "authoring_store_prepare_checkpoint",
                    "authoring_store_prepare_document_checkpoint",
                    "authoring_store_prepare_remove_revision3_dataasset_stage_v1",
                    "authoring_store_prepare_revision3_checkpoint",
                    "authoring_store_prepare_revision3_dataasset_edit_v1",
                    "authoring_store_prepare_revision3_dataasset_stage_v1",
                    "authoring_store_prepare_revision3_dialog_line_v1",
                    "authoring_store_prepare_revision3_dialog_localization_edit_v1",
                    "authoring_store_prepare_revision3_installed_dataasset_edit_v1",
                    "authoring_store_prepare_revision3_npc_draft_v1",
                    "authoring_store_prepare_revision3_quest_context_edit_v1",
                    "authoring_store_prepare_revision3_quest_draft_v3",
                    "authoring_store_prepare_revision3_quest_outline_edit_v1",
                    "authoring_store_prepare_revision3_quest_outline_edit_v2",
                    "authoring_store_prepare_revision3_quest_transitions_edit_v1",
                    "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1",
                    "authoring_store_prepare_revision3_voice_take_selection_v1",
                    "authoring_store_prepare_revision3_voice_take_v1",
                    "authoring_store_prepare_revision3_voice_target_v1",
                    "authoring_store_read_revision3_content_index_v1",
                    "authoring_store_read_revision3_dataasset_package_index_v1",
                    "authoring_store_read_revision3_dialog_localization_edit_seed_v1",
                    "authoring_store_read_revision3_dialog_localization_v1",
                    "authoring_store_verify_asset",
                    "authoring_story_build_plan_v1_generate",
                    "authoring_story_catalog_v1_build",
                    "authoring_story_catalog_v1_build_for_game_root",
                    "authoring_story_catalog_v1_read",
                    "authoring_story_inventory_v1_build",
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
                    "mgr_remove",
                    "mgr_set_loadout",
                    "mgr_status",
                    "mgr_undeploy_all",
                    "mod_build",
                    "mod_deploy",
                    "mod_undeploy",
                    "script_compile",
                    "script_compile_install_state_v1",
                    "script_compile_report_v1",
                    "script_emit_module",
                    "script_list_modules",
                    "texture_extract",
                    "texture_index",
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
            .any(|command| command == "authoring_project_check"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_project_story_draft_insert_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_story_build_plan_v1_generate"));
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
            .any(|command| command == "authoring_story_inventory_v1_build"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_logical_npc_clone_draft_v1_generate"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_npc_archetype_catalog_v1_build_for_game_root"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_checkpoint"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_document_checkpoint"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_list_revision3_dataasset_stages_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_inspect_revision3_npc_source_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_inspect_revision3_quest_source_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_read_dataasset_extract_receipt_v2"));
        assert!(commands.iter().any(
            |command| command == "authoring_store_prepare_remove_revision3_dataasset_stage_v1"
        ));
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
            .any(|command| command == "authoring_store_prepare_revision3_quest_context_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_draft_v3"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_outline_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_quest_outline_edit_v2"));
        assert!(commands.iter().any(
            |command| command == "authoring_store_prepare_revision3_quest_transitions_edit_v1"
        ));
        assert!(commands.iter().any(|command| command
            == "authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_take_selection_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_take_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_prepare_revision3_voice_target_v1"));
        assert!(commands
            .iter()
            .any(|command| command == "authoring_store_build_revision3_voice_v1"));
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

    #[test]
    fn legacy_script_compile_never_resets_the_caller_work_tree() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let script = game.join("G1R/Script");
        let work = root.path().join("caller-work");
        let victim_tree = work.join("tree");
        std::fs::create_dir_all(&script).unwrap();
        std::fs::create_dir_all(&victim_tree).unwrap();
        std::fs::write(
            script.join("PrecompiledScript_Shipping.Cache"),
            b"invalid-but-readable-pristine",
        )
        .unwrap();
        std::fs::write(victim_tree.join("keep.txt"), b"caller-owned").unwrap();

        let response = script_compile(json!({
            "game_dir": game.display().to_string(),
            "op": "add",
            "module_name": "NeverRuns",
            "rel_path": "NeverRuns.as",
            "as_path": root.path().join("missing.as").display().to_string(),
            "work_dir": work.display().to_string(),
            "allow_new_symbols": false,
        }));

        assert_eq!(response["ok"], false);
        assert_eq!(
            std::fs::read(victim_tree.join("keep.txt")).unwrap(),
            b"caller-owned"
        );
        assert!(!game.join(".gore-install-mutation.lock").exists());
        let owned_children = std::fs::read_dir(&work)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("gore-owned-compile-")
            })
            .count();
        assert_eq!(
            owned_children, 1,
            "legacy compile must use its own staging child"
        );
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
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bundle = build_bundle(&spec).unwrap();
        let dir = root.join(name);
        gore_mod::write_bundle(&dir, &bundle).unwrap();
        dir
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
        assert_eq!(imp["entry"]["name"], "Probe");
        assert_eq!(imp["entry"]["kind"], "goremod");
        let id = imp["entry"]["id"].as_str().unwrap().to_string();

        let list = mgr_call(
            "mgr_library_list",
            json!({"library_dir": lib.display().to_string(), "loadout_path": lo.display().to_string()}),
        );
        assert_eq!(list["ok"], true);
        let mods = list["mods"].as_array().unwrap();
        assert_eq!(mods.len(), 1, "one imported mod: {list}");
        assert_eq!(mods[0]["id"], id);
        assert_eq!(mods[0]["name"], "Probe");

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
    fn mgr_set_loadout_persists() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        let set = mgr_call(
            "mgr_set_loadout",
            json!({
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

    // Removing a mod must also drop it from the persisted loadout (mirror `gore mgr remove`), so a
    // later mgr_apply reading the on-disk loadout does not fail on the deleted mod's metadata.
    #[test]
    fn mgr_remove_drops_loadout_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("library");
        let lo = tmp.path().join("loadout.json");
        std::fs::create_dir_all(&lib).unwrap();
        mgr_call(
            "mgr_set_loadout",
            json!({
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
        let lo = tmp.path().join("loadout.json");
        let v = mgr_call(
            "mgr_status",
            json!({"game_root": game.display().to_string(), "loadout_path": lo.display().to_string()}),
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
