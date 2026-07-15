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
part '../project/revision3_dataasset_package_index.dart';
part '../project/revision3_installed_dataasset_inspection.dart';
part '../project/revision3_npc_draft.dart';
part '../project/revision3_npc_source_inspection.dart';
part '../project/revision3_quest_context.dart';
part '../project/revision3_quest_outline.dart';
part '../project/revision3_quest_outline_v2.dart';
part '../project/revision3_quest_source_inspection.dart';
part '../project/revision3_quest_transitions.dart';
part '../project/revision3_voice_build.dart';
part '../project/revision3_voice_take.dart';
part '../project/revision3_voice_take_selection.dart';
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
const _authoringRevision3QuestGeneratorVersion = 2;
const _authoringRevision3MultiObjectiveQuestGeneratorVersion = 3;
const _authoringRevision3SemanticQuestGeneratorVersion = 4;
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
    'gore-story-build.revision3-quest-v2.input-fingerprint\u0000';
const _maxAuthoringStoryMutationJsonBytes = 20 * 1024 * 1024;
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
const _maxAuthoringStoryInventoryJsonBytes = 24 * 1024 * 1024;
const _maxAuthoringStoryInventoryEntries = 100000;
const _maxAuthoringStoryInventoryEntryBytes = 512;
const _maxAuthoringStoryInventoryTotalBytes = 16 * 1024 * 1024;
const _maxAuthoringStoryBuildPlanJsonBytes = 32 * 1024 * 1024;
const _maxAuthoringStoryBuildModules = 4096;
const _maxAuthoringStoryBuildSourceBytes = 16 * 1024 * 1024;
const _maxAuthoringStoryBuildDiagnostics = 65536;
const _maxAuthoringStoryBuildRelatedPerDiagnostic = 1024;
const _maxAuthoringStoryBuildRelatedTotal = 65536;
const _maxAuthoringStoryBuildPropertyPathBytes = 2 * 1024;
const _maxAuthoringStoryBuildDiagnosticMessageBytes = 16 * 1024;
const _maxAuthoringStoryBuildSealedInputsPerModule = 16;
const _maxAuthoringStoryBuildSealedInputsTotal =
    _maxAuthoringStoryBuildModules *
    _maxAuthoringStoryBuildSealedInputsPerModule;
// This one FFI command deliberately stays within signed 64-bit JSON integers. A base at this
// maximum can advance once to `_maxAuthoringStoryAppliedRevision` without becoming a double.
const _maxAuthoringStoryBaseRevision = 0x7ffffffffffffffe;
const _maxAuthoringSignedJsonInteger = 0x7fffffffffffffff;
const _maxAuthoringStoryAppliedRevision = _maxAuthoringSignedJsonInteger;
const _maxAuthoringRevision3QuestBasisRevision = _maxAuthoringStoryBaseRevision;
const _maxAuthoringNpcDraftInputBytes = 16 * 1024;
const _maxAuthoringQuestDraftInputBytes = 20 * 1024 * 1024;
const _maxAuthoringDraftSourceBytes = 1024 * 1024;
const _maxAuthoringDraftMetadataBytes = 4 * 1024;
const _maxAuthoringLogicalNameBytes = 1024;
const _maxAuthoringReferencedAssetBytes = 64 * 1024 * 1024;
const _maxAuthoringDiagnostics = 262144;
const _maxAuthoringDiagnosticMessageBytes = 4096;
const _maxAuthoringDiagnosticPathBytes = 4096;
const _maxAuthoringRelatedEntities = 100000;
final _nativeErrorCodePattern = RegExp(r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$');

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

  /// Parse, canonicalize, and validate one format-2 authoring snapshot without retaining state.
  Future<AuthoringProjectCheckResult> authoringProjectCheck({
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    final r = await _call('authoring_project_check', {
      'project_json': projectJson,
      'profile': profile.wireName,
    });
    return AuthoringProjectCheckResult.fromJson(r);
  }

  /// Atomically evaluate one Story Draft insert against exact canonical revision-2 project bytes.
  Future<AuthoringStoryDraftInsertResult> authoringProjectStoryDraftInsertV1({
    required String projectJson,
    required String mutationJson,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringDraftRequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringDraftRequestString(
      mutationJson,
      'mutationJson',
      _maxAuthoringStoryMutationJsonBytes,
    );
    const command = 'authoring_project_story_draft_insert_v1';
    _authoringStoryMutationEnvelopePreflight(
      command,
      projectJson,
      mutationJson,
      profile.wireName,
    );
    final response = await _call(command, {
      'project_json': projectJson,
      'mutation_json': mutationJson,
      'profile': profile.wireName,
    });
    return AuthoringStoryDraftInsertResult.fromJson(
      response,
      projectJson: projectJson,
      mutationJson: mutationJson,
      profile: profile,
    );
  }

  /// Derive a sealed, runtime-unqualified and permanently build-blocked source plan.
  Future<AuthoringStoryBuildPlanResult> authoringStoryBuildPlanV1Generate({
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringDraftRequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    const command = 'authoring_story_build_plan_v1_generate';
    _authoringStoryBuildPlanEnvelopePreflight(
      command,
      projectJson,
      profile.wireName,
    );
    final response = await _call(command, {
      'project_json': projectJson,
      'profile': profile.wireName,
    });
    return AuthoringStoryBuildPlanResult._fromJson(
      response,
      projectJson: projectJson,
      profile: profile,
    );
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

  /// Build a sealed base-game-only collision inventory without writing or launching the game.
  Future<AuthoringStoryInventoryBuildResult> authoringStoryInventoryV1Build({
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) async {
    _authoringStoryCatalogPath(executable, 'executable');
    _authoringStoryCatalogPath(shippingCache, 'shippingCache');
    _authoringStoryCatalogPath(bindsCache, 'bindsCache');
    const command = 'authoring_story_inventory_v1_build';
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
    return AuthoringStoryInventoryBuildResult._fromJson(
      response,
      executable: executable,
      shippingCache: shippingCache,
      bindsCache: bindsCache,
    );
  }

  /// Validate and preview one bounded logical-NPC clone entirely in memory.
  Future<AuthoringLogicalNpcCloneDraftResult>
  authoringLogicalNpcCloneDraftV1Generate({required String inputJson}) async {
    _authoringDraftRequestString(
      inputJson,
      'inputJson',
      _maxAuthoringNpcDraftInputBytes,
    );
    const command = 'authoring_logical_npc_clone_draft_v1_generate';
    _authoringDraftEnvelopePreflight(command, inputJson);
    final response = await _call(command, {'input_json': inputJson});
    return AuthoringLogicalNpcCloneDraftResult.fromJson(response);
  }

  /// Validate and preview one bounded discovery-shaped quest entirely in memory.
  Future<AuthoringDraftQuestSkeletonResult>
  authoringDraftQuestSkeletonV1Generate({required String inputJson}) async {
    _authoringDraftRequestString(
      inputJson,
      'inputJson',
      _maxAuthoringQuestDraftInputBytes,
    );
    const command = 'authoring_draft_quest_skeleton_v1_generate';
    _authoringDraftEnvelopePreflight(command, inputJson);
    final response = await _call(command, {'input_json': inputJson});
    return AuthoringDraftQuestSkeletonResult.fromJson(response);
  }

  /// Open the fixed canonical head and reconstruct one immutable format-2 checkpoint.
  Future<AuthoringStoreOpenedResult> authoringStoreOpen({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open', {
      'root': root,
      'verification': verification.wireName,
      'profile': profile.wireName,
    });
    return AuthoringStoreOpenedResult.fromJson(response);
  }

  /// Open the fixed canonical head and dispatch one closed schema-revision-1/2 document.
  Future<AuthoringStoreOpenedResult> authoringStoreOpenDocument({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open_document', {
      'root': root,
      'verification': verification.wireName,
      'profile': profile.wireName,
    });
    return AuthoringStoreOpenedResult.fromJson(response);
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

  /// Prepare immutable objects without publishing `gore-project.json`.
  ///
  /// `expectedHead == null` is a strict CAS assertion that the fixed head is absent. The raw
  /// project string is passed through unchanged so Rust can reject duplicate JSON keys.
  Future<AuthoringCheckpointPreparation> authoringStorePrepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final response = await _call('authoring_store_prepare_checkpoint', {
      'root': root,
      'expected_head_json': expectedHead?.canonicalJson,
      'project_json': projectJson,
      'profile': profile.wireName,
    });
    return AuthoringCheckpointPreparation.fromJson(response);
  }

  /// Prepare immutable objects for a closed schema-revision-1/2 document without publishing it.
  ///
  /// The raw document string and exact optional head CAS token are preserved byte-for-byte.
  Future<AuthoringCheckpointPreparation>
  authoringStorePrepareDocumentCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final response =
        await _call('authoring_store_prepare_document_checkpoint', {
          'root': root,
          'expected_head_json': expectedHead?.canonicalJson,
          'project_json': projectJson,
          'profile': profile.wireName,
        });
    return AuthoringCheckpointPreparation.fromJson(response);
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

  /// Prepare a count-preserving outline edit for one exact-current managed
  /// revision-3 Quest and its already-owned generated ScriptModule.
  ///
  /// Native code derives private collision context from the Store. No game
  /// root, catalog selector, source text, build, runtime, deployment, or
  /// publication authority crosses this boundary.
  Future<AuthoringRevision3QuestOutlineEditPreparation>
  authoringStorePrepareRevision3QuestOutlineEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV1 request,
  }) async {
    const command = 'authoring_store_prepare_revision3_quest_outline_edit_v1';
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
      return AuthoringRevision3QuestOutlineEditPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException._malformed(command: command, reason: error.message);
    }
  }

  /// Prepare a stable-slot-aware outline edit for one exact-current semantic
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

  /// Reopen one candidate head from its exact canonical UTF-8 JSON bytes without publishing it.
  Future<AuthoringStoreOpenedResult> authoringStoreOpenHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open_head_bytes', {
      'root': root,
      'head_json': head.canonicalJson,
      'verification': verification.wireName,
      'profile': profile.wireName,
    });
    return AuthoringStoreOpenedResult.fromJson(response);
  }

  /// Reopen one revision-dispatched candidate from its exact canonical head bytes.
  Future<AuthoringStoreOpenedResult> authoringStoreOpenHeadBytesDocument({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open_head_bytes_document', {
      'root': root,
      'head_json': head.canonicalJson,
      'verification': verification.wireName,
      'profile': profile.wireName,
    });
    return AuthoringStoreOpenedResult.fromJson(response);
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

  /// Stream one bounded Ogg into the content-addressed store under a strict head CAS token.
  Future<AuthoringImportedOgg> authoringStoreImportOgg({
    required String root,
    required String source,
    required String logicalName,
    required AuthoringWorkingHead? expectedHead,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(source, 'source', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(
      logicalName,
      'logicalName',
      _maxAuthoringLogicalNameBytes,
    );
    final response = await _call('authoring_store_import_ogg', {
      'root': root,
      'source': source,
      'logical_name': logicalName,
      'expected_head_json': expectedHead?.canonicalJson,
    });
    return AuthoringImportedOgg.fromJson(response);
  }

  /// Verify that one typed logical asset reference resolves to its sealed immutable object.
  Future<void> authoringStoreVerifyAsset({
    required String root,
    required AuthoringAssetRef asset,
    required AuthoringAssetVerification verification,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_verify_asset', {
      'root': root,
      'asset': asset.toJson(),
      'verification': verification.wireName,
    });
    _authoringExactFields(response, const {'ok'}, 'verify response');
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

  /// Load (or build, if absent/`rebuild`) the texture index. Returns {assetPath: packageIdString}.
  Future<Map<String, String>> textureIndex(
    String game, {
    bool rebuild = false,
  }) async {
    final r = await _call('texture_index', {'game': game, 'rebuild': rebuild});
    final entries = (r['entries'] as Map).cast<String, Object?>();
    return entries.map((k, v) => MapEntry(k, v as String));
  }

  /// Extract a texture to a temp PNG; returns the FFI result map (png_path, width, height, format).
  Future<Map<String, Object?>> textureExtract(
    String game, {
    String? asset,
    String? packageId,
  }) async {
    final payload = <String, Object?>{'game': game};
    if (asset != null) payload['asset'] = asset;
    if (packageId != null) payload['package_id'] = packageId;
    return _call('texture_extract', payload);
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

  /// Compile a staged .as into a 1-module mini-cache via the game; returns {mini_path, module}.
  Future<Map<String, Object?>> scriptCompile({
    required String gameDir,
    required String op,
    required String moduleName,
    required String relPath,
    required String asPath,
    required String workDir,
    bool allowNewSymbols = false,
  }) => _call('script_compile', {
    'game_dir': gameDir,
    'op': op,
    'module_name': moduleName,
    'rel_path': relPath,
    'as_path': asPath,
    'work_dir': workDir,
    'allow_new_symbols': allowNewSymbols,
  });

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

enum AuthoringValidationProfile {
  production('production'),
  experimental('experimental');

  const AuthoringValidationProfile(this.wireName);
  final String wireName;
}

enum AuthoringAssetVerification {
  structural('structural'),
  full('full');

  const AuthoringAssetVerification(this.wireName);
  final String wireName;
}

enum AuthoringDiagnosticSeverity { error, warning, info }

final _authoringDiagnosticCodePattern = RegExp(
  r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$',
);
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

void _authoringStoreRequestString(String value, String field, int maxBytes) {
  if (value.isEmpty || utf8.encode(value).length > maxBytes) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'must be 1..=$maxBytes UTF-8 bytes',
    );
  }
}

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

void _authoringDraftEnvelopePreflight(String command, String inputJson) {
  // Conservative allocation-free upper bound for jsonEncode's command envelope. Counting every
  // control scalar as `\u00XX` may reject a short-escape-heavy input early, but can never let an
  // oversized allocation reach the UI isolate.
  final envelopeBytes =
      '{"command":"","payload":{"input_json":""}}'.length + command.length;
  var encodedBytes = envelopeBytes;
  for (var index = 0; index < inputJson.length; index++) {
    final unit = inputJson.codeUnitAt(index);
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
        '<${inputJson.length} characters>',
        'inputJson',
        'escaped command envelope exceeds the '
            '$goreCoreTransportMaxRequestBytes-byte transport limit',
      );
    }
    encodedBytes += added;
  }
}

void _authoringStoryMutationEnvelopePreflight(
  String command,
  String projectJson,
  String mutationJson,
  String profile,
) {
  var encodedBytes =
      '{"command":"","payload":{"project_json":"","mutation_json":"","profile":""}}'
          .length +
      command.length +
      profile.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    projectJson,
    'projectJson',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(
    mutationJson,
    'mutationJson',
    encodedBytes,
  );
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

void _authoringStoryBuildPlanEnvelopePreflight(
  String command,
  String projectJson,
  String profile,
) {
  var encodedBytes =
      '{"command":"","payload":{"project_json":"","profile":""}}'.length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    projectJson,
    'projectJson',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(profile, 'profile', encodedBytes);
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

const _authoringStoryRequestBindingDomain =
    'gore-authoring.story-draft-insert-v1.request-binding\u0000';

String _authoringStoryRequestBindingSha256(
  String projectJson,
  String mutationJson,
  AuthoringValidationProfile profile,
) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryRequestBindingDomain));
  for (final bytes in <List<int>>[
    utf8.encode(projectJson),
    utf8.encode(mutationJson),
    utf8.encode(profile.wireName),
  ]) {
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
    input
      ..add(length)
      ..add(bytes);
  }
  input.close();
  return output.value.toString();
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

