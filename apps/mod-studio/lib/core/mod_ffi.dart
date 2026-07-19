import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:path/path.dart' as p;

import '../dataasset/domain/dataasset_inspection.dart';
import '../dataasset/domain/dataasset_semantic_edit.dart';
import '../dataasset/domain/reviewed_dataasset_schema.dart';
import '../project/revision3_content_index.dart';
import '../scripts/domain/script_compile_install_state.dart';
import '../scripts/domain/script_compile_report.dart';
import 'core_service.dart';

export '../dataasset/domain/dataasset_inspection.dart';
export '../dataasset/domain/dataasset_semantic_edit.dart';
export '../dataasset/domain/reviewed_dataasset_schema.dart';

part '../project/revision3_dataasset_stage.dart';
part '../project/revision3_dataasset_build.dart';
part '../project/revision3_dataasset_package_index.dart';
part '../project/revision3_installed_dataasset_inspection.dart';
part '../project/revision3_dialog_localization_edit.dart';
part '../project/revision3_dialog_line_entry.dart';
part '../project/revision3_dialog_voice_slot_creation.dart';
part '../project/revision3_dialog_voice_slot_removal.dart';
part '../project/revision3_managed_compiler_check.dart';
part '../project/revision3_project_build_plan.dart';
part '../project/revision3_project_compiler_check.dart';
part '../project/revision3_npc_draft.dart';
part '../project/revision3_npc_greeting.dart';
part '../project/revision3_npc_profile_edit.dart';
part '../project/revision3_npc_source_inspection.dart';
part '../project/revision3_quest_context.dart';
part '../project/revision3_quest_outline_v2.dart';
part '../project/revision3_quest_source_inspection.dart';
part '../project/revision3_quest_transcript.dart';
part '../project/revision3_quest_transitions.dart';
part '../project/revision3_project_export.dart';
part '../project/revision3_project_history_wire.dart';
part '../project/revision3_story_draft_removal.dart';
part '../project/revision3_voice_build.dart';
part '../project/revision3_voice_batch.dart';
part '../project/revision3_voice_take.dart';
part '../project/revision3_voice_take_media_qa_authoring.dart';
part '../project/revision3_voice_take_preview.dart';
part '../project/revision3_voice_take_removal.dart';
part '../project/revision3_voice_take_selection.dart';
part '../project/revision3_voice_take_status.dart';
part '../project/revision3_voice_target.dart';

const _maxNativeErrorCodeLength = 128;
const _maxNativeErrorMessageLength = 64 * 1024;
const _maxVoiceOggPathBytes = 32 * 1024;
const _maxVoiceOggInspectRequestBytes = _maxVoiceOggPathBytes * 6 + 256;
const _maxVoiceOggBytes = 64 * 1024 * 1024;
const _maxDataAssetPathBytes = 32 * 1024;
const _maxDataAssetInspectRequestBytes = _maxDataAssetPathBytes * 12 + 512;
const _maxDataAssetExportIndex = 0x7fffffff;
const _maxAuthoringStorePathBytes = 32 * 1024;
const _maxAuthoringHeadJsonBytes = 64 * 1024;
const _maxAuthoringProjectJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringRevision3SnapshotBytes =
    _maxAuthoringProjectJsonBytes + 1024 * 1024;
const _maxAuthoringRevision3ContentIndexJsonBytes = 32 * 1024 * 1024;
const _maxAuthoringRevision3DataAssetResponseBytes = 64 * 1024 * 1024;
const _maxAuthoringRevision3DataAssetPackageIndexJsonBytes = 64 * 1024 * 1024;
const _maxAuthoringRevision3DataAssetEditRequestBytes =
    _maxDataAssetPathBytes * 8 +
    _maxAuthoringHeadJsonBytes * 2 +
    8 * 1024 * 1024;
const _maxAuthoringRevision3InstalledDataAssetEditRequestBytes =
    (_maxAuthoringStorePathBytes * 2 + _maxAuthoringHeadJsonBytes) * 6 +
    8 * 1024 +
    8 * 1024 * 1024;
const _maxAuthoringRevision3ReviewedInstalledDataAssetEditRequestBytes =
    (_maxAuthoringStorePathBytes * 2 + _maxAuthoringHeadJsonBytes) * 6 +
    12 * 1024;
const _maxAuthoringRevision3DataAssetStages = 1024;
const _maxAuthoringRevision3DataAssetManifestBytes = 8 * 1024 * 1024;
const _maxAuthoringRevision3DataAssetManifestStringBytes = 32 * 1024;
const _maxAuthoringRevision3QuestRequestJsonBytes = 64 * 1024;
const _maxAuthoringRevision3NpcRequestJsonBytes = 32 * 1024;
const _maxAuthoringRevision3QuestCollisionArtifactBytes = 24 * 1024 * 1024;
const _authoringRevision3QuestGeneratorId =
    'gore-authoring.draft-quest-skeleton';
const _authoringRevision3QuestGeneratorVersion = 4;
const _maxAuthoringRevision3QuestObjectives = 8;
const _maxAuthoringRevision3QuestObjectiveTitleBytes = 128;
const _maxAuthoringRevision3QuestObjectiveTitlesBytes =
    _maxAuthoringRevision3QuestObjectives *
    _maxAuthoringRevision3QuestObjectiveTitleBytes;
const _authoringRevision3QuestCollisionCatalogLayer =
    'base-game-plus-exact-revision3-project.story-collisions.v2';
const _authoringRevision3QuestCollisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';
const _authoringRevision3QuestFingerprintDomain =
    'gore-authoring.revision3-quest.input-fingerprint\u0000';
const _maxAuthoringStoryCatalogJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringStoryCatalogNpcs = 2;
const _maxAuthoringStoryCatalogQuestParents = 1;
const _maxAuthoringNpcCatalogJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringNpcCatalogRecords = 4096;
const _maxAuthoringNpcCatalogRejections = 100000;
const _maxAuthoringNpcCatalogTextBytes = 1024;
const _maxAuthoringNpcCatalogTotalTextBytes = 12 * 1024 * 1024;
const _maxAuthoringNpcCatalogFunctionBytecodeDwords = 1024 * 1024;
const _maxAuthoringNpcCatalogFunctionBytecodeBytes =
    _maxAuthoringNpcCatalogFunctionBytecodeDwords * Uint32List.bytesPerElement;