String? _authoringRequiredNullableString(
  Map<String, Object?> json,
  String field, {
  int maxBytes = _maxAuthoringDiagnosticPathBytes,
}) {
  if (!json.containsKey(field)) {
    throw FormatException('authoring response is missing field $field');
  }
  final value = json[field];
  if (value == null) return null;
  if (value is! String ||
      value.isEmpty ||
      utf8.encode(value).length > maxBytes) {
    throw FormatException(
      'authoring response field $field is not a valid nullable string',
    );
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

({
  Map<String, Object?> project,
  String projectId,
  int schemaRevision,
  int revision,
})
_authoringRequireCanonicalProjectJson(String projectJson) {
  final Object? decoded;
  try {
    decoded = jsonDecode(projectJson);
  } on FormatException {
    throw const FormatException('authoring store project JSON is invalid');
  }
  final project = _authoringRequiredObject(decoded, 'store project');
  final fields = project.keys.toList(growable: false);
  if (fields.length != _authoringProjectTopLevelFields.length) {
    throw const FormatException(
      'authoring store project JSON has an invalid top-level schema',
    );
  }
  for (var index = 0; index < fields.length; index++) {
    if (fields[index] != _authoringProjectTopLevelFields[index]) {
      throw const FormatException(
        'authoring store project JSON has non-canonical field order',
      );
    }
  }
  if (project['format'] != 2) {
    throw const FormatException(
      'authoring store project JSON has an unsupported format',
    );
  }
  final schemaRevision = project['schema_revision'];
  if (schemaRevision is! int || (schemaRevision != 1 && schemaRevision != 2)) {
    throw const FormatException(
      'authoring store project JSON has an unsupported schema revision',
    );
  }
  final projectId = _authoringEntityId(
    _authoringRequiredString(project, 'project_id', maxBytes: 32),
    'project_id',
  );
  if (projectId == '00000000000000000000000000000000') {
    throw const FormatException('authoring store project ID must not be zero');
  }
  final revision = _authoringRequiredInt(project, 'revision');
  if (jsonEncode(decoded) != projectJson) {
    throw const FormatException(
      'authoring store project JSON is not canonical',
    );
  }
  return (
    project: project,
    projectId: projectId,
    schemaRevision: schemaRevision,
    revision: revision,
  );
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

Map<String, Object?> _authoringDraftObjectField(
  Map<String, Object?> json,
  String field,
) => _authoringRequiredObject(json[field], 'draft field $field');

String _authoringDraftSha256(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(json, field, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(value)) {
    throw FormatException('authoring draft field $field is not a SHA-256');
  }
  return value;
}

String _authoringDraftVerifiedSourceSha256(
  Map<String, Object?> json,
  String source,
) {
  final declared = _authoringDraftSha256(json, 'source_sha256');
  final actual = crypto.sha256.convert(utf8.encode(source)).toString();
  if (declared != actual) {
    throw const FormatException(
      'authoring draft source and source_sha256 disagree',
    );
  }
  return declared;
}

String _authoringDraftFixedString(
  Map<String, Object?> json,
  String field,
  String expected,
) {
  final value = _authoringRequiredString(
    json,
    field,
    maxBytes: _maxAuthoringDraftMetadataBytes,
  );
  if (value != expected) {
    throw FormatException('authoring draft field $field is not supported');
  }
  return value;
}

final _authoringDraftIdentifierPattern = RegExp(r'^[A-Za-z_][A-Za-z0-9_]*$');
final _authoringDraftTechnicalIdPattern = RegExp(
  r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$',
);
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

String _authoringDraftPascalTechnicalId(String technicalId) => technicalId
    .split('_')
    .map(
      (segment) =>
          '${segment.substring(0, 1)}${segment.substring(1).toLowerCase()}',
    )
    .join();

List<AuthoringDraftDiagnostic> _authoringDraftDiagnostics(
  Map<String, Object?> json, {
  required bool valid,
}) {
  final raw = json['diagnostics'];
  if (raw is! List || raw.length > 1) {
    throw const FormatException(
      'authoring draft diagnostics must be a bounded list',
    );
  }
  final diagnostics = raw
      .map(
        (item) => AuthoringDraftDiagnostic.fromJson(
          _authoringRequiredObject(item, 'draft diagnostic'),
        ),
      )
      .toList(growable: false);
  if ((valid && diagnostics.isNotEmpty) ||
      (!valid && diagnostics.length != 1)) {
    throw const FormatException(
      'authoring draft validity and diagnostics disagree',
    );
  }
  return List.unmodifiable(diagnostics);
}

class AuthoringDraftDiagnostic {
  const AuthoringDraftDiagnostic._({
    required this.code,
    required this.field,
    required this.message,
  });

  static const _codes = <String>{
    'NPC_EMPTY_VALUE',
    'NPC_VALUE_TOO_LONG',
    'NPC_TOO_MANY_MODULE_SEGMENTS',
    'NPC_INVALID_IDENTIFIER_START',
    'NPC_INVALID_IDENTIFIER_CHARACTER',
    'NPC_RESERVED_IDENTIFIER',
    'NPC_RESERVED_MODULE_SEGMENT',
    'NPC_UNEXPECTED_PARENT_CLASS_PREFIX',
    'NPC_CLASS_NAME_COLLISION',
    'QUEST_INVALID_SEAL',
    'QUEST_GENERATION_MISMATCH',
    'QUEST_ZERO_ENTITY_ID',
    'QUEST_EMPTY_VALUE',
    'QUEST_VALUE_TOO_LONG',
    'QUEST_INVALID_CHARACTER',
    'QUEST_RESERVED_IDENTIFIER',
    'QUEST_NON_CANONICAL_IDENTIFIER',
    'QUEST_TOO_MANY_MODULE_SEGMENTS',
    'QUEST_RESERVED_MODULE_SEGMENT',
    'QUEST_INVALID_PARENT_CLASS',
    'QUEST_PARENT_CLASS_COLLISION',
    'QUEST_NON_CANONICAL_TEXT',
    'QUEST_TOO_MANY_COLLISION_ENTRIES',
    'QUEST_COLLISION_CATALOG_TOO_LARGE',
    'QUEST_UNSAFE_COLLISION_ENTRY',
    'QUEST_DUPLICATE_COLLISION_ENTRY',
    'QUEST_GENERATED_NAME_COLLISION',
    'QUEST_GENERATED_SYMBOL_COLLISION',
  };

  final String code;
  final String field;
  final String message;

  factory AuthoringDraftDiagnostic.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'code',
      'field',
      'message',
    }, 'draft diagnostic');
    final code = _authoringRequiredString(json, 'code', maxBytes: 128);
    final field = _authoringRequiredString(
      json,
      'field',
      maxBytes: _maxAuthoringDiagnosticPathBytes,
    );
    final message = _authoringRequiredString(
      json,
      'message',
      maxBytes: _maxAuthoringDiagnosticMessageBytes,
    );
    if (!_codes.contains(code) ||
        !_authoringDiagnosticCodePattern.hasMatch(code)) {
      throw const FormatException('unknown authoring draft diagnostic code');
    }
    return AuthoringDraftDiagnostic._(
      code: code,
      field: field,
      message: message,
    );
  }
}

class AuthoringLogicalNpcCloneDraftResult {
  const AuthoringLogicalNpcCloneDraftResult._({
    required this.valid,
    required this.generated,
    required this.diagnostics,
  });

  final bool valid;
  final AuthoringLogicalNpcCloneGenerated? generated;
  final List<AuthoringDraftDiagnostic> diagnostics;

  factory AuthoringLogicalNpcCloneDraftResult.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'valid',
      'generated',
      'diagnostics',
    }, 'NPC draft response');
    if (json['ok'] != true) {
      throw const FormatException('authoring NPC draft response is not ok');
    }
    final valid = _authoringRequiredBool(json, 'valid');
    final generatedValue = json['generated'];
    final generated = generatedValue == null
        ? null
        : AuthoringLogicalNpcCloneGenerated.fromJson(
            _authoringRequiredObject(generatedValue, 'NPC generated preview'),
          );
    if (valid != (generated != null)) {
      throw const FormatException(
        'authoring NPC draft validity and generated preview disagree',
      );
    }
    return AuthoringLogicalNpcCloneDraftResult._(
      valid: valid,
      generated: generated,
      diagnostics: _authoringDraftDiagnostics(json, valid: valid),
    );
  }
}

class AuthoringLogicalNpcCloneGenerated {
  const AuthoringLogicalNpcCloneGenerated._({
    required this.generatorId,
    required this.generatorVersion,
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.uniqueName,
    required this.classes,
    required this.source,
    required this.sourceSha256,
    required this.inputFingerprint,
    required this.status,
  });

  final String generatorId;
  final int generatorVersion;
  final String moduleNamespace;
  final String moduleRelativePath;
  final String uniqueName;
  final AuthoringLogicalNpcCloneClasses classes;
  final String source;
  final String sourceSha256;
  final String inputFingerprint;
  final AuthoringLogicalNpcCloneStatus status;

  factory AuthoringLogicalNpcCloneGenerated.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'generator_id',
      'generator_version',
      'module_namespace',
      'module_relative_path',
      'unique_name',
      'classes',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    }, 'NPC generated preview');
    final generatorId = _authoringDraftFixedString(
      json,
      'generator_id',
      'gore-authoring.logical-npc-clone-draft',
    );
    final generatorVersion = _authoringRequiredInt(
      json,
      'generator_version',
      min: 1,
      max: 1,
    );
    final moduleNamespace = _authoringRequiredString(
      json,
      'module_namespace',
      maxBytes: 255,
    );
    _authoringDraftValidateModuleNamespace(moduleNamespace);
    final moduleRelativePath = _authoringRequiredString(
      json,
      'module_relative_path',
      maxBytes: 258,
    );
    if (moduleRelativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
      throw const FormatException('NPC generated module path is inconsistent');
    }
    final uniqueName = _authoringRequiredString(
      json,
      'unique_name',
      maxBytes: 64,
    );
    _authoringDraftValidateIdentifier(uniqueName, 'unique_name', maxBytes: 64);
    final classes = AuthoringLogicalNpcCloneClasses.fromJson(
      _authoringDraftObjectField(json, 'classes'),
      uniqueName: uniqueName,
    );
    final status = AuthoringLogicalNpcCloneStatus.fromJson(
      _authoringDraftObjectField(json, 'status'),
    );
    final source = _authoringRequiredString(
      json,
      'source',
      maxBytes: _maxAuthoringDraftSourceBytes,
    );
    return AuthoringLogicalNpcCloneGenerated._(
      generatorId: generatorId,
      generatorVersion: generatorVersion,
      moduleNamespace: moduleNamespace,
      moduleRelativePath: moduleRelativePath,
      uniqueName: uniqueName,
      classes: classes,
      source: source,
      sourceSha256: _authoringDraftVerifiedSourceSha256(json, source),
      inputFingerprint: _authoringDraftSha256(json, 'input_fingerprint'),
      status: status,
    );
  }
}

class AuthoringLogicalNpcCloneClasses {
  const AuthoringLogicalNpcCloneClasses._({
    required this.characterDefinition,
    required this.aiAgentConfig,
    required this.spawnDefinition,
  });

  final String characterDefinition;
  final String aiAgentConfig;
  final String spawnDefinition;

  factory AuthoringLogicalNpcCloneClasses.fromJson(
    Map<String, Object?> json, {
    required String uniqueName,
  }) {
    _authoringExactFields(json, const {
      'character_definition',
      'ai_agent_config',
      'spawn_definition',
    }, 'NPC classes');
    final characterDefinition = _authoringRequiredString(
      json,
      'character_definition',
      maxBytes: 160,
    );
    final aiAgentConfig = _authoringRequiredString(
      json,
      'ai_agent_config',
      maxBytes: 160,
    );
    final spawnDefinition = _authoringRequiredString(
      json,
      'spawn_definition',
      maxBytes: 160,
    );
    if (characterDefinition != 'UCharacterDefinition_Human_$uniqueName' ||
        aiAgentConfig != 'UAIAgentConfig_Human_$uniqueName' ||
        spawnDefinition != 'USpawnAIAgentDefinition_$uniqueName') {
      throw const FormatException('NPC generated classes are inconsistent');
    }
    return AuthoringLogicalNpcCloneClasses._(
      characterDefinition: characterDefinition,
      aiAgentConfig: aiAgentConfig,
      spawnDefinition: spawnDefinition,
    );
  }
}

enum AuthoringDraftAuthoringStatus { offlineDraft }

enum AuthoringDraftRuntimeStatus { runtimeUnqualified }

enum AuthoringDraftQuestTransitionStatus { transitionsRuntimeUnqualified }

class AuthoringLogicalNpcCloneStatus {
  const AuthoringLogicalNpcCloneStatus._({
    required this.authoring,
    required this.runtime,
  });

  final AuthoringDraftAuthoringStatus authoring;
  final AuthoringDraftRuntimeStatus runtime;

  factory AuthoringLogicalNpcCloneStatus.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {'authoring', 'runtime'}, 'NPC status');
    _authoringDraftFixedString(json, 'authoring', 'offline_draft');
    _authoringDraftFixedString(json, 'runtime', 'runtime_unqualified');
    return const AuthoringLogicalNpcCloneStatus._(
      authoring: AuthoringDraftAuthoringStatus.offlineDraft,
      runtime: AuthoringDraftRuntimeStatus.runtimeUnqualified,
    );
  }
}

class AuthoringDraftQuestSkeletonResult {
  const AuthoringDraftQuestSkeletonResult._({
    required this.valid,
    required this.generated,
    required this.diagnostics,
  });

  final bool valid;
  final AuthoringDraftQuestGenerated? generated;
  final List<AuthoringDraftDiagnostic> diagnostics;

  factory AuthoringDraftQuestSkeletonResult.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'valid',
      'generated',
      'diagnostics',
    }, 'quest draft response');
    if (json['ok'] != true) {
      throw const FormatException('authoring quest draft response is not ok');
    }
    final valid = _authoringRequiredBool(json, 'valid');
    final generatedValue = json['generated'];
    final generated = generatedValue == null
        ? null
        : AuthoringDraftQuestGenerated.fromJson(
            _authoringRequiredObject(generatedValue, 'quest generated preview'),
          );
    if (valid != (generated != null)) {
      throw const FormatException(
        'authoring quest draft validity and generated preview disagree',
      );
    }
    return AuthoringDraftQuestSkeletonResult._(
      valid: valid,
      generated: generated,
      diagnostics: _authoringDraftDiagnostics(json, valid: valid),
    );
  }
}

class AuthoringDraftQuestGenerated {
  const AuthoringDraftQuestGenerated._({
    required this.target,
    required this.questId,
    required this.generatorId,
    required this.generatorVersion,
    required this.giver,
    required this.parentQuest,
    required this.collisionCatalog,
    required this.technicalNames,
    required this.fixedShape,
    required this.source,
    required this.sourceSha256,
    required this.inputFingerprint,
    required this.status,
  });

  final AuthoringDraftGameGeneration target;
  final String questId;
  final String generatorId;
  final int generatorVersion;
  final AuthoringDraftQuestGiver giver;
  final AuthoringDraftParentQuest parentQuest;
  final AuthoringDraftCatalogAnchor collisionCatalog;
  final AuthoringDraftQuestTechnicalNames technicalNames;
  final AuthoringDraftQuestFixedShape fixedShape;
  final String source;
  final String sourceSha256;
  final String inputFingerprint;
  final AuthoringDraftQuestStatus status;