// A signed-safe basis can advance once to the maximum applied revision.
const _maxAuthoringStoryBaseRevision = 0x7ffffffffffffffe;
const _maxAuthoringSignedJsonInteger = 0x7fffffffffffffff;
const _maxAuthoringStoryAppliedRevision = _maxAuthoringSignedJsonInteger;
const _maxAuthoringRevision3QuestBasisRevision = _maxAuthoringStoryBaseRevision;
const _maxAuthoringDraftSourceBytes = 1024 * 1024;
const _maxTextureIndexEntries = 65536;
const _maxTextureAssetPathCodeUnits = 1024;
const _maxTexturePreviewBytes = 64 * 1024 * 1024;
final _nativeErrorCodePattern = RegExp(r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$');
final _textureAssetPathPattern = RegExp(
  r'^/[A-Za-z0-9_]+(?:/[A-Za-z0-9_+\-]+)+$',
);
final _texturePackageIdPattern = RegExp(r'^(0|[1-9][0-9]{0,19})$');
final _texturePreviewTokenPattern = RegExp(r'^[0-9a-f]{64}$');
final _maximumTexturePackageId = (BigInt.one << 64) - BigInt.one;

bool _isValidTextureBuildId(Object? value) =>
    value is String &&
    value.isNotEmpty &&
    value == value.trim() &&
    value.length <= 512 &&
    !value.codeUnits.any((codeUnit) => codeUnit < 0x20 || codeUnit == 0x7f);

bool _isValidTextureAssetPath(Object? value) =>
    value is String &&
    value.length <= _maxTextureAssetPathCodeUnits &&
    _textureAssetPathPattern.hasMatch(value);

bool _isValidTexturePackageId(Object? value) {
  if (value is! String || !_texturePackageIdPattern.hasMatch(value)) {
    return false;
  }
  final parsed = BigInt.tryParse(value);
  return parsed != null && parsed <= _maximumTexturePackageId;
}

final class TextureIndexSnapshot {
  TextureIndexSnapshot({
    required this.buildId,
    required Map<String, String> entries,
  }) : entries = Map<String, String>.unmodifiable(entries);

  final String buildId;
  final Map<String, String> entries;
}

/// Typed wrappers over the gore-ffi commands for audio, read-only voice inspection, stateless
/// authoring checks, and unified mod build/deploy.
class ModFfi {
  ModFfi(this._core);
  final GoreCoreFfiService _core;

  Future<Map<String, Object?>> _call(
    String cmd,
    Map<String, Object?> payload,
  ) async {
    final Map<String, Object?> r;
    try {
      r = await _core.execute(cmd, payload: payload);
    } on FormatException {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'response could not be decoded',
      );
    }

    final ok = r['ok'];
    if (ok == true) return r;
    if (ok != false) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field ok must be a bool',
      );
    }
    if (r.length != 2 || !r.containsKey('error')) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'error response has an invalid schema',
      );
    }

    final error = r['error'];
    if (error is! Map ||
        error.length != 2 ||
        !error.containsKey('code') ||
        !error.containsKey('message')) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field error has an invalid schema',
      );
    }
    final code = error['code'];
    if (code is! String ||
        code.isEmpty ||
        utf8.encode(code).length > _maxNativeErrorCodeLength ||
        !_nativeErrorCodePattern.hasMatch(code)) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field error.code is invalid',
      );
    }
    final message = error['message'];
    if (message is! String ||
        utf8.encode(message).length > _maxNativeErrorMessageLength ||
        message.trim().isEmpty) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field error.message is invalid',
      );
    }
    throw ModFfiException(command: cmd, code: code, message: message);
  }

  Future<List<AudioSampleInfo>> audioList(String bank, {String? key}) async {
    final payload = <String, Object?>{'bank': bank};
    if (key != null) payload['key'] = key;
    final r = await _call('audio_list', payload);
    final list = (r['samples'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => AudioSampleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Extract one sample to a temp .ogg; returns its path.
  Future<String> audioExtract(String bank, String sample, {String? key}) async {
    final payload = <String, Object?>{'bank': bank, 'sample': sample};
    if (key != null) payload['key'] = key;
    final r = await _call('audio_extract', payload);
    return r['ogg_path'] as String;
  }

  /// Resolve every eligible exact ASCII case-insensitive `${locId}.ogg` basename in one read-only
  /// voice snapshot.
  /// Ambiguous matches are returned in full and are never selected by the backend.
  Future<VoiceArchiveMatchLineResult> voiceArchiveMatchLine({
    required String archive,
    required String locId,
  }) async {
    final r = await _call('voice_archive_match_line', {
      'archive': archive,
      'loc_id': locId,
    });
    return VoiceArchiveMatchLineResult.fromJson(r);
  }

  /// Safely inspect one selected Ogg before it can enter a Voice edit.
  Future<VoiceOggInspectionResult> voiceOggInspectV1({
    required String oggPath,
  }) async {
    _voiceOggInspectPath(oggPath);
    const command = 'voice_ogg_inspect_v1';
    _voiceOggInspectEnvelopePreflight(command, oggPath);
    final response = await _call(command, {'ogg_path': oggPath});
    return VoiceOggInspectionResult.fromJson(response);
  }

  /// Inspect a cooked package pair against an exact USMAP snapshot without
  /// writing either input or retaining runtime state.
  Future<DataAssetInspection> dataAssetFixedInspectV1({
    required String uassetPath,
    required String usmapPath,
    int? exportIndex,
  }) async {
    _dataAssetInspectPath(uassetPath, 'uassetPath');
    _dataAssetInspectPath(usmapPath, 'usmapPath');
    if (exportIndex != null &&
        (exportIndex < 0 || exportIndex > _maxDataAssetExportIndex)) {
      throw ArgumentError.value(
        exportIndex,
        'exportIndex',
        'must be 0..=$_maxDataAssetExportIndex',
      );
    }
    const command = 'dataasset_fixed_inspect_v1';
    _dataAssetInspectEnvelopePreflight(
      command,
      uassetPath,
      usmapPath,
      exportIndex,
    );
    final payload = <String, Object?>{
      'uasset_path': uassetPath,
      'usmap_path': usmapPath,
    };
    if (exportIndex != null) payload['export_index'] = exportIndex;
    final response = await _call(command, payload);
    return DataAssetInspection.fromJson(response);
  }

  /// Verify and project one exact pinned `story_catalog.v1` document without filesystem access.
  Future<AuthoringStoryCatalogSelections> authoringStoryCatalogV1Read({
    required String catalogJson,
  }) async {
    _authoringDraftRequestString(
      catalogJson,
      'catalogJson',
      _maxAuthoringStoryCatalogJsonBytes,
    );
    const command = 'authoring_story_catalog_v1_read';
    _authoringSingleRawJsonEnvelopePreflight(
      command,
      'catalog_json',
      'catalogJson',
      catalogJson,
    );
    final response = await _call(command, {'catalog_json': catalogJson});
    return AuthoringStoryCatalogSelections._fromJson(
      response,
      catalogJson: catalogJson,
    );
  }

  /// Build the pinned Story catalog from one installed generation entirely in memory.
  Future<AuthoringStoryCatalogBuildResult> authoringStoryCatalogV1Build({
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) async {
    _authoringStoryCatalogPath(executable, 'executable');
    _authoringStoryCatalogPath(shippingCache, 'shippingCache');
    _authoringStoryCatalogPath(bindsCache, 'bindsCache');
    const command = 'authoring_story_catalog_v1_build';
    _authoringStoryCatalogBuildEnvelopePreflight(
      command,
      executable,
      shippingCache,
      bindsCache,
    );
    final response = await _call(command, {
      'executable': executable,
      'shipping_cache': shippingCache,
      'binds_cache': bindsCache,
    });
    return AuthoringStoryCatalogBuildResult._fromJson(
      response,
      executable: executable,
      shippingCache: shippingCache,
      bindsCache: bindsCache,
    );
  }

  /// Build the pinned Story catalog while native gore-mod selects the pristine cache.
  Future<AuthoringStoryCatalogBuildResult>
  authoringStoryCatalogV1BuildForGameRoot({required String gameRoot}) async {
    _authoringStoryCatalogPath(gameRoot, 'gameRoot');
    const command = 'authoring_story_catalog_v1_build_for_game_root';
    _authoringSingleRawJsonEnvelopePreflight(
      command,
      'game_root',
      'gameRoot',
      gameRoot,
    );
    final response = await _call(command, {'game_root': gameRoot});
    return AuthoringStoryCatalogBuildResult._fromGameRootJson(
      response,
      gameRoot: gameRoot,
    );
  }

  /// Build one generation-sealed, read-only NPC archetype catalog from the native game root.
  Future<AuthoringNpcArchetypeCatalogBuildResult>
  authoringNpcArchetypeCatalogV1BuildForGameRoot({
    required String gameRoot,
  }) async {
    _authoringStoryCatalogPath(gameRoot, 'gameRoot');
    const command = 'authoring_npc_archetype_catalog_v1_build_for_game_root';
    _authoringSingleRawJsonEnvelopePreflight(
      command,
      'game_root',
      'gameRoot',
      gameRoot,
    );
    final response = await _call(command, {'game_root': gameRoot});
    return AuthoringNpcArchetypeCatalogBuildResult._fromJson(
      response,
      gameRoot: gameRoot,
    );
  }

  /// Build and then pass the exact raw catalog through the existing pinned chooser reader.
  Future<AuthoringStoryCatalogSelections> authoringStoryCatalogV1BuildAndRead({
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) async {
    final built = await authoringStoryCatalogV1Build(
      executable: executable,
      shippingCache: shippingCache,
      bindsCache: bindsCache,
    );
    return authoringStoryCatalogV1Read(catalogJson: built.catalogJson);
  }

  /// Build from a root-owned pristine snapshot, then project its exact canonical catalog.
  Future<AuthoringStoryCatalogSelections>
  authoringStoryCatalogV1BuildAndReadForGameRoot({
    required String gameRoot,
  }) async {
    final built = await authoringStoryCatalogV1BuildForGameRoot(
      gameRoot: gameRoot,
    );
    return authoringStoryCatalogV1Read(catalogJson: built.catalogJson);
  }

  /// Open the fixed head as one exact schema-revision-3 checkpoint.
  ///
  /// This is a read-only reconstruction. It grants no build, runtime, deployment, or publication
  /// authority.
  Future<AuthoringRevision3StoreOpenedResult> authoringStoreOpenRevision3({
    required String root,
    required AuthoringAssetVerification verification,
  }) async {
    const command = 'authoring_store_open_revision3';
    _authoringRevision3StorePath(root);
    _authoringRevision3OpenEnvelopePreflight(
      command,
      root,
      verification.wireName,
    );
    final response = await _call(command, {
      'root': root,
      'verification': verification.wireName,
    });
    return AuthoringRevision3StoreOpenedResult.fromJson(response);
  }

  /// Read the bounded semantic index of one exact, currently-published revision-3 checkpoint.
  ///
  /// Native code fully verifies the project and its assets before and after projection. This
  /// remains a read-only content view and grants no build, runtime, deployment, or publication
  /// authority.
  Future<AuthoringRevision3ContentIndexResult>
  authoringStoreReadRevision3ContentIndexV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_read_revision3_content_index_v1';
    _authoringRevision3StorePath(root);
    _authoringRevision3ContentReadEnvelopePreflight(
      command,
      expectedHead.canonicalJson,
      root,
    );
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
    });
    return AuthoringRevision3ContentIndexResult.fromJson(
      response,
      expectedHead: expectedHead,
    );
  }

  /// Read bounded text previews for one exact-current managed LocalizationEntry.
  ///
  /// The wire carries no game root or project JSON and grants no mutation,
  /// publication, build, deployment, topic, save, or runtime authority.
  Future<AuthoringRevision3DialogLocalizationReadResult>
  authoringStoreReadRevision3DialogLocalizationV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) async {
    const command = 'authoring_store_read_revision3_dialog_localization_v1';
    _authoringRevision3Path(root, 'root');
    final request = AuthoringRevision3DialogLocalizationReadRequestV1(
      expectedHead: expectedHead,
      localizationId: localizationId,
      expectedLocalizationRevision: expectedLocalizationRevision,
      expectedLocId: expectedLocId,
    );
    final response = await _call(command, request._payload(root));
    try {
      return AuthoringRevision3DialogLocalizationReadResult.fromJson(
        response,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Read one exact-current authored LocalizationEntry together with the
  /// bounded DialogLine and VoiceSlot facts needed by a safe text editor.
  ///
  /// This is read-only and grants no build, runtime, or publication authority.
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  authoringStoreReadRevision3DialogLocalizationEditSeedV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) async {
    const command =
        'authoring_store_read_revision3_dialog_localization_edit_seed_v1';
    _authoringRevision3Path(root, 'root');
    final request = AuthoringRevision3DialogLocalizationEditSeedRequestV1(
      expectedHead: expectedHead,
      localizationId: localizationId,
      expectedLocalizationRevision: expectedLocalizationRevision,
      expectedLocId: expectedLocId,
    );
    final response = await _call(command, request._payload(root));
    try {
      return AuthoringRevision3DialogLocalizationEditSeed.fromJson(
        response,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare immutable schema-revision-3 objects without publishing the fixed head.
  ///
  /// This is prepare-only: `expectedHead == null` asserts that the fixed head is absent, while a
  /// head value is passed through as its exact canonical CAS string. The project string is also
  /// preserved byte-for-byte so native duplicate-key and canonical-byte checks remain authoritative.
  Future<AuthoringRevision3CheckpointPreparation>
  authoringStorePrepareRevision3Checkpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    const command = 'authoring_store_prepare_revision3_checkpoint';
    _authoringRevision3StorePath(root);
    _authoringRevision3RequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRevision3PrepareEnvelopePreflight(
      command,
      root,
      expectedHead?.canonicalJson,
      projectJson,
    );
    final response = await _call(command, {
      'root': root,
      'expected_head_json': expectedHead?.canonicalJson,
      'project_json': projectJson,
    });
    return AuthoringRevision3CheckpointPreparation.fromJson(response);
  }

  /// Read a bounded authenticated lineage rooted at the exact current R3 head.
  ///
  /// Native code follows only parent records sealed by the published snapshot;
  /// it never scans physical CAS directories or grants mutation authority.
  Future<AuthoringRevision3ProjectHistoryResult>
  authoringStoreListRevision3HistoryV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_list_revision3_history_v1';
    _authoringRevision3StorePath(root);
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3ProjectHistoryResult.fromJson(
        response,
        expectedHead: expectedHead,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare an append-only restore candidate from one authenticated ancestor.
  ///
  /// This installs immutable objects only. The managed session still owns full
  /// candidate reopen, exact fixed-head CAS publication, and published reopen.
  Future<AuthoringRevision3ProjectHistoryRestorePreparation>
  authoringStorePrepareRevision3HistoryRestoreV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required AuthoringWorkingHead targetHead,
  }) async {
    const command = 'authoring_store_prepare_revision3_history_restore_v1';
    _authoringRevision3StorePath(root);
    if (targetHead.canonicalJson == expectedHead.canonicalJson) {
      throw ArgumentError.value(
        targetHead,
        'targetHead',
        'must name an older authenticated checkpoint',
      );
    }
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
      'target_head_json': targetHead.canonicalJson,
    });
    try {
      return AuthoringRevision3ProjectHistoryRestorePreparation.fromJson(
        response,
        expectedHead: expectedHead,
        targetHead: targetHead,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one revision-3 Quest Draft checkpoint without publishing the fixed head.
  ///
  /// Both nested JSON strings cross the native boundary byte-for-byte. Native code rebuilds the
  /// game/catalog capability, validates the exact published basis, installs only immutable
  /// objects, and returns an unpublished structural candidate. This wrapper grants no build,
  /// runtime, deployment, source-inspection, artifact, or publication authority.
  Future<AuthoringRevision3QuestDraftPreparation>
  authoringStorePrepareRevision3QuestDraftV3({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required String questRequestJson,
  }) async {
    const command = 'authoring_store_prepare_revision3_quest_draft_v3';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    // The native model contains unsigned 64-bit counters and seals. Keep the entire JSON-number
    // surface inside Dart/native's shared signed wire domain before crossing the FFI boundary;
    // checking only the project revision would still allow an unsafe nested seal or asset length.
    _authoringRequireCanonicalRevision3ProjectJson(currentProjectJson);
    _authoringRevision3RequestString(
      questRequestJson,
      'questRequestJson',
      _maxAuthoringRevision3QuestRequestJsonBytes,
    );
    _authoringRevision3QuestPrepareEnvelopePreflight(
      command,
      root,
      gameRoot,
      currentProjectJson,
      questRequestJson,
    );
    final response = await _call(command, {
      'current_project_json': currentProjectJson,
      'game_root': gameRoot,
      'quest_request_json': questRequestJson,
      'root': root,
    });
    try {
      return AuthoringRevision3QuestDraftPreparation.fromJson(response);
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Inspect the deterministic generated source for one exact-current
  /// revision-3 Quest.
  ///
  /// Native code reopens the exact fixed head and privately verifies its
  /// persisted collision evidence against fresh game inputs. The returned
  /// plan is source-inspection-only: it grants no compile, build, runtime,
  /// deployment, mutation, or publication authority.
  Future<AuthoringRevision3QuestSourceInspectionResult>
  authoringStoreInspectRevision3QuestSourceV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  }) async {
    const command = 'authoring_store_inspect_revision3_quest_source_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    final requestedQuestId = _authoringRevision3QuestSourceInspectionEntityId(
      questId,
      'questId',
    );
    final response = await _call(command, <String, Object?>{
      'root': root,
      'game_root': gameRoot,
      'expected_head_json': expectedHead.canonicalJson,
      'quest_id': requestedQuestId,
    });
    try {
      return AuthoringRevision3QuestSourceInspectionResult.fromJson(
        response,
        expectedHead: expectedHead,
        requestedQuestId: requestedQuestId,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Inspect the persisted source/readiness evidence for one exact-current
  /// revision-3 NPC Draft. This is a project-only read: no game installation,
  /// compiler, build, spawn, deployment, mutation, or publication authority is
  /// involved.
  Future<AuthoringRevision3NpcSourceInspectionResult>
  authoringStoreInspectRevision3NpcSourceV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  }) async {
    const command = 'authoring_store_inspect_revision3_npc_source_v1';
    _authoringRevision3Path(root, 'root');
    final requestedNpcId = _authoringRevision3NpcEntityId(<String, Object?>{
      'npc_id': npcId,
    }, 'npc_id');
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'npc_id': requestedNpcId,
      'root': root,
    });
    try {
      return AuthoringRevision3NpcSourceInspectionResult.fromJson(
        response,
        expectedHead: expectedHead,
        requestedNpcId: requestedNpcId,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Ask the game compiler to check every native-derived managed ScriptModule
  /// from one exact revision-3 Store head in one common compiler run. Native
  /// retains no compiled artifact and this result grants no build, deployment,
  /// runtime, or publication authority.
  Future<AuthoringRevision3ProjectCompilerCheckResult>
  authoringStoreCheckRevision3ProjectCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_check_revision3_project_compiler_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('expectedHead', expectedHead.canonicalJson),
      ('gameRoot', gameRoot),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'root': root,
      'game_root': gameRoot,
      'expected_head_json': expectedHead.canonicalJson,
    });
    try {
      return AuthoringRevision3ProjectCompilerCheckResult.fromJson(
        response,
        expectedHead: expectedHead,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Ask the game compiler to check the native-derived source for one exact
  /// revision-3 Quest without returning or adopting a compiled artifact.
  Future<AuthoringRevision3ManagedCompilerCheckResult>
  authoringStoreCheckRevision3QuestCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  }) => _authoringStoreCheckRevision3ManagedCompilerV1(
    command: 'authoring_store_check_revision3_quest_compiler_v1',
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    entityId: questId,
    entityWireField: 'quest_id',
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.questDraft,
  );

  /// Ask the game compiler to check the native-derived source for one exact
  /// revision-3 NPC without returning or adopting a compiled artifact.
  Future<AuthoringRevision3ManagedCompilerCheckResult>
  authoringStoreCheckRevision3NpcCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  }) => _authoringStoreCheckRevision3ManagedCompilerV1(
    command: 'authoring_store_check_revision3_npc_compiler_v1',
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    entityId: npcId,
    entityWireField: 'npc_id',
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.npcDraft,
  );

  Future<AuthoringRevision3ManagedCompilerCheckResult>
  _authoringStoreCheckRevision3ManagedCompilerV1({
    required String command,
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String entityId,
    required String entityWireField,
    required AuthoringRevision3ManagedCompilerEntityKind expectedKind,
  }) async {
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    final requestedEntityId = _authoringRevision3ManagedCompilerEntityId(
      entityId,
      entityWireField,
    );
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('expectedHead', expectedHead.canonicalJson),
      ('gameRoot', gameRoot),
      (entityWireField, requestedEntityId),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'root': root,
      'game_root': gameRoot,
      'expected_head_json': expectedHead.canonicalJson,
      entityWireField: requestedEntityId,
    });
    try {
      return AuthoringRevision3ManagedCompilerCheckResult.fromJson(
        response,
        expectedHead: expectedHead,
        requestedEntityId: requestedEntityId,
        expectedKind: expectedKind,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Read the metadata-only installed package-candidate index for the exact
  /// current revision-3 project generation. The native command reads no export
  /// payload and grants no extraction, mutation, build, runtime, or publication
  /// authority.
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  authoringStoreReadRevision3DataAssetPackageIndexV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_read_revision3_dataasset_package_index_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('expectedHead', expectedHead.canonicalJson),
      ('gameRoot', gameRoot),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'game_root': gameRoot,
      'root': root,
    });
    try {
      return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
        response,
        expectedHead: expectedHead,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Inspect one package selected by its original ordinal from an exact
  /// installed package snapshot. Native code rebuilds and compares that
  /// snapshot before it resolves the target; no caller-supplied `/Game` path,
  /// extraction path, edit, build, or deployment authority crosses the wire.
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  authoringStoreInspectRevision3InstalledDataAssetV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) async {
    const command = 'authoring_store_inspect_revision3_installed_dataasset_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    if (expectedSnapshot.head.canonicalJson != expectedHead.canonicalJson ||
        candidate.ordinal < 0 ||
        candidate.ordinal >= expectedSnapshot.index.candidates.length ||
        !identical(
          candidate,
          expectedSnapshot.index.candidates[candidate.ordinal],
        )) {
      throw ArgumentError(
        'candidate must come from the exact installed DataAsset snapshot',
        'candidate',
      );
    }
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('candidateOrdinal', candidate.ordinal.toString()),
      ('expectedHead', expectedHead.canonicalJson),
      (
        'expectedPackageIndexByteLength',
        expectedSnapshot.packageIndexSeal.byteLength.toString(),
      ),
      ('expectedPackageIndexSha256', expectedSnapshot.packageIndexSeal.sha256),
      (
        'expectedSourceSnapshotByteLength',
        expectedSnapshot.sourceSnapshotSeal.byteLength.toString(),
      ),
      (
        'expectedSourceSnapshotSha256',
        expectedSnapshot.sourceSnapshotSeal.sha256,
      ),
      ('gameRoot', gameRoot),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'candidate_ordinal': candidate.ordinal,
      'expected_head_json': expectedHead.canonicalJson,
      'expected_package_index_seal': _installedDataAssetSealJson(
        expectedSnapshot.packageIndexSeal,
      ),
      'expected_source_snapshot_seal': _installedDataAssetSealJson(
        expectedSnapshot.sourceSnapshotSeal,
      ),
      'game_root': gameRoot,
      'root': root,
    });
    try {
      return AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
        response,
        expectedSnapshot: expectedSnapshot,
        requestedOrdinal: candidate.ordinal,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one typed fixed-leaf edit from the exact installed snapshot and
  /// read-only inspection that produced the selector. Native code reopens and
  /// revalidates both source authorities, independently reconstructs the
  /// cooked package, and returns only an unpublished managed stage candidate.
  Future<AuthoringRevision3DataAssetStagePreparation>
  authoringStorePrepareRevision3InstalledDataAssetEditV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required DataAssetInstalledSemanticEditIntent intent,
  }) async {
    const command =
        'authoring_store_prepare_revision3_installed_dataasset_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    if (intent.snapshot.head.canonicalJson != expectedHead.canonicalJson ||
        intent.inspection.head.canonicalJson != expectedHead.canonicalJson) {
      throw ArgumentError(
        'installed DataAsset edit must use the exact requested head',
        'intent',
      );
    }
    final payload = <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      ...intent.toNativeFields(),
      'game_root': gameRoot,
      'root': root,
    };
    _authoringRevision3DataAssetEditEnvelopePreflight(
      command,
      payload,
      maxBytes: _maxAuthoringRevision3InstalledDataAssetEditRequestBytes,
    );
    final response = await _call(command, payload);
    try {
      final prepared = AuthoringRevision3DataAssetStagePreparation.fromJson(
        response,
        expectedHead: expectedHead,
        expectedIntentBindingSha256: intent.intentBindingSha256,
        expectedInstalledIntent: intent,
      );
      final selector = intent.selector;
      if (prepared.installedSource == null ||
          prepared.stage.targetPath != intent.expectedTargetPath ||
          prepared.stage.selectorKind != selector.kind.wireName ||
          prepared.stage.selectorPathDepth != selector.path.length ||
          prepared.stage.replacementByteLength != selector.kind.width) {
        throw const FormatException(
          'prepared stage does not match the installed typed selector shape',
        );
      }
      return prepared;
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one closed reviewed edit from an exact installed inspection.
  ///
  /// Only the candidate ordinal, snapshot seals, and semantic schema/value
  /// cross the wire. Native code rediscovers the target and exact selector,
  /// lowers the value, and returns an unpublished managed stage candidate.
  Future<AuthoringRevision3DataAssetStagePreparation>
  authoringStorePrepareRevision3ReviewedInstalledDataAssetEditV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) async {
    const command =
        'authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    if (intent.snapshot.head.canonicalJson != expectedHead.canonicalJson ||
        intent.inspection.head.canonicalJson != expectedHead.canonicalJson) {
      throw ArgumentError(
        'reviewed installed DataAsset edit must use the exact requested head',
        'intent',
      );
    }
    final payload = <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      ...intent.toNativeFields(),
      'game_root': gameRoot,
      'root': root,
    };
    _authoringRevision3DataAssetEditEnvelopePreflight(
      command,
      payload,
      maxBytes:
          _maxAuthoringRevision3ReviewedInstalledDataAssetEditRequestBytes,
    );
    final response = await _call(command, payload);
    try {
      final prepared = AuthoringRevision3DataAssetStagePreparation.fromJson(
        response,
        expectedHead: expectedHead,
        expectedIntentBindingSha256: intent.expectedStageIntentBindingSha256,
        expectedReviewedInstalledIntent: intent,
      );
      if (prepared.installedSource == null ||
          prepared.reviewedEdit == null ||
          prepared.stage.targetPath != intent.expectedTargetPath) {
        throw const FormatException(
          'prepared stage does not match the reviewed installed intent',
        );
      }
      return prepared;
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare a stable-slot-aware outline edit for one exact-current
  /// Quest. Native code preserves the transition graph and returns only an
  /// unpublished, build-blocked candidate.
  Future<AuthoringRevision3QuestOutlineEditPreparationV2>
  authoringStorePrepareRevision3QuestOutlineEditV2({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV2 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_quest_outline_edit_v2';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'quest_outline_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3QuestOutlineEditPreparationV2.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one project-only ordered Quest transcript edit. The native
  /// boundary returns an unpublished, build-blocked candidate; fixed-head CAS
  /// publication remains exclusively owned by the managed Dart session.
  Future<AuthoringRevision3QuestTranscriptPreparation>
  authoringStorePrepareRevision3QuestTranscriptV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTranscriptRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_quest_transcript_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'quest_transcript_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3QuestTranscriptPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one project-only ordered NPC greeting edit. Native code returns
  /// an unpublished, build-blocked candidate; the managed Dart session owns
  /// the only fixed-head publication authority.
  Future<AuthoringRevision3NpcGreetingPreparation>
  authoringStorePrepareRevision3NpcGreetingV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3NpcGreetingRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_npc_greeting_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'npc_greeting_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3NpcGreetingPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one fresh-Story-catalog-backed context edit for an exact-current
  /// managed revision-3 Quest and its already-owned generated ScriptModule.
  ///
  /// Catalog IDs are the only catalog selections crossing the intent wire.
  /// Native code owns game/catalog authority and cannot publish the fixed head.
  Future<AuthoringRevision3QuestContextEditPreparation>
  authoringStorePrepareRevision3QuestContextEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3QuestContextEditRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_quest_context_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'game_root': gameRoot,
      'quest_context_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3QuestContextEditPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one exact-current Quest transition-plan edit without publishing
  /// the fixed Store head or granting build/runtime authority.
  Future<AuthoringRevision3QuestTransitionsEditPreparation>
  authoringStorePrepareRevision3QuestTransitionsEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTransitionsEditRequestV1 request,
  }) async {
    const command =
        'authoring_store_prepare_revision3_quest_transitions_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactCurrent(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'quest_transitions_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3QuestTransitionsEditPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one revision-3 NPC Draft and its deterministic ScriptModule without publishing the
  /// fixed head.
  ///
  /// The request is bound to the exact canonical project/head by its typed constructor. Native
  /// code rebuilds fresh Story, NPC-archetype, and base/current collision inputs. This wrapper
  /// grants no build, runtime, catalog, collision, source-inspection, deployment, or native
  /// publication authority.
  Future<AuthoringRevision3NpcDraftPreparation>
  authoringStorePrepareRevision3NpcDraftV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_npc_draft_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    _authoringRevision3NpcPrepareEnvelopePreflight(
      command,
      root,
      gameRoot,
      currentProjectJson,
      request.canonicalJson,
    );
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'game_root': gameRoot,
      'npc_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3NpcDraftPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one exact existing NPC display-name/archetype edit without
  /// publishing the fixed head or writing game/save files.
  Future<AuthoringRevision3NpcProfileEditPreparation>
  authoringStorePrepareRevision3NpcProfileEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcProfileEditRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_npc_profile_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (request.expectedProjectId != current.projectId ||
        request.expectedRevision != current.revision ||
        request.expectedTargetCanonicalJson !=
            jsonEncode(current.project['target'])) {
      throw const FormatException(
        'revision-3 NPC profile request is not bound to the current project',
      );
    }
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('currentProjectJson', currentProjectJson),
      ('gameRoot', gameRoot),
      ('npcProfileRequestJson', request.canonicalJson),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'game_root': gameRoot,
      'npc_profile_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3NpcProfileEditPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Import one validated Ogg take for an existing revision-3 dialog line/locale and prepare an
  /// unpublished VoiceSlot/VoiceTake candidate.
  ///
  /// Native code binds the request to the exact fixed head and project, installs only immutable
  /// CAS objects, and fully reopens the candidate. This wrapper grants no archive-target, build,
  /// runtime, deployment, or native publication authority.
  Future<AuthoringRevision3VoiceTakePreparation>
  authoringStorePrepareRevision3VoiceTakeV1({
    required String root,
    required String gameRoot,
    required String source,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_voice_take_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3Path(source, 'source');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    _authoringRevision3VoicePrepareEnvelopePreflight(
      command,
      currentProjectJson,
      gameRoot,
      root,
      source,
      request.canonicalJson,
    );
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'game_root': gameRoot,
      'root': root,
      'source': source,
      'voice_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTakePreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Atomically create and retain one native-owned temp directory after
  /// proving it cannot overlap the managed Store.
  Future<AuthoringRevision3VoiceTakePreviewRegistration>
  authoringStoreRegisterRevision3VoiceTakePreviewV1({
    required String root,
  }) async {
    const command = 'authoring_store_register_revision3_voice_take_preview_v1';
    _authoringRevision3Path(root, 'root');
    if (!p.isAbsolute(root)) {
      throw ArgumentError.value(
        '<${root.length} characters>',
        'root',
        'must be an absolute managed Store directory',
      );
    }
    final response = await _call(command, <String, Object?>{'root': root});
    try {
      return AuthoringRevision3VoiceTakePreviewRegistration.fromJson(response);
    } on FormatException catch (error) {
      // A malformed same-build success is fail-closed: never guess a token or ambient path.
      // Native may retain that isolated slot/root until process exit in this exceptional ABI
      // mismatch; validated successes expose only token-bound release authority.
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Inspect one exact-current managed VoiceTake in place. The response is
  /// pathless and grants no preview-file, mutation, build, or runtime authority.
  Future<AuthoringRevision3VoiceTakeMediaQaResult>
  authoringStoreInspectRevision3VoiceTakeMediaV1({
    required String root,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  }) async {
    const command = 'authoring_store_inspect_revision3_voice_take_media_v1';
    _authoringRevision3Path(root, 'root');
    if (!p.isAbsolute(root)) {
      throw ArgumentError.value(
        '<${root.length} characters>',
        'root',
        'must be an absolute managed Store directory',
      );
    }
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('root', root),
      ('voiceTakePreviewRequestJson', request.canonicalJson),
    ]);
    final response = await _call(command, <String, Object?>{
      'root': root,
      'voice_take_preview_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTakeMediaQaResult.fromJson(
        response,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Materialize one exact-current managed CAS VoiceTake through an already
  /// retained opaque preview capability. Ownership is never consumed here.
  Future<AuthoringRevision3VoiceTakePreviewMaterialization>
  authoringStoreMaterializeRevision3VoiceTakePreviewV1({
    required String root,
    required String cleanupToken,
    required String previewRoot,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  }) async {
    const command =
        'authoring_store_materialize_revision3_voice_take_preview_v1';
    _authoringRevision3Path(root, 'root');
    if (!_authoringRevision3VoiceTakePreviewCleanupTokenPattern.hasMatch(
      cleanupToken,
    )) {
      throw ArgumentError('cleanupToken is not one opaque preview token');
    }
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('cleanupToken', cleanupToken),
      ('root', root),
      ('voiceTakePreviewRequestJson', request.canonicalJson),
    ]);
    final response = await _call(command, <String, Object?>{
      'root': root,
      'cleanup_token': cleanupToken,
      'voice_take_preview_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTakePreviewMaterialization.fromJson(
        response,
        previewRoot: previewRoot,
        cleanupToken: cleanupToken,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Release one opaque preview capability. Failures retain the token. A
  /// successful repeat is recognized only within the bounded process-local
  /// tombstone window, with no cross-restart guarantee.
  Future<void> authoringStoreReleaseRevision3VoiceTakePreviewV1({
    required String cleanupToken,
  }) async {
    const command = 'authoring_store_release_revision3_voice_take_preview_v1';
    if (!_authoringRevision3VoiceTakePreviewCleanupTokenPattern.hasMatch(
      cleanupToken,
    )) {
      throw ArgumentError('cleanupToken is not one opaque preview token');
    }
    final response = await _call(command, <String, Object?>{
      'cleanup_token': cleanupToken,
    });
    _authoringExactFields(response, const <String>{
      'ok',
      'outcome',
      'cleanup_status',
      'project_write_status',
      'game_write_status',
      'save_write_status',
      'build_status',
      'deployment_status',
      'runtime_status',
    }, 'revision-3 Voice take preview cleanup response');
    if (response['ok'] != true ||
        response['outcome'] != 'preview_cleanup_complete' ||
        response['cleanup_status'] != 'performed' ||
        response['project_write_status'] != 'not_performed' ||
        response['game_write_status'] != 'not_performed' ||
        response['save_write_status'] != 'not_performed' ||
        response['build_status'] != 'not_performed' ||
        response['deployment_status'] != 'not_performed' ||
        response['runtime_status'] != 'not_qualified') {
      throw const ModFfiException._malformed(
        command: command,
        reason: 'cleanup response grants invalid authority',
      );
    }
  }

  /// Scan one bounded folder of direct Ogg children and derive an exact,
  /// read-only, all-or-nothing Voice batch plan for one canonical locale.
  Future<AuthoringRevision3VoiceBatchPlanResult>
  authoringStorePlanRevision3VoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String locale,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_plan_revision3_voice_batch_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3Path(sourceFolder, 'sourceFolder');
    final canonicalLocale = _authoringRevision3VoiceLocale(locale);
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRevision3RequestString(
      expectedHead.canonicalJson,
      'expectedHeadJson',
      _maxAuthoringHeadJsonBytes,
    );
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'expected_head_json': expectedHead.canonicalJson,
      'game_root': gameRoot,
      'locale': canonicalLocale,
      'root': root,
      'source_folder': sourceFolder,
    });
    try {
      return AuthoringRevision3VoiceBatchPlanResult.fromJson(
        response,
        expectedHead: expectedHead,
        currentProjectJson: currentProjectJson,
        expectedLocale: canonicalLocale,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Re-scan and re-plan one exact ready Voice folder plan, then prepare one
  /// unpublished project revision containing every ready recording or none.
  Future<AuthoringRevision3VoiceBatchPreparation>
  authoringStorePrepareRevision3VoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String currentProjectJson,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) async {
    const command = 'authoring_store_prepare_revision3_voice_batch_v1';
    if (!plan.canPrepare) {
      throw ArgumentError.value(plan.status, 'plan', 'must be ready');
    }
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3Path(sourceFolder, 'sourceFolder');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (current.projectId != plan.projectId ||
        current.revision != plan.revision) {
      throw const FormatException(
        'revision-3 Voice batch plan is not bound to the current project',
      );
    }
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'expected_head_json': plan.basisHead.canonicalJson,
      'game_root': gameRoot,
      'locale': plan.locale,
      'root': root,
      'source_folder': sourceFolder,
      'expected_source_manifest_sha256': plan.sourceManifestSha256,
      'expected_plan_sha256': plan.planSha256,
    });
    try {
      return AuthoringRevision3VoiceBatchPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        plan: plan,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Create one project-local DialogLine and either create or exactly reuse
  /// one managed LocalizationEntry, optionally with an empty VoiceSlot.
  ///
  /// This prepare-only route accepts no game root and grants no topic, build,
  /// runtime, deployment, save, or native publication authority.
  Future<AuthoringRevision3DialogLineEntryPreparation>
  authoringStorePrepareRevision3DialogLineV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogLineEntryRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_dialog_line_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRevision3RequestString(
      request.canonicalJson,
      'dialogLineRequestJson',
      _maxAuthoringRevision3DialogLineRequestBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'dialog_line_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3DialogLineEntryPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare an exact managed LocalizationEntry text-map replacement without
  /// publishing the fixed project head.
  ///
  /// The native transaction fully reopens the immutable candidate. No game
  /// root, build, runtime, deployment, or native publication authority crosses
  /// this boundary.
  Future<AuthoringRevision3DialogLocalizationEditPreparation>
  authoringStorePrepareRevision3DialogLocalizationEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogLocalizationEditRequestV1 request,
  }) async {
    const command =
        'authoring_store_prepare_revision3_dialog_localization_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRevision3RequestString(
      request.canonicalJson,
      'dialogLocalizationEditRequestJson',
      _maxAuthoringRevision3DialogLocalizationEditRequestBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'localization_edit_request_json': request.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3DialogLocalizationEditPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Change or clear the selected take of one exact-current revision-3 Voice
  /// slot and prepare an unpublished candidate.
  ///
  /// The request is derived from the managed project itself. No game root,
  /// audio import, archive target, build, runtime, deployment, or native
  /// publication authority crosses this boundary.
  Future<AuthoringRevision3VoiceTakeSelectionPreparation>
  authoringStorePrepareRevision3VoiceTakeSelectionV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_voice_take_selection_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'root': root,
      'voice_take_selection_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTakeSelectionPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Detach one exact-current Voice take from one line/language slot and
  /// prepare an unpublished project-only candidate. The VoiceTake entity is
  /// removed only when no other exact slot still uses it; immutable audio CAS
  /// metadata remains preserved in either case.
  Future<AuthoringRevision3VoiceTakeRemovalPreparation>
  authoringStorePrepareRevision3VoiceTakeRemovalV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRemovalRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_voice_take_removal_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'root': root,
      'voice_take_removal_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTakeRemovalPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Create one exact empty managed dialog Voice slot, preparing only an
  /// unpublished project candidate.
  Future<AuthoringRevision3DialogVoiceSlotCreationPreparation>
  authoringStorePrepareRevision3DialogVoiceSlotCreationV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogVoiceSlotCreationRequestV1 request,
  }) async {
    const command =
        'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'root': root,
      'dialog_voice_slot_creation_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3DialogVoiceSlotCreationPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Remove one exact empty and unselected dialog Voice slot, preparing only
  /// an unpublished project candidate.
  Future<AuthoringRevision3DialogVoiceSlotRemovalPreparation>
  authoringStorePrepareRevision3DialogVoiceSlotRemovalV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request,
  }) async {
    const command =
        'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'root': root,
      'dialog_voice_slot_removal_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3DialogVoiceSlotRemovalPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Change one exact retained Voice take review status and prepare an
  /// unpublished candidate.
  ///
  /// This project-only boundary carries no game root, media import, build,
  /// runtime, deployment, or publication authority.
  Future<AuthoringRevision3VoiceTakeStatusPreparation>
  authoringStorePrepareRevision3VoiceTakeStatusV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeStatusRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_voice_take_status_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRevision3RequestString(
      request.canonicalJson,
      'voiceTakeStatusRequestJson',
      _maxAuthoringRevision3VoiceTakeStatusRequestBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'root': root,
      'voice_take_status_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTakeStatusPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Inspect the installed locale archive and prepare an unpublished exact Voice target binding.
  ///
  /// The caller supplies only line/slot/locale/LocID intent. Native code alone derives zero, one,
  /// or multiple sealed existing-member matches from the configured installation.
  Future<AuthoringRevision3VoiceTargetPreparation>
  authoringStorePrepareRevision3VoiceTargetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTargetRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_voice_target_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'game_root': gameRoot,
      'root': root,
      'voice_target_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3VoiceTargetPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Evaluate the exact current managed revision-3 Voice graph without output,
  /// game, build, deployment, or publication authority.
  Future<AuthoringRevision3VoiceBuildPlanResult>
  authoringStorePlanRevision3VoiceV1({
    required String root,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_plan_revision3_voice_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRequireCanonicalRevision3ProjectJson(currentProjectJson);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3VoiceBuildPlanResult.fromJson(
        response,
        expectedHead: expectedHead,
        expectedProjectJson: currentProjectJson,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Classify the exact current managed revision-3 project without creating
  /// output or granting build, deployment, runtime, or publication authority.
  Future<AuthoringRevision3ProjectBuildPlanResult>
  authoringStorePlanRevision3ProjectBuildV1({
    required String root,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_plan_revision3_project_build_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRequireCanonicalRevision3ProjectJson(currentProjectJson);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
    });
    try {
      return AuthoringRevision3ProjectBuildPlanResult.fromJson(
        response,
        expectedHead: expectedHead,
        expectedProjectJson: currentProjectJson,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Build the exact current managed revision-3 Voice selection into a brand-new, sealed bundle.
  ///
  /// Native code reads Ogg bytes only through the managed Store, refuses unresolved/ambiguous or
  /// unapproved slots as a structured blocked result, never overwrites [output], and performs no
  /// deployment or game write.
  Future<AuthoringRevision3VoiceBuildResult>
  authoringStoreBuildRevision3VoiceV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String output,
  }) async {
    const command = 'authoring_store_build_revision3_voice_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3Path(output, 'output');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRequireCanonicalRevision3ProjectJson(currentProjectJson);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'expected_head_json': expectedHead.canonicalJson,
      'game_root': gameRoot,
      'output': output,
      'root': root,
    });
    try {
      return AuthoringRevision3VoiceBuildResult.fromJson(
        response,
        expectedHead: expectedHead,
        expectedProjectJson: currentProjectJson,
        expectedOutput: output,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Inspect one restorable managed revision-3 V2 snapshot without importing
  /// or publishing it.
  ///
  /// The native command receives only [source]. It verifies the complete
  /// archive and returns a closed read-only receipt; it accepts no destination,
  /// Store root, game path, save path, or publication authority. The project
  /// layer owns strict receipt parsing so this core bridge stays dependency-free.
  Future<Map<String, Object?>> authoringStoreInspectRevision3ExactSnapshotV2({
    required String source,
  }) async {
    const command = 'authoring_store_inspect_revision3_exact_snapshot_v2';
    _authoringRevision3Path(source, 'source');
    return _call(command, <String, Object?>{'source': source});
  }

  /// Materialize one previously inspected managed revision-3 V2 snapshot into
  /// an absent destination directory without adopting it into a Studio session.
  ///
  /// The native boundary receives only the exact source and destination
  /// spellings plus the inspected whole-archive seal. Strict project receipt
  /// parsing and any later session adoption remain owned by the project layer.
  Future<Map<String, Object?>> authoringStoreImportRevision3ExactSnapshotV2({
    required String source,
    required String destination,
    required int expectedArchiveByteLength,
    required String expectedArchiveSha256,
  }) async {
    const command = 'authoring_store_import_revision3_exact_snapshot_v2';
    _authoringRevision3Path(source, 'source');
    _authoringRevision3Path(destination, 'destination');
    _authoringRevision3ExactSnapshotArchiveSealPreflight(
      byteLength: expectedArchiveByteLength,
      sha256: expectedArchiveSha256,
    );
    return _call(command, <String, Object?>{
      'source': source,
      'destination': destination,
      'expected_archive': <String, Object?>{
        'byte_len': expectedArchiveByteLength,
        'sha256': expectedArchiveSha256,
      },
    });
  }

  /// Build one exact-basis reviewed revision-3 DataAsset stage into a new,
  /// receipt-bound PAK/UCAS/UTOC triplet.
  ///
  /// Native code owns every filesystem and publication decision. The returned
  /// value is a strict projection of the terminal receipt: only the
  /// caller-bound output spelling is pathful, while every artifact seal is
  /// path-free. It grants neither deployment nor runtime authority and is
  /// never retried automatically when publication is uncertain.
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  authoringStoreBuildRevision3ReviewedDataAssetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
    required String packName,
    required String output,
  }) async {
    const command = 'authoring_store_build_revision3_reviewed_dataasset_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(gameRoot, 'gameRoot');
    _authoringRevision3Path(output, 'output');
    _authoringRevision3RequestString(
      currentProjectJson,
      'currentProjectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringRequireCanonicalRevision3ProjectJson(currentProjectJson);
    _authoringRevision3DataAssetTargetPath(targetPath, 'targetPath');
    _authoringRevision3ReviewedDataAssetPackName(packName, 'packName');
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('currentProjectJson', currentProjectJson),
      ('expectedHead', expectedHead.canonicalJson),
      ('gameRoot', gameRoot),
      ('output', output),
      ('packName', packName),
      ('root', root),
      ('targetPath', targetPath),
    ]);
    final payload = <String, Object?>{
      'current_project_json': currentProjectJson,
      'expected_head_json': expectedHead.canonicalJson,
      'game_root': gameRoot,
      'output': output,
      'pack_name': packName,
      'root': root,
      'target_path': targetPath,
    };
    _authoringRevision3DataAssetEditEnvelopePreflight(
      command,
      payload,
      maxBytes: _maxAuthoringRevision3ReviewedDataAssetBuildRequestBytes,
    );
    final response = await _call(command, payload);
    try {
      return AuthoringRevision3ReviewedDataAssetBuildResult.fromJson(
        response,
        expectedHead: expectedHead,
        expectedProjectJson: currentProjectJson,
        expectedTargetPath: targetPath,
        expectedPackName: packName,
        expectedOutput: output,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Verify one PatchReceipt-v2 chain and prepare a fixed-leaf DataAsset stage without
  /// publishing the fixed revision-3 head.
  ///
  /// The receipt path is an input capability only. It is never retained by the returned DTO,
  /// which exposes only the closed, offset-free stage manifest and an unpublished candidate.
  Future<AuthoringRevision3DataAssetStagePreparation>
  authoringStorePrepareRevision3DataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String patchReceiptPath,
  }) async {
    const command = 'authoring_store_prepare_revision3_dataasset_stage_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(patchReceiptPath, 'patchReceiptPath');
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('expectedHead', expectedHead.canonicalJson),
      ('patchReceiptPath', patchReceiptPath),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'patch_receipt_path': patchReceiptPath,
      'root': root,
    });
    return AuthoringRevision3DataAssetStagePreparation.fromJson(
      response,
      expectedHead: expectedHead,
    );
  }

  /// Strictly reopen one ExtractReceipt-v2 and return only the target plus
  /// package/USMAP facts needed to bind a visible inspection before authoring.
  Future<DataAssetExtractReceiptSummary>
  authoringReadDataAssetExtractReceiptV2({
    required String extractReceiptPath,
  }) async {
    const command = 'authoring_read_dataasset_extract_receipt_v2';
    _authoringRevision3Path(extractReceiptPath, 'extractReceiptPath');
    final response = await _call(command, <String, Object?>{
      'extract_receipt_path': extractReceiptPath,
    });
    try {
      return DataAssetExtractReceiptSummary.fromJson(response);
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare one typed, selector-bound fixed-leaf edit directly from an exact
  /// ExtractReceipt-v2 proof without publishing the fixed revision-3 head.
  ///
  /// Raw offsets and replacement bytes never cross this Dart boundary. Native
  /// code encodes the semantic replacement, reconstructs a private receipt
  /// chain, reverifies the live game generation, and returns only the same
  /// closed unpublished stage DTO as the PatchReceipt-v2 import route.
  Future<AuthoringRevision3DataAssetStagePreparation>
  authoringStorePrepareRevision3DataAssetEditV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required DataAssetSemanticEditIntent intent,
  }) async {
    const command = 'authoring_store_prepare_revision3_dataasset_edit_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3Path(intent.extractReceiptPath, 'extractReceiptPath');
    final payload = <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      ...intent.toNativeFields(),
      'root': root,
    };
    _authoringRevision3DataAssetEditEnvelopePreflight(command, payload);
    final response = await _call(command, payload);
    try {
      final prepared = AuthoringRevision3DataAssetStagePreparation.fromJson(
        response,
        expectedHead: expectedHead,
        expectedIntentBindingSha256: intent.intentBindingSha256,
      );
      final selector = intent.selector;
      if (prepared.stage.targetPath != intent.expectedTargetPath ||
          prepared.stage.selectorKind != selector.kind.wireName ||
          prepared.stage.selectorPathDepth != selector.path.length ||
          prepared.stage.replacementByteLength != selector.kind.width) {
        throw const FormatException(
          'prepared stage does not match the typed selector shape',
        );
      }
      return prepared;
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// List all verified fixed-leaf DataAsset stages at one exact published revision-3 head.
  ///
  /// This command is read-only and grants no artifact, build, runtime, pack, deployment, or
  /// publication authority.
  Future<AuthoringRevision3DataAssetStageListResult>
  authoringStoreListRevision3DataAssetStagesV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    const command = 'authoring_store_list_revision3_dataasset_stages_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('expectedHead', expectedHead.canonicalJson),
      ('root', root),
    ]);
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
    });
    return AuthoringRevision3DataAssetStageListResult.fromJson(
      response,
      expectedHead: expectedHead,
    );
  }

  /// Prepare removal of one exact managed DataAsset stage without publishing the candidate.
  Future<AuthoringRevision3DataAssetStageRemovalPreparation>
  authoringStorePrepareRemoveRevision3DataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
  }) async {
    const command =
        'authoring_store_prepare_remove_revision3_dataasset_stage_v1';
    _authoringRevision3Path(root, 'root');
    _authoringRevision3DataAssetTargetPath(targetPath, 'targetPath');
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('expectedHead', expectedHead.canonicalJson),
      ('root', root),
      ('targetPath', targetPath),
    ]);
    final response = await _call(command, <String, Object?>{
      'expected_head_json': expectedHead.canonicalJson,
      'root': root,
      'target_path': targetPath,
    });
    return AuthoringRevision3DataAssetStageRemovalPreparation.fromJson(
      response,
      expectedHead: expectedHead,
      requestedTargetPath: targetPath,
    );
  }

  /// Prepare removal of one exact Story Draft and its uniquely-owned generated
  /// ScriptModule without publishing the returned candidate head.
  Future<AuthoringRevision3StoryDraftRemovalPreparation>
  authoringStorePrepareRemoveRevision3StoryDraftV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3StoryDraftRemovalRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_remove_revision3_story_draft_v1';
    _authoringRevision3Path(root, 'root');
    request._requireMatchesProject(currentProjectJson);
    _authoringRevision3DataAssetEnvelopePreflight(command, <(String, String)>[
      ('currentProjectJson', currentProjectJson),
      ('root', root),
      ('storyDraftRemovalRequestJson', request.canonicalJson),
    ]);
    final response = await _call(command, <String, Object?>{
      'current_project_json': currentProjectJson,
      'root': root,
      'story_draft_removal_request_json': request.canonicalJson,
    });
    try {
      return AuthoringRevision3StoryDraftRemovalPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Reopen one schema-revision-3 candidate from its exact canonical head bytes.
  ///
  /// This only inspects already-prepared immutable objects and never publishes the candidate.
  Future<AuthoringRevision3StoreOpenedResult>
  authoringStoreOpenRevision3HeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    const command = 'authoring_store_open_revision3_head_bytes';
    _authoringRevision3StorePath(root);
    _authoringRevision3OpenHeadEnvelopePreflight(
      command,
      root,
      head.canonicalJson,
      verification.wireName,
    );
    final response = await _call(command, {
      'root': root,
      'head_json': head.canonicalJson,
      'verification': verification.wireName,
    });
    return AuthoringRevision3StoreOpenedResult.fromJson(response);
  }

  /// Build the unified bundle into `outDir`; returns the bundle dir.
  Future<String> modBuild(Map<String, Object?> spec, String outDir) async {
    final r = await _call('mod_build', {'out_dir': outDir, 'spec': spec});
    return r['bundle_dir'] as String;
  }

  Future<void> modDeploy(String bundleDir, String gameRoot) =>
      _call('mod_deploy', {'bundle_dir': bundleDir, 'game_root': gameRoot});

  /// Undeploy the active mod. Returns true if a deployment was actually undone, false if nothing
  /// was deployed (the FFI returns a null record in that case).
  Future<bool> modUndeploy(String gameRoot) async {
    final r = await _call('mod_undeploy', {'game_root': gameRoot});
    return r['record'] != null;
  }

  /// Load one generation-bound texture index atomically with its native build ID.
  Future<TextureIndexSnapshot> textureIndex(
    String game, {
    bool rebuild = false,
  }) async {
    final r = await _call('texture_index', {'game': game, 'rebuild': rebuild});
    final buildId = r['build_id'];
    if (!_isValidTextureBuildId(buildId)) {
      throw const FormatException(
        'texture_index returned an invalid native build ID',
      );
    }
    final entries = r['entries'];
    if (entries is! Map || entries.length > _maxTextureIndexEntries) {
      throw const FormatException('texture_index returned invalid entries');
    }
    final typedEntries = <String, String>{};
    final foldedAssetPaths = <String>{};
    for (final entry in entries.entries) {
      final key = entry.key;
      final value = entry.value;
      if (!_isValidTextureAssetPath(key)) {
        throw const FormatException(
          'texture_index returned an invalid asset path',
        );
      }
      if (!_isValidTexturePackageId(value)) {
        throw const FormatException(
          'texture_index returned an invalid package ID',
        );
      }
      final assetPath = key as String;
      if (!foldedAssetPaths.add(assetPath.toLowerCase())) {
        throw const FormatException(
          'texture_index returned case-colliding asset paths',
        );
      }
      typedEntries[assetPath] = value as String;
    }
    final count = r['count'];
    if (count is! int ||
        count < 0 ||
        count > _maxTextureIndexEntries ||
        count != typedEntries.length) {
      throw const FormatException('texture_index count does not match entries');
    }
    return TextureIndexSnapshot(
      buildId: buildId as String,
      entries: typedEntries,
    );
  }

  /// Extract one indexed texture from the same exact native build.
  Future<Map<String, Object?>> textureExtract(
    String game, {
    required String expectedBuildId,
    required String asset,
    required String packageId,
  }) async {
    if (!_isValidTextureBuildId(expectedBuildId)) {
      throw ArgumentError.value(
        expectedBuildId,
        'expectedBuildId',
        'expected one bounded native texture build ID',
      );
    }
    if (!_isValidTextureAssetPath(asset)) {
      throw ArgumentError.value(
        asset,
        'asset',
        'expected one canonical Unreal long package name',
      );
    }
    if (!_isValidTexturePackageId(packageId)) {
      throw ArgumentError.value(
        packageId,
        'packageId',
        'expected one canonical unsigned 64-bit decimal package ID',
      );
    }
    return _call('texture_extract', {
      'game': game,
      'expected_build_id': expectedBuildId,
      'asset': asset,
      'package_id': packageId,
    });
  }

  /// Read the next bounded byte chunk from one native-owned texture preview.
  Future<Map<String, Object?>> texturePreviewRead({
    required String previewToken,
    required int offset,
  }) {
    if (!_texturePreviewTokenPattern.hasMatch(previewToken)) {
      throw ArgumentError.value(
        previewToken,
        'previewToken',
        'expected one opaque 64-digit lowercase hex token',
      );
    }
    if (offset < 0 || offset >= _maxTexturePreviewBytes) {
      throw ArgumentError.value(
        offset,
        'offset',
        'expected one in-range texture preview offset',
      );
    }
    return _call('texture_preview_read', {
      'preview_token': previewToken,
      'offset': offset,
    });
  }

  /// Release one native-owned texture preview capability.
  Future<Map<String, Object?>> texturePreviewRelease({
    required String previewToken,
  }) {
    if (!_texturePreviewTokenPattern.hasMatch(previewToken)) {
      throw ArgumentError.value(
        previewToken,
        'previewToken',
        'expected one opaque 64-digit lowercase hex token',
      );
    }
    return _call('texture_preview_release', {'preview_token': previewToken});
  }

  /// Auto-detect the game install via Steam; returns the exe path hint, or null.
  Future<String?> findGameExe() async {
    final r = await _call('find_game', const {});
    if (r['found'] != true) return null;
    return r['exe'] as String?;
  }

  /// List modules in a precompiled script cache: [{name, file}].
  Future<List<ScriptModuleInfo>> scriptListModules(String cache) async {
    final r = await _call('script_list_modules', {'cache': cache});
    final list = (r['modules'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => ScriptModuleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Emit recompilable .as source for one module.
  Future<String> scriptEmitModule(String cache, String module) async {
    final r = await _call('script_emit_module', {
      'cache': cache,
      'module': module,
    });
    return r['source'] as String;
  }

  /// Compile through the transactional game compiler and retain bounded structured diagnostics.
  ///
  /// Compiler failure is returned as [ScriptCompileReport.failure], not thrown. Transport/schema
  /// failures remain exceptions. The report separately proves whether the temporary live-install
  /// transaction restored every path exactly.
  Future<ScriptCompileReport> scriptCompileReportV1({
    required String gameDir,
    required String op,
    required String moduleName,
    required String relPath,
    required String asPath,
    required String workDir,
    bool allowNewSymbols = false,
  }) async {
    const command = 'script_compile_report_v1';
    final response = await _call(command, {
      'game_dir': gameDir,
      'op': op,
      'module_name': moduleName,
      'rel_path': relPath,
      'as_path': asPath,
      'work_dir': workDir,
      'allow_new_symbols': allowNewSymbols,
    });
    try {
      final report = ScriptCompileReport.fromJson(response);
      if (report.compiled) {
        _requireOwnedScriptCompileOutput(
          workDir: workDir,
          miniPath: report.miniPath!,
        );
      }
      return report;
    } on FormatException {
      throw const ModFfiException._malformed(
        command: command,
        reason: 'compile report schema is invalid',
      );
    }
  }

  /// Read-only, fail-closed inspection of whether the configured game install
  /// may enter a compiler/deploy mutation window.
  Future<ScriptCompileInstallState> scriptCompileInstallStateV1({
    required String gameDir,
  }) async {
    const command = 'script_compile_install_state_v1';
    final response = await _call(command, {'game_dir': gameDir});
    try {
      return ScriptCompileInstallState.fromJson(response);
    } on FormatException {
      throw const ModFfiException._malformed(
        command: command,
        reason: 'compile install-state schema is invalid',
      );
    }
  }
}

const _scriptCompileOwnedChildPrefix = 'gore-owned-compile-';
const _scriptCompileOwnedMarkerName = '.gore-owned-compile-v1';
const _scriptCompileOwnedMarkerBytes = 'gore-owned-compile-staging-v1\n';
final _scriptCompileOwnedChildPattern = RegExp(
  r'^gore-owned-compile-[0-9a-f]{12}$',
);

void _requireOwnedScriptCompileOutput({
  required String workDir,
  required String miniPath,
}) {
  try {
    _requireOwnedScriptCompileOutputOnDisk(
      workDir: workDir,
      miniPath: miniPath,
    );
  } on FileSystemException {
    throw const FormatException('compile output ownership could not be read');
  }
}

void _requireOwnedScriptCompileOutputOnDisk({
  required String workDir,
  required String miniPath,
}) {
  final normalizedWork = p.normalize(p.absolute(workDir));
  final normalizedMini = p.normalize(p.absolute(miniPath));
  if (!p.isAbsolute(workDir) ||
      !p.isAbsolute(miniPath) ||
      p.normalize(workDir) != workDir ||
      p.normalize(miniPath) != miniPath ||
      p.basename(normalizedMini) != 'module.cache') {
    throw const FormatException('compile output path is not canonical');
  }
  final child = p.dirname(normalizedMini);
  final childName = p.basename(child);
  if (!p.equals(p.dirname(child), normalizedWork) ||
      !childName.startsWith(_scriptCompileOwnedChildPrefix) ||
      !_scriptCompileOwnedChildPattern.hasMatch(childName)) {
    throw const FormatException('compile output is not a direct owned child');
  }
  if (FileSystemEntity.typeSync(normalizedWork, followLinks: false) !=
          FileSystemEntityType.directory ||
      FileSystemEntity.typeSync(child, followLinks: false) !=
          FileSystemEntityType.directory ||
      FileSystemEntity.typeSync(normalizedMini, followLinks: false) !=
          FileSystemEntityType.file) {
    throw const FormatException('compile output is not a regular owned file');
  }
  final marker = p.join(child, _scriptCompileOwnedMarkerName);
  if (FileSystemEntity.typeSync(marker, followLinks: false) !=
      FileSystemEntityType.file) {
    throw const FormatException('compile output ownership marker is missing');
  }
  final markerFile = File(marker);
  final markerBytes = markerFile.lengthSync();
  if (markerBytes != utf8.encode(_scriptCompileOwnedMarkerBytes).length ||
      markerFile.readAsStringSync() != _scriptCompileOwnedMarkerBytes) {
    throw const FormatException('compile output ownership marker is invalid');
  }
}

class ModFfiException implements Exception {
  const ModFfiException({
    required this.command,
    required this.code,
    required this.message,
  });

  const ModFfiException._malformed({
    required this.command,
    required String reason,
  }) : code = malformedNativeResponseCode,
       message = 'malformed native response: $reason';

  /// Local code used when gore_ffi does not return its documented error shape.
  static const malformedNativeResponseCode = 'MALFORMED_NATIVE_RESPONSE';

  final String command;
  final String code;
  final String message;

  @override
  String toString() => '$command: $message [$code]';
}

class AudioSampleInfo {
  AudioSampleInfo({
    required this.index,
    required this.name,
    required this.freq,
    required this.channels,
    required this.seconds,
  });
  final int index;
  final String name;
  final int freq;
  final int channels;
  final double seconds;

  factory AudioSampleInfo.fromJson(Map<String, Object?> j) => AudioSampleInfo(
    index: (j['index'] as num).toInt(),
    name: j['name'] as String,
    freq: (j['freq'] as num).toInt(),
    channels: (j['channels'] as num).toInt(),
    seconds: (j['seconds'] as num).toDouble(),
  );
}

enum AuthoringAssetVerification {
  structural('structural'),
  full('full');

  const AuthoringAssetVerification(this.wireName);
  final String wireName;
}

final _authoringEntityIdPattern = RegExp(r'^[0-9a-f]{32}$');
final _authoringSha256Pattern = RegExp(r'^[0-9a-f]{64}$');
const _authoringProjectTopLevelFields = <String>[
  'format',
  'schema_revision',
  'project_id',
  'revision',
  'meta',
  'target',
  'authoring_locales',
  'entities',
  'asset_store',
];

void _authoringRevision3RequestString(
  String value,
  String field,
  int maxBytes,
) {
  // The revision-3 bridge validates UTF-16 explicitly so jsonEncode cannot silently replace an
  // unpaired surrogate while constructing a security-sensitive nested-string request.
  _authoringDraftRequestString(value, field, maxBytes);
}

void _authoringRevision3StorePath(String value) {
  _authoringRevision3Path(value, 'root');
}

void _authoringRevision3Path(String value, String field) {
  _authoringRevision3RequestString(value, field, _maxAuthoringStorePathBytes);
  if (value.contains('\u0000')) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'must not contain NUL',
    );
  }
}

void _authoringRevision3ExactSnapshotArchiveSealPreflight({
  required int byteLength,
  required String sha256,
}) {
  if (byteLength < 1 ||
      byteLength > _maxAuthoringRevision3ExactSnapshotArchiveBytesV2) {
    throw ArgumentError.value(
      byteLength,
      'expectedArchiveByteLength',
      'must be 1..=$_maxAuthoringRevision3ExactSnapshotArchiveBytesV2',
    );
  }
  _authoringRevision3RequestString(sha256, 'expectedArchiveSha256', 64);
  if (!_authoringSha256Pattern.hasMatch(sha256)) {
    throw ArgumentError.value(
      '<redacted>',
      'expectedArchiveSha256',
      'must be one canonical lowercase SHA-256 digest',
    );
  }
}

void _authoringRevision3OpenEnvelopePreflight(
  String command,
  String root,
  String verification,
) {
  var encodedBytes =
      '{"command":"","payload":{"root":"","verification":""}}'.length +
      command.length +
      verification.length;
  _authoringAddEscapedJsonStringBytes(root, 'root', encodedBytes);
}

void _authoringRevision3ContentReadEnvelopePreflight(
  String command,
  String expectedHeadJson,
  String root,
) {
  var encodedBytes =
      '{"command":"","payload":{"expected_head_json":"","root":""}}'.length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    expectedHeadJson,
    'expectedHead',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(root, 'root', encodedBytes);
}

void _authoringRevision3OpenHeadEnvelopePreflight(
  String command,
  String root,
  String headJson,
  String verification,
) {
  var encodedBytes =
      '{"command":"","payload":{"root":"","head_json":"","verification":""}}'
          .length +
      command.length +
      verification.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    root,
    'root',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(headJson, 'head', encodedBytes);
}

void _authoringRevision3PrepareEnvelopePreflight(
  String command,
  String root,
  String? expectedHeadJson,
  String projectJson,
) {
  var encodedBytes = expectedHeadJson == null
      ? '{"command":"","payload":{"root":"","expected_head_json":null,"project_json":""}}'
                .length +
            command.length
      : '{"command":"","payload":{"root":"","expected_head_json":"","project_json":""}}'
                .length +
            command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    root,
    'root',
    encodedBytes,
  );
  if (expectedHeadJson != null) {
    encodedBytes = _authoringAddEscapedJsonStringBytes(
      expectedHeadJson,
      'expectedHead',
      encodedBytes,
    );
  }
  _authoringAddEscapedJsonStringBytes(projectJson, 'projectJson', encodedBytes);
}

void _authoringRevision3QuestPrepareEnvelopePreflight(
  String command,
  String root,
  String gameRoot,
  String currentProjectJson,
  String questRequestJson,
) {
  var encodedBytes =
      '{"command":"","payload":{"current_project_json":"","game_root":"","quest_request_json":"","root":""}}'
          .length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    currentProjectJson,
    'currentProjectJson',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    gameRoot,
    'gameRoot',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    questRequestJson,
    'questRequestJson',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(root, 'root', encodedBytes);
}

void _authoringRevision3NpcPrepareEnvelopePreflight(
  String command,
  String root,
  String gameRoot,
  String currentProjectJson,
  String npcRequestJson,
) {
  var encodedBytes =
      '{"command":"","payload":{"current_project_json":"","game_root":"","npc_request_json":"","root":""}}'
          .length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    currentProjectJson,
    'currentProjectJson',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    gameRoot,
    'gameRoot',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    npcRequestJson,
    'npcRequestJson',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(root, 'root', encodedBytes);
}

void _authoringRevision3DataAssetEnvelopePreflight(
  String command,
  List<(String, String)> fields,
) {
  // This deliberately overcounts punctuation. It prevents a large escaped path from allocating
  // a transport envelope before the native route can apply its tighter command-local budget.
  var encodedBytes = 256 + command.length;
  for (final (name, value) in fields) {
    encodedBytes = _authoringAddEscapedJsonStringBytes(
      value,
      name,
      encodedBytes,
    );
  }
}

void _authoringRevision3DataAssetEditEnvelopePreflight(
  String command,
  Map<String, Object?> payload, {
  int maxBytes = _maxAuthoringRevision3DataAssetEditRequestBytes,
}) {
  final wire = jsonEncode(<String, Object?>{
    'command': command,
    'payload': payload,
  });
  final byteLength = utf8.encode(wire).length;
  if (byteLength > maxBytes) {
    throw ArgumentError.value(
      '<$byteLength UTF-8 bytes>',
      'intent',
      'escaped command envelope exceeds the '
          '$maxBytes-byte native limit',
    );
  }
}