  factory AuthoringDraftQuestGenerated.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'target',
      'quest_id',
      'generator_id',
      'generator_version',
      'giver',
      'parent_quest',
      'collision_catalog',
      'technical_names',
      'fixed_shape',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    }, 'quest generated preview');
    final questId = _authoringRequiredString(json, 'quest_id', maxBytes: 32);
    _authoringEntityId(questId, 'quest_id');
    if (questId == '00000000000000000000000000000000') {
      throw const FormatException('authoring quest ID must not be zero');
    }
    final status = AuthoringDraftQuestStatus.fromJson(
      _authoringDraftObjectField(json, 'status'),
    );
    final target = AuthoringDraftGameGeneration.fromJson(
      _authoringDraftObjectField(json, 'target'),
    );
    final giver = AuthoringDraftQuestGiver.fromJson(
      _authoringDraftObjectField(json, 'giver'),
    );
    final parentQuest = AuthoringDraftParentQuest.fromJson(
      _authoringDraftObjectField(json, 'parent_quest'),
    );
    final collisionCatalog = AuthoringDraftCatalogAnchor.fromJson(
      _authoringDraftObjectField(json, 'collision_catalog'),
    );
    if (!target.sameGeneration(giver.generation) ||
        !target.sameGeneration(parentQuest.generation) ||
        !target.sameGeneration(collisionCatalog.generation)) {
      throw const FormatException(
        'quest generated provenance has inconsistent game generations',
      );
    }
    final technicalNames = AuthoringDraftQuestTechnicalNames.fromJson(
      _authoringDraftObjectField(json, 'technical_names'),
    );
    if (parentQuest.runtimeClass.toLowerCase() ==
            technicalNames.rootClass.toLowerCase() ||
        parentQuest.runtimeClass.toLowerCase() ==
            technicalNames.objectiveClass.toLowerCase()) {
      throw const FormatException(
        'quest generated class collides with its parent class',
      );
    }
    final source = _authoringRequiredString(
      json,
      'source',
      maxBytes: _maxAuthoringDraftSourceBytes,
    );
    return AuthoringDraftQuestGenerated._(
      target: target,
      questId: questId,
      generatorId: _authoringDraftFixedString(
        json,
        'generator_id',
        'gore-authoring.draft-quest-skeleton',
      ),
      generatorVersion: _authoringRequiredInt(
        json,
        'generator_version',
        min: 1,
        max: 1,
      ),
      giver: giver,
      parentQuest: parentQuest,
      collisionCatalog: collisionCatalog,
      technicalNames: technicalNames,
      fixedShape: AuthoringDraftQuestFixedShape.fromJson(
        _authoringDraftObjectField(json, 'fixed_shape'),
      ),
      source: source,
      sourceSha256: _authoringDraftVerifiedSourceSha256(json, source),
      inputFingerprint: _authoringDraftSha256(json, 'input_fingerprint'),
      status: status,
    );
  }
}

class AuthoringDraftQuestStatus {
  const AuthoringDraftQuestStatus._({
    required this.authoring,
    required this.discovery,
    required this.transitions,
  });

  final AuthoringDraftAuthoringStatus authoring;
  final AuthoringDraftRuntimeStatus discovery;
  final AuthoringDraftQuestTransitionStatus transitions;

  factory AuthoringDraftQuestStatus.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'authoring',
      'discovery',
      'transitions',
    }, 'quest status');
    _authoringDraftFixedString(json, 'authoring', 'offline_draft');
    _authoringDraftFixedString(json, 'discovery', 'runtime_unqualified');
    _authoringDraftFixedString(
      json,
      'transitions',
      'transitions_runtime_unqualified',
    );
    return const AuthoringDraftQuestStatus._(
      authoring: AuthoringDraftAuthoringStatus.offlineDraft,
      discovery: AuthoringDraftRuntimeStatus.runtimeUnqualified,
      transitions:
          AuthoringDraftQuestTransitionStatus.transitionsRuntimeUnqualified,
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

class AuthoringDraftGameGeneration {
  const AuthoringDraftGameGeneration._({required this.executable});
  final AuthoringDraftContentSeal executable;

  bool sameGeneration(AuthoringDraftGameGeneration other) =>
      executable.byteLength == other.executable.byteLength &&
      executable.sha256 == other.executable.sha256;

  factory AuthoringDraftGameGeneration.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {'executable'}, 'draft game generation');
    return AuthoringDraftGameGeneration._(
      executable: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'executable'),
      ),
    );
  }
}

class AuthoringDraftQuestGiver {
  const AuthoringDraftQuestGiver._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
    required this.canonicalSelector,
    required this.runtimeUniqueName,
  });
  final AuthoringDraftGameGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;
  final String canonicalSelector;
  final String runtimeUniqueName;

  factory AuthoringDraftQuestGiver.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'generation',
      'source_seal',
      'catalog_layer',
      'canonical_selector',
      'runtime_unique_name',
    }, 'quest giver');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    final canonicalSelector = _authoringRequiredString(
      json,
      'canonical_selector',
      maxBytes: 96,
    );
    final runtimeUniqueName = _authoringRequiredString(
      json,
      'runtime_unique_name',
      maxBytes: 96,
    );
    _authoringDraftValidateCatalogLayer(catalogLayer, 'giver.catalog_layer');
    _authoringDraftValidateIdentifier(
      canonicalSelector,
      'giver.canonical_selector',
    );
    _authoringDraftValidateIdentifier(
      runtimeUniqueName,
      'giver.runtime_unique_name',
    );
    return AuthoringDraftQuestGiver._(
      generation: AuthoringDraftGameGeneration.fromJson(
        _authoringDraftObjectField(json, 'generation'),
      ),
      sourceSeal: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'source_seal'),
      ),
      catalogLayer: catalogLayer,
      canonicalSelector: canonicalSelector,
      runtimeUniqueName: runtimeUniqueName,
    );
  }
}

class AuthoringDraftParentQuest {
  const AuthoringDraftParentQuest._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
    required this.canonicalSelector,
    required this.runtimeClass,
  });
  final AuthoringDraftGameGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;
  final String canonicalSelector;
  final String runtimeClass;

  factory AuthoringDraftParentQuest.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'generation',
      'source_seal',
      'catalog_layer',
      'canonical_selector',
      'runtime_class',
    }, 'parent quest');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    final canonicalSelector = _authoringRequiredString(
      json,
      'canonical_selector',
      maxBytes: 96,
    );
    final runtimeClass = _authoringRequiredString(
      json,
      'runtime_class',
      maxBytes: 96,
    );
    _authoringDraftValidateCatalogLayer(
      catalogLayer,
      'parent_quest.catalog_layer',
    );
    _authoringDraftValidateIdentifier(
      canonicalSelector,
      'parent_quest.canonical_selector',
    );
    _authoringDraftValidateIdentifier(
      runtimeClass,
      'parent_quest.runtime_class',
    );
    if (!runtimeClass.startsWith('UQuest_')) {
      throw const FormatException(
        'authoring draft parent runtime class has an invalid prefix',
      );
    }
    return AuthoringDraftParentQuest._(
      generation: AuthoringDraftGameGeneration.fromJson(
        _authoringDraftObjectField(json, 'generation'),
      ),
      sourceSeal: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'source_seal'),
      ),
      catalogLayer: catalogLayer,
      canonicalSelector: canonicalSelector,
      runtimeClass: runtimeClass,
    );
  }
}

class AuthoringDraftCatalogAnchor {
  const AuthoringDraftCatalogAnchor._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
  });
  final AuthoringDraftGameGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;

  factory AuthoringDraftCatalogAnchor.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'generation',
      'source_seal',
      'catalog_layer',
    }, 'collision catalog anchor');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    _authoringDraftValidateCatalogLayer(
      catalogLayer,
      'collision_catalog.catalog_layer',
    );
    return AuthoringDraftCatalogAnchor._(
      generation: AuthoringDraftGameGeneration.fromJson(
        _authoringDraftObjectField(json, 'generation'),
      ),
      sourceSeal: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'source_seal'),
      ),
      catalogLayer: catalogLayer,
    );
  }
}

class AuthoringDraftQuestTechnicalNames {
  const AuthoringDraftQuestTechnicalNames._({
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.rootClass,
    required this.objectiveClass,
    required this.textHelper,
    required this.rootGetter,
    required this.objectiveGetter,
  });
  final String moduleNamespace;
  final String moduleRelativePath;
  final String rootClass;
  final String objectiveClass;
  final String textHelper;
  final String rootGetter;
  final String objectiveGetter;

  factory AuthoringDraftQuestTechnicalNames.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'module_namespace',
      'module_relative_path',
      'root_class',
      'objective_class',
      'text_helper',
      'root_getter',
      'objective_getter',
    }, 'quest technical names');
    final moduleNamespace = _authoringRequiredString(
      json,
      'module_namespace',
      maxBytes: 255,
    );
    _authoringDraftValidateModuleNamespace(moduleNamespace);
    final moduleRelativePath = _authoringRequiredString(
      json,
      'module_relative_path',
      maxBytes: 258,
    );
    if (moduleRelativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
      throw const FormatException(
        'quest generated module path is inconsistent',
      );
    }
    final rootClass = _authoringRequiredString(
      json,
      'root_class',
      maxBytes: 96,
    );
    final objectiveClass = _authoringRequiredString(
      json,
      'objective_class',
      maxBytes: 96,
    );
    final textHelper = _authoringRequiredString(
      json,
      'text_helper',
      maxBytes: 96,
    );
    final rootGetter = _authoringRequiredString(
      json,
      'root_getter',
      maxBytes: 96,
    );
    final objectiveGetter = _authoringRequiredString(
      json,
      'objective_getter',
      maxBytes: 96,
    );
    for (final entry in <String, String>{
      'root_class': rootClass,
      'objective_class': objectiveClass,
      'text_helper': textHelper,
      'root_getter': rootGetter,
      'objective_getter': objectiveGetter,
    }.entries) {
      _authoringDraftValidateIdentifier(entry.value, entry.key);
    }
    if (!rootClass.startsWith('UQuest_')) {
      throw const FormatException('quest root class has an invalid prefix');
    }
    final technicalId = rootClass.substring('UQuest_'.length);
    if (!_authoringDraftTechnicalIdPattern.hasMatch(technicalId)) {
      throw const FormatException('quest technical ID is not canonical');
    }
    final pascal = _authoringDraftPascalTechnicalId(technicalId);
    if (objectiveClass != '${rootClass}_OBJ_DONE' ||
        rootGetter != 'Get$pascal' ||
        objectiveGetter != 'Get${pascal}Objective') {
      throw const FormatException('quest generated technical names disagree');
    }
    final foldedSymbols = <String>{
      rootClass.toLowerCase(),
      objectiveClass.toLowerCase(),
      textHelper.toLowerCase(),
      rootGetter.toLowerCase(),
      objectiveGetter.toLowerCase(),
    };
    if (foldedSymbols.length != 5) {
      throw const FormatException('quest generated symbols collide');
    }
    return AuthoringDraftQuestTechnicalNames._(
      moduleNamespace: moduleNamespace,
      moduleRelativePath: moduleRelativePath,
      rootClass: rootClass,
      objectiveClass: objectiveClass,
      textHelper: textHelper,
      rootGetter: rootGetter,
      objectiveGetter: objectiveGetter,
    );
  }
}

class AuthoringDraftQuestFixedShape {
  const AuthoringDraftQuestFixedShape._({
    required this.questBaseClass,
    required this.rootKind,
    required this.objectiveKind,
    required this.rootExternalStart,
    required this.objectiveExternalStart,
    required this.objectiveExternalSuccess,
    required this.objectiveSucceedsParent,
  });
  final String questBaseClass;
  final String rootKind;
  final String objectiveKind;
  final bool rootExternalStart;
  final bool objectiveExternalStart;
  final bool objectiveExternalSuccess;
  final bool objectiveSucceedsParent;

  factory AuthoringDraftQuestFixedShape.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'quest_base_class',
      'root_kind',
      'objective_kind',
      'root_external_start',
      'objective_external_start',
      'objective_external_success',
      'objective_succeeds_parent',
    }, 'quest fixed shape');
    final questBaseClass = _authoringDraftFixedString(
      json,
      'quest_base_class',
      'UG1RQuest',
    );
    final rootKind = _authoringDraftFixedString(
      json,
      'root_kind',
      'EQuestKind::Side',
    );
    final objectiveKind = _authoringDraftFixedString(
      json,
      'objective_kind',
      'EQuestKind::Subobjective',
    );
    final rootExternalStart = _authoringRequiredBool(
      json,
      'root_external_start',
    );
    final objectiveExternalStart = _authoringRequiredBool(
      json,
      'objective_external_start',
    );
    final objectiveExternalSuccess = _authoringRequiredBool(
      json,
      'objective_external_success',
    );
    final objectiveSucceedsParent = _authoringRequiredBool(
      json,
      'objective_succeeds_parent',
    );
    if (!rootExternalStart ||
        !objectiveExternalStart ||
        !objectiveExternalSuccess ||
        !objectiveSucceedsParent) {
      throw const FormatException('quest fixed shape is not supported');
    }
    return AuthoringDraftQuestFixedShape._(
      questBaseClass: questBaseClass,
      rootKind: rootKind,
      objectiveKind: objectiveKind,
      rootExternalStart: rootExternalStart,
      objectiveExternalStart: objectiveExternalStart,
      objectiveExternalSuccess: objectiveExternalSuccess,
      objectiveSucceedsParent: objectiveSucceedsParent,
    );
  }
}

class AuthoringProjectCheckResult {
  const AuthoringProjectCheckResult._({
    required this.canonicalProjectJson,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final String canonicalProjectJson;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringProjectCheckResult.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'canonical_project_json',
      'diagnostics',
      'blocks_build',
    }, 'project-check response');
    if (json['ok'] != true) {
      throw const FormatException('authoring project-check response is not ok');
    }
    final canonicalProjectJson = _authoringRequiredString(
      json,
      'canonical_project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringProjectCheckResult._(
      canonicalProjectJson: canonicalProjectJson,
      diagnostics: List.unmodifiable(diagnostics),
      blocksBuild: blocksBuild,
    );
  }
}

enum AuthoringStoryBuildRuntimeQualification { runtimeUnqualified }

enum AuthoringStoryBuildPublicationStatus { notSupported }

final class AuthoringStoryBuildProjectProvenance {
  const AuthoringStoryBuildProjectProvenance._({
    required this.projectId,
    required this.projectRevision,
    required this.canonicalDocument,
    required this.targetExecutable,
  });

  final String projectId;
  final int projectRevision;
  final AuthoringDraftContentSeal canonicalDocument;
  final AuthoringDraftContentSeal targetExecutable;
}

/// Read-only inspection plan. This type intentionally exposes no compile/deploy operation.
final class AuthoringStoryBuildPlanResult {
  const AuthoringStoryBuildPlanResult._({
    required this.requestBindingSha256,
    required this.planJson,
    required this.planSeal,
    required this.validationProfile,
    required this.project,
    required this.runtimeQualification,
    required this.publicationStatus,
    required this.moduleCount,
    required this.diagnosticCount,
    required this.blockingDiagnosticIndexes,
    required this.blocksBuild,
  });

  final String requestBindingSha256;
  final String planJson;
  final AuthoringDraftContentSeal planSeal;
  final AuthoringValidationProfile validationProfile;
  final AuthoringStoryBuildProjectProvenance project;
  final AuthoringStoryBuildRuntimeQualification runtimeQualification;
  final AuthoringStoryBuildPublicationStatus publicationStatus;
  final int moduleCount;
  final int diagnosticCount;
  final List<int> blockingDiagnosticIndexes;
  final bool blocksBuild;