void _authoringRevision3DataAssetTargetPath(String value, String field) {
  _authoringRevision3RequestString(value, field, 512);
  final segments = value.startsWith('/Game/')
      ? value.substring('/Game/'.length).split('/')
      : const <String>[];
  if (segments.isEmpty ||
      segments.length > 32 ||
      segments.any(
        (segment) =>
            segment.isEmpty ||
            !_authoringRevision3DataAssetSegmentPattern.hasMatch(segment) ||
            _authoringRevision3DataAssetWindowsReservedName(segment),
      )) {
    throw ArgumentError.value(
      value,
      field,
      'must be a canonical extensionless /Game asset path',
    );
  }
}

final _authoringRevision3DataAssetSegmentPattern = RegExp(r'^[A-Za-z0-9_]+$');

bool _authoringRevision3DataAssetWindowsReservedName(String value) {
  final upper = value.toUpperCase();
  return const <String>{'CON', 'PRN', 'AUX', 'NUL'}.contains(upper) ||
      (upper.length == 4 &&
          (upper.startsWith('COM') || upper.startsWith('LPT')) &&
          upper.codeUnitAt(3) >= 0x31 &&
          upper.codeUnitAt(3) <= 0x39);
}

void _voiceOggInspectPath(String value) {
  if (value.isEmpty || value.contains('\u0000')) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      'oggPath',
      'must be a non-empty path without NUL',
    );
  }
  _authoringDraftRequestString(value, 'oggPath', _maxVoiceOggPathBytes);
}

void _dataAssetInspectPath(String value, String field) {
  if (value.isEmpty || value.contains('\u0000')) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'must be a non-empty path without NUL',
    );
  }
  _authoringDraftRequestString(value, field, _maxDataAssetPathBytes);
}

void _dataAssetInspectEnvelopePreflight(
  String command,
  String uassetPath,
  String usmapPath,
  int? exportIndex,
) {
  var encodedBytes = exportIndex == null
      ? '{"command":"","payload":{"uasset_path":"","usmap_path":""}}'.length +
            command.length
      : '{"command":"","payload":{"uasset_path":"","usmap_path":"","export_index":}}'
                .length +
            command.length +
            exportIndex.toString().length;
  encodedBytes = _dataAssetAddEscapedJsonStringBytes(
    uassetPath,
    'uassetPath',
    encodedBytes,
  );
  _dataAssetAddEscapedJsonStringBytes(usmapPath, 'usmapPath', encodedBytes);
}

int _dataAssetAddEscapedJsonStringBytes(
  String value,
  String field,
  int encodedBytes,
) {
  if (encodedBytes > _maxDataAssetInspectRequestBytes) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'escaped command envelope exceeds the '
          '$_maxDataAssetInspectRequestBytes-byte native request limit',
    );
  }
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    final int added;
    if (unit <= 0x1f || unit == 0x2028 || unit == 0x2029) {
      added = 6;
    } else if (unit == 0x22 || unit == 0x5c) {
      added = 2;
    } else if (unit <= 0x7f) {
      added = 1;
    } else if (unit <= 0x7ff) {
      added = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      // The path preflight already proved this is a paired surrogate.
      index++;
      added = 4;
    } else {
      added = 3;
    }
    if (added > _maxDataAssetInspectRequestBytes - encodedBytes) {
      throw ArgumentError.value(
        '<${value.length} characters>',
        field,
        'escaped command envelope exceeds the '
            '$_maxDataAssetInspectRequestBytes-byte native request limit',
      );
    }
    encodedBytes += added;
  }
  return encodedBytes;
}

void _voiceOggInspectEnvelopePreflight(String command, String oggPath) {
  var encodedBytes =
      '{"command":"","payload":{"ogg_path":""}}'.length + command.length;
  for (var index = 0; index < oggPath.length; index++) {
    final unit = oggPath.codeUnitAt(index);
    final int added;
    if (unit <= 0x1f || unit == 0x2028 || unit == 0x2029) {
      added = 6;
    } else if (unit == 0x22 || unit == 0x5c) {
      added = 2;
    } else if (unit <= 0x7f) {
      added = 1;
    } else if (unit <= 0x7ff) {
      added = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      // `_voiceOggInspectPath` already proved this is a paired surrogate.
      index++;
      added = 4;
    } else {
      added = 3;
    }
    if (added > _maxVoiceOggInspectRequestBytes - encodedBytes) {
      throw ArgumentError.value(
        '<${oggPath.length} characters>',
        'oggPath',
        'escaped command envelope exceeds the '
            '$_maxVoiceOggInspectRequestBytes-byte native request limit',
      );
    }
    encodedBytes += added;
  }
}

void _authoringDraftRequestString(String value, String field, int maxBytes) {
  if (value.isEmpty) {
    throw ArgumentError.value(value, field, 'must not be empty');
  }
  var bytes = 0;
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    if (unit <= 0x7f) {
      bytes += 1;
    } else if (unit <= 0x7ff) {
      bytes += 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw ArgumentError.value(
          value.length,
          field,
          'contains invalid UTF-16',
        );
      }
      final low = value.codeUnitAt(++index);
      if (low < 0xdc00 || low > 0xdfff) {
        throw ArgumentError.value(
          value.length,
          field,
          'contains invalid UTF-16',
        );
      }
      bytes += 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw ArgumentError.value(value.length, field, 'contains invalid UTF-16');
    } else {
      bytes += 3;
    }
    if (bytes > maxBytes) {
      throw ArgumentError.value(
        '<${value.length} characters>',
        field,
        'must be 1..=$maxBytes UTF-8 bytes',
      );
    }
  }
}

void _authoringSingleRawJsonEnvelopePreflight(
  String command,
  String wireField,
  String dartField,
  String value,
) {
  final encodedBytes =
      '{"command":"","payload":{"":""}}'.length +
      command.length +
      wireField.length;
  _authoringAddEscapedJsonStringBytes(value, dartField, encodedBytes);
}

void _authoringStoryCatalogPath(String value, String field) {
  _authoringDraftRequestString(value, field, _maxAuthoringStorePathBytes);
  if (value.contains('\u0000')) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'must not contain NUL',
    );
  }
}

void _authoringStoryCatalogBuildEnvelopePreflight(
  String command,
  String executable,
  String shippingCache,
  String bindsCache,
) {
  var encodedBytes =
      '{"command":"","payload":{"executable":"","shipping_cache":"","binds_cache":""}}'
          .length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    executable,
    'executable',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    shippingCache,
    'shippingCache',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(bindsCache, 'bindsCache', encodedBytes);
}

int _authoringAddEscapedJsonStringBytes(
  String value,
  String field,
  int encodedBytes,
) {
  if (encodedBytes > goreCoreTransportMaxRequestBytes) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'escaped command envelope exceeds the '
          '$goreCoreTransportMaxRequestBytes-byte transport limit',
    );
  }
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    final int added;
    if (unit <= 0x1f || unit == 0x2028 || unit == 0x2029) {
      added = 6;
    } else if (unit == 0x22 || unit == 0x5c) {
      added = 2;
    } else if (unit <= 0x7f) {
      added = 1;
    } else if (unit <= 0x7ff) {
      added = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      // The raw-input preflight already proved this is a paired surrogate.
      index++;
      added = 4;
    } else {
      added = 3;
    }
    if (added > goreCoreTransportMaxRequestBytes - encodedBytes) {
      throw ArgumentError.value(
        '<${value.length} characters>',
        field,
        'escaped command envelope exceeds the '
            '$goreCoreTransportMaxRequestBytes-byte transport limit',
      );
    }
    encodedBytes += added;
  }
  return encodedBytes;
}

final class _AuthoringDigestCollector implements Sink<crypto.Digest> {
  crypto.Digest? _value;

  crypto.Digest get value =>
      _value ??
      (throw const FormatException('authoring SHA-256 digest was not emitted'));

  @override
  void add(crypto.Digest data) {
    if (_value != null) {
      throw const FormatException('authoring SHA-256 emitted more than once');
    }
    _value = data;
  }

  @override
  void close() {}
}

Map<String, Object?> _authoringDecodeDuplicateSafeObject(
  String raw,
  String context,
) {
  try {
    _AuthoringDuplicateKeyJsonScanner(raw).validate();
    return _authoringRequiredObject(jsonDecode(raw), context);
  } on FormatException {
    throw FormatException(
      'authoring $context JSON is invalid or has duplicate keys',
    );
  }
}

void _authoringRequireSignedSafeUnsignedJsonNumbers(
  Object? root,
  String context,
) {
  final pending = <Object?>[root];
  while (pending.isNotEmpty) {
    final value = pending.removeLast();
    if (value == null || value is String || value is bool) continue;
    if (value is int) {
      if (value < 0 || value > _maxAuthoringSignedJsonInteger) {
        throw FormatException(
          'authoring $context contains an integer outside the signed-safe unsigned range',
        );
      }
      continue;
    }
    if (value is List) {
      pending.addAll(value);
      continue;
    }
    if (value is Map) {
      for (final entry in value.entries) {
        if (entry.key is! String) {
          throw FormatException(
            'authoring $context contains a non-string object key',
          );
        }
        pending.add(entry.value);
      }
      continue;
    }
    // JSON decimals/exponents decode as doubles. Revision-3's closed model has only unsigned
    // integer numbers, so accepting one here would either lose precision or expand the schema.
    throw FormatException(
      'authoring $context contains a non-integer JSON number or unsupported value',
    );
  }
}

final class _AuthoringDuplicateKeyJsonScanner {
  _AuthoringDuplicateKeyJsonScanner(this.source);

  static const _maxDepth = 128;
  final String source;
  int _index = 0;

  void validate() {
    _value(0);
    _whitespace();
    if (_index != source.length) throw const FormatException('trailing JSON');
  }

  void _value(int depth) {
    if (depth > _maxDepth) throw const FormatException('JSON too deep');
    _whitespace();
    if (_index >= source.length) throw const FormatException('missing JSON');
    switch (source.codeUnitAt(_index)) {
      case 0x7b:
        _object(depth + 1);
      case 0x5b:
        _array(depth + 1);
      case 0x22:
        _string();
      default:
        final start = _index;
        while (_index < source.length &&
            !_isValueDelimiter(source.codeUnitAt(_index))) {
          _index++;
        }
        if (_index == start) throw const FormatException('invalid JSON value');
    }
  }

  void _object(int depth) {
    _index++;
    _whitespace();
    if (_take(0x7d)) return;
    final keys = <String>{};
    while (true) {
      _whitespace();
      if (_index >= source.length || source.codeUnitAt(_index) != 0x22) {
        throw const FormatException('object key required');
      }
      final rawKey = _string();
      final key = jsonDecode(rawKey);
      if (key is! String || !keys.add(key)) {
        throw const FormatException('duplicate object key');
      }
      _whitespace();
      if (!_take(0x3a)) throw const FormatException('colon required');
      _value(depth);
      _whitespace();
      if (_take(0x7d)) return;
      if (!_take(0x2c)) throw const FormatException('comma required');
    }
  }

  void _array(int depth) {
    _index++;
    _whitespace();
    if (_take(0x5d)) return;
    while (true) {
      _value(depth);
      _whitespace();
      if (_take(0x5d)) return;
      if (!_take(0x2c)) throw const FormatException('comma required');
    }
  }

  String _string() {
    final start = _index++;
    while (_index < source.length) {
      final unit = source.codeUnitAt(_index++);
      if (unit == 0x22) return source.substring(start, _index);
      if (unit == 0x5c) {
        if (_index >= source.length) {
          throw const FormatException('unterminated escape');
        }
        _index++;
      } else if (unit <= 0x1f) {
        throw const FormatException('control character in string');
      }
    }
    throw const FormatException('unterminated string');
  }

  void _whitespace() {
    while (_index < source.length) {
      final unit = source.codeUnitAt(_index);
      if (unit != 0x20 && unit != 0x09 && unit != 0x0a && unit != 0x0d) return;
      _index++;
    }
  }

  bool _take(int unit) {
    if (_index < source.length && source.codeUnitAt(_index) == unit) {
      _index++;
      return true;
    }
    return false;
  }

  static bool _isValueDelimiter(int unit) =>
      unit == 0x20 ||
      unit == 0x09 ||
      unit == 0x0a ||
      unit == 0x0d ||
      unit == 0x2c ||
      unit == 0x5d ||
      unit == 0x7d;
}

void _authoringExactFields(
  Map<String, Object?> json,
  Set<String> expected,
  String context,
) {
  if (json.length != expected.length || !expected.every(json.containsKey)) {
    throw FormatException('authoring $context has an invalid schema');
  }
}

String _authoringRequiredString(
  Map<String, Object?> json,
  String field, {
  int maxBytes = _maxAuthoringProjectJsonBytes,
}) {
  final value = json[field];
  if (value is! String ||
      value.isEmpty ||
      utf8.encode(value).length > maxBytes) {
    throw FormatException(
      'authoring response field $field is not a bounded string',
    );
  }
  return value;
}

String _authoringRevision3ResponseString(
  Map<String, Object?> json,
  String field, {
  required int maxBytes,
}) {
  final value = json[field];
  if (value is! String) {
    throw FormatException(
      'authoring revision-3 response field $field is not a string',
    );
  }
  try {
    _authoringRevision3RequestString(value, field, maxBytes);
  } on ArgumentError {
    throw FormatException(
      'authoring revision-3 response field $field is not bounded UTF-8',
    );
  }
  return value;
}

int _authoringRequiredInt(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int? max,
}) {
  final value = json[field];
  if (value is! int || value < min || (max != null && value > max)) {
    throw FormatException(
      'authoring response field $field is not an integer in range',
    );
  }
  return value;
}

bool _authoringRequiredBool(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! bool) {
    throw FormatException('authoring response field $field is not a bool');
  }
  return value;
}

Map<String, Object?> _authoringRequiredObject(Object? value, String context) {
  if (value is! Map) {
    throw FormatException('authoring response $context is not an object');
  }
  final object = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw FormatException(
        'authoring response $context contains a non-string key',
      );
    }
    object[entry.key as String] = entry.value;
  }
  return object;
}

String _authoringEntityId(String value, String field) {
  if (!_authoringEntityIdPattern.hasMatch(value)) {
    throw FormatException(
      'authoring response field $field is not a canonical entity ID',
    );
  }
  return value;
}

({Map<String, Object?> project, String projectId, int revision})
_authoringRequireCanonicalRevision3ProjectJson(String projectJson) {
  final project = _authoringDecodeDuplicateSafeObject(
    projectJson,
    'revision-3 store project',
  );
  final fields = project.keys.toList(growable: false);
  if (fields.length != _authoringProjectTopLevelFields.length) {
    throw const FormatException(
      'authoring revision-3 store project JSON has an invalid top-level schema',
    );
  }
  for (var index = 0; index < fields.length; index++) {
    if (fields[index] != _authoringProjectTopLevelFields[index]) {
      throw const FormatException(
        'authoring revision-3 store project JSON has non-canonical field order',
      );
    }
  }
  if (project['format'] != 2 || project['schema_revision'] != 3) {
    throw const FormatException(
      'authoring revision-3 store project JSON has an unsupported schema',
    );
  }
  final projectId = _authoringEntityId(
    _authoringRequiredString(project, 'project_id', maxBytes: 32),
    'project_id',
  );
  if (projectId == '00000000000000000000000000000000') {
    throw const FormatException(
      'authoring revision-3 store project ID must not be zero',
    );
  }
  _authoringRequireSignedSafeUnsignedJsonNumbers(
    project,
    'revision-3 store project',
  );
  final revision = _authoringRequiredInt(project, 'revision');
  if (jsonEncode(project) != projectJson) {
    throw const FormatException(
      'authoring revision-3 store project JSON is not canonical',
    );
  }
  return (project: project, projectId: projectId, revision: revision);
}

String _authoringDraftSha256(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(json, field, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(value)) {
    throw FormatException('authoring draft field $field is not a SHA-256');
  }
  return value;
}

final _authoringDraftIdentifierPattern = RegExp(r'^[A-Za-z_][A-Za-z0-9_]*$');
final _authoringDraftCatalogLayerPattern = RegExp(
  r'^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$',
);
const _authoringDraftReservedIdentifiers = <String>{
  'abstract',
  'access',
  'and',
  'and_eq',
  'as',
  'auto',
  'bool',
  'break',
  'case',
  'cast',
  'catch',
  'class',
  'const',
  'continue',
  'default',
  'delegate',
  'do',
  'double',
  'else',
  'enum',
  'event',
  'explicit',
  'external',
  'false',
  'final',
  'float',
  'for',
  'from',
  'funcdef',
  'get',
  'if',
  'import',
  'in',
  'inout',
  'int',
  'int8',
  'int16',
  'int32',
  'int64',
  'interface',
  'is',
  'mixin',
  'namespace',
  'not',
  'not_eq',
  'null',
  'or',
  'or_eq',
  'out',
  'override',
  'private',
  'property',
  'protected',
  'return',
  'set',
  'shared',
  'super',
  'switch',
  'struct',
  'this',
  'true',
  'try',
  'typedef',
  'uint',
  'uint8',
  'uint16',
  'uint32',
  'uint64',
  'void',
  'while',
  'xor',
  'xor_eq',
  'staticclass',
  'spawn',
  'getorcreate',
  'create',
  'getg1r',
};

void _authoringDraftValidateIdentifier(
  String value,
  String field, {
  int maxBytes = 96,
}) {
  if (utf8.encode(value).length > maxBytes ||
      !_authoringDraftIdentifierPattern.hasMatch(value) ||
      value.startsWith('__') ||
      _authoringDraftReservedIdentifiers.contains(value.toLowerCase())) {
    throw FormatException('authoring draft field $field is not an identifier');
  }
}

bool _authoringDraftReservedPortableSegment(String value) {
  final upper = value.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(upper)) return true;
  if (upper.length == 4 &&
      (upper.startsWith('COM') || upper.startsWith('LPT'))) {
    final suffix = upper.codeUnitAt(3);
    return suffix >= 0x31 && suffix <= 0x39;
  }
  return false;
}

void _authoringDraftValidateModuleNamespace(String value) {
  if (utf8.encode(value).length > 255) {
    throw const FormatException('authoring draft module namespace is too long');
  }
  final segments = value.split('.');
  if (segments.isEmpty || segments.length > 16) {
    throw const FormatException('authoring draft module namespace is invalid');
  }
  for (final segment in segments) {
    _authoringDraftValidateIdentifier(segment, 'module_namespace');
    if (_authoringDraftReservedPortableSegment(segment)) {
      throw const FormatException(
        'authoring draft module namespace is not portable',
      );
    }
  }
}

void _authoringDraftValidateCatalogLayer(String value, String field) {
  if (utf8.encode(value).length > 128 ||
      !_authoringDraftCatalogLayerPattern.hasMatch(value)) {
    throw FormatException(
      'authoring draft field $field is not a canonical catalog layer',
    );
  }
}

final class AuthoringDraftContentSeal {
  const AuthoringDraftContentSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;

  factory AuthoringDraftContentSeal.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'byte_len',
      'sha256',
    }, 'draft content seal');
    return AuthoringDraftContentSeal._(
      byteLength: _authoringRequiredInt(json, 'byte_len', min: 1),
      sha256: _authoringDraftSha256(json, 'sha256'),
    );
  }
}

enum AuthoringNpcCatalogLinkageQualification { sealedLinkageVerified }

enum AuthoringNpcCatalogRuntimeQualification { runtimeUnqualified }

enum AuthoringNpcCatalogSupportStatus { notSupported }

enum AuthoringNpcCatalogBlueprintFamily { humanBase, humanWoman, other }

enum AuthoringNpcCatalogRejectionKind {
  missingInitDefaults,
  ambiguousInitDefaults,
  invalidInitDefaultsBytecode,
  missingDefaultEdge,
  ambiguousDefaultEdge,
  missingReferencedClass,
  wrongAncestry,
  inheritanceCycle,
  nonInheritableClass,
}

final class AuthoringNpcCatalogQualification {
  const AuthoringNpcCatalogQualification._();

  AuthoringNpcCatalogLinkageQualification get linkage =>
      AuthoringNpcCatalogLinkageQualification.sealedLinkageVerified;
  AuthoringNpcCatalogRuntimeQualification get runtime =>
      AuthoringNpcCatalogRuntimeQualification.runtimeUnqualified;
  AuthoringNpcCatalogSupportStatus get build =>
      AuthoringNpcCatalogSupportStatus.notSupported;
  AuthoringNpcCatalogSupportStatus get deploy =>
      AuthoringNpcCatalogSupportStatus.notSupported;
  AuthoringNpcCatalogSupportStatus get publication =>
      AuthoringNpcCatalogSupportStatus.notSupported;

  factory AuthoringNpcCatalogQualification._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'linkage',
      'runtime',
      'build',
      'deploy',
      'publication',
    }, 'NPC catalog qualification');
    if (json['linkage'] != 'sealed_linkage_verified' ||
        json['runtime'] != 'runtime_unqualified' ||
        json['build'] != 'not_supported' ||
        json['deploy'] != 'not_supported' ||
        json['publication'] != 'not_supported') {
      throw const FormatException(
        'authoring NPC catalog qualification overstates its capabilities',
      );
    }
    return const AuthoringNpcCatalogQualification._();
  }
}

final class AuthoringNpcCatalogSource {
  const AuthoringNpcCatalogSource._({
    required this.shippingCache,
    required this.bindsCache,
    required this.sourcePairSeal,
  });

  final AuthoringDraftContentSeal shippingCache;
  final AuthoringDraftContentSeal bindsCache;
  final AuthoringDraftContentSeal sourcePairSeal;

  factory AuthoringNpcCatalogSource._fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'shipping_cache',
      'binds_cache',
      'source_pair_seal',
    }, 'NPC catalog source');
    return AuthoringNpcCatalogSource._(
      shippingCache: _authoringStoryCatalogSeal(
        json['shipping_cache'],
        'NPC source shipping_cache',
      ),
      bindsCache: _authoringStoryCatalogSeal(
        json['binds_cache'],
        'NPC source binds_cache',
      ),
      sourcePairSeal: _authoringStoryCatalogSeal(
        json['source_pair_seal'],
        'NPC source source_pair_seal',
      ),
    );
  }
}

final class AuthoringNpcCatalogClassEvidence {
  const AuthoringNpcCatalogClassEvidence._({
    required this.className,
    required this.superClass,
    required this.moduleName,
    required this.relativePath,
    required this.sourceSeal,
  });

  final String className;
  final String? superClass;
  final String moduleName;
  final String relativePath;
  final AuthoringDraftContentSeal sourceSeal;
}

final class AuthoringNpcCatalogDefaultEdgeEvidence {
  const AuthoringNpcCatalogDefaultEdgeEvidence._({
    required this.ownerClass,
    required this.fieldName,
    required this.assignedValue,
    required this.instructionOffsetDwords,
    required this.initDefaultsBytecodeSeal,
    required this.evidenceSha256,
  });

  final String ownerClass;
  final String fieldName;
  final String assignedValue;
  final int instructionOffsetDwords;
  final AuthoringDraftContentSeal initDefaultsBytecodeSeal;
  final String evidenceSha256;
}

final class AuthoringNpcCatalogRecord {
  const AuthoringNpcCatalogRecord._({
    required this.spawn,
    required this.aiConfig,
    required this.characterDefinition,
    required this.actorBlueprint,
    required this.blueprintFamily,
    required this.spawnAiEdge,
    required this.spawnBlueprintEdge,
    required this.aiCharacterEdge,
    required this.evidenceSha256,
  });

  final AuthoringNpcCatalogClassEvidence spawn;
  final AuthoringNpcCatalogClassEvidence aiConfig;
  final AuthoringNpcCatalogClassEvidence characterDefinition;
  final String actorBlueprint;
  final AuthoringNpcCatalogBlueprintFamily blueprintFamily;
  final AuthoringNpcCatalogDefaultEdgeEvidence spawnAiEdge;
  final AuthoringNpcCatalogDefaultEdgeEvidence spawnBlueprintEdge;
  final AuthoringNpcCatalogDefaultEdgeEvidence aiCharacterEdge;
  final String evidenceSha256;
}

final class AuthoringNpcCatalogRejection {
  const AuthoringNpcCatalogRejection._({
    required this.spawnClass,
    required this.kind,
    this.ownerClass,
    this.fieldName,
    this.detail,
    this.role,
    this.className,
    this.requiredBase,
    this.count,
  });

  final String spawnClass;
  final AuthoringNpcCatalogRejectionKind kind;
  final String? ownerClass;
  final String? fieldName;
  final String? detail;
  final String? role;
  final String? className;
  final String? requiredBase;
  final int? count;
}