  factory AuthoringStoryBuildPlanResult._fromJson(
    Map<String, Object?> json, {
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_binding_sha256',
      'plan_json',
      'plan_seal',
      'validation_profile',
      'project',
      'runtime_qualification',
      'publication_status',
      'module_count',
      'diagnostic_count',
      'blocking_diagnostic_indexes',
      'blocks_build',
    }, 'Story build-plan response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring Story build-plan response is not ok',
      );
    }
    final requestBindingSha256 = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    final expectedBinding = _authoringStoryBuildRequestBinding(
      projectJson,
      profile.wireName,
    );
    if (!_authoringSha256Pattern.hasMatch(requestBindingSha256) ||
        requestBindingSha256 != expectedBinding) {
      throw const FormatException(
        'authoring Story build-plan response is not bound to its exact request',
      );
    }

    final rawProject = _authoringDecodeDuplicateSafeObject(
      projectJson,
      'Story build source project',
    );
    if (jsonEncode(rawProject) != projectJson) {
      throw const FormatException(
        'authoring Story build source project is not canonical JSON',
      );
    }
    _authoringExactFields(rawProject, const {
      'format',
      'schema_revision',
      'project_id',
      'revision',
      'meta',
      'target',
      'authoring_locales',
      'entities',
      'asset_store',
    }, 'Story build source project');
    _authoringRequiredInt(rawProject, 'format', min: 2, max: 2);
    _authoringRequiredInt(rawProject, 'schema_revision', min: 2, max: 2);
    final projectId = _authoringStoryBuildId(
      rawProject['project_id'],
      'project_id',
    );
    final projectRevision = _authoringRequiredInt(
      rawProject,
      'revision',
      max: _maxAuthoringStoryAppliedRevision,
    );
    final rawTarget = _authoringRequiredObject(
      rawProject['target'],
      'Story build source target',
    );
    _authoringExactFields(rawTarget, const {
      'executable',
    }, 'Story build source target');
    final targetExecutable = _authoringStoryBuildSeal(
      rawTarget['executable'],
      'source target executable',
    );
    final projectEntities = _authoringRequiredObject(
      rawProject['entities'],
      'Story build source entities',
    );
    final canonicalDocument = _authoringStoryBuildBytesSeal(projectJson);

    final planJson = _authoringRequiredString(
      json,
      'plan_json',
      maxBytes: _maxAuthoringStoryBuildPlanJsonBytes,
    );
    final rawPlan = _authoringDecodeDuplicateSafeObject(
      planJson,
      'Story build plan',
    );
    _authoringExactFields(rawPlan, const {
      'format',
      'schema_revision',
      'validation_profile',
      'project',
      'publication_status',
      'modules',
      'diagnostics',
      'blocks_build',
    }, 'Story build plan');
    if (rawPlan['format'] != 'story_build_plan') {
      throw const FormatException(
        'authoring Story build-plan format is unsupported',
      );
    }
    _authoringRequiredInt(rawPlan, 'schema_revision', min: 1, max: 1);
    if (rawPlan['validation_profile'] != profile.wireName ||
        json['validation_profile'] != profile.wireName) {
      throw const FormatException(
        'authoring Story build-plan validation profile is inconsistent',
      );
    }
    if (rawPlan['publication_status'] != 'not_supported' ||
        json['publication_status'] != 'not_supported' ||
        rawPlan['blocks_build'] != true ||
        json['blocks_build'] != true ||
        json['runtime_qualification'] != 'runtime_unqualified') {
      throw const FormatException(
        'authoring Story build-plan response overstates its capabilities',
      );
    }

    final rawPlanProject = _authoringRequiredObject(
      rawPlan['project'],
      'Story build plan project provenance',
    );
    final responseProject = _authoringRequiredObject(
      json['project'],
      'Story build response project provenance',
    );
    final planProject = _authoringStoryBuildProject(
      rawPlanProject,
      projectId: projectId,
      projectRevision: projectRevision,
      canonicalDocument: canonicalDocument,
      targetExecutable: targetExecutable,
    );
    final outerProject = _authoringStoryBuildProject(
      responseProject,
      projectId: projectId,
      projectRevision: projectRevision,
      canonicalDocument: canonicalDocument,
      targetExecutable: targetExecutable,
    );
    if (!_authoringStoryBuildSameProject(planProject, outerProject)) {
      throw const FormatException(
        'authoring Story build-plan project provenance is inconsistent',
      );
    }

    final validation = _AuthoringStoryBuildPlanValidator(
      projectId: projectId,
      projectRevision: projectRevision,
      projectEntities: projectEntities,
      targetExecutable: targetExecutable,
    ).validate(rawPlan['modules'], rawPlan['diagnostics']);
    final moduleCount = _authoringRequiredInt(
      json,
      'module_count',
      max: _maxAuthoringStoryBuildModules,
    );
    final diagnosticCount = _authoringRequiredInt(
      json,
      'diagnostic_count',
      max: _maxAuthoringStoryBuildDiagnostics,
    );
    if (moduleCount != validation.moduleCount ||
        diagnosticCount != validation.diagnosticCount) {
      throw const FormatException(
        'authoring Story build-plan counts disagree with the canonical plan',
      );
    }
    final blockerIndexes = _authoringStoryBuildIndexes(
      json['blocking_diagnostic_indexes'],
      diagnosticCount,
    );
    if (!_authoringIntListsEqual(
      blockerIndexes,
      validation.blockingDiagnosticIndexes,
    )) {
      throw const FormatException(
        'authoring Story build-plan blocker indexes are inconsistent',
      );
    }

    if (jsonEncode(rawPlan) != planJson) {
      throw const FormatException(
        'authoring Story build plan is not canonical JSON',
      );
    }
    final planSeal = _authoringStoryBuildSeal(json['plan_seal'], 'plan seal');
    if (!_authoringStoryCatalogSameSeal(
      planSeal,
      _authoringStoryBuildBytesSeal(planJson),
    )) {
      throw const FormatException('authoring Story build-plan seal is invalid');
    }

    return AuthoringStoryBuildPlanResult._(
      requestBindingSha256: requestBindingSha256,
      planJson: planJson,
      planSeal: planSeal,
      validationProfile: profile,
      project: planProject,
      runtimeQualification:
          AuthoringStoryBuildRuntimeQualification.runtimeUnqualified,
      publicationStatus: AuthoringStoryBuildPublicationStatus.notSupported,
      moduleCount: moduleCount,
      diagnosticCount: diagnosticCount,
      blockingDiagnosticIndexes: List.unmodifiable(blockerIndexes),
      blocksBuild: true,
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

enum AuthoringStoryInventoryCoverage { baseGameOnly }

enum AuthoringStoryInventoryRuntimeQualification { runtimeUnqualified }

enum AuthoringStoryInventoryPublicationStatus { notSupported }

final _authoringStoryCatalogIdPattern = RegExp(r'^[a-z0-9][a-z0-9._:-]*$');
final _authoringStoryCatalogAliasPattern = RegExp(r'^Catalog_[0-9a-f]{64}$');
final _authoringStoryBuildIdPattern = RegExp(r'^[0-9a-f]{32}$');
const _authoringStoryBuildRequestBindingDomain =
    'gore-story-build.authoring-plan-v1.request-binding\u0000';
const _authoringStoryCatalogSelectorDomain =
    'gore-story-catalog.authoring-selector-v1\u0000';
const _authoringStoryCatalogBuildBindingDomain =
    'gore-story-catalog.authoring-build-v1.request-binding\u0000';
const _authoringStoryCatalogGameRootBindingDomain =
    'gore-story-catalog.authoring-build-for-game-root-v1.request-binding\u0000';
const _authoringNpcCatalogGameRootBindingDomain =
    'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000';
const _authoringStoryInventoryBuildBindingDomain =
    'gore-story-inventory.authoring-build-v1.request-binding\u0000';
const _authoringStoryInventoryCatalogLayer =
    'base-game.g1r.scripts.inventory.v1';

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
final class AuthoringStoryInventoryBuildResult {
  const AuthoringStoryInventoryBuildResult._({
    required this.requestBindingSha256,
    required this.inventoryJson,
    required this.generation,
    required this.storyCatalogSeal,
    required this.sourcePairSeal,
    required this.payloadSeal,
    required this.catalogLayer,
    required this.coverage,
    required this.runtimeQualification,
    required this.publicationStatus,
    required this.modules,
    required this.relativePaths,
    required this.symbols,
  });

  final String requestBindingSha256;
  final String inventoryJson;
  final AuthoringStoryCatalogGeneration generation;
  final AuthoringDraftContentSeal storyCatalogSeal;
  final AuthoringDraftContentSeal sourcePairSeal;
  final AuthoringDraftContentSeal payloadSeal;
  final String catalogLayer;
  final AuthoringStoryInventoryCoverage coverage;
  final AuthoringStoryInventoryRuntimeQualification runtimeQualification;
  final AuthoringStoryInventoryPublicationStatus publicationStatus;
  final List<String> modules;
  final List<String> relativePaths;
  final List<String> symbols;

  factory AuthoringStoryInventoryBuildResult._fromJson(
    Map<String, Object?> json, {
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_binding_sha256',
      'inventory_json',
      'generation',
      'story_catalog_seal',
      'source_pair_seal',
      'payload_seal',
      'catalog_layer',
      'coverage',
      'runtime_qualification',
      'publication_status',
    }, 'Story inventory build response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring Story inventory build response is not ok',
      );
    }
    final requestBindingSha256 = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    final expectedBinding = _authoringStoryInventoryBuildBindingSha256(
      executable,
      shippingCache,
      bindsCache,
    );
    if (!_authoringSha256Pattern.hasMatch(requestBindingSha256) ||
        requestBindingSha256 != expectedBinding) {
      throw const FormatException(
        'authoring Story inventory build response is not bound to its exact paths',
      );
    }

    final inventoryJson = _authoringRequiredString(
      json,
      'inventory_json',
      maxBytes: _maxAuthoringStoryInventoryJsonBytes,
    );
    final rawArtifact = _authoringDecodeDuplicateSafeObject(
      inventoryJson,
      'Story inventory build result',
    );
    _authoringExactFields(rawArtifact, const {
      'format',
      'schema_revision',
      'inventory',
      'payload_seal',
    }, 'Story inventory build result');
    if (rawArtifact['format'] != 'story_script_collision_inventory') {
      throw const FormatException(
        'authoring Story inventory build result has an unsupported format',
      );
    }
    _authoringRequiredInt(rawArtifact, 'schema_revision', min: 1, max: 1);
    final rawPayload = _authoringRequiredObject(
      rawArtifact['inventory'],
      'Story inventory payload',
    );
    _authoringExactFields(rawPayload, const {
      'generation',
      'story_catalog_seal',
      'catalog_layer',
      'coverage',
      'runtime_qualification',
      'publication_status',
      'source',
      'modules',
      'relative_paths',
      'symbols',
    }, 'Story inventory payload');

    // Bound every attacker-controlled collision entry before re-encoding or hashing the complete
    // payload. This keeps a near-limit outer JSON from causing a second unbounded materialization
    // before the stricter inventory count/per-entry/aggregate limits have been enforced.
    final entryLimits = _AuthoringStoryInventoryEntryLimits();
    final modules = entryLimits.decode(rawPayload['modules'], 'modules');
    final relativePaths = entryLimits.decode(
      rawPayload['relative_paths'],
      'relative_paths',
    );
    final symbols = entryLimits.decode(rawPayload['symbols'], 'symbols');

    if (jsonEncode(rawArtifact) != inventoryJson) {
      throw const FormatException(
        'authoring Story inventory build result is not canonical JSON',
      );
    }

    final generation = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        json['generation'],
        'Story inventory build generation',
      ),
    );
    final rawGeneration = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        rawPayload['generation'],
        'Story inventory raw generation',
      ),
    );
    if (!_authoringStoryCatalogSameGeneration(generation, rawGeneration)) {
      throw const FormatException(
        'authoring Story inventory generation disagrees with its artifact',
      );
    }

    final storyCatalogSeal = _authoringStoryCatalogSeal(
      json['story_catalog_seal'],
      'inventory story_catalog_seal',
    );
    final rawStoryCatalogSeal = _authoringStoryCatalogSeal(
      rawPayload['story_catalog_seal'],
      'raw inventory story_catalog_seal',
    );
    final sourcePairSeal = _authoringStoryCatalogSeal(
      json['source_pair_seal'],
      'inventory source_pair_seal',
    );
    final payloadSeal = _authoringStoryCatalogSeal(
      json['payload_seal'],
      'inventory payload_seal',
    );
    final rawPayloadSeal = _authoringStoryCatalogSeal(
      rawArtifact['payload_seal'],
      'raw inventory payload_seal',
    );
    if (!_authoringStoryCatalogSameSeal(
          storyCatalogSeal,
          rawStoryCatalogSeal,
        ) ||
        !_authoringStoryCatalogSameSeal(payloadSeal, rawPayloadSeal)) {
      throw const FormatException(
        'authoring Story inventory response seals disagree with its artifact',
      );
    }

    final source = _authoringRequiredObject(
      rawPayload['source'],
      'Story inventory source',
    );
    _authoringExactFields(source, const {
      'shipping_cache',
      'binds_cache',
      'source_pair_seal',
    }, 'Story inventory source');
    final sourceShipping = _authoringStoryCatalogSeal(
      source['shipping_cache'],
      'inventory source.shipping_cache',
    );
    final sourceBinds = _authoringStoryCatalogSeal(
      source['binds_cache'],
      'inventory source.binds_cache',
    );
    final rawSourcePair = _authoringStoryCatalogSeal(
      source['source_pair_seal'],
      'raw inventory source_pair_seal',
    );
    if (!_authoringStoryCatalogSameSeal(
          sourceShipping,
          generation.shippingCache,
        ) ||
        !_authoringStoryCatalogSameSeal(sourceBinds, generation.bindsCache) ||
        !_authoringStoryCatalogSameSeal(sourcePairSeal, rawSourcePair) ||
        sourcePairSeal.byteLength !=
            generation.shippingCache.byteLength +
                generation.bindsCache.byteLength) {
      throw const FormatException(
        'authoring Story inventory source provenance is inconsistent',
      );
    }

    final payloadBytes = utf8.encode(jsonEncode(rawPayload));
    if (payloadSeal.byteLength != payloadBytes.length ||
        payloadSeal.sha256 != crypto.sha256.convert(payloadBytes).toString()) {
      throw const FormatException(
        'authoring Story inventory payload seal is invalid',
      );
    }

    final catalogLayer = _authoringRequiredString(
      rawPayload,
      'catalog_layer',
      maxBytes: 128,
    );
    if (catalogLayer != _authoringStoryInventoryCatalogLayer ||
        json['catalog_layer'] != catalogLayer) {
      throw const FormatException(
        'authoring Story inventory layer is not the base-game inventory layer',
      );
    }
    final coverage = switch (rawPayload['coverage']) {
      'base_game_only' => AuthoringStoryInventoryCoverage.baseGameOnly,
      _ => throw const FormatException(
        'authoring Story inventory coverage is unsupported',
      ),
    };
    final runtimeQualification = switch (rawPayload['runtime_qualification']) {
      'runtime_unqualified' =>
        AuthoringStoryInventoryRuntimeQualification.runtimeUnqualified,
      _ => throw const FormatException(
        'authoring Story inventory runtime qualification is unsupported',
      ),
    };
    final publicationStatus = switch (rawPayload['publication_status']) {
      'not_supported' => AuthoringStoryInventoryPublicationStatus.notSupported,
      _ => throw const FormatException(
        'authoring Story inventory publication status is unsupported',
      ),
    };
    if (json['coverage'] != 'base_game_only' ||
        json['runtime_qualification'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'authoring Story inventory response overstates artifact capabilities',
      );
    }

    return AuthoringStoryInventoryBuildResult._(
      requestBindingSha256: requestBindingSha256,
      inventoryJson: inventoryJson,
      generation: generation,
      storyCatalogSeal: storyCatalogSeal,
      sourcePairSeal: sourcePairSeal,
      payloadSeal: payloadSeal,
      catalogLayer: catalogLayer,
      coverage: coverage,
      runtimeQualification: runtimeQualification,
      publicationStatus: publicationStatus,
      modules: modules,
      relativePaths: relativePaths,
      symbols: symbols,
    );
  }
}

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