/// Strict projection of the native, read-only `npc_archetype_catalog.v1` artifact.
///
/// Static linkage evidence does not authorize build, deploy, publication, runtime use, or an
/// offline-clone qualification. Consumers must preserve those capability boundaries.
/// Generation/catalog authenticity remains owned by the already trusted bundled native core:
/// Dart verifies canonical structure and internal seals, but deliberately does not duplicate the
/// native version allowlist and thereby reject a future generation supported by that same core.
final class AuthoringNpcArchetypeCatalogBuildResult {
  const AuthoringNpcArchetypeCatalogBuildResult._({
    required this.requestBindingSha256,
    required this.catalogJson,
    required this.generation,
    required this.catalogSeal,
    required this.storyCatalogSeal,
    required this.source,
    required this.payloadSeal,
    required this.extractorRecordsSha256,
    required this.qualification,
    required this.records,
    required this.rejections,
  });

  final String requestBindingSha256;
  final String catalogJson;
  final AuthoringStoryCatalogGeneration generation;
  final AuthoringDraftContentSeal catalogSeal;
  final AuthoringDraftContentSeal storyCatalogSeal;
  final AuthoringNpcCatalogSource source;
  final AuthoringDraftContentSeal payloadSeal;
  final String extractorRecordsSha256;
  final AuthoringNpcCatalogQualification qualification;
  final List<AuthoringNpcCatalogRecord> records;
  final List<AuthoringNpcCatalogRejection> rejections;

  int get recordCount => records.length;
  int get rejectionCount => rejections.length;

  factory AuthoringNpcArchetypeCatalogBuildResult._fromJson(
    Map<String, Object?> json, {
    required String gameRoot,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_binding_sha256',
      'catalog_json',
      'generation',
      'catalog_seal',
      'source',
      'payload_seal',
      'record_count',
      'rejection_count',
      'qualification',
    }, 'NPC archetype catalog response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring NPC archetype catalog response is not ok',
      );
    }
    final requestBindingSha256 = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    final expectedBinding = _authoringNpcCatalogGameRootBindingSha256(gameRoot);
    if (!_authoringSha256Pattern.hasMatch(requestBindingSha256) ||
        requestBindingSha256 != expectedBinding) {
      throw const FormatException(
        'authoring NPC catalog response is not bound to its exact game root',
      );
    }

    final catalogJson = _authoringRequiredString(
      json,
      'catalog_json',
      maxBytes: _maxAuthoringNpcCatalogJsonBytes,
    );
    final rawArtifact = _authoringDecodeDuplicateSafeObject(
      catalogJson,
      'NPC archetype catalog',
    );
    _authoringExactFields(rawArtifact, const {
      'format',
      'schema_revision',
      'catalog',
      'catalog_seal',
    }, 'NPC archetype catalog');
    if (rawArtifact['format'] != 'npc_archetype_catalog') {
      throw const FormatException(
        'authoring NPC archetype catalog format is unsupported',
      );
    }
    _authoringRequiredInt(rawArtifact, 'schema_revision', min: 1, max: 1);

    final rawCatalog = _authoringRequiredObject(
      rawArtifact['catalog'],
      'NPC archetype catalog body',
    );
    _authoringExactFields(rawCatalog, const {
      'generation',
      'story_catalog_seal',
      'qualification',
      'source',
      'payload',
      'payload_seal',
    }, 'NPC archetype catalog body');

    final generation = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        rawCatalog['generation'],
        'NPC archetype catalog generation',
      ),
    );
    final outerGeneration = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        json['generation'],
        'NPC archetype response generation',
      ),
    );
    if (!_authoringStoryCatalogSameGeneration(generation, outerGeneration)) {
      throw const FormatException(
        'authoring NPC catalog generation disagrees with its response',
      );
    }

    final qualification = AuthoringNpcCatalogQualification._fromJson(
      _authoringRequiredObject(
        rawCatalog['qualification'],
        'NPC archetype catalog qualification',
      ),
    );
    AuthoringNpcCatalogQualification._fromJson(
      _authoringRequiredObject(
        json['qualification'],
        'NPC archetype response qualification',
      ),
    );

    final source = AuthoringNpcCatalogSource._fromJson(
      _authoringRequiredObject(
        rawCatalog['source'],
        'NPC archetype catalog source',
      ),
    );
    final outerSource = AuthoringNpcCatalogSource._fromJson(
      _authoringRequiredObject(json['source'], 'NPC archetype response source'),
    );
    if (!_authoringNpcCatalogSameSource(source, outerSource) ||
        !_authoringStoryCatalogSameSeal(
          source.shippingCache,
          generation.shippingCache,
        ) ||
        !_authoringStoryCatalogSameSeal(
          source.bindsCache,
          generation.bindsCache,
        )) {
      throw const FormatException(
        'authoring NPC catalog source provenance is inconsistent',
      );
    }
    final sourceIdentity = jsonEncode(<String, Object?>{
      'shipping_cache': _authoringNpcCatalogSealJson(source.shippingCache),
      'binds_cache': _authoringNpcCatalogSealJson(source.bindsCache),
    });
    _authoringNpcCatalogRequireBytesSeal(
      source.sourcePairSeal,
      sourceIdentity,
      'source pair',
    );

    final rawPayload = _authoringRequiredObject(
      rawCatalog['payload'],
      'NPC archetype catalog payload',
    );
    _authoringExactFields(rawPayload, const {
      'extractor_records_sha256',
      'records',
      'rejections',
    }, 'NPC archetype catalog payload');
    final extractorRecordsSha256 = _authoringNpcCatalogDigest(
      rawPayload['extractor_records_sha256'],
      'extractor_records_sha256',
    );
    final budget = _AuthoringNpcCatalogTextBudget();
    final records = _authoringNpcCatalogRecords(rawPayload['records'], budget);
    final rejections = _authoringNpcCatalogRejections(
      rawPayload['rejections'],
      budget,
    );

    final recordCount = _authoringRequiredInt(
      json,
      'record_count',
      max: _maxAuthoringNpcCatalogRecords,
    );
    final rejectionCount = _authoringRequiredInt(
      json,
      'rejection_count',
      max: _maxAuthoringNpcCatalogRejections,
    );
    if (recordCount != records.length || rejectionCount != rejections.length) {
      throw const FormatException(
        'authoring NPC catalog response counts disagree with its payload',
      );
    }

    final payloadSeal = _authoringStoryCatalogSeal(
      rawCatalog['payload_seal'],
      'NPC payload_seal',
    );
    final outerPayloadSeal = _authoringStoryCatalogSeal(
      json['payload_seal'],
      'NPC response payload_seal',
    );
    if (!_authoringStoryCatalogSameSeal(payloadSeal, outerPayloadSeal)) {
      throw const FormatException(
        'authoring NPC catalog payload seal disagrees with its response',
      );
    }
    _authoringNpcCatalogRequireBytesSeal(
      payloadSeal,
      jsonEncode(rawPayload),
      'payload',
    );

    final storyCatalogSeal = _authoringStoryCatalogSeal(
      rawCatalog['story_catalog_seal'],
      'NPC story_catalog_seal',
    );
    final catalogSeal = _authoringStoryCatalogSeal(
      rawArtifact['catalog_seal'],
      'NPC catalog_seal',
    );
    final outerCatalogSeal = _authoringStoryCatalogSeal(
      json['catalog_seal'],
      'NPC response catalog_seal',
    );
    if (!_authoringStoryCatalogSameSeal(catalogSeal, outerCatalogSeal)) {
      throw const FormatException(
        'authoring NPC catalog seal disagrees with its response',
      );
    }
    _authoringNpcCatalogRequireBytesSeal(
      catalogSeal,
      jsonEncode(rawCatalog),
      'catalog',
    );
    if (jsonEncode(rawArtifact) != catalogJson) {
      throw const FormatException(
        'authoring NPC archetype catalog is not canonical JSON',
      );
    }

    return AuthoringNpcArchetypeCatalogBuildResult._(
      requestBindingSha256: requestBindingSha256,
      catalogJson: catalogJson,
      generation: generation,
      catalogSeal: catalogSeal,
      storyCatalogSeal: storyCatalogSeal,
      source: source,
      payloadSeal: payloadSeal,
      extractorRecordsSha256: extractorRecordsSha256,
      qualification: qualification,
      records: List<AuthoringNpcCatalogRecord>.unmodifiable(records),
      rejections: List<AuthoringNpcCatalogRejection>.unmodifiable(rejections),
    );
  }
}

final class _AuthoringNpcCatalogTextBudget {
  int _used = 0;

  String add(Object? raw, String context) {
    if (raw is! String || raw.isEmpty) {
      throw FormatException('authoring NPC catalog $context is not text');
    }
    final byteLength = utf8.encode(raw).length;
    if (byteLength > _maxAuthoringNpcCatalogTextBytes ||
        byteLength > _maxAuthoringNpcCatalogTotalTextBytes - _used) {
      throw FormatException(
        'authoring NPC catalog $context exceeds its text budget',
      );
    }
    _used += byteLength;
    return raw;
  }
}

List<AuthoringNpcCatalogRecord> _authoringNpcCatalogRecords(
  Object? raw,
  _AuthoringNpcCatalogTextBudget budget,
) {
  if (raw is! List || raw.length > _maxAuthoringNpcCatalogRecords) {
    throw const FormatException(
      'authoring NPC catalog records is not a bounded array',
    );
  }
  final records = <AuthoringNpcCatalogRecord>[];
  String? previous;
  for (var index = 0; index < raw.length; index++) {
    final record = _authoringNpcCatalogRecord(
      _authoringRequiredObject(raw[index], 'NPC catalog record $index'),
      budget,
    );
    if (previous != null &&
        _authoringNpcCatalogCompareUtf8(previous, record.spawn.className) >=
            0) {
      throw const FormatException(
        'authoring NPC catalog records are not sorted and unique',
      );
    }
    previous = record.spawn.className;
    records.add(record);
  }
  return records;
}

AuthoringNpcCatalogRecord _authoringNpcCatalogRecord(
  Map<String, Object?> json,
  _AuthoringNpcCatalogTextBudget budget,
) {
  _authoringExactFields(json, const {
    'spawn',
    'ai_config',
    'character_definition',
    'actor_blueprint',
    'blueprint_family',
    'spawn_ai_edge',
    'spawn_blueprint_edge',
    'ai_character_edge',
    'evidence_sha256',
  }, 'NPC catalog record');
  final spawn = _authoringNpcCatalogClass(
    json['spawn'],
    'record spawn',
    budget,
  );
  final aiConfig = _authoringNpcCatalogClass(
    json['ai_config'],
    'record AI config',
    budget,
  );
  final characterDefinition = _authoringNpcCatalogClass(
    json['character_definition'],
    'record character definition',
    budget,
  );
  final actorBlueprint = budget.add(
    json['actor_blueprint'],
    'record actor blueprint',
  );
  final spawnAiEdge = _authoringNpcCatalogEdge(
    json['spawn_ai_edge'],
    'record spawn-AI edge',
    budget,
  );
  final spawnBlueprintEdge = _authoringNpcCatalogEdge(
    json['spawn_blueprint_edge'],
    'record spawn-blueprint edge',
    budget,
  );
  final aiCharacterEdge = _authoringNpcCatalogEdge(
    json['ai_character_edge'],
    'record AI-character edge',
    budget,
  );
  if (spawnAiEdge.ownerClass != spawn.className ||
      spawnAiEdge.assignedValue != aiConfig.className ||
      spawnBlueprintEdge.ownerClass != spawn.className ||
      spawnBlueprintEdge.assignedValue != actorBlueprint ||
      aiCharacterEdge.ownerClass != aiConfig.className ||
      aiCharacterEdge.assignedValue != characterDefinition.className) {
    throw const FormatException(
      'authoring NPC catalog record linkage evidence is inconsistent',
    );
  }
  return AuthoringNpcCatalogRecord._(
    spawn: spawn,
    aiConfig: aiConfig,
    characterDefinition: characterDefinition,
    actorBlueprint: actorBlueprint,
    blueprintFamily: switch (json['blueprint_family']) {
      'human_base' => AuthoringNpcCatalogBlueprintFamily.humanBase,
      'human_woman' => AuthoringNpcCatalogBlueprintFamily.humanWoman,
      'other' => AuthoringNpcCatalogBlueprintFamily.other,
      _ => throw const FormatException(
        'authoring NPC catalog blueprint family is unsupported',
      ),
    },
    spawnAiEdge: spawnAiEdge,
    spawnBlueprintEdge: spawnBlueprintEdge,
    aiCharacterEdge: aiCharacterEdge,
    evidenceSha256: _authoringNpcCatalogDigest(
      json['evidence_sha256'],
      'record evidence_sha256',
      nonzero: true,
    ),
  );
}

AuthoringNpcCatalogClassEvidence _authoringNpcCatalogClass(
  Object? raw,
  String context,
  _AuthoringNpcCatalogTextBudget budget,
) {
  final json = _authoringRequiredObject(raw, 'NPC catalog $context');
  _authoringExactFields(json, const {
    'class_name',
    'super_class',
    'module_name',
    'relative_path',
    'source_seal',
  }, 'NPC catalog class evidence');
  final superClassRaw = json['super_class'];
  final String? superClass;
  if (superClassRaw == null) {
    superClass = null;
  } else {
    superClass = budget.add(superClassRaw, '$context super class');
  }
  return AuthoringNpcCatalogClassEvidence._(
    className: budget.add(json['class_name'], '$context class name'),
    superClass: superClass,
    moduleName: budget.add(json['module_name'], '$context module name'),
    relativePath: budget.add(json['relative_path'], '$context relative path'),
    sourceSeal: _authoringNpcCatalogNonemptySeal(
      json['source_seal'],
      'NPC $context source_seal',
    ),
  );
}

AuthoringNpcCatalogDefaultEdgeEvidence _authoringNpcCatalogEdge(
  Object? raw,
  String context,
  _AuthoringNpcCatalogTextBudget budget,
) {
  final json = _authoringRequiredObject(raw, 'NPC catalog $context');
  _authoringExactFields(json, const {
    'owner_class',
    'field_name',
    'assigned_value',
    'instruction_offset_dwords',
    'init_defaults_bytecode_seal',
    'evidence_sha256',
  }, 'NPC catalog default edge');
  final initDefaultsBytecodeSeal = _authoringNpcCatalogNonemptySeal(
    json['init_defaults_bytecode_seal'],
    'NPC $context bytecode seal',
  );
  if (initDefaultsBytecodeSeal.byteLength % Uint32List.bytesPerElement != 0 ||
      initDefaultsBytecodeSeal.byteLength >
          _maxAuthoringNpcCatalogFunctionBytecodeBytes) {
    throw FormatException(
      'authoring NPC catalog $context bytecode seal is not a bounded DWORD stream',
    );
  }
  final bytecodeDwords =
      initDefaultsBytecodeSeal.byteLength ~/ Uint32List.bytesPerElement;
  final instructionOffsetDwords = _authoringRequiredInt(
    json,
    'instruction_offset_dwords',
    max: bytecodeDwords - 1,
  );
  return AuthoringNpcCatalogDefaultEdgeEvidence._(
    ownerClass: budget.add(json['owner_class'], '$context owner class'),
    fieldName: budget.add(json['field_name'], '$context field name'),
    assignedValue: budget.add(
      json['assigned_value'],
      '$context assigned value',
    ),
    instructionOffsetDwords: instructionOffsetDwords,
    initDefaultsBytecodeSeal: initDefaultsBytecodeSeal,
    evidenceSha256: _authoringNpcCatalogDigest(
      json['evidence_sha256'],
      '$context evidence_sha256',
      nonzero: true,
    ),
  );
}

List<AuthoringNpcCatalogRejection> _authoringNpcCatalogRejections(
  Object? raw,
  _AuthoringNpcCatalogTextBudget budget,
) {
  if (raw is! List || raw.length > _maxAuthoringNpcCatalogRejections) {
    throw const FormatException(
      'authoring NPC catalog rejections is not a bounded array',
    );
  }
  final rejections = <AuthoringNpcCatalogRejection>[];
  String? previous;
  for (var index = 0; index < raw.length; index++) {
    final json = _authoringRequiredObject(
      raw[index],
      'NPC catalog rejection $index',
    );
    _authoringExactFields(json, const {
      'spawn_class',
      'reason',
    }, 'NPC catalog rejection');
    final spawnClass = budget.add(json['spawn_class'], 'rejection spawn class');
    if (previous != null &&
        _authoringNpcCatalogCompareUtf8(previous, spawnClass) >= 0) {
      throw const FormatException(
        'authoring NPC catalog rejections are not sorted and unique',
      );
    }
    previous = spawnClass;
    rejections.add(
      _authoringNpcCatalogRejection(
        spawnClass,
        _authoringRequiredObject(
          json['reason'],
          'NPC catalog rejection reason',
        ),
        budget,
      ),
    );
  }
  return rejections;
}

AuthoringNpcCatalogRejection _authoringNpcCatalogRejection(
  String spawnClass,
  Map<String, Object?> json,
  _AuthoringNpcCatalogTextBudget budget,
) {
  final kind = json['kind'];
  String text(String field) => budget.add(json[field], 'rejection $field');
  int count() => _authoringRequiredInt(json, 'count', min: 1, max: 0x7fffffff);
  switch (kind) {
    case 'missing_init_defaults':
      _authoringExactFields(json, const {
        'kind',
        'owner_class',
      }, 'NPC missing-init-defaults rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.missingInitDefaults,
        ownerClass: text('owner_class'),
      );
    case 'ambiguous_init_defaults':
      _authoringExactFields(json, const {
        'kind',
        'owner_class',
        'count',
      }, 'NPC ambiguous-init-defaults rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.ambiguousInitDefaults,
        ownerClass: text('owner_class'),
        count: count(),
      );
    case 'invalid_init_defaults_bytecode':
      _authoringExactFields(json, const {
        'kind',
        'owner_class',
        'detail',
      }, 'NPC invalid-init-defaults rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.invalidInitDefaultsBytecode,
        ownerClass: text('owner_class'),
        detail: text('detail'),
      );
    case 'missing_default_edge':
      _authoringExactFields(json, const {
        'kind',
        'owner_class',
        'field_name',
      }, 'NPC missing-default-edge rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.missingDefaultEdge,
        ownerClass: text('owner_class'),
        fieldName: text('field_name'),
      );
    case 'ambiguous_default_edge':
      _authoringExactFields(json, const {
        'kind',
        'owner_class',
        'field_name',
        'count',
      }, 'NPC ambiguous-default-edge rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.ambiguousDefaultEdge,
        ownerClass: text('owner_class'),
        fieldName: text('field_name'),
        count: count(),
      );
    case 'missing_referenced_class':
      _authoringExactFields(json, const {
        'kind',
        'role',
        'class_name',
      }, 'NPC missing-referenced-class rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.missingReferencedClass,
        role: text('role'),
        className: text('class_name'),
      );
    case 'wrong_ancestry':
      _authoringExactFields(json, const {
        'kind',
        'role',
        'class_name',
        'required_base',
      }, 'NPC wrong-ancestry rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.wrongAncestry,
        role: text('role'),
        className: text('class_name'),
        requiredBase: text('required_base'),
      );
    case 'inheritance_cycle':
      _authoringExactFields(json, const {
        'kind',
        'role',
        'class_name',
      }, 'NPC inheritance-cycle rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.inheritanceCycle,
        role: text('role'),
        className: text('class_name'),
      );
    case 'non_inheritable_class':
      _authoringExactFields(json, const {
        'kind',
        'role',
        'class_name',
      }, 'NPC non-inheritable-class rejection');
      return AuthoringNpcCatalogRejection._(
        spawnClass: spawnClass,
        kind: AuthoringNpcCatalogRejectionKind.nonInheritableClass,
        role: text('role'),
        className: text('class_name'),
      );
    default:
      throw const FormatException(
        'authoring NPC catalog rejection kind is unsupported',
      );
  }
}

String _authoringNpcCatalogDigest(
  Object? raw,
  String context, {
  bool nonzero = false,
}) {
  if (raw is! String ||
      !_authoringSha256Pattern.hasMatch(raw) ||
      (nonzero && _authoringNpcCatalogIsZeroDigest(raw))) {
    throw FormatException('authoring NPC catalog $context is not a digest');
  }
  return raw;
}

AuthoringDraftContentSeal _authoringNpcCatalogNonemptySeal(
  Object? raw,
  String context,
) {
  final seal = _authoringStoryCatalogSeal(raw, context);
  if (_authoringNpcCatalogIsZeroDigest(seal.sha256)) {
    throw FormatException('authoring NPC catalog $context must be nonempty');
  }
  return seal;
}

bool _authoringNpcCatalogIsZeroDigest(String value) {
  for (final unit in value.codeUnits) {
    if (unit != 0x30) return false;
  }
  return true;
}

int _authoringNpcCatalogCompareUtf8(String left, String right) {
  final leftBytes = utf8.encode(left);
  final rightBytes = utf8.encode(right);
  final shared = leftBytes.length < rightBytes.length
      ? leftBytes.length
      : rightBytes.length;
  for (var index = 0; index < shared; index++) {
    final difference = leftBytes[index] - rightBytes[index];
    if (difference != 0) return difference;
  }
  return leftBytes.length - rightBytes.length;
}

void _authoringNpcCatalogRequireBytesSeal(
  AuthoringDraftContentSeal seal,
  String canonicalJson,
  String context,
) {
  final bytes = utf8.encode(canonicalJson);
  if (seal.byteLength != bytes.length ||
      seal.sha256 != crypto.sha256.convert(bytes).toString()) {
    throw FormatException('authoring NPC catalog $context seal is invalid');
  }
}

Map<String, Object?> _authoringNpcCatalogSealJson(
  AuthoringDraftContentSeal seal,
) => <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

bool _authoringNpcCatalogSameSource(
  AuthoringNpcCatalogSource left,
  AuthoringNpcCatalogSource right,
) =>
    _authoringStoryCatalogSameSeal(left.shippingCache, right.shippingCache) &&
    _authoringStoryCatalogSameSeal(left.bindsCache, right.bindsCache) &&
    _authoringStoryCatalogSameSeal(left.sourcePairSeal, right.sourcePairSeal);

enum AuthoringStoryCatalogNpcDiscoveryStatus { sealedCacheDefaultsVerified }

enum AuthoringStoryCatalogNpcAuthoringQualification { offlineQualified }

enum AuthoringStoryCatalogRuntimeQualification { runtimeUnqualified }

enum AuthoringStoryCatalogQuestParentRole { chapter }

enum AuthoringStoryCatalogQuestParentQualification { curatedDefaultsVerified }

enum AuthoringStoryCatalogCollisionStatus { inventoryUnavailable }

final _authoringStoryCatalogIdPattern = RegExp(r'^[a-z0-9][a-z0-9._:-]*$');
final _authoringStoryCatalogAliasPattern = RegExp(r'^Catalog_[0-9a-f]{64}$');
const _authoringStoryCatalogSelectorDomain =
    'gore-story-catalog.authoring-selector-v1\u0000';
const _authoringStoryCatalogBuildBindingDomain =
    'gore-story-catalog.authoring-build-v1.request-binding\u0000';
const _authoringStoryCatalogGameRootBindingDomain =
    'gore-story-catalog.authoring-build-for-game-root-v1.request-binding\u0000';
const _authoringNpcCatalogGameRootBindingDomain =
    'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000';

final class AuthoringStoryCatalogBuildResult {
  const AuthoringStoryCatalogBuildResult._({
    required this.requestBindingSha256,
    required this.catalogJson,
    required this.generation,
    required this.catalogSeal,
  });

  final String requestBindingSha256;
  final String catalogJson;
  final AuthoringStoryCatalogGeneration generation;
  final AuthoringDraftContentSeal catalogSeal;

  factory AuthoringStoryCatalogBuildResult._fromJson(
    Map<String, Object?> json, {
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) => AuthoringStoryCatalogBuildResult._fromBoundJson(
    json,
    expectedBinding: _authoringStoryCatalogBuildBindingSha256(
      executable,
      shippingCache,
      bindsCache,
    ),
  );

  factory AuthoringStoryCatalogBuildResult._fromGameRootJson(
    Map<String, Object?> json, {
    required String gameRoot,
  }) => AuthoringStoryCatalogBuildResult._fromBoundJson(
    json,
    expectedBinding: _authoringStoryCatalogGameRootBindingSha256(gameRoot),
  );

  factory AuthoringStoryCatalogBuildResult._fromBoundJson(
    Map<String, Object?> json, {
    required String expectedBinding,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_binding_sha256',
      'catalog_json',
      'generation',
      'catalog_seal',
    }, 'Story catalog build response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring Story catalog build response is not ok',
      );
    }
    final requestBindingSha256 = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    if (!_authoringSha256Pattern.hasMatch(requestBindingSha256) ||
        requestBindingSha256 != expectedBinding) {
      throw const FormatException(
        'authoring Story catalog build response is not bound to its exact paths',
      );
    }
    final catalogJson = _authoringRequiredString(
      json,
      'catalog_json',
      maxBytes: _maxAuthoringStoryCatalogJsonBytes,
    );
    final rawCatalog = _authoringDecodeDuplicateSafeObject(
      catalogJson,
      'Story catalog build result',
    );
    if (jsonEncode(rawCatalog) != catalogJson) {
      throw const FormatException(
        'authoring Story catalog build result is not canonical JSON',
      );
    }
    _authoringExactFields(rawCatalog, const {
      'format',
      'schema_revision',
      'catalog',
      'catalog_seal',
    }, 'Story catalog build result');
    if (rawCatalog['format'] != 'story_catalog') {
      throw const FormatException(
        'authoring Story catalog build result has an unsupported format',
      );
    }
    _authoringRequiredInt(rawCatalog, 'schema_revision', min: 1, max: 1);
    final rawPayload = _authoringRequiredObject(
      rawCatalog['catalog'],
      'Story catalog build payload',
    );
    _authoringExactFields(rawPayload, const {
      'generation',
      'record_set_id',
      'record_set_seal',
      'npcs',
      'quest_parents',
    }, 'Story catalog build payload');
    final generation = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        json['generation'],
        'Story catalog build generation',
      ),
    );
    final rawGeneration = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        rawPayload['generation'],
        'Story catalog build raw generation',
      ),
    );
    final catalogSeal = _authoringStoryCatalogSeal(
      json['catalog_seal'],
      'build catalog_seal',
    );
    final rawCatalogSeal = _authoringStoryCatalogSeal(
      rawCatalog['catalog_seal'],
      'build raw catalog_seal',
    );
    if (!_authoringStoryCatalogSameGeneration(generation, rawGeneration) ||
        !_authoringStoryCatalogSameSeal(catalogSeal, rawCatalogSeal)) {
      throw const FormatException(
        'authoring Story catalog build response disagrees with its raw catalog',
      );
    }
    return AuthoringStoryCatalogBuildResult._(
      requestBindingSha256: requestBindingSha256,
      catalogJson: catalogJson,
      generation: generation,
      catalogSeal: catalogSeal,
    );
  }
}