final class _AuthoringStoryInventoryEntryLimits {
  int _count = 0;
  int _bytes = 0;

  List<String> decode(Object? raw, String context) {
    if (raw is! List ||
        raw.length > _maxAuthoringStoryInventoryEntries - _count) {
      throw FormatException(
        'authoring Story inventory $context exceeds its entry limit',
      );
    }
    final entries = <String>[];
    String? previous;
    for (var index = 0; index < raw.length; index++) {
      final value = raw[index];
      if (value is! String ||
          value.isEmpty ||
          value.length > _maxAuthoringStoryInventoryEntryBytes ||
          value.codeUnits.any(
            (unit) =>
                unit > 0x7f ||
                unit <= 0x1f ||
                unit == 0x7f ||
                (unit >= 0x41 && unit <= 0x5a),
          ) ||
          (previous != null && previous.compareTo(value) >= 0)) {
        throw FormatException(
          'authoring Story inventory $context entry $index is invalid',
        );
      }
      if (value.length > _maxAuthoringStoryInventoryTotalBytes - _bytes) {
        throw const FormatException(
          'authoring Story inventory entries exceed their aggregate byte limit',
        );
      }
      _bytes += value.length;
      previous = value;
      entries.add(value);
    }
    _count += entries.length;
    return List.unmodifiable(entries);
  }
}

const _authoringStoryBuildDiagnosticCodeOrder = <String>[
  'ENTITY_KEY_ID_MISMATCH',
  'REFERENCE_PROJECT_MISMATCH',
  'REFERENCE_DECLARED_KIND_MISMATCH',
  'MISSING_REFERENCE',
  'REFERENCE_TARGET_KIND_MISMATCH',
  'LOCALE_SLOT_MISMATCH',
  'SLOT_TAKE_LOCALE_MISMATCH',
  'DUPLICATE_VOICE_CANDIDATE',
  'MISSING_SELECTED_VOICE_TAKE',
  'SELECTED_VOICE_TAKE_NOT_CANDIDATE',
  'SELECTED_VOICE_TAKE_NOT_APPROVED',
  'DUPLICATE_VOICE_TARGET',
  'UNRESOLVED_VOICE_TARGET',
  'AMBIGUOUS_VOICE_TARGET',
  'INVALID_AMBIGUOUS_TARGET_CARDINALITY',
  'DUPLICATE_VOICE_TARGET_CANDIDATE',
  'LOCALE_NOT_AUTHORED',
  'MISSING_LOCALIZATION_VALUE',
  'INVALID_LOCALIZATION_ID',
  'INVALID_ASSET_METADATA',
  'MISSING_ASSET',
  'ASSET_SIZE_MISMATCH',
  'ASSET_MEDIA_TYPE_MISMATCH',
  'INVALID_ARCHIVE_SEAL',
  'INVALID_VOICE_TARGET',
  'MEMBER_PROOF_OPERATION_MISMATCH',
  'INVALID_MEMBER_PROOF',
  'UNQUALIFIED_VOICE_ADD',
  'INVALID_OGG_METADATA',
  'OPUS_DECODE_UNPROVEN',
  'INVALID_GENERATION_ANCHOR',
  'INVALID_ORIGIN',
  'ORIGIN_GENERATION_MISMATCH',
  'INVALID_GENERATOR_INPUT',
  'GENERATOR_CONTRACT_DRIFT',
  'GENERATED_SCRIPT_DRIFT',
  'SCRIPT_MODULE_OWNERSHIP_MISMATCH',
  'RUNTIME_UNQUALIFIED',
  'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'PROJECT_IDENTITY_MISMATCH',
  'PROJECT_REVISION_CONFLICT',
  'PROJECT_REVISION_OVERFLOW',
  'INVALID_STORY_MUTATION',
  'DUPLICATE_ENTITY_ID',
  'DUPLICATE_AUTHORED_RUNTIME_ID',
  'DUPLICATE_SCRIPT_MODULE_NAMESPACE',
  'DUPLICATE_SCRIPT_MODULE_PATH',
  'DUPLICATE_GENERATED_SYMBOL',
];

final _authoringStoryBuildDiagnosticCodeRanks = <String, int>{
  for (
    var index = 0;
    index < _authoringStoryBuildDiagnosticCodeOrder.length;
    index++
  )
    _authoringStoryBuildDiagnosticCodeOrder[index]: index,
};

const _authoringStoryBuildEntityKinds = <String>{
  'localization_entry',
  'dialog_line',
  'voice_slot',
  'voice_take',
  'npc_draft',
  'quest_draft',
  'script_module',
};

final class _AuthoringStoryBuildPlanValidation {
  const _AuthoringStoryBuildPlanValidation({
    required this.moduleCount,
    required this.diagnosticCount,
    required this.blockingDiagnosticIndexes,
  });

  final int moduleCount;
  final int diagnosticCount;
  final List<int> blockingDiagnosticIndexes;
}

final class _AuthoringStoryBuildPlanValidator {
  _AuthoringStoryBuildPlanValidator({
    required this.projectId,
    required this.projectRevision,
    required this.projectEntities,
    required this.targetExecutable,
  });

  final String projectId;
  final int projectRevision;
  final Map<String, Object?> projectEntities;
  final AuthoringDraftContentSeal targetExecutable;
  int _sourceBytes = 0;
  int _sealedInputCount = 0;
  int _relatedEntityCount = 0;

  _AuthoringStoryBuildPlanValidation validate(
    Object? rawModules,
    Object? rawDiagnostics,
  ) {
    if (rawModules is! List ||
        rawModules.length > _maxAuthoringStoryBuildModules) {
      throw const FormatException(
        'authoring Story build-plan modules exceed their bound',
      );
    }
    final ownerIds = <String>{};
    _StoryBuildModuleKey? previousModuleKey;
    for (var index = 0; index < rawModules.length; index++) {
      final module = _authoringRequiredObject(
        rawModules[index],
        'Story build module $index',
      );
      _authoringExactFields(module, const {
        'script_module',
        'draft_input',
        'persisted_source',
        'sealed_inputs',
        'generated',
      }, 'Story build module $index');
      final scriptRef = _typedRef(module['script_module'], 'script_module');
      if (scriptRef.kind != 'script_module') {
        throw const FormatException(
          'authoring Story build-plan ScriptModule reference is invalid',
        );
      }
      final draftInput = _sealedProperty(module['draft_input'], 'draft_input');
      final persisted = _sealedProperty(
        module['persisted_source'],
        'persisted_source',
      );
      final sealedInputs = module['sealed_inputs'];
      if (sealedInputs is! List ||
          sealedInputs.length > _maxAuthoringStoryBuildSealedInputsPerModule ||
          sealedInputs.length >
              _maxAuthoringStoryBuildSealedInputsTotal - _sealedInputCount) {
        throw const FormatException(
          'authoring Story build-plan sealed inputs exceed their bound',
        );
      }
      _sealedInputCount += sealedInputs.length;
      final sealedProperties =
          <
            ({
              _StoryBuildProvenance provenance,
              AuthoringDraftContentSeal content,
            })
          >[];
      for (var sealIndex = 0; sealIndex < sealedInputs.length; sealIndex++) {
        sealedProperties.add(
          _sealedProperty(sealedInputs[sealIndex], 'sealed_inputs[$sealIndex]'),
        );
      }

      final generated = _authoringRequiredObject(
        module['generated'],
        'Story build generated module',
      );
      _authoringExactFields(generated, const {
        'generator_id',
        'generator_version',
        'owner',
        'module_namespace',
        'module_relative_path',
        'source',
        'source_sha256',
        'input_fingerprint',
        'status',
      }, 'Story build generated module');
      final generatorId = _boundedText(
        generated['generator_id'],
        'generator_id',
        256,
      );
      final generatorVersion = _authoringRequiredInt(
        generated,
        'generator_version',
        min: 1,
        max: 0xffffffff,
      );
      final owner = _typedRef(generated['owner'], 'owner');
      if (owner.kind != 'npc_draft' && owner.kind != 'quest_draft') {
        throw const FormatException(
          'authoring Story build-plan owner is not an NPC/Quest draft',
        );
      }
      ownerIds.add(owner.id);
      final namespace = _boundedText(
        generated['module_namespace'],
        'module_namespace',
        512,
      );
      final relativePath = _boundedText(
        generated['module_relative_path'],
        'module_relative_path',
        2 * 1024,
      );
      final source = _boundedText(
        generated['source'],
        'source',
        _maxAuthoringStoryBuildSourceBytes,
      );
      final sourceLength = utf8.encode(source).length;
      if (sourceLength > _maxAuthoringStoryBuildSourceBytes - _sourceBytes) {
        throw const FormatException(
          'authoring Story build-plan sources exceed their aggregate bound',
        );
      }
      _sourceBytes += sourceLength;
      final sourceSha = generated['source_sha256'];
      final inputFingerprint = generated['input_fingerprint'];
      if (sourceSha is! String ||
          !_authoringSha256Pattern.hasMatch(sourceSha) ||
          sourceSha != crypto.sha256.convert(utf8.encode(source)).toString() ||
          inputFingerprint is! String ||
          !_authoringSha256Pattern.hasMatch(inputFingerprint)) {
        throw const FormatException(
          'authoring Story build-plan generated seals are invalid',
        );
      }
      final status = _authoringRequiredObject(
        generated['status'],
        'Story build module status',
      );
      _authoringExactFields(status, const {
        'authoring',
        'runtime',
      }, 'Story build module status');
      if (status['authoring'] != 'offline_draft' ||
          status['runtime'] != 'runtime_unqualified') {
        throw const FormatException(
          'authoring Story build-plan module overstates runtime qualification',
        );
      }
      if (persisted.content.byteLength != sourceLength ||
          persisted.content.sha256 != sourceSha ||
          draftInput.provenance.propertyPath != 'payload.data.input' ||
          persisted.provenance.propertyPath != 'payload.data.source' ||
          persisted.provenance.entityId != scriptRef.id ||
          persisted.provenance.entityKind != 'script_module' ||
          draftInput.provenance.scope != 'entity' ||
          draftInput.provenance.entityId != owner.id ||
          draftInput.provenance.entityKind != owner.kind) {
        throw const FormatException(
          'authoring Story build-plan source provenance is inconsistent',
        );
      }
      _requireSealedInputLocations(
        sealedProperties,
        ownerId: owner.id,
        ownerKind: owner.kind,
        entityRevision: draftInput.provenance.entityRevision,
      );
      _requireProjectModuleBinding(
        scriptRef: scriptRef,
        owner: owner,
        generatorId: generatorId,
        generatorVersion: generatorVersion,
        generated: generated,
        draftInput: draftInput,
        persistedSource: persisted,
        sealedInputs: sealedProperties,
      );
      final key = _StoryBuildModuleKey(
        relativePath: relativePath,
        namespace: namespace,
        ownerId: owner.id,
        scriptModuleId: scriptRef.id,
      );
      if (previousModuleKey != null &&
          _compareStoryBuildModuleKeys(previousModuleKey, key) >= 0) {
        throw const FormatException(
          'authoring Story build-plan modules are not canonical',
        );
      }
      previousModuleKey = key;
    }

    if (rawDiagnostics is! List ||
        rawDiagnostics.length > _maxAuthoringStoryBuildDiagnostics) {
      throw const FormatException(
        'authoring Story build-plan diagnostics exceed their bound',
      );
    }
    final blockers = <int>[];
    final runtimeBlockers = <String>{};
    final causalBlockers = <String>{};
    var combinedBlocker = false;
    _StoryBuildDiagnosticKey? previousDiagnosticKey;
    for (var index = 0; index < rawDiagnostics.length; index++) {
      final diagnostic = _authoringRequiredObject(
        rawDiagnostics[index],
        'Story build diagnostic $index',
      );
      _exactOptionalFields(
        diagnostic,
        required: const {'code', 'severity', 'message', 'blocks_build'},
        optional: const {'entity', 'property_path', 'related_entities'},
        context: 'Story build diagnostic $index',
      );
      final code = diagnostic['code'];
      final severity = diagnostic['severity'];
      final codeRank = code is String
          ? _authoringStoryBuildDiagnosticCodeRanks[code]
          : null;
      if (code is! String ||
          codeRank == null ||
          severity is! String ||
          !const {'error', 'warning', 'info'}.contains(severity)) {
        throw const FormatException(
          'authoring Story build-plan diagnostic identity is invalid',
        );
      }
      final entity = diagnostic.containsKey('entity')
          ? _authoringStoryBuildId(diagnostic['entity'], 'diagnostic.entity')
          : null;
      final propertyPath = diagnostic.containsKey('property_path')
          ? _boundedText(
              diagnostic['property_path'],
              'property_path',
              _maxAuthoringStoryBuildPropertyPathBytes,
            )
          : null;
      final message = _boundedText(
        diagnostic['message'],
        'message',
        _maxAuthoringStoryBuildDiagnosticMessageBytes,
      );
      final hasRelated = diagnostic.containsKey('related_entities');
      final related = hasRelated
          ? diagnostic['related_entities']
          : const <Object?>[];
      if (related is! List ||
          (hasRelated && related.isEmpty) ||
          related.length > _maxAuthoringStoryBuildRelatedPerDiagnostic ||
          related.length >
              _maxAuthoringStoryBuildRelatedTotal - _relatedEntityCount) {
        throw const FormatException(
          'authoring Story build-plan related entities exceed their bound',
        );
      }
      _relatedEntityCount += related.length;
      final relatedIds = <String>[];
      String? previous;
      for (final raw in related) {
        final id = _authoringStoryBuildId(raw, 'related entity');
        if (previous != null && previous.compareTo(id) >= 0) {
          throw const FormatException(
            'authoring Story build-plan related entities are not canonical',
          );
        }
        previous = id;
        relatedIds.add(id);
      }
      final blocks = diagnostic['blocks_build'];
      if (blocks is! bool) {
        throw const FormatException(
          'authoring Story build-plan diagnostic gate is invalid',
        );
      }
      final diagnosticKey = _StoryBuildDiagnosticKey(
        severityRank: _authoringStoryBuildSeverityRank(severity),
        entity: entity,
        propertyPath: propertyPath,
        codeRank: codeRank,
        message: message,
        relatedEntities: List.unmodifiable(relatedIds),
        blocksBuild: blocks,
      );
      if (previousDiagnosticKey != null &&
          _compareStoryBuildDiagnosticKeys(
                previousDiagnosticKey,
                diagnosticKey,
              ) >=
              0) {
        throw const FormatException(
          'authoring Story build-plan diagnostics are not canonical and unique',
        );
      }
      previousDiagnosticKey = diagnosticKey;
      if (blocks) blockers.add(index);
      if (code == 'RUNTIME_UNQUALIFIED' &&
          severity == 'error' &&
          blocks &&
          entity != null) {
        runtimeBlockers.add(entity);
      }
      if (code != 'RUNTIME_UNQUALIFIED' && blocks && entity != null) {
        causalBlockers.add(entity);
      }
      if (code == 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE' &&
          severity == 'error' &&
          blocks &&
          entity == null &&
          propertyPath == 'schema_revision' &&
          message ==
              'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented' &&
          relatedIds.isEmpty) {
        combinedBlocker = true;
      }
    }
    if (blockers.isEmpty ||
        !combinedBlocker ||
        !ownerIds.every(runtimeBlockers.contains)) {
      throw const FormatException(
        'authoring Story build-plan blockers are incomplete',
      );
    }
    _requireOmittedDraftBlockers(ownerIds, causalBlockers);
    return _AuthoringStoryBuildPlanValidation(
      moduleCount: rawModules.length,
      diagnosticCount: rawDiagnostics.length,
      blockingDiagnosticIndexes: List.unmodifiable(blockers),
    );
  }