/// Closed projection of one native, sealed base-game collision-inventory artifact.
///
/// This remains runtime-unqualified and is not a resolved-loadout capability. In particular,
/// merely obtaining this DTO does not enable Quest creation.
final class AuthoringStoryCatalogSelections {
  const AuthoringStoryCatalogSelections._({
    required this.requestCatalogSha256,
    required this.schemaRevision,
    required this.generation,
    required this.catalogSeal,
    required this.npcs,
    required this.questParents,
    required this.questCollisionCatalog,
    required this.blocksBuild,
  });

  final String requestCatalogSha256;
  final int schemaRevision;
  final AuthoringStoryCatalogGeneration generation;
  final AuthoringDraftContentSeal catalogSeal;
  final List<AuthoringStoryCatalogNpcSelection> npcs;
  final List<AuthoringStoryCatalogQuestParentSelection> questParents;
  final AuthoringStoryCatalogCollisionAvailability questCollisionCatalog;
  final bool blocksBuild;

  factory AuthoringStoryCatalogSelections._fromJson(
    Map<String, Object?> json, {
    required String catalogJson,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_catalog_sha256',
      'selections',
    }, 'Story catalog response');
    if (json['ok'] != true) {
      throw const FormatException('authoring Story catalog response is not ok');
    }
    final requestCatalogSha256 = _authoringRequiredString(
      json,
      'request_catalog_sha256',
      maxBytes: 64,
    );
    final expectedBinding = crypto.sha256
        .convert(utf8.encode(catalogJson))
        .toString();
    if (!_authoringSha256Pattern.hasMatch(requestCatalogSha256) ||
        requestCatalogSha256 != expectedBinding) {
      throw const FormatException(
        'authoring Story catalog response is not bound to its exact request',
      );
    }

    final selections = _authoringRequiredObject(
      json['selections'],
      'Story catalog selections',
    );
    _authoringExactFields(selections, const {
      'schema_revision',
      'generation',
      'catalog_seal',
      'npcs',
      'quest_parents',
      'quest_collision_catalog',
      'blocks_build',
    }, 'Story catalog selections');
    final schemaRevision = _authoringRequiredInt(
      selections,
      'schema_revision',
      min: 1,
      max: 1,
    );
    final generation = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        selections['generation'],
        'Story catalog generation',
      ),
    );
    final catalogSeal = _authoringStoryCatalogSeal(
      selections['catalog_seal'],
      'catalog_seal',
    );
    final npcs = _authoringStoryCatalogList(
      selections['npcs'],
      expectedLength: _maxAuthoringStoryCatalogNpcs,
      context: 'NPC selections',
      decode: AuthoringStoryCatalogNpcSelection._fromJson,
    );
    final questParents = _authoringStoryCatalogList(
      selections['quest_parents'],
      expectedLength: _maxAuthoringStoryCatalogQuestParents,
      context: 'Quest-parent selections',
      decode: AuthoringStoryCatalogQuestParentSelection._fromJson,
    );
    _authoringStoryCatalogRequireSortedIds(
      npcs.map((entry) => entry.catalogId),
      'NPC selections',
    );
    _authoringStoryCatalogRequireSortedIds(
      questParents.map((entry) => entry.catalogId),
      'Quest-parent selections',
    );
    final questCollisionCatalog =
        AuthoringStoryCatalogCollisionAvailability._fromJson(
          _authoringRequiredObject(
            selections['quest_collision_catalog'],
            'Story catalog collision availability',
          ),
        );
    if (!_authoringStoryCatalogSameSeal(
      questCollisionCatalog.sourceSeal,
      generation.shippingCache,
    )) {
      throw const FormatException(
        'authoring Story catalog collision seal is not the generation Shipping cache',
      );
    }

    final aliases = <String>{};
    for (final npc in npcs) {
      for (final alias in <String>[
        npc.characterDefinition.authoringSelector,
        npc.aiAgentConfig.authoringSelector,
        npc.spawnDefinition.authoringSelector,
        npc.questGiver.authoringSelector,
      ]) {
        if (!aliases.add(alias)) {
          throw const FormatException(
            'authoring Story catalog selector aliases are not unique',
          );
        }
      }
    }
    for (final parent in questParents) {
      if (!aliases.add(parent.questClass.authoringSelector)) {
        throw const FormatException(
          'authoring Story catalog selector aliases are not unique',
        );
      }
    }
    final blocksBuild = _authoringRequiredBool(selections, 'blocks_build');
    if (!blocksBuild ||
        npcs.any((entry) => !entry.blocksBuild) ||
        questParents.any((entry) => !entry.blocksBuild) ||
        !questCollisionCatalog.blocksDraftCreation) {
      throw const FormatException(
        'authoring Story catalog readiness gates are inconsistent',
      );
    }
    return AuthoringStoryCatalogSelections._(
      requestCatalogSha256: requestCatalogSha256,
      schemaRevision: schemaRevision,
      generation: generation,
      catalogSeal: catalogSeal,
      npcs: List.unmodifiable(npcs),
      questParents: List.unmodifiable(questParents),
      questCollisionCatalog: questCollisionCatalog,
      blocksBuild: blocksBuild,
    );
  }
}

final class AuthoringStoryCatalogGeneration {
  const AuthoringStoryCatalogGeneration._({
    required this.edition,
    required this.executable,
    required this.shippingCache,
    required this.bindsCache,
  });

  final String edition;
  final AuthoringDraftContentSeal executable;
  final AuthoringDraftContentSeal shippingCache;
  final AuthoringDraftContentSeal bindsCache;

  factory AuthoringStoryCatalogGeneration._fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'edition',
      'executable',
      'shipping_cache',
      'binds_cache',
    }, 'Story catalog generation');
    final edition = _authoringRequiredString(json, 'edition', maxBytes: 64);
    if (edition != 'g1r-steam') {
      throw const FormatException(
        'authoring Story catalog edition is not supported',
      );
    }
    return AuthoringStoryCatalogGeneration._(
      edition: edition,
      executable: _authoringStoryCatalogSeal(
        json['executable'],
        'generation.executable',
      ),
      shippingCache: _authoringStoryCatalogSeal(
        json['shipping_cache'],
        'generation.shipping_cache',
      ),
      bindsCache: _authoringStoryCatalogSeal(
        json['binds_cache'],
        'generation.binds_cache',
      ),
    );
  }
}

final class AuthoringStoryCatalogClassSelection {
  const AuthoringStoryCatalogClassSelection._({
    required this.catalogLayer,
    required this.authoringSelector,
    required this.sourceCatalogSelector,
    required this.runtimeClass,
    required this.sourceSeal,
  });

  final String catalogLayer;
  final String authoringSelector;
  final String sourceCatalogSelector;
  final String runtimeClass;
  final AuthoringDraftContentSeal sourceSeal;

  factory AuthoringStoryCatalogClassSelection._fromJson(
    Map<String, Object?> json, {
    required String catalogId,
    required String role,
    required String expectedCatalogLayer,
  }) {
    _authoringExactFields(json, const {
      'catalog_layer',
      'authoring_selector',
      'source_catalog_selector',
      'runtime_class',
      'source_seal',
    }, 'Story catalog class selection');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    _authoringDraftValidateCatalogLayer(catalogLayer, 'catalog_layer');
    if (catalogLayer != expectedCatalogLayer) {
      throw const FormatException(
        'authoring Story catalog class layer is not the pinned base-game layer',
      );
    }
    final authoringSelector = _authoringStoryCatalogSelectorAlias(
      json,
      'authoring_selector',
    );
    if (authoringSelector !=
        _authoringStoryCatalogExpectedSelector(catalogId, role)) {
      throw const FormatException(
        'authoring Story catalog class selector alias disagrees with its record and role',
      );
    }
    final sourceCatalogSelector = _authoringRequiredString(
      json,
      'source_catalog_selector',
      maxBytes: 4096,
    );
    final runtimeClass = _authoringRequiredString(
      json,
      'runtime_class',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(runtimeClass, 'runtime_class');
    _authoringStoryCatalogSourceSelector(sourceCatalogSelector, runtimeClass);
    return AuthoringStoryCatalogClassSelection._(
      catalogLayer: catalogLayer,
      authoringSelector: authoringSelector,
      sourceCatalogSelector: sourceCatalogSelector,
      runtimeClass: runtimeClass,
      sourceSeal: _authoringStoryCatalogSeal(
        json['source_seal'],
        'class.source_seal',
      ),
    );
  }
}

final class AuthoringStoryCatalogQuestGiverSelection {
  const AuthoringStoryCatalogQuestGiverSelection._({
    required this.catalogLayer,
    required this.authoringSelector,
    required this.sourceCatalogSelector,
    required this.runtimeUniqueName,
    required this.sourceSeal,
  });

  final String catalogLayer;
  final String authoringSelector;
  final String sourceCatalogSelector;
  final String runtimeUniqueName;
  final AuthoringDraftContentSeal sourceSeal;

  factory AuthoringStoryCatalogQuestGiverSelection._fromJson(
    Map<String, Object?> json, {
    required String catalogId,
    required String expectedCatalogLayer,
  }) {
    _authoringExactFields(json, const {
      'catalog_layer',
      'authoring_selector',
      'source_catalog_selector',
      'runtime_unique_name',
      'source_seal',
    }, 'Story catalog Quest-giver selection');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    _authoringDraftValidateCatalogLayer(
      catalogLayer,
      'quest_giver.catalog_layer',
    );
    if (catalogLayer != expectedCatalogLayer) {
      throw const FormatException(
        'authoring Story catalog Quest-giver layer is not the pinned base-game layer',
      );
    }
    final sourceCatalogSelector = _authoringRequiredString(
      json,
      'source_catalog_selector',
      maxBytes: 4096,
    );
    final runtimeUniqueName = _authoringRequiredString(
      json,
      'runtime_unique_name',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(
      runtimeUniqueName,
      'quest_giver.runtime_unique_name',
    );
    _authoringStoryCatalogSourceSelector(
      sourceCatalogSelector,
      'UCharacterDefinition_Human_$runtimeUniqueName',
    );
    final authoringSelector = _authoringStoryCatalogSelectorAlias(
      json,
      'authoring_selector',
    );
    if (authoringSelector !=
        _authoringStoryCatalogExpectedSelector(catalogId, 'quest_giver')) {
      throw const FormatException(
        'authoring Story catalog Quest-giver selector alias disagrees with its record and role',
      );
    }
    return AuthoringStoryCatalogQuestGiverSelection._(
      catalogLayer: catalogLayer,
      authoringSelector: authoringSelector,
      sourceCatalogSelector: sourceCatalogSelector,
      runtimeUniqueName: runtimeUniqueName,
      sourceSeal: _authoringStoryCatalogSeal(
        json['source_seal'],
        'quest_giver.source_seal',
      ),
    );
  }
}

final class AuthoringStoryCatalogNpcSelection {
  const AuthoringStoryCatalogNpcSelection._({
    required this.catalogId,
    required this.displayName,
    required this.runtimeUniqueName,
    required this.characterDefinition,
    required this.aiAgentConfig,
    required this.spawnDefinition,
    required this.questGiver,
    required this.discoveryStatus,
    required this.authoringQualification,
    required this.runtimeQualification,
    required this.evidenceId,
    required this.blocksBuild,
  });

  final String catalogId;
  final String displayName;
  final String runtimeUniqueName;
  final AuthoringStoryCatalogClassSelection characterDefinition;
  final AuthoringStoryCatalogClassSelection aiAgentConfig;
  final AuthoringStoryCatalogClassSelection spawnDefinition;
  final AuthoringStoryCatalogQuestGiverSelection questGiver;
  final AuthoringStoryCatalogNpcDiscoveryStatus discoveryStatus;
  final AuthoringStoryCatalogNpcAuthoringQualification authoringQualification;
  final AuthoringStoryCatalogRuntimeQualification runtimeQualification;
  final String evidenceId;
  final bool blocksBuild;

  factory AuthoringStoryCatalogNpcSelection._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'catalog_id',
      'display_name',
      'runtime_unique_name',
      'character_definition',
      'ai_agent_config',
      'spawn_definition',
      'quest_giver',
      'discovery_status',
      'authoring_qualification',
      'runtime_qualification',
      'evidence_id',
      'blocks_build',
    }, 'Story catalog NPC selection');
    final catalogId = _authoringStoryCatalogId(json, 'catalog_id', 'g1r:npc:');
    final runtimeUniqueName = _authoringRequiredString(
      json,
      'runtime_unique_name',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(runtimeUniqueName, 'runtime_unique_name');
    final characterDefinition = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['character_definition'],
        'Story catalog character definition',
      ),
      catalogId: catalogId,
      role: 'character_definition',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    final aiAgentConfig = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['ai_agent_config'],
        'Story catalog AI-agent config',
      ),
      catalogId: catalogId,
      role: 'ai_agent_config',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    final spawnDefinition = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['spawn_definition'],
        'Story catalog spawn definition',
      ),
      catalogId: catalogId,
      role: 'spawn_definition',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    if (!characterDefinition.runtimeClass.startsWith('UCharacterDefinition_') ||
        !aiAgentConfig.runtimeClass.startsWith('UAIAgentConfig_') ||
        !spawnDefinition.runtimeClass.startsWith('USpawnAIAgentDefinition_')) {
      throw const FormatException(
        'authoring Story catalog NPC class roles are inconsistent',
      );
    }
    final questGiver = AuthoringStoryCatalogQuestGiverSelection._fromJson(
      _authoringRequiredObject(
        json['quest_giver'],
        'Story catalog Quest giver',
      ),
      catalogId: catalogId,
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    if (questGiver.runtimeUniqueName != runtimeUniqueName ||
        questGiver.catalogLayer != characterDefinition.catalogLayer ||
        questGiver.sourceCatalogSelector !=
            characterDefinition.sourceCatalogSelector ||
        !_authoringStoryCatalogSameSeal(
          questGiver.sourceSeal,
          characterDefinition.sourceSeal,
        )) {
      throw const FormatException(
        'authoring Story catalog Quest giver disagrees with its NPC provenance',
      );
    }
    return AuthoringStoryCatalogNpcSelection._(
      catalogId: catalogId,
      displayName: _authoringStoryCatalogFriendlyText(json, 'display_name'),
      runtimeUniqueName: runtimeUniqueName,
      characterDefinition: characterDefinition,
      aiAgentConfig: aiAgentConfig,
      spawnDefinition: spawnDefinition,
      questGiver: questGiver,
      discoveryStatus: switch (json['discovery_status']) {
        'sealed_cache_defaults_verified' =>
          AuthoringStoryCatalogNpcDiscoveryStatus.sealedCacheDefaultsVerified,
        _ => throw const FormatException(
          'authoring Story catalog NPC discovery status is unsupported',
        ),
      },
      authoringQualification: switch (json['authoring_qualification']) {
        'offline_qualified' =>
          AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified,
        _ => throw const FormatException(
          'authoring Story catalog NPC authoring qualification is unsupported',
        ),
      },
      runtimeQualification: _authoringStoryCatalogRuntimeQualification(
        json['runtime_qualification'],
      ),
      evidenceId: _authoringStoryCatalogEvidenceId(json, 'evidence_id'),
      blocksBuild: _authoringRequiredBool(json, 'blocks_build'),
    );
  }
}

final class AuthoringStoryCatalogQuestParentSelection {
  const AuthoringStoryCatalogQuestParentSelection._({
    required this.catalogId,
    required this.displayName,
    required this.questClass,
    required this.parentClassName,
    required this.role,
    required this.qualification,
    required this.transitionQualification,
    required this.evidenceId,
    required this.blocksBuild,
  });

  final String catalogId;
  final String displayName;
  final AuthoringStoryCatalogClassSelection questClass;
  final String parentClassName;
  final AuthoringStoryCatalogQuestParentRole role;
  final AuthoringStoryCatalogQuestParentQualification qualification;
  final AuthoringStoryCatalogRuntimeQualification transitionQualification;
  final String evidenceId;
  final bool blocksBuild;

  factory AuthoringStoryCatalogQuestParentSelection._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'catalog_id',
      'display_name',
      'quest_class',
      'parent_class_name',
      'role',
      'qualification',
      'transition_qualification',
      'evidence_id',
      'blocks_build',
    }, 'Story catalog Quest-parent selection');
    final catalogId = _authoringStoryCatalogId(
      json,
      'catalog_id',
      'g1r:quest-parent:',
    );
    final questClass = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['quest_class'],
        'Story catalog Quest class',
      ),
      catalogId: catalogId,
      role: 'quest_parent',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    final parentClassName = _authoringRequiredString(
      json,
      'parent_class_name',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(parentClassName, 'parent_class_name');
    if (!questClass.runtimeClass.startsWith('UQuest_') ||
        !parentClassName.startsWith('UQuest_') ||
        questClass.runtimeClass == parentClassName) {
      throw const FormatException(
        'authoring Story catalog Quest parent classes are inconsistent',
      );
    }
    return AuthoringStoryCatalogQuestParentSelection._(
      catalogId: catalogId,
      displayName: _authoringStoryCatalogFriendlyText(json, 'display_name'),
      questClass: questClass,
      parentClassName: parentClassName,
      role: switch (json['role']) {
        'chapter' => AuthoringStoryCatalogQuestParentRole.chapter,
        _ => throw const FormatException(
          'authoring Story catalog Quest-parent role is unsupported',
        ),
      },
      qualification: switch (json['qualification']) {
        'curated_defaults_verified' =>
          AuthoringStoryCatalogQuestParentQualification.curatedDefaultsVerified,
        _ => throw const FormatException(
          'authoring Story catalog Quest-parent qualification is unsupported',
        ),
      },
      transitionQualification: _authoringStoryCatalogRuntimeQualification(
        json['transition_qualification'],
      ),
      evidenceId: _authoringStoryCatalogEvidenceId(json, 'evidence_id'),
      blocksBuild: _authoringRequiredBool(json, 'blocks_build'),
    );
  }
}

final class AuthoringStoryCatalogCollisionAvailability {
  const AuthoringStoryCatalogCollisionAvailability._({
    required this.status,
    required this.catalogLayer,
    required this.sourceSeal,
    required this.blocksDraftCreation,
  });

  final AuthoringStoryCatalogCollisionStatus status;
  final String catalogLayer;
  final AuthoringDraftContentSeal sourceSeal;
  final bool blocksDraftCreation;

  factory AuthoringStoryCatalogCollisionAvailability._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'status',
      'catalog_layer',
      'source_seal',
      'blocks_draft_creation',
    }, 'Story catalog collision availability');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    if (catalogLayer != 'resolved-loadout.scripts.v1') {
      throw const FormatException(
        'authoring Story catalog collision layer is unsupported',
      );
    }
    final blocksDraftCreation = _authoringRequiredBool(
      json,
      'blocks_draft_creation',
    );
    if (!blocksDraftCreation) {
      throw const FormatException(
        'authoring Story catalog must block Quest Draft creation without an inventory',
      );
    }
    return AuthoringStoryCatalogCollisionAvailability._(
      status: switch (json['status']) {
        'inventory_unavailable' =>
          AuthoringStoryCatalogCollisionStatus.inventoryUnavailable,
        _ => throw const FormatException(
          'authoring Story catalog collision status is unsupported',
        ),
      },
      catalogLayer: catalogLayer,
      sourceSeal: _authoringStoryCatalogSeal(
        json['source_seal'],
        'quest_collision_catalog.source_seal',
      ),
      blocksDraftCreation: blocksDraftCreation,
    );
  }
}

List<T> _authoringStoryCatalogList<T>(
  Object? raw, {
  required int expectedLength,
  required String context,
  required T Function(Map<String, Object?>) decode,
}) {
  if (raw is! List || raw.length != expectedLength) {
    throw FormatException('authoring Story catalog $context has invalid size');
  }
  return [
    for (var index = 0; index < raw.length; index++)
      decode(
        _authoringRequiredObject(
          raw[index],
          'Story catalog $context entry $index',
        ),
      ),
  ];
}

void _authoringStoryCatalogRequireSortedIds(
  Iterable<String> values,
  String context,
) {
  String? previous;
  for (final value in values) {
    if (previous != null && previous.compareTo(value) >= 0) {
      throw FormatException(
        'authoring Story catalog $context is not sorted and unique',
      );
    }
    previous = value;
  }
}

AuthoringDraftContentSeal _authoringStoryCatalogSeal(
  Object? raw,
  String context,
) {
  final seal = AuthoringDraftContentSeal.fromJson(
    _authoringRequiredObject(raw, 'Story catalog $context'),
  );
  if (seal.byteLength > _maxAuthoringSignedJsonInteger) {
    throw FormatException(
      'authoring Story catalog $context exceeds wire range',
    );
  }
  return seal;
}

bool _authoringStoryCatalogSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

bool _authoringStoryCatalogSameGeneration(
  AuthoringStoryCatalogGeneration left,
  AuthoringStoryCatalogGeneration right,
) =>
    left.edition == right.edition &&
    _authoringStoryCatalogSameSeal(left.executable, right.executable) &&
    _authoringStoryCatalogSameSeal(left.shippingCache, right.shippingCache) &&
    _authoringStoryCatalogSameSeal(left.bindsCache, right.bindsCache);

String _authoringStoryCatalogSelectorAlias(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 72);
  if (!_authoringStoryCatalogAliasPattern.hasMatch(value)) {
    throw const FormatException(
      'authoring Story catalog selector alias is invalid',
    );
  }
  return value;
}

String _authoringStoryCatalogExpectedSelector(String catalogId, String role) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryCatalogSelectorDomain));
  for (final value in <String>[catalogId, role]) {
    final bytes = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
    input
      ..add(length)
      ..add(bytes);
  }
  input.close();
  return 'Catalog_${output.value}';
}

String _authoringStoryCatalogBuildBindingSha256(
  String executable,
  String shippingCache,
  String bindsCache,
) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryCatalogBuildBindingDomain));
  for (final value in <String>[executable, shippingCache, bindsCache]) {
    final bytes = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
    input
      ..add(length)
      ..add(bytes);
  }
  input.close();
  return output.value.toString();
}

String _authoringStoryCatalogGameRootBindingSha256(String gameRoot) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryCatalogGameRootBindingDomain));
  final bytes = utf8.encode(gameRoot);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
  input
    ..add(length)
    ..add(bytes)
    ..close();
  return output.value.toString();
}

String _authoringNpcCatalogGameRootBindingSha256(String gameRoot) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringNpcCatalogGameRootBindingDomain));
  final bytes = utf8.encode(gameRoot);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
  input
    ..add(length)
    ..add(bytes)
    ..close();
  return output.value.toString();
}

void _authoringStoryCatalogSourceSelector(String value, String runtimeClass) {
  if (!value.startsWith('script-class:') ||
      !value.endsWith('/$runtimeClass') ||
      value.codeUnits.any(
        (unit) => unit < 0x21 || unit > 0x7e || unit == 0x22 || unit == 0x5c,
      )) {
    throw const FormatException(
      'authoring Story catalog source selector is invalid',
    );
  }
}

String _authoringStoryCatalogId(
  Map<String, Object?> json,
  String field,
  String prefix,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 256);
  if (!value.startsWith(prefix) ||
      !_authoringStoryCatalogIdPattern.hasMatch(value)) {
    throw const FormatException('authoring Story catalog ID is invalid');
  }
  return value;
}

String _authoringStoryCatalogFriendlyText(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 256);
  if (value.trim() != value ||
      value.runes.any((rune) => rune < 0x20 || rune == 0x7f)) {
    throw const FormatException(
      'authoring Story catalog display text is invalid',
    );
  }
  return value;
}

String _authoringStoryCatalogEvidenceId(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 512);
  if (!_authoringStoryCatalogIdPattern.hasMatch(value)) {
    throw const FormatException(
      'authoring Story catalog evidence ID is invalid',
    );
  }
  return value;
}

AuthoringStoryCatalogRuntimeQualification
_authoringStoryCatalogRuntimeQualification(Object? value) => switch (value) {
  'runtime_unqualified' =>
    AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified,
  _ => throw const FormatException(
    'authoring Story catalog runtime qualification is unsupported',
  ),
};

enum AuthoringStoryDraftKind {
  npcDraft('npc_draft'),
  questDraft('quest_draft');

  const AuthoringStoryDraftKind(this.wireName);
  final String wireName;
}

bool _authoringJsonDeepEquals(Object? left, Object? right, [int depth = 0]) {
  if (depth > 128) {
    throw const FormatException(
      'authoring JSON exceeds the maximum nesting depth',
    );
  }
  if (left is Map && right is Map) {
    if (left.length != right.length) return false;
    for (final entry in left.entries) {
      if (entry.key is! String ||
          !right.containsKey(entry.key) ||
          !_authoringJsonDeepEquals(entry.value, right[entry.key], depth + 1)) {
        return false;
      }
    }
    return true;
  }
  if (left is List && right is List) {
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (!_authoringJsonDeepEquals(left[index], right[index], depth + 1)) {
        return false;
      }
    }
    return true;
  }
  if (left is num || right is num) {
    return left.runtimeType == right.runtimeType && left == right;
  }
  return left == right;
}

void _authoringRequireTypedStoryRef(
  Object? value, {
  required String projectId,
  required String id,
  required String kind,
  required String context,
}) {
  final ref = _authoringRequiredObject(value, 'Story Draft candidate $context');
  _authoringExactFields(ref, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'Story Draft candidate $context');
  if (ref['project_id'] != projectId ||
      ref['id'] != id ||
      ref['expected_kind'] != kind) {
    throw FormatException(
      'authoring Story Draft candidate $context is not exact',
    );
  }
}

class AuthoringWorkingHead {
  const AuthoringWorkingHead._({
    required this.canonicalJson,
    required this.snapshotByteLength,
    required this.snapshotSha256,
  });

  /// Exact canonical UTF-8 bytes represented as a Dart string. Do not re-encode this from fields
  /// when using it as a compare-and-swap token.
  final String canonicalJson;
  final int snapshotByteLength;
  final String snapshotSha256;

  factory AuthoringWorkingHead.fromCanonicalJson(String value) {
    if (value.isEmpty ||
        utf8.encode(value).length > _maxAuthoringHeadJsonBytes) {
      throw const FormatException(
        'authoring head JSON is empty or exceeds its size limit',
      );
    }
    final Object? decoded;
    try {
      decoded = jsonDecode(value);
    } on FormatException {
      throw const FormatException('authoring head JSON is invalid');
    }
    final object = _authoringRequiredObject(decoded, 'head');
    _authoringExactFields(object, const {'store_format', 'snapshot'}, 'head');
    final storeFormat = object['store_format'];
    if (storeFormat is! int || storeFormat != 1) {
      throw const FormatException('authoring head store_format is not 1');
    }
    final snapshot = _authoringRequiredObject(
      object['snapshot'],
      'head snapshot',
    );
    _authoringExactFields(snapshot, const {
      'byte_len',
      'sha256',
    }, 'head snapshot');
    final byteLength = _authoringRequiredInt(
      snapshot,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3SnapshotBytes,
    );
    final sha256 = _authoringRequiredString(snapshot, 'sha256', maxBytes: 64);
    if (!_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException('authoring head SHA-256 is not canonical');
    }
    if (jsonEncode(object) != value) {
      throw const FormatException('authoring head JSON is not canonical');
    }
    return AuthoringWorkingHead._(
      canonicalJson: value,
      snapshotByteLength: byteLength,
      snapshotSha256: sha256,
    );
  }
}

/// Bounded author intent for the native revision-3 Quest transaction.
///
/// Catalog IDs remain selectors only. Native code resolves them from a freshly rebuilt trusted
/// catalog; this DTO carries no resolved game values or authority evidence.
final class AuthoringRevision3QuestDraftIntentV3 {
  const AuthoringRevision3QuestDraftIntentV3._({
    required this.moduleNamespace,
    required this.technicalId,
    required this.textHelper,
    required this.parentCatalogId,
    required this.giverCatalogId,
    required this.title,
    required this.description,
    required this.objectiveTitle,
    required this.additionalObjectiveTitles,
  });

  factory AuthoringRevision3QuestDraftIntentV3({
    required String moduleNamespace,
    required String technicalId,
    required String textHelper,
    required String parentCatalogId,
    required String giverCatalogId,
    required String title,
    required String description,
    required String objectiveTitle,
    List<String> additionalObjectiveTitles = const <String>[],
  }) => AuthoringRevision3QuestDraftIntentV3.fromJson(<String, Object?>{
    'module_namespace': moduleNamespace,
    'technical_id': technicalId,
    'text_helper': textHelper,
    'parent_catalog_id': parentCatalogId,
    'giver_catalog_id': giverCatalogId,
    'title': title,
    'description': description,
    'objective_title': objectiveTitle,
    if (additionalObjectiveTitles.isNotEmpty)
      'additional_objective_titles': additionalObjectiveTitles,
  });

  final String moduleNamespace;
  final String technicalId;
  final String textHelper;
  final String parentCatalogId;
  final String giverCatalogId;
  final String title;
  final String description;
  final String objectiveTitle;
  final List<String> additionalObjectiveTitles;

  factory AuthoringRevision3QuestDraftIntentV3.fromJson(
    Map<String, Object?> json,
  ) {
    final hasAdditional = json.containsKey('additional_objective_titles');
    _authoringExactFields(json, <String>{
      'module_namespace',
      'technical_id',
      'text_helper',
      'parent_catalog_id',
      'giver_catalog_id',
      'title',
      'description',
      'objective_title',
      if (hasAdditional) 'additional_objective_titles',
    }, 'revision-3 Quest intent');
    _authoringRequireRevision3QuestFieldOrder(json, <String>[
      'module_namespace',
      'technical_id',
      'text_helper',
      'parent_catalog_id',
      'giver_catalog_id',
      'title',
      'description',
      'objective_title',
      if (hasAdditional) 'additional_objective_titles',
    ], 'intent');
    final objectiveTitle = _authoringRevision3QuestRequestString(
      json,
      'objective_title',
    );
    _authoringRevision3QuestValidateObjectiveTitle(
      objectiveTitle,
      'request objective 1',
    );
    final additionalObjectiveTitles = hasAdditional
        ? _authoringRevision3QuestObjectiveTitleList(
            json['additional_objective_titles'],
            firstTitle: objectiveTitle,
            requireAdditional: true,
            context: 'request',
          )
        : const <String>[];
    return AuthoringRevision3QuestDraftIntentV3._(
      moduleNamespace: _authoringRevision3QuestRequestString(
        json,
        'module_namespace',
      ),
      technicalId: _authoringRevision3QuestRequestString(json, 'technical_id'),
      textHelper: _authoringRevision3QuestRequestString(json, 'text_helper'),
      parentCatalogId: _authoringRevision3QuestRequestString(
        json,
        'parent_catalog_id',
      ),
      giverCatalogId: _authoringRevision3QuestRequestString(
        json,
        'giver_catalog_id',
      ),
      title: _authoringRevision3QuestRequestString(json, 'title'),
      description: _authoringRevision3QuestRequestString(json, 'description'),
      objectiveTitle: objectiveTitle,
      additionalObjectiveTitles: additionalObjectiveTitles,
    );
  }

  Map<String, Object?> toJson() => <String, Object?>{
    'module_namespace': moduleNamespace,
    'technical_id': technicalId,
    'text_helper': textHelper,
    'parent_catalog_id': parentCatalogId,
    'giver_catalog_id': giverCatalogId,
    'title': title,
    'description': description,
    'objective_title': objectiveTitle,
    if (additionalObjectiveTitles.isNotEmpty)
      'additional_objective_titles': additionalObjectiveTitles,
  };
}

/// Exact canonical request-v3 transport bound to one published R3 project/head.
final class AuthoringRevision3QuestDraftRequestV3 {
  const AuthoringRevision3QuestDraftRequestV3._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.questId,
    required this.scriptModuleId,
    required this.displayName,
    required this.intent,
  });

  factory AuthoringRevision3QuestDraftRequestV3({
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectId,
    required int expectedRevision,
    required String questId,
    required String scriptModuleId,
    required String displayName,
    required AuthoringRevision3QuestDraftIntentV3 intent,
  }) => AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'expected_head': jsonDecode(expectedHead.canonicalJson),
      'expected_project_id': expectedProjectId,
      'expected_revision': expectedRevision,
      'quest_id': questId,
      'script_module_id': scriptModuleId,
      'display_name': displayName,
      'intent': intent.toJson(),
    }),
  );

  /// Exact canonical UTF-8 JSON passed unchanged as `quest_request_json`.
  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String questId;
  final String scriptModuleId;
  final String displayName;
  final AuthoringRevision3QuestDraftIntentV3 intent;

  factory AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
    String value,
  ) {
    try {
      _authoringRevision3RequestString(
        value,
        'questRequestJson',
        _maxAuthoringRevision3QuestRequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Quest request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest request',
    );
    _authoringExactFields(request, const {
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'quest_id',
      'script_module_id',
      'display_name',
      'intent',
    }, 'revision-3 Quest request');
    _authoringRequireRevision3QuestFieldOrder(request, const <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'quest_id',
      'script_module_id',
      'display_name',
      'intent',
    ], 'request');
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 Quest request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Quest expected head',
        ),
      ),
    );
    final expectedProjectId = _authoringRevision3QuestEntityId(
      request,
      'expected_project_id',
    );
    final questId = _authoringRevision3QuestEntityId(request, 'quest_id');
    final scriptModuleId = _authoringRevision3QuestEntityId(
      request,
      'script_module_id',
    );
    if (questId == scriptModuleId) {
      throw const FormatException(
        'authoring revision-3 Quest request entity IDs must be distinct',
      );
    }
    return AuthoringRevision3QuestDraftRequestV3._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: expectedProjectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3QuestBasisRevision,
      ),
      questId: questId,
      scriptModuleId: scriptModuleId,
      displayName: _authoringRevision3QuestRequestString(
        request,
        'display_name',
      ),
      intent: AuthoringRevision3QuestDraftIntentV3.fromJson(
        _authoringRequiredObject(
          request['intent'],
          'revision-3 Quest request intent',
        ),
      ),
    );
  }
}

enum AuthoringRevision3QuestBuildStatus { blocked }

enum AuthoringRevision3QuestRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3QuestArtifactAuthority { notGranted }

enum AuthoringRevision3QuestSourceInspection { freshCapabilityRequired }

enum AuthoringRevision3QuestNativePublicationStatus { notSupported }

/// Strict result of the native prepare-only revision-3 Quest transaction.
///
/// `publicationStatus` describes the native command only. A managed session may subsequently
/// publish [head] as its fixed project checkpoint after exact full reopen and byte-CAS checks.
final class AuthoringRevision3QuestDraftPreparation {
  const AuthoringRevision3QuestDraftPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.questId,
    required this.scriptModuleId,
    required this.displayName,
    required this.moduleNamespace,
    required this.technicalId,
    required this.textHelper,
    required this.title,
    required this.description,
    required this.objectiveTitle,
    required this.additionalObjectiveTitles,
    required this.artifactDeduplicated,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.artifactAuthority,
    required this.sourceInspection,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String questId;
  final String scriptModuleId;
  final String displayName;
  final String moduleNamespace;
  final String technicalId;
  final String textHelper;
  final String title;
  final String description;
  final String objectiveTitle;
  final List<String> additionalObjectiveTitles;
  final bool artifactDeduplicated;
  final AuthoringRevision3QuestBuildStatus buildStatus;
  final AuthoringRevision3QuestRuntimeStatus runtimeStatus;
  final AuthoringRevision3QuestArtifactAuthority artifactAuthority;
  final AuthoringRevision3QuestSourceInspection sourceInspection;
  final AuthoringRevision3QuestNativePublicationStatus publicationStatus;

  factory AuthoringRevision3QuestDraftPreparation.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'revision',
      'quest_id',
      'script_module_id',
      'artifact_deduplicated',
      'build_status',
      'runtime_status',
      'artifact_authority',
      'source_inspection',
      'publication_status',
    }, 'revision-3 Quest preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 Quest preparation response is not prepared',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest candidate did not advance its head',
      );
    }
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final project = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    if (revision != project.revision) {
      throw const FormatException(
        'authoring revision-3 Quest response revision disagrees with its project',
      );
    }
    final questId = _authoringRevision3QuestEntityId(json, 'quest_id');
    final scriptModuleId = _authoringRevision3QuestEntityId(
      json,
      'script_module_id',
    );
    if (questId == scriptModuleId) {
      throw const FormatException(
        'authoring revision-3 Quest response entity IDs must be distinct',
      );
    }
    final candidate = _authoringRequireRevision3QuestCandidatePair(
      projectJson,
      basisHead: basisHead,
      projectId: project.projectId,
      questId: questId,
      scriptModuleId: scriptModuleId,
    );
    return AuthoringRevision3QuestDraftPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: project.projectId,
      revision: revision,
      questId: questId,
      scriptModuleId: scriptModuleId,
      displayName: candidate.displayName,
      moduleNamespace: candidate.moduleNamespace,
      technicalId: candidate.technicalId,
      textHelper: candidate.textHelper,
      title: candidate.title,
      description: candidate.description,
      objectiveTitle: candidate.objectiveTitle,
      additionalObjectiveTitles: candidate.additionalObjectiveTitles,
      artifactDeduplicated: _authoringRequiredBool(
        json,
        'artifact_deduplicated',
      ),
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3QuestBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 Quest response has an unsupported build status',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3QuestRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 Quest response has an unsupported runtime status',
        ),
      },
      artifactAuthority: switch (json['artifact_authority']) {
        'not_granted' => AuthoringRevision3QuestArtifactAuthority.notGranted,
        _ => throw const FormatException(
          'authoring revision-3 Quest response grants unsupported artifact authority',
        ),
      },
      sourceInspection: switch (json['source_inspection']) {
        'fresh_capability_required' =>
          AuthoringRevision3QuestSourceInspection.freshCapabilityRequired,
        _ => throw const FormatException(
          'authoring revision-3 Quest response grants unsupported source inspection',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3QuestNativePublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 Quest response grants unsupported publication authority',
        ),
      },
    );
  }
}

String _authoringRevision3QuestRequestString(
  Map<String, Object?> json,
  String field,
) {
  final value = json[field];
  if (value is! String) {
    throw FormatException(
      'authoring revision-3 Quest request field $field is not a string',
    );
  }
  try {
    _authoringRevision3RequestString(
      value,
      field,
      _maxAuthoringRevision3QuestRequestJsonBytes,
    );
  } on ArgumentError {
    throw FormatException(
      'authoring revision-3 Quest request field $field is not bounded UTF-8',
    );
  }
  return value;
}

List<String> _authoringRevision3QuestObjectiveTitleList(
  Object? value, {
  required String firstTitle,
  required bool requireAdditional,
  required String context,
}) {
  if (value is! List<Object?> ||
      (requireAdditional && value.isEmpty) ||
      value.length >= _maxAuthoringRevision3QuestObjectives) {
    throw FormatException(
      'authoring revision-3 Quest $context objective list is not bounded',
    );
  }
  final output = <String>[];
  final folded = <String>{firstTitle.toLowerCase()};
  var totalBytes = utf8.encode(firstTitle).length;
  for (var index = 0; index < value.length; index++) {
    final title = value[index];
    if (title is! String) {
      throw FormatException(
        'authoring revision-3 Quest $context objective ${index + 2} is not text',
      );
    }
    _authoringRevision3QuestValidateObjectiveTitle(
      title,
      '$context objective ${index + 2}',
    );
    totalBytes += utf8.encode(title).length;
    if (totalBytes > _maxAuthoringRevision3QuestObjectiveTitlesBytes ||
        !folded.add(title.toLowerCase())) {
      throw FormatException(
        'authoring revision-3 Quest $context objective list is duplicate or too large',
      );
    }
    output.add(title);
  }
  return List<String>.unmodifiable(output);
}

void _authoringRevision3QuestValidateObjectiveTitle(
  String value,
  String context,
) {
  if (value.isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length >
          _maxAuthoringRevision3QuestObjectiveTitleBytes) {
    throw FormatException(
      'authoring revision-3 Quest $context is not bounded canonical text',
    );
  }
  for (final rune in value.runes) {
    if (rune < 0x20 || rune > 0x7e || rune == 0x22 || rune == 0x5c) {
      throw FormatException(
        'authoring revision-3 Quest $context contains unsupported text',
      );
    }
  }
}

void _authoringRequireRevision3QuestFieldOrder(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  final fields = json.keys.toList(growable: false);
  for (var index = 0; index < expected.length; index++) {
    if (fields[index] != expected[index]) {
      throw FormatException(
        'authoring revision-3 Quest $context has non-canonical field order',
      );
    }
  }
}

String _authoringRevision3QuestEntityId(
  Map<String, Object?> json,
  String field,
) {
  final id = _authoringEntityId(
    _authoringRequiredString(json, field, maxBytes: 32),
    field,
  );
  if (id == '00000000000000000000000000000000') {
    throw FormatException(
      'authoring revision-3 Quest field $field must not be zero',
    );
  }
  return id;
}

({
  String displayName,
  String moduleNamespace,
  String technicalId,
  String textHelper,
  String title,
  String description,
  String objectiveTitle,
  List<String> additionalObjectiveTitles,
})
_authoringRequireRevision3QuestCandidatePair(
  String projectJson, {
  required AuthoringWorkingHead basisHead,
  required String projectId,
  required String questId,
  required String scriptModuleId,
}) {
  final project = _authoringDecodeDuplicateSafeObject(
    projectJson,
    'revision-3 Quest candidate project',
  );
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Quest candidate entities',
  );
  final target = _authoringRequireRevision3QuestGeneration(
    project['target'],
    'project target',
  );
  final questEntity = _authoringRevision3QuestCandidateEntityData(
    entities,
    questId,
    'quest_draft',
  );
  final questData = questEntity.data;
  _authoringExactFields(questData, const {
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  }, 'revision-3 Quest candidate Quest data');
  _authoringRequireRevision3QuestFieldOrder(questData, const <String>[
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  ], 'candidate Quest data');
  final generatorVersion = _authoringRequireRevision3QuestGenerator(
    questData,
    'Quest',
  );
  final questInput = _authoringRequiredObject(
    questData['input'],
    'revision-3 Quest candidate Quest input',
  );
  final hasAdditionalObjectives = questInput.containsKey(
    'additional_objective_titles',
  );
  _authoringExactFields(questInput, <String>{
    'target',
    'quest_id',
    'module_namespace',
    'technical_id',
    'text_helper',
    'parent_quest',
    'giver',
    'title',
    'description',
    'objective_title',
    if (hasAdditionalObjectives) 'additional_objective_titles',
    'transition_plan',
    'collision_catalog',
  }, 'revision-3 Quest candidate Quest input');
  _authoringRequireRevision3QuestFieldOrder(questInput, <String>[
    'target',
    'quest_id',
    'module_namespace',
    'technical_id',
    'text_helper',
    'parent_quest',
    'giver',
    'title',
    'description',
    'objective_title',
    if (hasAdditionalObjectives) 'additional_objective_titles',
    'transition_plan',
    'collision_catalog',
  ], 'candidate Quest input');
  if (questInput['quest_id'] != questId) {
    throw const FormatException(
      'authoring revision-3 Quest candidate input identity disagrees',
    );
  }
  final inputTarget = _authoringRequireRevision3QuestGeneration(
    questInput['target'],
    'Quest input target',
  );
  if (inputTarget != target) {
    throw const FormatException(
      'authoring revision-3 Quest candidate target generation disagrees',
    );
  }
  final moduleNamespace = _authoringRevision3QuestCandidateString(
    questInput,
    'module_namespace',
  );
  final technicalId = _authoringRevision3QuestCandidateString(
    questInput,
    'technical_id',
  );
  final textHelper = _authoringRevision3QuestCandidateString(
    questInput,
    'text_helper',
  );
  final title = _authoringRevision3QuestCandidateString(questInput, 'title');
  final description = _authoringRevision3QuestCandidateString(
    questInput,
    'description',
  );
  final objectiveTitle = _authoringRevision3QuestCandidateString(
    questInput,
    'objective_title',
  );
  _authoringRevision3QuestValidateObjectiveTitle(
    objectiveTitle,
    'candidate objective 1',
  );
  final additionalObjectiveTitles = hasAdditionalObjectives
      ? _authoringRevision3QuestObjectiveTitleList(
          questInput['additional_objective_titles'],
          firstTitle: objectiveTitle,
          requireAdditional: true,
          context: 'candidate',
        )
      : const <String>[];
  final transitionPlan = AuthoringRevision3QuestTransitionPlanV1.fromJson(
    questInput['transition_plan'],
    context: 'revision-3 Quest candidate transition plan',
  );
  if (transitionPlan.objectiveOrder.length !=
      1 + additionalObjectiveTitles.length) {
    throw const FormatException(
      'authoring revision-3 Quest candidate transition plan does not cover every objective',
    );
  }
  _authoringRequireRevision3QuestResolvedCatalogValue(
    questInput['parent_quest'],
    target: target,
    runtimeField: 'runtime_class',
    context: 'parent Quest',
  );
  _authoringRequireRevision3QuestResolvedCatalogValue(
    questInput['giver'],
    target: target,
    runtimeField: 'runtime_unique_name',
    context: 'giver',
  );
  final collision = _authoringRequiredObject(
    questInput['collision_catalog'],
    'revision-3 Quest candidate collision artifact reference',
  );
  _authoringExactFields(collision, const {
    'generation',
    'catalog_layer',
    'artifact',
    'source_seal',
    'basis_snapshot',
  }, 'revision-3 Quest candidate collision artifact reference');
  _authoringRequireRevision3QuestFieldOrder(collision, const <String>[
    'generation',
    'catalog_layer',
    'artifact',
    'source_seal',
    'basis_snapshot',
  ], 'candidate collision artifact reference');
  if (_authoringRequireRevision3QuestGeneration(
        collision['generation'],
        'collision generation',
      ) !=
      target) {
    throw const FormatException(
      'authoring revision-3 Quest collision generation disagrees',
    );
  }
  if (collision['catalog_layer'] !=
      _authoringRevision3QuestCollisionCatalogLayer) {
    throw const FormatException(
      'authoring revision-3 Quest collision catalog layer is unsupported',
    );
  }
  final artifact = _authoringRequireRevision3QuestContentSeal(
    collision['artifact'],
    'collision artifact',
  );
  if (artifact.byteLength > _maxAuthoringRevision3QuestCollisionArtifactBytes) {
    throw const FormatException(
      'authoring revision-3 Quest collision artifact exceeds its closed-model limit',
    );
  }
  final sourceSeal = _authoringRequireRevision3QuestContentSeal(
    collision['source_seal'],
    'collision source seal',
  );
  if (sourceSeal.byteLength != artifact.byteLength) {
    throw const FormatException(
      'authoring revision-3 Quest collision raw and semantic artifact lengths disagree',
    );
  }
  final basisSnapshot = _authoringRequireRevision3QuestContentSeal(
    collision['basis_snapshot'],
    'collision basis snapshot',
  );
  if (basisSnapshot.byteLength != basisHead.snapshotByteLength ||
      basisSnapshot.sha256 != basisHead.snapshotSha256) {
    throw const FormatException(
      'authoring revision-3 Quest collision basis snapshot disagrees with its head',
    );
  }
  final assetStore = _authoringRequiredObject(
    project['asset_store'],
    'revision-3 Quest candidate AssetStore',
  );
  _authoringExactFields(assetStore, const {
    'assets',
  }, 'revision-3 Quest candidate AssetStore');
  final assets = _authoringRequiredObject(
    assetStore['assets'],
    'revision-3 Quest candidate AssetStore assets',
  );
  final artifactMeta = _authoringRequiredObject(
    assets[artifact.sha256],
    'revision-3 Quest collision artifact metadata',
  );
  _authoringExactFields(artifactMeta, const {
    'byte_len',
    'media_type',
  }, 'revision-3 Quest collision artifact metadata');
  if (artifactMeta['byte_len'] != artifact.byteLength ||
      artifactMeta['media_type'] !=
          _authoringRevision3QuestCollisionMediaType) {
    throw const FormatException(
      'authoring revision-3 Quest collision artifact metadata disagrees',
    );
  }
  final questOrigin = _authoringRequiredObject(
    questEntity.entity['origin'],
    'revision-3 Quest candidate Quest origin',
  );
  _authoringExactFields(questOrigin, const {
    'type',
    'authored_runtime_id',
  }, 'revision-3 Quest candidate Quest origin');
  if (questOrigin['type'] != 'new' ||
      questOrigin['authored_runtime_id'] != technicalId) {
    throw const FormatException(
      'authoring revision-3 Quest candidate Quest origin disagrees',
    );
  }
  _authoringRequireRevision3QuestTypedRef(
    questData['script_module'],
    projectId: projectId,
    id: scriptModuleId,
    kind: 'script_module',
    context: 'Quest script module',
  );

  final moduleEntity = _authoringRevision3QuestCandidateEntityData(
    entities,
    scriptModuleId,
    'script_module',
  );
  final moduleData = moduleEntity.data;
  _authoringExactFields(moduleData, const {
    'generator_id',
    'generator_version',
    'owner',
    'module_namespace',
    'module_relative_path',
    'source',
    'source_sha256',
    'input_fingerprint',
    'status',
  }, 'revision-3 Quest candidate ScriptModule data');
  _authoringRequireRevision3QuestFieldOrder(moduleData, const <String>[
    'generator_id',
    'generator_version',
    'owner',
    'module_namespace',
    'module_relative_path',
    'source',
    'source_sha256',
    'input_fingerprint',
    'status',
  ], 'candidate ScriptModule data');
  _authoringRequireRevision3QuestGenerator(
    moduleData,
    'ScriptModule',
    expectedVersion: generatorVersion,
  );
  _authoringRequireRevision3QuestTypedRef(
    moduleData['owner'],
    projectId: projectId,
    id: questId,
    kind: 'quest_draft',
    context: 'ScriptModule owner',
  );
  if (moduleData['module_namespace'] != moduleNamespace) {
    throw const FormatException(
      'authoring revision-3 Quest candidate module namespace disagrees',
    );
  }
  _authoringRevision3QuestCandidateString(moduleData, 'module_relative_path');
  final source = _authoringRevision3QuestCandidateString(moduleData, 'source');
  final sourceSha256 = _authoringRequiredString(
    moduleData,
    'source_sha256',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(sourceSha256) ||
      crypto.sha256.convert(utf8.encode(source)).toString() != sourceSha256) {
    throw const FormatException(
      'authoring revision-3 Quest candidate source seal disagrees',
    );
  }
  final inputFingerprint = _authoringRequiredString(
    moduleData,
    'input_fingerprint',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(inputFingerprint) ||
      _authoringRevision3QuestInputFingerprint(questInput) !=
          inputFingerprint) {
    throw const FormatException(
      'authoring revision-3 Quest candidate input fingerprint disagrees',
    );
  }
  final moduleOrigin = _authoringRequiredObject(
    moduleEntity.entity['origin'],
    'revision-3 Quest candidate ScriptModule origin',
  );
  _authoringExactFields(moduleOrigin, const {
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'revision-3 Quest candidate ScriptModule origin');
  if (moduleOrigin['type'] != 'generated') {
    throw const FormatException(
      'authoring revision-3 Quest candidate ScriptModule origin is unsupported',
    );
  }
  _authoringRequireRevision3QuestGenerator(
    moduleOrigin,
    'ScriptModule origin',
    expectedVersion: generatorVersion,
  );
  _authoringRequireRevision3QuestTypedRef(
    moduleOrigin['owner'],
    projectId: projectId,
    id: questId,
    kind: 'quest_draft',
    context: 'ScriptModule origin owner',
  );
  final status = _authoringRequiredObject(
    moduleData['status'],
    'revision-3 Quest candidate ScriptModule status',
  );
  _authoringExactFields(status, const {
    'authoring',
    'runtime',
  }, 'revision-3 Quest candidate ScriptModule status');
  if (status['authoring'] != 'offline_draft' ||
      status['runtime'] != 'runtime_unqualified') {
    throw const FormatException(
      'authoring revision-3 Quest candidate ScriptModule status is unsupported',
    );
  }
  final displayName = _authoringRevision3QuestCandidateString(
    questEntity.entity,
    'display_name',
  );
  return (
    displayName: displayName,
    moduleNamespace: moduleNamespace,
    technicalId: technicalId,
    textHelper: textHelper,
    title: title,
    description: description,
    objectiveTitle: objectiveTitle,
    additionalObjectiveTitles: additionalObjectiveTitles,
  );
}