  void _requireOmittedDraftBlockers(
    Set<String> plannedOwnerIds,
    Set<String> causalBlockerEntityIds,
  ) {
    for (final entry in projectEntities.entries) {
      final id = _authoringStoryBuildId(entry.key, 'source entity map key');
      final rawEntity = _authoringRequiredObject(
        entry.value,
        'Story build source entity',
      );
      final payload = _authoringRequiredObject(
        rawEntity['payload'],
        'Story build source entity payload',
      );
      final kind = payload['kind'];
      if (kind != 'npc_draft' && kind != 'quest_draft') continue;

      final entity = _projectEntity(id, kind as String);
      _authoringExactFields(entity.data, const {
        'generator_id',
        'generator_version',
        'input',
        'script_module',
      }, 'Story build source draft');
      final scriptRef = _typedRef(
        entity.data['script_module'],
        'source draft script_module',
      );
      if (scriptRef.kind != 'script_module') {
        throw const FormatException(
          'authoring Story build-plan source draft ScriptModule is invalid',
        );
      }
      if (!plannedOwnerIds.contains(id) &&
          !causalBlockerEntityIds.contains(id) &&
          !causalBlockerEntityIds.contains(scriptRef.id)) {
        throw const FormatException(
          'authoring Story build-plan omitted a draft without a causal blocker',
        );
      }
    }
  }

  void _requireProjectModuleBinding({
    required ({String projectId, String id, String kind}) scriptRef,
    required ({String projectId, String id, String kind}) owner,
    required String generatorId,
    required int generatorVersion,
    required Map<String, Object?> generated,
    required ({
      _StoryBuildProvenance provenance,
      AuthoringDraftContentSeal content,
    })
    draftInput,
    required ({
      _StoryBuildProvenance provenance,
      AuthoringDraftContentSeal content,
    })
    persistedSource,
    required List<
      ({_StoryBuildProvenance provenance, AuthoringDraftContentSeal content})
    >
    sealedInputs,
  }) {
    final ownerEntity = _projectEntity(owner.id, owner.kind);
    final moduleEntity = _projectEntity(scriptRef.id, 'script_module');
    final ownerData = ownerEntity.data;
    _authoringExactFields(ownerData, const {
      'generator_id',
      'generator_version',
      'input',
      'script_module',
    }, 'Story build source draft');
    final sourceGeneratorId = _boundedText(
      ownerData['generator_id'],
      'source draft generator_id',
      256,
    );
    final sourceGeneratorVersion = _authoringRequiredInt(
      ownerData,
      'generator_version',
      min: 1,
      max: 0xffffffff,
    );
    final sourceScriptRef = _typedRef(
      ownerData['script_module'],
      'source draft script_module',
    );
    final sourceInput = _authoringRequiredObject(
      ownerData['input'],
      'Story build source draft input',
    );
    if (sourceGeneratorId != generatorId ||
        sourceGeneratorVersion != generatorVersion ||
        sourceScriptRef != scriptRef ||
        jsonEncode(moduleEntity.data) != jsonEncode(generated) ||
        draftInput.provenance.entityRevision != ownerEntity.revision ||
        persistedSource.provenance.entityRevision != moduleEntity.revision ||
        !_authoringStoryCatalogSameSeal(
          draftInput.content,
          _authoringStoryBuildBytesSeal(jsonEncode(sourceInput)),
        )) {
      throw const FormatException(
        'authoring Story build-plan module is not bound to project entities',
      );
    }

    final expectedInputs = _sourceSealedInputs(sourceInput, owner.kind);
    for (final input in sealedInputs) {
      final expected = expectedInputs[input.provenance.propertyPath];
      if (expected == null ||
          !_authoringStoryCatalogSameSeal(input.content, expected)) {
        throw const FormatException(
          'authoring Story build-plan sealed content disagrees with project entities',
        );
      }
    }
  }

  _StoryBuildProjectEntity _projectEntity(String id, String expectedKind) {
    if (!projectEntities.containsKey(id)) {
      throw const FormatException(
        'authoring Story build-plan module does not resolve in project entities',
      );
    }
    final entity = _authoringRequiredObject(
      projectEntities[id],
      'Story build source entity',
    );
    _authoringExactFields(entity, const {
      'id',
      'display_name',
      'origin',
      'revision',
      'payload',
    }, 'Story build source entity');
    final embeddedId = _authoringStoryBuildId(entity['id'], 'source entity.id');
    final revision = _authoringRequiredInt(
      entity,
      'revision',
      max: _maxAuthoringStoryAppliedRevision,
    );
    final payload = _authoringRequiredObject(
      entity['payload'],
      'Story build source entity payload',
    );
    _authoringExactFields(payload, const {
      'kind',
      'data',
    }, 'Story build source entity payload');
    if (embeddedId != id || payload['kind'] != expectedKind) {
      throw const FormatException(
        'authoring Story build-plan entity identity is inconsistent',
      );
    }
    return _StoryBuildProjectEntity(
      revision: revision,
      data: _authoringRequiredObject(
        payload['data'],
        'Story build source entity data',
      ),
    );
  }

  Map<String, AuthoringDraftContentSeal> _sourceSealedInputs(
    Map<String, Object?> sourceInput,
    String ownerKind,
  ) {
    final expected = <String, AuthoringDraftContentSeal>{
      'target.executable': targetExecutable,
    };
    final parents = ownerKind == 'npc_draft'
        ? const [
            'parent_character_definition',
            'parent_ai_agent_config',
            'parent_spawn_definition',
          ]
        : const ['parent_quest', 'giver', 'collision_catalog'];
    for (final parentName in parents) {
      final parent = _authoringRequiredObject(
        sourceInput[parentName],
        'Story build source input $parentName',
      );
      final generation = _authoringRequiredObject(
        parent['generation'],
        'Story build source input $parentName generation',
      );
      expected['payload.data.input.$parentName.generation.executable'] =
          _authoringStoryBuildSeal(
            generation['executable'],
            'source input $parentName executable',
          );
      expected['payload.data.input.$parentName.source_seal'] =
          _authoringStoryBuildSeal(
            parent['source_seal'],
            'source input $parentName source seal',
          );
    }
    return expected;
  }

  ({String projectId, String id, String kind}) _typedRef(
    Object? raw,
    String context,
  ) {
    final object = _authoringRequiredObject(raw, 'Story build $context ref');
    _authoringExactFields(object, const {
      'project_id',
      'id',
      'expected_kind',
    }, 'Story build $context ref');
    final refProject = _authoringStoryBuildId(
      object['project_id'],
      '$context.project_id',
    );
    final id = _authoringStoryBuildId(object['id'], '$context.id');
    final kind = object['expected_kind'];
    if (refProject != projectId ||
        kind is! String ||
        !_authoringStoryBuildEntityKinds.contains(kind)) {
      throw const FormatException(
        'authoring Story build-plan reference is inconsistent',
      );
    }
    return (projectId: refProject, id: id, kind: kind);
  }

  ({_StoryBuildProvenance provenance, AuthoringDraftContentSeal content})
  _sealedProperty(Object? raw, String context) {
    final object = _authoringRequiredObject(raw, 'Story build $context');
    _authoringExactFields(object, const {
      'provenance',
      'content',
    }, 'Story build $context');
    return (
      provenance: _provenance(object['provenance'], context),
      content: _authoringStoryBuildSeal(object['content'], '$context.content'),
    );
  }

  _StoryBuildProvenance _provenance(Object? raw, String context) {
    final object = _authoringRequiredObject(raw, 'Story build provenance');
    final scope = object['scope'];
    if (scope == 'project') {
      _authoringExactFields(object, const {
        'scope',
        'project_id',
        'project_revision',
        'property_path',
      }, 'Story build project provenance');
    } else if (scope == 'entity') {
      _authoringExactFields(object, const {
        'scope',
        'project_id',
        'project_revision',
        'entity_id',
        'entity_revision',
        'entity_kind',
        'property_path',
      }, 'Story build entity provenance');
    } else {
      throw const FormatException(
        'authoring Story build-plan provenance scope is invalid',
      );
    }
    if (_authoringStoryBuildId(object['project_id'], 'provenance.project_id') !=
            projectId ||
        _authoringRequiredInt(
              object,
              'project_revision',
              max: _maxAuthoringStoryAppliedRevision,
            ) !=
            projectRevision) {
      throw const FormatException(
        'authoring Story build-plan provenance targets another project',
      );
    }
    final entityId = scope == 'entity'
        ? _authoringStoryBuildId(object['entity_id'], 'provenance.entity_id')
        : null;
    final entityKind = scope == 'entity' ? object['entity_kind'] : null;
    if (scope == 'entity' &&
        (entityKind is! String ||
            !_authoringStoryBuildEntityKinds.contains(entityKind))) {
      throw const FormatException(
        'authoring Story build-plan provenance entity kind is invalid',
      );
    }
    final entityRevision = scope == 'entity'
        ? _authoringRequiredInt(
            object,
            'entity_revision',
            max: _maxAuthoringStoryAppliedRevision,
          )
        : null;
    final path = _boundedText(
      object['property_path'],
      '$context.property_path',
      _maxAuthoringStoryBuildPropertyPathBytes,
    );
    return _StoryBuildProvenance(
      scope: scope as String,
      propertyPath: path,
      entityId: entityId,
      entityKind: entityKind as String?,
      entityRevision: entityRevision,
    );
  }

  void _requireSealedInputLocations(
    List<
      ({_StoryBuildProvenance provenance, AuthoringDraftContentSeal content})
    >
    actual, {
    required String ownerId,
    required String ownerKind,
    required int? entityRevision,
  }) {
    final paths = ownerKind == 'npc_draft'
        ? <String>[
            'payload.data.input.parent_character_definition.generation.executable',
            'payload.data.input.parent_character_definition.source_seal',
            'payload.data.input.parent_ai_agent_config.generation.executable',
            'payload.data.input.parent_ai_agent_config.source_seal',
            'payload.data.input.parent_spawn_definition.generation.executable',
            'payload.data.input.parent_spawn_definition.source_seal',
          ]
        : <String>[
            'payload.data.input.parent_quest.generation.executable',
            'payload.data.input.parent_quest.source_seal',
            'payload.data.input.giver.generation.executable',
            'payload.data.input.giver.source_seal',
            'payload.data.input.collision_catalog.generation.executable',
            'payload.data.input.collision_catalog.source_seal',
          ];
    paths.sort();
    if (actual.length != 7 ||
        actual.first.provenance.scope != 'project' ||
        actual.first.provenance.propertyPath != 'target.executable') {
      throw const FormatException(
        'authoring Story build-plan sealed provenance is incomplete',
      );
    }
    for (var index = 0; index < paths.length; index++) {
      final provenance = actual[index + 1].provenance;
      if (provenance.scope != 'entity' ||
          provenance.entityId != ownerId ||
          provenance.entityKind != ownerKind ||
          provenance.entityRevision != entityRevision ||
          provenance.propertyPath != paths[index]) {
        throw const FormatException(
          'authoring Story build-plan sealed provenance is incomplete',
        );
      }
    }
  }
}

final class _StoryBuildProjectEntity {
  const _StoryBuildProjectEntity({required this.revision, required this.data});

  final int revision;
  final Map<String, Object?> data;
}

final class _StoryBuildModuleKey {
  const _StoryBuildModuleKey({
    required this.relativePath,
    required this.namespace,
    required this.ownerId,
    required this.scriptModuleId,
  });

  final String relativePath;
  final String namespace;
  final String ownerId;
  final String scriptModuleId;
}

int _compareStoryBuildModuleKeys(
  _StoryBuildModuleKey left,
  _StoryBuildModuleKey right,
) {
  for (final comparison in <int>[
    _compareStoryBuildText(left.relativePath, right.relativePath),
    _compareStoryBuildText(left.namespace, right.namespace),
    _compareStoryBuildText(left.ownerId, right.ownerId),
    _compareStoryBuildText(left.scriptModuleId, right.scriptModuleId),
  ]) {
    if (comparison != 0) return comparison;
  }
  return 0;
}

final class _StoryBuildDiagnosticKey {
  const _StoryBuildDiagnosticKey({
    required this.severityRank,
    required this.entity,
    required this.propertyPath,
    required this.codeRank,
    required this.message,
    required this.relatedEntities,
    required this.blocksBuild,
  });

  final int severityRank;
  final String? entity;
  final String? propertyPath;
  final int codeRank;
  final String message;
  final List<String> relatedEntities;
  final bool blocksBuild;
}

int _authoringStoryBuildSeverityRank(String severity) => switch (severity) {
  'error' => 0,
  'warning' => 1,
  'info' => 2,
  _ => throw const FormatException(
    'authoring Story build-plan diagnostic severity is invalid',
  ),
};

int _compareStoryBuildDiagnosticKeys(
  _StoryBuildDiagnosticKey left,
  _StoryBuildDiagnosticKey right,
) {
  var comparison = left.severityRank.compareTo(right.severityRank);
  if (comparison != 0) return comparison;
  comparison = _compareStoryBuildOptionalText(left.entity, right.entity);
  if (comparison != 0) return comparison;
  comparison = _compareStoryBuildOptionalText(
    left.propertyPath,
    right.propertyPath,
  );
  if (comparison != 0) return comparison;
  comparison = left.codeRank.compareTo(right.codeRank);
  if (comparison != 0) return comparison;
  comparison = _compareStoryBuildText(left.message, right.message);
  if (comparison != 0) return comparison;
  comparison = _compareStoryBuildTextLists(
    left.relatedEntities,
    right.relatedEntities,
  );
  if (comparison != 0) return comparison;
  return (left.blocksBuild ? 1 : 0).compareTo(right.blocksBuild ? 1 : 0);
}

int _compareStoryBuildOptionalText(String? left, String? right) {
  if (left == null) return right == null ? 0 : -1;
  if (right == null) return 1;
  return _compareStoryBuildText(left, right);
}

int _compareStoryBuildTextLists(List<String> left, List<String> right) {
  final sharedLength = left.length < right.length ? left.length : right.length;
  for (var index = 0; index < sharedLength; index++) {
    final comparison = _compareStoryBuildText(left[index], right[index]);
    if (comparison != 0) return comparison;
  }
  return left.length.compareTo(right.length);
}

int _compareStoryBuildText(String left, String right) {
  final leftBytes = utf8.encode(left);
  final rightBytes = utf8.encode(right);
  final sharedLength = leftBytes.length < rightBytes.length
      ? leftBytes.length
      : rightBytes.length;
  for (var index = 0; index < sharedLength; index++) {
    final comparison = leftBytes[index].compareTo(rightBytes[index]);
    if (comparison != 0) return comparison;
  }
  return leftBytes.length.compareTo(rightBytes.length);
}

final class _StoryBuildProvenance {
  const _StoryBuildProvenance({
    required this.scope,
    required this.propertyPath,
    required this.entityId,
    required this.entityKind,
    required this.entityRevision,
  });
  final String scope;
  final String propertyPath;
  final String? entityId;
  final String? entityKind;
  final int? entityRevision;
}

void _exactOptionalFields(
  Map<String, Object?> json, {
  required Set<String> required,
  required Set<String> optional,
  required String context,
}) {
  if (!required.every(json.containsKey) ||
      json.keys.any(
        (key) => !required.contains(key) && !optional.contains(key),
      )) {
    throw FormatException('authoring $context has an invalid schema');
  }
}

String _boundedText(Object? raw, String field, int maxBytes) {
  if (raw is! String || raw.isEmpty || utf8.encode(raw).length > maxBytes) {
    throw FormatException('authoring Story build-plan $field is invalid');
  }
  return raw;
}

String _authoringStoryBuildId(Object? raw, String field) {
  if (raw is! String || !_authoringStoryBuildIdPattern.hasMatch(raw)) {
    throw FormatException('authoring Story build-plan $field is invalid');
  }
  return raw;
}

AuthoringDraftContentSeal _authoringStoryBuildSeal(
  Object? raw,
  String context,
) => _authoringStoryCatalogSeal(raw, 'Story build $context');

AuthoringDraftContentSeal _authoringStoryBuildBytesSeal(String value) {
  final bytes = utf8.encode(value);
  return AuthoringDraftContentSeal._(
    byteLength: bytes.length,
    sha256: crypto.sha256.convert(bytes).toString(),
  );
}

AuthoringStoryBuildProjectProvenance _authoringStoryBuildProject(
  Map<String, Object?> raw, {
  required String projectId,
  required int projectRevision,
  required AuthoringDraftContentSeal canonicalDocument,
  required AuthoringDraftContentSeal targetExecutable,
}) {
  _authoringExactFields(raw, const {
    'project_id',
    'project_revision',
    'canonical_document',
    'target_executable',
  }, 'Story build project provenance');
  final actual = AuthoringStoryBuildProjectProvenance._(
    projectId: _authoringStoryBuildId(raw['project_id'], 'project.project_id'),
    projectRevision: _authoringRequiredInt(
      raw,
      'project_revision',
      max: _maxAuthoringStoryAppliedRevision,
    ),
    canonicalDocument: _authoringStoryBuildSeal(
      raw['canonical_document'],
      'canonical document',
    ),
    targetExecutable: _authoringStoryBuildSeal(
      raw['target_executable'],
      'target executable',
    ),
  );
  if (actual.projectId != projectId ||
      actual.projectRevision != projectRevision ||
      !_authoringStoryCatalogSameSeal(
        actual.canonicalDocument,
        canonicalDocument,
      ) ||
      !_authoringStoryCatalogSameSeal(
        actual.targetExecutable,
        targetExecutable,
      )) {
    throw const FormatException(
      'authoring Story build-plan provenance disagrees with project_json',
    );
  }
  return actual;
}

bool _authoringStoryBuildSameProject(
  AuthoringStoryBuildProjectProvenance left,
  AuthoringStoryBuildProjectProvenance right,
) =>
    left.projectId == right.projectId &&
    left.projectRevision == right.projectRevision &&
    _authoringStoryCatalogSameSeal(
      left.canonicalDocument,
      right.canonicalDocument,
    ) &&
    _authoringStoryCatalogSameSeal(
      left.targetExecutable,
      right.targetExecutable,
    );

List<int> _authoringStoryBuildIndexes(Object? raw, int diagnosticCount) {
  if (raw is! List || raw.isEmpty || raw.length > diagnosticCount) {
    throw const FormatException(
      'authoring Story build-plan blocker indexes are invalid',
    );
  }
  final output = <int>[];
  var previous = -1;
  for (final value in raw) {
    if (value is! int || value <= previous || value >= diagnosticCount) {
      throw const FormatException(
        'authoring Story build-plan blocker indexes are invalid',
      );
    }
    output.add(value);
    previous = value;
  }
  return output;
}

bool _authoringIntListsEqual(List<int> left, List<int> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

AuthoringDraftContentSeal _authoringStoryCatalogSeal(
  Object? raw,
  String context,
) {
  final seal = AuthoringDraftContentSeal.fromJson(
    _authoringRequiredObject(raw, 'Story catalog $context'),
  );
  if (seal.byteLength > 0x7fffffffffffffff) {
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

String _authoringStoryBuildRequestBinding(String projectJson, String profile) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryBuildRequestBindingDomain));
  for (final value in <String>[projectJson, profile]) {
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

String _authoringStoryInventoryBuildBindingSha256(
  String executable,
  String shippingCache,
  String bindsCache,
) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryInventoryBuildBindingDomain));
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

sealed class AuthoringStoryDraftInsertResult {
  const AuthoringStoryDraftInsertResult();

  String get requestBindingSha256;

  factory AuthoringStoryDraftInsertResult.fromJson(
    Map<String, Object?> json, {
    required String projectJson,
    required String mutationJson,
    required AuthoringValidationProfile profile,
  }) {
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring Story Draft insert response is not ok',
      );
    }
    final expectedBinding = _authoringStoryRequestBindingSha256(
      projectJson,
      mutationJson,
      profile,
    );
    final actualBinding = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    if (!_authoringSha256Pattern.hasMatch(actualBinding) ||
        actualBinding != expectedBinding) {
      throw const FormatException(
        'authoring Story Draft response is not bound to its exact request',
      );
    }
    return switch (json['outcome']) {
      'applied' => AuthoringStoryDraftInsertApplied._fromJson(
        json,
        _authoringStoryRequestContext(projectJson, mutationJson),
        actualBinding,
      ),
      'rejected' => AuthoringStoryDraftInsertRejected._fromJson(
        json,
        actualBinding,
      ),
      _ => throw const FormatException(
        'authoring Story Draft insert outcome is not supported',
      ),
    };
  }
}

final class AuthoringStoryDraftInsertApplied
    extends AuthoringStoryDraftInsertResult {
  const AuthoringStoryDraftInsertApplied._({
    required this.requestBindingSha256,
    required this.projectJson,
    required this.revision,
    required this.draftId,
    required this.draftKind,
    required this.scriptModuleId,
    required this.diagnostics,
    required this.blocksBuild,
  });

  @override
  final String requestBindingSha256;
  final String projectJson;
  final int revision;
  final String draftId;
  final AuthoringStoryDraftKind draftKind;
  final String scriptModuleId;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringStoryDraftInsertApplied._fromJson(
    Map<String, Object?> json,
    _AuthoringStoryRequestContext context,
    String requestBindingSha256,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'request_binding_sha256',
      'project_json',
      'revision',
      'draft_id',
      'draft_kind',
      'script_module_id',
      'diagnostics',
      'blocks_build',
    }, 'Story Draft applied response');
    if (json['ok'] != true || json['outcome'] != 'applied') {
      throw const FormatException(
        'authoring Story Draft applied response has an invalid discriminator',
      );
    }
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final project = _authoringRequireCanonicalProjectJson(projectJson);
    if (project.schemaRevision != 2) {
      throw const FormatException(
        'authoring Story Draft candidate is not schema revision 2',
      );
    }
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    if (revision != context.baseRevision + 1 ||
        revision != project.revision ||
        project.projectId != context.projectId) {
      throw const FormatException(
        'authoring Story Draft candidate identity or revision disagrees with its base',
      );
    }
    final draftId = _authoringEntityId(
      _authoringRequiredString(json, 'draft_id', maxBytes: 32),
      'draft_id',
    );
    final scriptModuleId = _authoringEntityId(
      _authoringRequiredString(json, 'script_module_id', maxBytes: 32),
      'script_module_id',
    );
    if (draftId != context.draftId ||
        scriptModuleId != context.scriptModuleId) {
      throw const FormatException(
        'authoring Story Draft response entity IDs disagree with its request',
      );
    }
    final draftKind = switch (json['draft_kind']) {
      'npc_draft' => AuthoringStoryDraftKind.npcDraft,
      'quest_draft' => AuthoringStoryDraftKind.questDraft,
      _ => throw const FormatException(
        'authoring Story Draft response kind is not supported',
      ),
    };
    if (draftKind != context.draftKind) {
      throw const FormatException(
        'authoring Story Draft response kind disagrees with its request',
      );
    }
    _authoringRequireStoryCandidateOwnership(project.project, context);
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringRequireRevision2CombinedGate(
      blocksBuild,
      diagnostics,
      'Story Draft applied response',
    );
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringStoryDraftInsertApplied._(
      requestBindingSha256: requestBindingSha256,
      projectJson: projectJson,
      revision: revision,
      draftId: draftId,
      draftKind: draftKind,
      scriptModuleId: scriptModuleId,
      diagnostics: diagnostics,
      blocksBuild: blocksBuild,
    );
  }
}

final class AuthoringStoryDraftInsertRejected
    extends AuthoringStoryDraftInsertResult {
  const AuthoringStoryDraftInsertRejected._({
    required this.requestBindingSha256,
    required this.diagnostics,
  });

  @override
  final String requestBindingSha256;
  final List<AuthoringDiagnostic> diagnostics;

  factory AuthoringStoryDraftInsertRejected._fromJson(
    Map<String, Object?> json,
    String requestBindingSha256,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'request_binding_sha256',
      'diagnostics',
    }, 'Story Draft rejected response');
    if (json['ok'] != true || json['outcome'] != 'rejected') {
      throw const FormatException(
        'authoring Story Draft rejected response has an invalid discriminator',
      );
    }
    final diagnostics = _authoringDiagnostics(json);
    if (!diagnostics.any(
      (diagnostic) =>
          diagnostic.severity == AuthoringDiagnosticSeverity.error &&
          diagnostic.blocksBuild,
    )) {
      throw const FormatException(
        'authoring Story Draft rejection has no blocking error diagnostic',
      );
    }
    return AuthoringStoryDraftInsertRejected._(
      requestBindingSha256: requestBindingSha256,
      diagnostics: diagnostics,
    );
  }
}

final class _AuthoringStoryRequestContext {
  const _AuthoringStoryRequestContext({
    required this.baseProject,
    required this.projectId,
    required this.baseRevision,
    required this.draftId,
    required this.scriptModuleId,
    required this.displayName,
    required this.draftKind,
    required this.input,
    required this.moduleNamespace,
    required this.runtimeId,
    required this.generatorId,
    required this.generatorVersion,
  });

  final Map<String, Object?> baseProject;
  final String projectId;
  final int baseRevision;
  final String draftId;
  final String scriptModuleId;
  final String displayName;
  final AuthoringStoryDraftKind draftKind;
  final Map<String, Object?> input;
  final String moduleNamespace;
  final String runtimeId;
  final String generatorId;
  final int generatorVersion;
}

_AuthoringStoryRequestContext _authoringStoryRequestContext(
  String projectJson,
  String mutationJson,
) {
  final project = _authoringRequireCanonicalProjectJson(projectJson);
  if (project.schemaRevision != 2) {
    throw const FormatException(
      'authoring Story Draft base is not canonical schema revision 2',
    );
  }
  if (project.revision > _maxAuthoringStoryBaseRevision) {
    throw const FormatException(
      'authoring Story Draft base revision exceeds the signed wire contract',
    );
  }
  final mutation = _authoringDecodeDuplicateSafeObject(
    mutationJson,
    'Story Draft mutation',
  );
  _authoringExactFields(mutation, const {
    'expected_project_id',
    'expected_revision',
    'draft_id',
    'script_module_id',
    'display_name',
    'draft',
  }, 'Story Draft mutation');
  final expectedProjectId = _authoringEntityId(
    _authoringRequiredString(mutation, 'expected_project_id', maxBytes: 32),
    'expected_project_id',
  );
  final expectedRevision = _authoringRequiredInt(
    mutation,
    'expected_revision',
    max: _maxAuthoringStoryBaseRevision,
  );
  if (expectedProjectId != project.projectId ||
      expectedRevision != project.revision) {
    throw const FormatException(
      'authoring Story Draft mutation does not name its exact base project',
    );
  }
  final draftId = _authoringEntityId(
    _authoringRequiredString(mutation, 'draft_id', maxBytes: 32),
    'draft_id',
  );
  final scriptModuleId = _authoringEntityId(
    _authoringRequiredString(mutation, 'script_module_id', maxBytes: 32),
    'script_module_id',
  );
  if (draftId == '00000000000000000000000000000000' ||
      scriptModuleId == '00000000000000000000000000000000' ||
      draftId == scriptModuleId) {
    throw const FormatException(
      'authoring Story Draft mutation IDs must be distinct and non-zero',
    );
  }
  final displayName = _authoringRequiredString(
    mutation,
    'display_name',
    maxBytes: 256,
  );
  final draft = _authoringRequiredObject(
    mutation['draft'],
    'Story Draft mutation draft',
  );
  _authoringExactFields(draft, const {
    'kind',
    'input',
  }, 'Story Draft mutation draft');
  final input = _authoringRequiredObject(
    draft['input'],
    'Story Draft mutation input',
  );
  final (
    draftKind,
    moduleNamespace,
    runtimeId,
    generatorId,
    generatorVersion,
  ) = switch (draft['kind']) {
    'npc' => () {
      _authoringExactFields(input, const {
        'module_namespace',
        'unique_name',
        'parent_character_definition',
        'parent_ai_agent_config',
        'parent_spawn_definition',
      }, 'NPC Story Draft mutation input');
      final moduleNamespace = _authoringRequiredString(
        input,
        'module_namespace',
        maxBytes: 255,
      );
      _authoringDraftValidateModuleNamespace(moduleNamespace);
      return (
        AuthoringStoryDraftKind.npcDraft,
        moduleNamespace,
        _authoringRequiredString(input, 'unique_name', maxBytes: 128),
        'gore-authoring.logical-npc-clone-draft',
        1,
      );
    }(),
    'quest' => () {
      _authoringExactFields(input, const {
        'module_namespace',
        'technical_id',
        'text_helper',
        'parent_quest',
        'giver',
        'title',
        'description',
        'objective_title',
        'collision_catalog',
      }, 'Quest Story Draft mutation input');
      final moduleNamespace = _authoringRequiredString(
        input,
        'module_namespace',
        maxBytes: 255,
      );
      _authoringDraftValidateModuleNamespace(moduleNamespace);
      return (
        AuthoringStoryDraftKind.questDraft,
        moduleNamespace,
        _authoringRequiredString(input, 'technical_id', maxBytes: 128),
        'gore-authoring.draft-quest-skeleton',
        1,
      );
    }(),
    _ => throw const FormatException(
      'authoring Story Draft mutation kind is not supported',
    ),
  };
  return _AuthoringStoryRequestContext(
    baseProject: project.project,
    projectId: project.projectId,
    baseRevision: project.revision,
    draftId: draftId,
    scriptModuleId: scriptModuleId,
    displayName: displayName,
    draftKind: draftKind,
    input: input,
    moduleNamespace: moduleNamespace,
    runtimeId: runtimeId,
    generatorId: generatorId,
    generatorVersion: generatorVersion,
  );
}