({Map<String, Object?> entity, Map<String, Object?> data})
_authoringRevision3QuestCandidateEntityData(
  Map<String, Object?> entities,
  String id,
  String kind,
) {
  final entity = _authoringRequiredObject(
    entities[id],
    'revision-3 Quest candidate $kind entity',
  );
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'revision-3 Quest candidate $kind entity');
  if (entity['id'] != id) {
    throw FormatException(
      'authoring revision-3 Quest candidate $kind key and ID disagree',
    );
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 Quest candidate $kind payload',
  );
  _authoringExactFields(payload, const {
    'kind',
    'data',
  }, 'revision-3 Quest candidate $kind payload');
  if (payload['kind'] != kind) {
    throw FormatException(
      'authoring revision-3 Quest candidate $kind payload disagrees',
    );
  }
  return (
    entity: entity,
    data: _authoringRequiredObject(
      payload['data'],
      'revision-3 Quest candidate $kind data',
    ),
  );
}

int _authoringRequireRevision3QuestGenerator(
  Map<String, Object?> json,
  String context, {
  int? expectedVersion,
}) {
  final version = json['generator_version'];
  if (json['generator_id'] != _authoringRevision3QuestGeneratorId ||
      version != _authoringRevision3QuestGeneratorVersion ||
      (expectedVersion != null && version != expectedVersion)) {
    throw FormatException(
      'authoring revision-3 Quest candidate $context generator is unsupported',
    );
  }
  return version as int;
}

({int byteLength, String sha256}) _authoringRequireRevision3QuestContentSeal(
  Object? value,
  String context,
) {
  final seal = _authoringRequiredObject(
    value,
    'revision-3 Quest candidate $context',
  );
  _authoringExactFields(seal, const {
    'byte_len',
    'sha256',
  }, 'revision-3 Quest candidate $context');
  _authoringRequireRevision3QuestFieldOrder(seal, const <String>[
    'byte_len',
    'sha256',
  ], 'candidate $context');
  final sha256 = _authoringRequiredString(seal, 'sha256', maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(sha256)) {
    throw FormatException(
      'authoring revision-3 Quest candidate $context SHA-256 is invalid',
    );
  }
  return (
    byteLength: _authoringRequiredInt(
      seal,
      'byte_len',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    ),
    sha256: sha256,
  );
}

({int byteLength, String sha256}) _authoringRequireRevision3QuestGeneration(
  Object? value,
  String context,
) {
  final generation = _authoringRequiredObject(
    value,
    'revision-3 Quest candidate $context generation',
  );
  _authoringExactFields(generation, const {
    'executable',
  }, 'revision-3 Quest candidate $context generation');
  return _authoringRequireRevision3QuestContentSeal(
    generation['executable'],
    '$context executable',
  );
}

void _authoringRequireRevision3QuestResolvedCatalogValue(
  Object? value, {
  required ({int byteLength, String sha256}) target,
  required String runtimeField,
  required String context,
}) {
  final resolved = _authoringRequiredObject(
    value,
    'revision-3 Quest candidate $context',
  );
  _authoringExactFields(resolved, <String>{
    'generation',
    'source_seal',
    'catalog_layer',
    'canonical_selector',
    runtimeField,
  }, 'revision-3 Quest candidate $context');
  _authoringRequireRevision3QuestFieldOrder(resolved, <String>[
    'generation',
    'source_seal',
    'catalog_layer',
    'canonical_selector',
    runtimeField,
  ], 'candidate $context');
  if (_authoringRequireRevision3QuestGeneration(
        resolved['generation'],
        '$context generation',
      ) !=
      target) {
    throw FormatException(
      'authoring revision-3 Quest candidate $context generation disagrees',
    );
  }
  _authoringRequireRevision3QuestContentSeal(
    resolved['source_seal'],
    '$context source seal',
  );
  for (final field in <String>[
    'catalog_layer',
    'canonical_selector',
    runtimeField,
  ]) {
    _authoringRevision3QuestCandidateString(resolved, field);
  }
}

String _authoringRevision3QuestCandidateString(
  Map<String, Object?> json,
  String field,
) => _authoringRequiredString(
  json,
  field,
  maxBytes: _maxAuthoringProjectJsonBytes,
);

String _authoringRevision3QuestInputFingerprint(
  Map<String, Object?> questInput,
) {
  final inputBytes = utf8.encode(jsonEncode(questInput));
  final length = ByteData(8)..setUint64(0, inputBytes.length, Endian.big);
  final bytes = BytesBuilder(copy: false)
    ..add(utf8.encode(_authoringRevision3QuestFingerprintDomain))
    ..add(length.buffer.asUint8List())
    ..add(inputBytes);
  return crypto.sha256.convert(bytes.takeBytes()).toString();
}

void _authoringRequireRevision3QuestTypedRef(
  Object? value, {
  required String projectId,
  required String id,
  required String kind,
  required String context,
}) {
  final ref = _authoringRequiredObject(
    value,
    'revision-3 Quest candidate $context reference',
  );
  _authoringExactFields(ref, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 Quest candidate $context reference');
  if (ref['project_id'] != projectId ||
      ref['id'] != id ||
      ref['expected_kind'] != kind) {
    throw FormatException(
      'authoring revision-3 Quest candidate $context reference is not exact',
    );
  }
}

enum AuthoringRevision3ContentAuthority { readOnlyExactCurrentProject }

enum AuthoringRevision3ContentBuildStatus { notEvaluated }

enum AuthoringRevision3ContentRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3ContentPublicationStatus { notApplicable }

/// Strict semantic projection of one exact, currently-published revision-3 checkpoint.
///
/// The nested index is preserved in its canonical form and parsed only after duplicate-key,
/// signed-wire, closed-schema, and project-identity checks. The status enums deliberately expose
/// the native command's lack of build, runtime, and publication authority.
final class AuthoringRevision3ContentIndexResult {
  const AuthoringRevision3ContentIndexResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.indexJson,
    required this.index,
    required this.contentAuthority,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;

  /// Exact canonical nested bytes returned by native code.
  final String indexJson;
  final Revision3ContentIndex index;
  final AuthoringRevision3ContentAuthority contentAuthority;
  final AuthoringRevision3ContentBuildStatus buildStatus;
  final AuthoringRevision3ContentRuntimeStatus runtimeStatus;
  final AuthoringRevision3ContentPublicationStatus publicationStatus;

  factory AuthoringRevision3ContentIndexResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'head_json',
      'project_id',
      'project_revision',
      'index_json',
      'content_authority',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 content-index response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring revision-3 content-index response is not ok',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson != expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 content-index response changed its exact head',
      );
    }
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    if (projectId == '00000000000000000000000000000000') {
      throw const FormatException(
        'authoring revision-3 content-index project ID must not be zero',
      );
    }
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringSignedJsonInteger,
    );
    final indexJson = _authoringRevision3ResponseString(
      json,
      'index_json',
      maxBytes: _maxAuthoringRevision3ContentIndexJsonBytes,
    );
    final indexObject = _authoringDecodeDuplicateSafeObject(
      indexJson,
      'revision-3 content index',
    );
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      indexObject,
      'revision-3 content index',
    );
    if (jsonEncode(indexObject) != indexJson) {
      throw const FormatException(
        'authoring revision-3 content index is not canonical',
      );
    }
    final index = Revision3ContentIndex.fromJsonObject(indexObject);
    if (index.projectId != projectId ||
        index.projectRevision != projectRevision) {
      throw const FormatException(
        'authoring revision-3 content index disagrees with its project identity',
      );
    }
    return AuthoringRevision3ContentIndexResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      indexJson: indexJson,
      index: index,
      contentAuthority: switch (json['content_authority']) {
        'read_only_exact_current_project' =>
          AuthoringRevision3ContentAuthority.readOnlyExactCurrentProject,
        _ => throw const FormatException(
          'authoring revision-3 content-index response grants unsupported content authority',
        ),
      },
      buildStatus: switch (json['build_status']) {
        'not_evaluated' => AuthoringRevision3ContentBuildStatus.notEvaluated,
        _ => throw const FormatException(
          'authoring revision-3 content-index response has an unsupported build status',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3ContentRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 content-index response has an unsupported runtime status',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_applicable' =>
          AuthoringRevision3ContentPublicationStatus.notApplicable,
        _ => throw const FormatException(
          'authoring revision-3 content-index response grants unsupported publication authority',
        ),
      },
    );
  }
}

/// Exact, read-only schema-revision-3 Store reconstruction.
///
/// Absence of diagnostics/readiness fields is intentional: this DTO carries immutable checkpoint
/// bytes only and does not claim that the project can build, run, deploy, or publish.
final class AuthoringRevision3StoreOpenedResult {
  const AuthoringRevision3StoreOpenedResult._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
  });

  final AuthoringWorkingHead head;

  /// The exact canonical nested string returned by native code; never reconstructed from fields.
  final String projectJson;
  final String projectId;
  final int projectRevision;

  factory AuthoringRevision3StoreOpenedResult.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
      'project_json',
    }, 'revision-3 store-open response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring revision-3 store-open response is not ok',
      );
    }
    final headJson = _authoringRevision3ResponseString(
      json,
      'head_json',
      maxBytes: _maxAuthoringHeadJsonBytes,
    );
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final project = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
    return AuthoringRevision3StoreOpenedResult._(
      head: AuthoringWorkingHead.fromCanonicalJson(headJson),
      projectJson: projectJson,
      projectId: project.projectId,
      projectRevision: project.revision,
    );
  }
}

/// Head token for a prepare-only schema-revision-3 checkpoint operation.
///
/// The native command may install immutable objects, but this result cannot publish the fixed head
/// and intentionally contains no readiness, diagnostics, runtime, or publication claims.
final class AuthoringRevision3CheckpointPreparation {
  const AuthoringRevision3CheckpointPreparation._({required this.head});

  final AuthoringWorkingHead head;

  factory AuthoringRevision3CheckpointPreparation.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
    }, 'revision-3 checkpoint-preparation response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring revision-3 checkpoint-preparation response is not ok',
      );
    }
    final headJson = _authoringRevision3ResponseString(
      json,
      'head_json',
      maxBytes: _maxAuthoringHeadJsonBytes,
    );
    return AuthoringRevision3CheckpointPreparation._(
      head: AuthoringWorkingHead.fromCanonicalJson(headJson),
    );
  }
}

enum VoiceOggCodec { vorbis, opus }

class VoiceOggContentSeal {
  const VoiceOggContentSeal._({required this.byteLength, required this.sha256});

  final int byteLength;
  final String sha256;

  factory VoiceOggContentSeal.fromJson(Map<String, Object?> json) {
    _voiceExactFields(json, const {'byte_len', 'sha256'}, 'content seal');
    final sha256 = _voiceRequiredString(json, 'sha256');
    if (!RegExp(r'^[0-9a-f]{64}$').hasMatch(sha256)) {
      throw const FormatException(
        'voice Ogg content seal has an invalid SHA-256',
      );
    }
    return VoiceOggContentSeal._(
      byteLength: _voiceRequiredInt(
        json,
        'byte_len',
        min: 27,
        max: _maxVoiceOggBytes,
      ),
      sha256: sha256,
    );
  }
}

class VoiceOggInspectionResult {
  const VoiceOggInspectionResult._({
    required this.codec,
    required this.pages,
    required this.streams,
    required this.contentSeal,
  });

  final VoiceOggCodec codec;
  final int pages;
  final int streams;
  final VoiceOggContentSeal contentSeal;

  factory VoiceOggInspectionResult.fromJson(Map<String, Object?> json) {
    _voiceExactFields(json, const {
      'ok',
      'codec',
      'pages',
      'streams',
      'content_seal',
    }, 'Ogg inspection response');
    if (json['ok'] != true) {
      throw const FormatException('voice Ogg inspection response is not ok');
    }
    final codec = switch (json['codec']) {
      'vorbis' => VoiceOggCodec.vorbis,
      'opus' => VoiceOggCodec.opus,
      _ => throw const FormatException('unknown voice Ogg codec'),
    };
    final pages = _voiceRequiredInt(json, 'pages', min: 1, max: 0xffffffff);
    final streams = _voiceRequiredInt(json, 'streams', min: 1, max: 0xffffffff);
    final contentSeal = VoiceOggContentSeal.fromJson(
      _voiceRequiredObject(json['content_seal'], 'Ogg content seal'),
    );
    if (streams > pages || pages > contentSeal.byteLength ~/ 27) {
      throw const FormatException(
        'voice Ogg stream/page counts are inconsistent with its byte length',
      );
    }
    return VoiceOggInspectionResult._(
      codec: codec,
      pages: pages,
      streams: streams,
      contentSeal: contentSeal,
    );
  }
}

enum VoiceArchiveLineResolution { unresolved, unique, ambiguous }

void _voiceExactFields(
  Map<String, Object?> json,
  Set<String> expected,
  String context,
) {
  if (json.length != expected.length || !expected.every(json.containsKey)) {
    throw FormatException('voice $context has an invalid schema');
  }
}

Map<String, Object?> _voiceRequiredObject(Object? value, String context) {
  if (value is! Map) {
    throw FormatException('voice $context is not an object');
  }
  final result = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw FormatException('voice $context has a non-string field');
    }
    result[entry.key as String] = entry.value;
  }
  return result;
}

String _voiceRequiredString(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! String) {
    throw FormatException('voice response field $field is not a string');
  }
  return value;
}

int _voiceRequiredInt(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int? max,
}) {
  final value = json[field];
  if (value is! int || value < min || (max != null && value > max)) {
    throw FormatException(
      'voice response field $field is not an integer in range '
      '$min..${max ?? 'unbounded'}',
    );
  }
  return value;
}

int? _voiceOptionalInt(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int? max,
}) {
  if (json[field] == null) return null;
  return _voiceRequiredInt(json, field, min: min, max: max);
}

bool _voiceRequiredBool(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! bool) {
    throw FormatException('voice response field $field is not a bool');
  }
  return value;
}

bool _isAscii(String value) => value.codeUnits.every((unit) => unit <= 0x7f);

bool _asciiCaseEquals(String left, String right) =>
    _isAscii(left) &&
    _isAscii(right) &&
    left.toLowerCase() == right.toLowerCase();

class VoiceArchiveMatchLineResult {
  const VoiceArchiveMatchLineResult({
    required this.archive,
    required this.archiveSize,
    required this.archiveSha256,
    required this.locId,
    required this.expectedBasename,
    required this.resolution,
    required this.matches,
  });

  final String archive;
  final int archiveSize;
  final String archiveSha256;
  final String locId;
  final String expectedBasename;
  final VoiceArchiveLineResolution resolution;
  final List<VoiceArchiveEntryInfo> matches;

  factory VoiceArchiveMatchLineResult.fromJson(Map<String, Object?> j) {
    final archive = _voiceRequiredString(j, 'archive');
    final archiveSize = _voiceRequiredInt(j, 'archive_size');
    final locId = _voiceRequiredString(j, 'loc_id');
    if (!_isAscii(locId)) {
      throw const FormatException(
        'voice match response has a non-ASCII loc_id',
      );
    }
    final expectedBasename = _voiceRequiredString(j, 'expected_basename');
    if (expectedBasename != '$locId.ogg') {
      throw const FormatException(
        'voice match expected_basename does not equal loc_id + .ogg',
      );
    }

    final rawMatches = j['matches'];
    if (rawMatches is! List) {
      throw const FormatException('voice match response has no matches array');
    }
    final matches = <VoiceArchiveEntryInfo>[];
    for (final rawMatch in rawMatches) {
      if (rawMatch is! Map) {
        throw const FormatException(
          'voice match response contains a non-object match',
        );
      }
      final match = VoiceArchiveEntryInfo.fromJson(
        rawMatch.cast<String, Object?>(),
      );
      if (!_asciiCaseEquals(match.basename, expectedBasename)) {
        throw const FormatException(
          'voice match basename does not match expected_basename',
        );
      }
      final pathParts = match.path.split('/');
      if (match.path.isEmpty ||
          match.path.contains('\\') ||
          pathParts.isEmpty ||
          pathParts.last != match.basename) {
        throw const FormatException(
          'voice match basename is inconsistent with its entry path',
        );
      }
      if (match.isDirectory || match.isSymlink || match.encrypted) {
        throw const FormatException(
          'voice match response contains an ineligible entry',
        );
      }
      final expectedCompression = switch (match.compressionCode) {
        0 => 'stored',
        8 => 'deflated',
        _ => throw const FormatException(
          'voice match response contains unsupported compression',
        ),
      };
      if (match.compression != expectedCompression) {
        throw const FormatException(
          'voice match compression label/code mismatch',
        );
      }
      matches.add(match);
    }

    final resolution = switch (j['resolution']) {
      'unresolved' => VoiceArchiveLineResolution.unresolved,
      'unique' => VoiceArchiveLineResolution.unique,
      'ambiguous' => VoiceArchiveLineResolution.ambiguous,
      final value => throw FormatException(
        'unknown voice line resolution: $value',
      ),
    };
    final matchCount = _voiceRequiredInt(j, 'match_count');
    final countMatchesResolution = switch (resolution) {
      VoiceArchiveLineResolution.unresolved => matchCount == 0,
      VoiceArchiveLineResolution.unique => matchCount == 1,
      VoiceArchiveLineResolution.ambiguous => matchCount > 1,
    };
    if (matchCount != matches.length || !countMatchesResolution) {
      throw FormatException(
        'voice match count/resolution mismatch: '
        '$matchCount/${matches.length}/$resolution',
      );
    }

    final archiveSha256 = _voiceRequiredString(j, 'archive_sha256');
    if (!RegExp(r'^[0-9a-f]{64}$').hasMatch(archiveSha256)) {
      throw const FormatException(
        'voice match response has an invalid archive SHA-256',
      );
    }
    return VoiceArchiveMatchLineResult(
      archive: archive,
      archiveSize: archiveSize,
      archiveSha256: archiveSha256,
      locId: locId,
      expectedBasename: expectedBasename,
      resolution: resolution,
      matches: List.unmodifiable(matches),
    );
  }
}

class VoiceArchiveEntryInfo {
  const VoiceArchiveEntryInfo({
    required this.index,
    required this.path,
    required this.basename,
    required this.compressedSize,
    required this.uncompressedSize,
    required this.crc32,
    required this.compression,
    required this.compressionCode,
    required this.lastModified,
    required this.unixMode,
    required this.isDirectory,
    required this.isSymlink,
    required this.encrypted,
  });

  final int index;
  final String path;
  final String basename;
  final int compressedSize;
  final int uncompressedSize;
  final int crc32;
  final String compression;
  final int compressionCode;
  final VoiceArchiveEntryTimestamp? lastModified;
  final int? unixMode;
  final bool isDirectory;
  final bool isSymlink;
  final bool encrypted;

  factory VoiceArchiveEntryInfo.fromJson(Map<String, Object?> j) {
    final rawLastModified = j['last_modified'];
    final lastModified = switch (rawLastModified) {
      null => null,
      final Map value => VoiceArchiveEntryTimestamp.fromJson(
        value.cast<String, Object?>(),
      ),
      _ => throw const FormatException(
        'voice archive entry has invalid last_modified metadata',
      ),
    };
    return VoiceArchiveEntryInfo(
      index: _voiceRequiredInt(j, 'index'),
      path: _voiceRequiredString(j, 'path'),
      basename: _voiceRequiredString(j, 'basename'),
      compressedSize: _voiceRequiredInt(j, 'compressed_size'),
      uncompressedSize: _voiceRequiredInt(j, 'uncompressed_size'),
      crc32: _voiceRequiredInt(j, 'crc32', max: 0xffffffff),
      compression: _voiceRequiredString(j, 'compression'),
      compressionCode: _voiceRequiredInt(j, 'compression_code', max: 0xffff),
      lastModified: lastModified,
      unixMode: _voiceOptionalInt(j, 'unix_mode', max: 0xffffffff),
      isDirectory: _voiceRequiredBool(j, 'is_directory'),
      isSymlink: _voiceRequiredBool(j, 'is_symlink'),
      encrypted: _voiceRequiredBool(j, 'encrypted'),
    );
  }
}

class VoiceArchiveEntryTimestamp {
  const VoiceArchiveEntryTimestamp({
    required this.year,
    required this.month,
    required this.day,
    required this.hour,
    required this.minute,
    required this.second,
  });

  final int year;
  final int month;
  final int day;
  final int hour;
  final int minute;
  final int second;

  factory VoiceArchiveEntryTimestamp.fromJson(Map<String, Object?> j) {
    final year = _voiceRequiredInt(j, 'year', min: 1980, max: 2107);
    final month = _voiceRequiredInt(j, 'month', min: 1, max: 12);
    final day = _voiceRequiredInt(j, 'day', min: 1, max: 31);
    final hour = _voiceRequiredInt(j, 'hour', max: 23);
    final minute = _voiceRequiredInt(j, 'minute', max: 59);
    final second = _voiceRequiredInt(j, 'second', max: 59);
    final timestamp = DateTime.utc(year, month, day, hour, minute, second);
    if (timestamp.year != year ||
        timestamp.month != month ||
        timestamp.day != day ||
        timestamp.hour != hour ||
        timestamp.minute != minute ||
        timestamp.second != second) {
      throw const FormatException(
        'voice archive entry has an invalid calendar timestamp',
      );
    }
    return VoiceArchiveEntryTimestamp(
      year: year,
      month: month,
      day: day,
      hour: hour,
      minute: minute,
      second: second,
    );
  }
}

class ScriptModuleInfo {
  ScriptModuleInfo({required this.name, required this.file});
  final String name;
  final String file;
  factory ScriptModuleInfo.fromJson(Map<String, Object?> j) => ScriptModuleInfo(
    name: j['name'] as String,
    file: (j['file'] as String?) ?? '',
  );
}