void _authoringRequireStoryCandidateOwnership(
  Map<String, Object?> project,
  _AuthoringStoryRequestContext context,
) {
  _authoringRequireStoryCandidatePreservesBase(project, context);
  final entities = project['entities'] as Map<dynamic, dynamic>;
  final draftEntity = _authoringStoryCandidateEntity(
    entities,
    context.draftId,
    context.draftKind.wireName,
    'Draft',
  );
  if (draftEntity['display_name'] != context.displayName ||
      draftEntity['revision'] != 0) {
    throw const FormatException(
      'authoring Story Draft candidate Draft metadata disagrees with the request',
    );
  }
  final draftOrigin = _authoringRequiredObject(
    draftEntity['origin'],
    'Story Draft candidate Draft origin',
  );
  _authoringExactFields(draftOrigin, const {
    'type',
    'authored_runtime_id',
  }, 'Story Draft candidate Draft origin');
  if (draftOrigin['type'] != 'new' ||
      draftOrigin['authored_runtime_id'] != context.runtimeId) {
    throw const FormatException(
      'authoring Story Draft candidate Draft origin disagrees with the request',
    );
  }
  final draftPayload = _authoringRequiredObject(
    draftEntity['payload'],
    'Story Draft candidate Draft payload',
  );
  final draftData = _authoringRequiredObject(
    draftPayload['data'],
    'Story Draft candidate Draft data',
  );
  _authoringExactFields(draftData, const {
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  }, 'Story Draft candidate Draft data');
  if (draftData['generator_id'] != context.generatorId ||
      draftData['generator_version'] != context.generatorVersion) {
    throw const FormatException(
      'authoring Story Draft candidate Draft generator disagrees with the request',
    );
  }
  final expectedInput = <String, Object?>{
    'target': context.baseProject['target'],
    if (context.draftKind == AuthoringStoryDraftKind.questDraft)
      'quest_id': context.draftId,
    ...context.input,
  };
  if (!_authoringJsonDeepEquals(draftData['input'], expectedInput)) {
    throw const FormatException(
      'authoring Story Draft candidate Draft input disagrees with the request',
    );
  }
  _authoringRequireTypedStoryRef(
    draftData['script_module'],
    projectId: context.projectId,
    id: context.scriptModuleId,
    kind: 'script_module',
    context: 'Draft ScriptModule reference',
  );

  final moduleEntity = _authoringStoryCandidateEntity(
    entities,
    context.scriptModuleId,
    'script_module',
    'ScriptModule',
  );
  if (moduleEntity['display_name'] != context.moduleNamespace ||
      moduleEntity['revision'] != 0) {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule metadata is invalid',
    );
  }
  final moduleOrigin = _authoringRequiredObject(
    moduleEntity['origin'],
    'Story Draft candidate ScriptModule origin',
  );
  _authoringExactFields(moduleOrigin, const {
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'Story Draft candidate ScriptModule origin');
  if (moduleOrigin['type'] != 'generated' ||
      moduleOrigin['generator_id'] != context.generatorId ||
      moduleOrigin['generator_version'] != context.generatorVersion) {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule origin generator is invalid',
    );
  }
  _authoringRequireTypedStoryRef(
    moduleOrigin['owner'],
    projectId: context.projectId,
    id: context.draftId,
    kind: context.draftKind.wireName,
    context: 'ScriptModule origin owner',
  );
  final modulePayload = _authoringRequiredObject(
    moduleEntity['payload'],
    'Story Draft candidate ScriptModule payload',
  );
  final moduleData = _authoringRequiredObject(
    modulePayload['data'],
    'Story Draft candidate ScriptModule data',
  );
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
  }, 'Story Draft candidate ScriptModule data');
  if (moduleData['generator_id'] != context.generatorId ||
      moduleData['generator_version'] != context.generatorVersion) {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule payload generator is invalid',
    );
  }
  if (moduleData['module_namespace'] != context.moduleNamespace ||
      moduleData['module_relative_path'] !=
          '${context.moduleNamespace.replaceAll('.', '/')}.as') {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule path is inconsistent',
    );
  }
  final source = _authoringRequiredString(
    moduleData,
    'source',
    maxBytes: _maxAuthoringDraftSourceBytes,
  );
  _authoringDraftVerifiedSourceSha256(moduleData, source);
  _authoringDraftSha256(moduleData, 'input_fingerprint');
  final status = _authoringRequiredObject(
    moduleData['status'],
    'Story Draft candidate ScriptModule status',
  );
  _authoringExactFields(status, const {
    'authoring',
    'runtime',
  }, 'Story Draft candidate ScriptModule status');
  if (status['authoring'] != 'offline_draft' ||
      status['runtime'] != 'runtime_unqualified') {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule status is invalid',
    );
  }
  _authoringRequireTypedStoryRef(
    moduleData['owner'],
    projectId: context.projectId,
    id: context.draftId,
    kind: context.draftKind.wireName,
    context: 'ScriptModule payload owner',
  );
}

void _authoringRequireStoryCandidatePreservesBase(
  Map<String, Object?> candidate,
  _AuthoringStoryRequestContext context,
) {
  for (final field in const <String>[
    'meta',
    'target',
    'authoring_locales',
    'asset_store',
  ]) {
    if (!_authoringJsonDeepEquals(
      candidate[field],
      context.baseProject[field],
    )) {
      throw FormatException(
        'authoring Story Draft candidate changed base field $field',
      );
    }
  }

  final baseEntities = context.baseProject['entities'];
  final candidateEntities = candidate['entities'];
  if (baseEntities is! Map || candidateEntities is! Map) {
    throw const FormatException(
      'authoring Story Draft base or candidate entities are not an object',
    );
  }
  if (baseEntities.containsKey(context.draftId) ||
      baseEntities.containsKey(context.scriptModuleId) ||
      candidateEntities.length != baseEntities.length + 2) {
    throw const FormatException(
      'authoring Story Draft candidate entity delta is not exactly two additions',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key is! String ||
        !candidateEntities.containsKey(entry.key) ||
        !_authoringJsonDeepEquals(candidateEntities[entry.key], entry.value)) {
      throw const FormatException(
        'authoring Story Draft candidate changed a preexisting entity',
      );
    }
  }
  for (final key in candidateEntities.keys) {
    if (key is! String ||
        (!baseEntities.containsKey(key) &&
            key != context.draftId &&
            key != context.scriptModuleId)) {
      throw const FormatException(
        'authoring Story Draft candidate added an unexpected entity',
      );
    }
  }
}

bool _authoringJsonDeepEquals(Object? left, Object? right, [int depth = 0]) {
  if (depth > 128) {
    throw const FormatException(
      'authoring Story Draft JSON exceeds the maximum nesting depth',
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

Map<String, Object?> _authoringStoryCandidateEntity(
  Map<dynamic, dynamic> entities,
  String id,
  String expectedKind,
  String context,
) {
  final entity = _authoringRequiredObject(
    entities[id],
    'Story Draft candidate $context entity',
  );
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'Story Draft candidate $context entity');
  if (entity['id'] != id) {
    throw FormatException(
      'authoring Story Draft candidate $context key and ID disagree',
    );
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'Story Draft candidate $context payload',
  );
  _authoringExactFields(payload, const {
    'kind',
    'data',
  }, 'Story Draft candidate $context payload');
  if (payload['kind'] != expectedKind) {
    throw FormatException(
      'authoring Story Draft candidate $context kind disagrees with the response',
    );
  }
  return entity;
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

class AuthoringDiagnostic {
  const AuthoringDiagnostic._({
    required this.code,
    required this.severity,
    required this.entity,
    required this.propertyPath,
    required this.message,
    required this.relatedEntities,
    required this.blocksBuild,
  });

  final String code;
  final AuthoringDiagnosticSeverity severity;
  final String? entity;
  final String? propertyPath;
  final String message;
  final List<String> relatedEntities;
  final bool blocksBuild;

  factory AuthoringDiagnostic.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'code',
      'severity',
      'entity',
      'property_path',
      'message',
      'related_entities',
      'blocks_build',
    }, 'diagnostic');
    final code = _authoringRequiredString(json, 'code', maxBytes: 128);
    if (!_authoringDiagnosticCodePattern.hasMatch(code)) {
      throw const FormatException('authoring diagnostic code is not canonical');
    }
    final severity = switch (json['severity']) {
      'error' => AuthoringDiagnosticSeverity.error,
      'warning' => AuthoringDiagnosticSeverity.warning,
      'info' => AuthoringDiagnosticSeverity.info,
      final value => throw FormatException(
        'unknown authoring diagnostic severity: $value',
      ),
    };
    final entityValue = _authoringRequiredNullableString(json, 'entity');
    final entity = entityValue == null
        ? null
        : _authoringEntityId(entityValue, 'entity');
    final propertyPath = _authoringRequiredNullableString(
      json,
      'property_path',
      maxBytes: _maxAuthoringDiagnosticPathBytes,
    );
    final message = _authoringRequiredString(
      json,
      'message',
      maxBytes: _maxAuthoringDiagnosticMessageBytes,
    );

    final rawRelatedEntities = json['related_entities'];
    if (rawRelatedEntities is! List ||
        rawRelatedEntities.length > _maxAuthoringRelatedEntities) {
      throw const FormatException(
        'authoring diagnostic related_entities is not an array',
      );
    }
    final relatedEntities = <String>[];
    for (var index = 0; index < rawRelatedEntities.length; index++) {
      final value = rawRelatedEntities[index];
      if (value is! String) {
        throw FormatException(
          'authoring diagnostic related entity at index $index is not a string',
        );
      }
      final id = _authoringEntityId(value, 'related_entities[$index]');
      if (relatedEntities.isNotEmpty &&
          relatedEntities.last.compareTo(id) >= 0) {
        throw const FormatException(
          'authoring diagnostic related entities are not canonical and unique',
        );
      }
      relatedEntities.add(id);
    }

    return AuthoringDiagnostic._(
      code: code,
      severity: severity,
      entity: entity,
      propertyPath: propertyPath,
      message: message,
      relatedEntities: List.unmodifiable(relatedEntities),
      blocksBuild: _authoringRequiredBool(json, 'blocks_build'),
    );
  }
}

List<AuthoringDiagnostic> _authoringDiagnostics(Map<String, Object?> json) {
  final rawDiagnostics = json['diagnostics'];
  if (rawDiagnostics is! List ||
      rawDiagnostics.length > _maxAuthoringDiagnostics) {
    throw const FormatException(
      'authoring response diagnostics is not a bounded array',
    );
  }
  final diagnostics = <AuthoringDiagnostic>[];
  for (var index = 0; index < rawDiagnostics.length; index++) {
    diagnostics.add(
      AuthoringDiagnostic.fromJson(
        _authoringRequiredObject(
          rawDiagnostics[index],
          'diagnostic at index $index',
        ),
      ),
    );
  }
  return List.unmodifiable(diagnostics);
}

void _authoringValidateBlocksBuild(
  bool blocksBuild,
  List<AuthoringDiagnostic> diagnostics,
) {
  if (blocksBuild != diagnostics.any((diagnostic) => diagnostic.blocksBuild)) {
    throw const FormatException(
      'authoring response blocks_build is inconsistent with diagnostics',
    );
  }
}

void _authoringRequireRevision2CombinedGate(
  bool blocksBuild,
  List<AuthoringDiagnostic> diagnostics,
  String context,
) {
  if (!blocksBuild ||
      !diagnostics.any(
        (diagnostic) =>
            diagnostic.code == 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE' &&
            diagnostic.severity == AuthoringDiagnosticSeverity.error &&
            diagnostic.entity == null &&
            diagnostic.propertyPath == 'schema_revision' &&
            diagnostic.blocksBuild,
      )) {
    throw FormatException(
      'authoring $context is missing its blocking combined-validation gate',
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
      max: _maxAuthoringProjectJsonBytes,
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

class AuthoringStoreOpenedResult {
  const AuthoringStoreOpenedResult._({
    required this.head,
    required this.projectJson,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringStoreOpenedResult.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
      'project_json',
      'diagnostics',
      'blocks_build',
    }, 'store-open response');
    if (json['ok'] != true) {
      throw const FormatException('authoring store-open response is not ok');
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final project = _authoringRequireCanonicalProjectJson(projectJson);
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    if (project.schemaRevision == 2) {
      _authoringRequireRevision2CombinedGate(
        blocksBuild,
        diagnostics,
        'revision-2 store response',
      );
    }
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringStoreOpenedResult._(
      head: head,
      projectJson: projectJson,
      diagnostics: diagnostics,
      blocksBuild: blocksBuild,
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
  if ((generatorVersion == _authoringRevision3QuestGeneratorVersion &&
          hasAdditionalObjectives) ||
      (generatorVersion ==
              _authoringRevision3MultiObjectiveQuestGeneratorVersion &&
          !hasAdditionalObjectives)) {
    throw const FormatException(
      'authoring revision-3 Quest objective shape disagrees with its generator version',
    );
  }
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
      (version != _authoringRevision3QuestGeneratorVersion &&
          version != _authoringRevision3MultiObjectiveQuestGeneratorVersion) ||
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

class AuthoringCheckpointPreparation {
  const AuthoringCheckpointPreparation._({
    required this.head,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final AuthoringWorkingHead head;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringCheckpointPreparation.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
      'diagnostics',
      'blocks_build',
    }, 'checkpoint-preparation response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring checkpoint-preparation response is not ok',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringCheckpointPreparation._(
      head: head,
      diagnostics: diagnostics,
      blocksBuild: blocksBuild,
    );
  }
}

class AuthoringAssetRef {
  const AuthoringAssetRef._({
    required this.sha256,
    required this.byteLength,
    required this.logicalName,
  });

  factory AuthoringAssetRef({
    required String sha256,
    required int byteLength,
    required String logicalName,
  }) => AuthoringAssetRef.fromJson({
    'sha256': sha256,
    'byte_len': byteLength,
    'logical_name': logicalName,
  });

  final String sha256;
  final int byteLength;
  final String logicalName;

  factory AuthoringAssetRef.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'sha256',
      'byte_len',
      'logical_name',
    }, 'asset reference');
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (!_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException('authoring asset SHA-256 is not canonical');
    }
    return AuthoringAssetRef._(
      sha256: sha256,
      byteLength: _authoringRequiredInt(
        json,
        'byte_len',
        min: 1,
        max: _maxAuthoringReferencedAssetBytes,
      ),
      logicalName: _authoringRequiredString(
        json,
        'logical_name',
        maxBytes: _maxAuthoringLogicalNameBytes,
      ),
    );
  }

  Map<String, Object?> toJson() => {
    'sha256': sha256,
    'byte_len': byteLength,
    'logical_name': logicalName,
  };
}

enum AuthoringOggCodec { vorbis, opus }

class AuthoringOggMetadata {
  const AuthoringOggMetadata._({
    required this.codec,
    required this.channels,
    required this.sampleRate,
    required this.pages,
    required this.logicalStreams,
  });

  final AuthoringOggCodec codec;
  final int channels;
  final int sampleRate;
  final int pages;
  final int logicalStreams;

  factory AuthoringOggMetadata.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'codec',
      'channels',
      'sample_rate',
      'pages',
      'logical_streams',
    }, 'Ogg metadata');
    final codec = switch (json['codec']) {
      'vorbis' => AuthoringOggCodec.vorbis,
      'opus' => AuthoringOggCodec.opus,
      _ => throw const FormatException('unknown authoring Ogg codec'),
    };
    return AuthoringOggMetadata._(
      codec: codec,
      channels: _authoringRequiredInt(json, 'channels', min: 1, max: 255),
      sampleRate: _authoringRequiredInt(
        json,
        'sample_rate',
        min: 1,
        max: 0xffffffff,
      ),
      pages: _authoringRequiredInt(json, 'pages', min: 1, max: 0xffffffff),
      logicalStreams: _authoringRequiredInt(
        json,
        'logical_streams',
        min: 1,
        max: 0xffffffff,
      ),
    );
  }
}

class AuthoringImportedOgg {
  const AuthoringImportedOgg._({
    required this.asset,
    required this.ogg,
    required this.deduplicated,
  });

  final AuthoringAssetRef asset;
  final AuthoringOggMetadata ogg;
  final bool deduplicated;

  factory AuthoringImportedOgg.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'asset',
      'ogg',
      'deduplicated',
    }, 'Ogg-import response');
    if (json['ok'] != true) {
      throw const FormatException('authoring Ogg-import response is not ok');
    }
    return AuthoringImportedOgg._(
      asset: AuthoringAssetRef.fromJson(
        _authoringRequiredObject(json['asset'], 'imported asset'),
      ),
      ogg: AuthoringOggMetadata.fromJson(
        _authoringRequiredObject(json['ogg'], 'imported Ogg metadata'),
      ),
      deduplicated: _authoringRequiredBool(json, 'deduplicated'),
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
